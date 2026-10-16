//! Example: the classic rainbow triangle, driven through the library.
//!
//! Demonstrates the lowest-level building block the library exposes — the
//! embedded Slang-compiled shaders (`shaders::BASIC_VERT` / `shaders::BASIC_FRAG`)
//! — wired into an SDL3 GPU pipeline, vertex buffer, and render loop.
//!
//! Run: `cargo run --example rainbow_triangle`
//! Exit: close the window or press `Esc`.

use std::mem::{offset_of, size_of};
use std::time::Instant;

use sdl3::event::Event;
use sdl3::gpu::{
    BufferBinding, BufferRegion, BufferUsageFlags, ColorTargetDescription, ColorTargetInfo, Device,
    FillMode, GraphicsPipelineTargetInfo, LoadOp, PrimitiveType, RasterizerState, ShaderFormat,
    ShaderStage, StoreOp, TransferBufferLocation, TransferBufferUsage, VertexAttribute,
    VertexBufferDescription, VertexElementFormat, VertexInputRate, VertexInputState,
};
use sdl3::keyboard::Keycode;
use sdl3::pixels::Color;

use sdl3_gpu_rs_playground::shaders;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

// "Vertex colors": each corner carries a different color and the GPU
// interpolates them across the triangle, giving the rainbow look.
const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    }, // Bottom-left: red
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    }, // Bottom-right: green
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    }, // Top-middle: blue
];

/// # Errors
///
/// Propagates SDL3 initialization, device/pipeline creation, and vertex
/// upload failures through `anyhow`.
#[allow(clippy::too_many_lines)]
pub fn main() -> anyhow::Result<()> {
    const MS_PER_UPDATE: u128 = 16;

    // Init SDL3
    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;

    // Create window
    let window = video_subsystem
        .window("Playground", 800, 600)
        .position_centered()
        .build()?;

    // Create GPU device & claim window for the GPU swapchain
    let shader_formats = ShaderFormat::SPIRV;
    let device = Device::new(shader_formats, true)?.with_window(&window)?;

    // Basic pipeline
    let basic_vert = device
        .create_shader()
        .with_code(
            ShaderFormat::SPIRV,
            shaders::BASIC_VERT,
            ShaderStage::Vertex,
        )
        .with_entrypoint(c"main")
        .build()?;
    let basic_frag = device
        .create_shader()
        .with_code(
            ShaderFormat::SPIRV,
            shaders::BASIC_FRAG,
            ShaderStage::Fragment,
        )
        .with_entrypoint(c"main")
        .build()?;

    let vertex_buffer_descriptions = [VertexBufferDescription::new()
        .with_slot(0)
        .with_pitch(size_of::<Vertex>().try_into()?)
        .with_input_rate(VertexInputRate::Vertex)
        .with_instance_step_rate(0)];
    let vertex_attributes = [
        VertexAttribute::new()
            .with_location(0)
            .with_buffer_slot(0)
            .with_format(VertexElementFormat::Float3)
            .with_offset(offset_of!(Vertex, position).try_into()?),
        VertexAttribute::new()
            .with_location(1)
            .with_buffer_slot(0)
            .with_format(VertexElementFormat::Float3)
            .with_offset(offset_of!(Vertex, color).try_into()?),
    ];

    let color_target_descriptions =
        [ColorTargetDescription::new().with_format(device.get_swapchain_texture_format(&window))];

    let basic_pipeline = device
        .create_graphics_pipeline()
        .with_vertex_shader(&basic_vert)
        .with_fragment_shader(&basic_frag)
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

    // Loading data into memory
    let vertices_size: u32 = (size_of::<Vertex>() * VERTICES.len()).try_into()?;
    let vertex_buffer = device
        .create_buffer()
        .with_usage(BufferUsageFlags::VERTEX)
        .with_size(vertices_size)
        .build()?;
    {
        let transfer_buffer = device
            .create_transfer_buffer()
            .with_usage(TransferBufferUsage::UPLOAD)
            .with_size(vertices_size)
            .build()?;
        let mut transfer_data = transfer_buffer.map::<Vertex>(&device, false);
        transfer_data.mem_mut().copy_from_slice(&VERTICES);
        transfer_data.unmap();

        let cmd_buf = device.acquire_command_buffer()?;
        let copy_pass = device.begin_copy_pass(&cmd_buf)?;

        let transfer_buf_location = TransferBufferLocation::new()
            .with_transfer_buffer(&transfer_buffer)
            .with_offset(0);
        let buf_region = BufferRegion::new()
            .with_buffer(&vertex_buffer)
            .with_offset(0)
            .with_size(vertices_size);
        copy_pass.upload_to_gpu_buffer(transfer_buf_location, buf_region, false);

        device.end_copy_pass(copy_pass);
        cmd_buf.submit()?;
    }

    // Game loop
    let mut event_pump = sdl_context.event_pump()?;

    let mut previous = Instant::now();
    let mut lag = 0;
    'main: loop {
        let current = Instant::now();
        let elapsed = current - previous;
        previous = current;
        lag += elapsed.as_millis();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | sdl3::event::Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => {}
            }
        }

        // Update scene
        while lag >= MS_PER_UPDATE {
            lag -= MS_PER_UPDATE;
        }

        // Acquire command buffer
        let mut cmd_buf = device.acquire_command_buffer()?;

        if let Ok(Some(swapchain_texture)) = cmd_buf.acquire_swapchain_texture(&window) {
            let color_target = ColorTargetInfo::default()
                .with_texture(&swapchain_texture)
                .with_clear_color(Color::RGB(0, 0, 0))
                .with_load_op(LoadOp::CLEAR)
                .with_store_op(StoreOp::STORE);
            // Begin render pass
            let render_pass = device.begin_render_pass(&cmd_buf, &[color_target], None)?;

            // Draw
            render_pass.bind_graphics_pipeline(&basic_pipeline);
            render_pass.bind_vertex_buffers(
                0,
                &[BufferBinding::new()
                    .with_buffer(&vertex_buffer)
                    .with_offset(0)],
            );
            render_pass.draw_primitives(VERTICES.len(), 1, 0, 0);

            // End render pass
            device.end_render_pass(render_pass);
        }
        cmd_buf.submit()?;
    }

    Ok(())
}
