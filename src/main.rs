use sdl3::event::Event;
use sdl3::gpu::{ColorTargetInfo, Device, LoadOp, ShaderFormat, StoreOp};
use sdl3::keyboard::Keycode;
use sdl3::pixels::Color;

// Shaders written in `shaders/*.slang` are compiled to SPIR-V by `build.rs` and
// embedded here. Use e.g. `shaders::BASIC_VERT` / `shaders::BASIC_FRAG` (or
// `shaders::get("basic.vert")`) when creating a pipeline.
mod shaders;

/// # Errors
pub fn main() -> anyhow::Result<()> {
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

    let mut event_pump = sdl_context.event_pump()?;

    'main: loop {
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

        // Acquire command buffer
        let mut cmd_buf = device.acquire_command_buffer()?;

        if let Ok(Some(swapchain_texture)) = cmd_buf.acquire_swapchain_texture(&window) {
            let color_target = ColorTargetInfo::default()
                .with_texture(&swapchain_texture)
                .with_clear_color(Color::RGB(255, 0, 0))
                .with_load_op(LoadOp::CLEAR)
                .with_store_op(StoreOp::STORE);
            // Begin render pass
            let render_pass = device.begin_render_pass(&cmd_buf, &[color_target], None)?;

            // End render pass
            device.end_render_pass(render_pass);
        }
        cmd_buf.submit()?;
    }

    Ok(())
}
