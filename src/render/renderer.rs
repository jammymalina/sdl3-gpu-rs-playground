use std::cell::RefCell;

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4};
use sdl3::gpu::{
    BufferBinding, ColorTargetInfo, DepthStencilTargetInfo, IndexElementSize, LoadOp, SampleCount,
    StoreOp, Texture, TextureCreateInfo, TextureType, TextureUsage,
};

use crate::{
    assets::{MaterialRepository, MeshRepository},
    camera::Camera,
    color,
    material::Material,
    render::context::{DEPTH_FORMAT, RenderContext, RenderContextConfig},
    scene::Scene,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct SceneData {
    view_projection: [[f32; 4]; 4],
    camera_position: [f32; 3],
    _padding0: f32,
    light_direction: [f32; 4],
    light_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct ObjectData {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
}

impl ObjectData {
    fn from_model(model: Mat4) -> Self {
        let normal = Mat4::from_mat3(Mat3::from_mat4(model).inverse().transpose());
        Self {
            model: model.to_cols_array_2d(),
            normal: normal.to_cols_array_2d(),
        }
    }
}

pub struct Renderer {
    // Declared first so the depth texture drops before the device it belongs to.
    depth_texture: RefCell<Texture<'static>>,
    context: RenderContext,
}

impl Renderer {
    /// # Errors
    pub fn new(context_config: &RenderContextConfig) -> anyhow::Result<Self> {
        let context = RenderContext::new(context_config)?;
        let (width, height) = context.window.size_in_pixels();
        let depth_texture = RefCell::new(Self::create_depth_texture(&context, width, height)?);
        Ok(Self {
            depth_texture,
            context,
        })
    }

    /// # Errors
    pub fn render(
        &self,
        scene: &Scene,
        camera: &impl Camera,
        mesh_repo: &MeshRepository,
        material_repo: &MaterialRepository,
    ) -> anyhow::Result<()> {
        let draw_commands = scene.extract_draw_list();
        let main_light = scene.main_light();

        let mut command_buffer = self.context.device.acquire_command_buffer()?;

        let scene_data = SceneData {
            view_projection: camera.world_to_clip().to_cols_array_2d(),
            camera_position: camera.transform().position.to_array(),
            _padding0: 0.0,
            light_direction: main_light.direction_to_light().extend(0.0).to_array(),
            light_color: main_light.final_color().extend(1.0).to_array(),
        };
        command_buffer.push_vertex_uniform_data(0, &scene_data);

        let (color_targets, width, height) = {
            let Some(swapchain) =
                command_buffer.wait_and_acquire_swapchain_texture(&self.context.window)?
            else {
                command_buffer.cancel();
                return Ok(());
            };
            let color_targets = [ColorTargetInfo::default()
                .with_texture(&swapchain)
                .with_load_op(LoadOp::CLEAR)
                .with_store_op(StoreOp::STORE)
                .with_clear_color(color::to_color_from_f32(scene.clear_color()))];
            (color_targets, swapchain.width(), swapchain.height())
        };

        let mut depth_texture = self.depth_texture.borrow_mut();
        if depth_texture.width() != width || depth_texture.height() != height {
            *depth_texture = Self::create_depth_texture(&self.context, width, height)?;
        }

        let depth_target = DepthStencilTargetInfo::new()
            .with_texture(&mut depth_texture)
            .with_cycle(true)
            .with_clear_depth(1.0)
            .with_load_op(LoadOp::CLEAR)
            .with_store_op(StoreOp::DONT_CARE)
            .with_stencil_load_op(LoadOp::DONT_CARE)
            .with_stencil_store_op(StoreOp::DONT_CARE);

        let render_pass = self.context.device.begin_render_pass(
            &command_buffer,
            &color_targets,
            Some(&depth_target),
        )?;

        for command in &draw_commands {
            let mesh = mesh_repo
                .get(command.mesh_key)
                .context("draw command references a missing mesh")?;
            let material: &Material = material_repo
                .get(command.material_key)
                .context("draw command references a missing material")?;

            command_buffer
                .push_vertex_uniform_data(1, &ObjectData::from_model(command.model_matrix));
            material.bind(&self.context, &command_buffer, &render_pass)?;

            render_pass.bind_vertex_buffers(
                0,
                &[
                    BufferBinding::new().with_buffer(&mesh.position_buffer),
                    BufferBinding::new().with_buffer(&mesh.attribute_buffer),
                ],
            );
            render_pass.bind_index_buffer(
                &BufferBinding::new().with_buffer(&mesh.index_buffer),
                IndexElementSize::_16BIT,
            );
            render_pass.draw_indexed_primitives(mesh.index_count, 1, 0, 0, 0);
        }

        self.context.device.end_render_pass(render_pass);
        command_buffer.submit()?;

        Ok(())
    }

    #[must_use]
    pub const fn context(&self) -> &RenderContext {
        &self.context
    }

    fn create_depth_texture(
        context: &RenderContext,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Texture<'static>> {
        Ok(context.device.create_texture(
            TextureCreateInfo::new()
                .with_type(TextureType::_2D)
                .with_format(DEPTH_FORMAT)
                .with_usage(TextureUsage::DEPTH_STENCIL_TARGET)
                .with_width(width)
                .with_height(height)
                .with_layer_count_or_depth(1)
                .with_num_levels(1)
                .with_sample_count(SampleCount::NoMultiSampling),
        )?)
    }
}
