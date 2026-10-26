use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use sdl3::gpu::{
    ColorTargetDescription, CommandBuffer, CompareOp, DepthStencilState, FillMode,
    GraphicsPipeline, GraphicsPipelineTargetInfo, PrimitiveType, RasterizerState, RenderPass,
    ShaderFormat, ShaderStage, VertexAttribute, VertexBufferDescription, VertexElementFormat,
    VertexInputRate, VertexInputState,
};
use std::mem::{offset_of, size_of};

use crate::{
    mesh::{VertexAttributes, VertexPosition},
    render::{
        context::{DEPTH_FORMAT, RenderContext},
        resource_cache::ResourceId,
    },
    shaders,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct BasicColor {
    pub base_color: [f32; 4],
    pub ambient_color: [f32; 4],
}

struct FlatColorResources {
    pipeline: GraphicsPipeline,
}

impl FlatColorResources {
    /// # Errors
    fn new(context: &RenderContext) -> anyhow::Result<Self> {
        let vertex_buffer_descriptions = [
            // Slot 0: Positions
            VertexBufferDescription::new()
                .with_slot(0)
                .with_pitch(size_of::<VertexPosition>().try_into()?)
                .with_input_rate(VertexInputRate::Vertex),
            // Slot 1: Attributes
            VertexBufferDescription::new()
                .with_slot(1)
                .with_pitch(size_of::<VertexAttributes>().try_into()?)
                .with_input_rate(VertexInputRate::Vertex),
        ];
        let vertex_attributes = [
            // loc 0: position -> Slot 0, offset 0
            VertexAttribute::new()
                .with_location(0)
                .with_buffer_slot(0)
                .with_format(VertexElementFormat::Float3)
                .with_offset(offset_of!(VertexPosition, position).try_into()?),
            // loc 1: normal   -> Slot 1, offset 0
            VertexAttribute::new()
                .with_location(1)
                .with_buffer_slot(1)
                .with_format(VertexElementFormat::Float3)
                .with_offset(offset_of!(VertexAttributes, normal).try_into()?),
        ];

        let vertex_info = shaders::find_stage("flat_color", "vertex").with_context(|| {
            "`flat_color.vertex` missing from the manifest - is the \
                `[shader(\"vertex\")]` entry point declared in shaders/flat_color.slang?"
        })?;
        let fragment_info = shaders::find_stage("flat_color", "fragment").with_context(|| {
            "`flat_color.fragment` missing from the manifest - is the \
                `[shader(\"fragment\")]` entry point declared in shaders/flat_color.slang?"
        })?;

        let device = &context.device;
        let vert_shader = device
            .create_shader()
            .with_code(ShaderFormat::SPIRV, vertex_info.bytes, ShaderStage::Vertex)
            .with_entrypoint(&vertex_info.entrypoint_cstring())
            // scene (b0), object (b1), color (b2)
            .with_uniform_buffers(3)
            .build()?;
        let frag_shader = device
            .create_shader()
            .with_code(
                ShaderFormat::SPIRV,
                fragment_info.bytes,
                ShaderStage::Fragment,
            )
            .with_entrypoint(&fragment_info.entrypoint_cstring())
            .build()?;

        let color_target_descriptions =
            [ColorTargetDescription::new().with_format(context.swapchain_texture_format)];

        let pipeline = device
            .create_graphics_pipeline()
            .with_vertex_shader(&vert_shader)
            .with_fragment_shader(&frag_shader)
            .with_vertex_input_state(
                VertexInputState::new()
                    .with_vertex_buffer_descriptions(&vertex_buffer_descriptions)
                    .with_vertex_attributes(&vertex_attributes),
            )
            .with_primitive_type(PrimitiveType::TriangleList)
            .with_rasterizer_state(RasterizerState::new().with_fill_mode(FillMode::Fill))
            .with_depth_stencil_state(
                DepthStencilState::new()
                    .with_enable_depth_test(true)
                    .with_enable_depth_write(true)
                    .with_compare_op(CompareOp::Less),
            )
            .with_target_info(
                GraphicsPipelineTargetInfo::new()
                    .with_color_target_descriptions(&color_target_descriptions)
                    .with_has_depth_stencil_target(true)
                    .with_depth_stencil_format(DEPTH_FORMAT),
            )
            .build()?;

        Ok(Self { pipeline })
    }
}

pub enum Material {
    FlatColor { basic_color: BasicColor },
}

impl Material {
    /// # Errors
    pub fn bind(
        &self,
        context: &RenderContext,
        command_buffer: &CommandBuffer,
        render_pass: &RenderPass,
    ) -> anyhow::Result<()> {
        match self {
            Self::FlatColor { basic_color } => {
                // Built on first use, then shared by every draw using this material type.
                let resources = context
                    .resources
                    .get_or_try_init(ResourceId::MaterialFlatColor, || {
                        FlatColorResources::new(context)
                    })?;

                command_buffer.push_vertex_uniform_data(2, basic_color);
                render_pass.bind_graphics_pipeline(&resources.pipeline);
            }
        }

        Ok(())
    }
}
