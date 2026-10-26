use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use once_cell::sync::OnceCell;
use sdl3::gpu::{
    ColorTargetDescription, FillMode, GraphicsPipeline, GraphicsPipelineTargetInfo, PrimitiveType,
    RasterizerState, ShaderFormat, ShaderStage, VertexAttribute, VertexBufferDescription,
    VertexElementFormat, VertexInputRate, VertexInputState,
};
use std::mem::{offset_of, size_of};

use crate::{
    mesh::{VertexAttributes, VertexPosition},
    render::context::RenderContext,
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

pub enum Material {
    FlatColor { basic_color: BasicColor },
}

impl Material {
    /// # Errors
    pub fn bind(&self, context: &RenderContext) -> anyhow::Result<()> {
        match self {
            Self::FlatColor { basic_color } => {
                static FLAT_RESOURCES: OnceCell<FlatColorResources> = OnceCell::new();

                // Compiled once, shared by every draw rendered with this material.
                let resources =
                    FLAT_RESOURCES.get_or_try_init(|| Self::init_flat_color(context))?;

                // TODO: bind `basic_color` (vertex uniform slot 2) and switch
                // the command buffer over to `resources.pipeline`.
                let _ = resources;
                let _ = basic_color;
            }
        }

        Ok(())
    }

    fn init_flat_color(context: &RenderContext) -> anyhow::Result<FlatColorResources> {
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
            .with_target_info(
                GraphicsPipelineTargetInfo::new()
                    .with_color_target_descriptions(&color_target_descriptions),
            )
            .build()?;

        Ok(FlatColorResources { pipeline })
    }
}
