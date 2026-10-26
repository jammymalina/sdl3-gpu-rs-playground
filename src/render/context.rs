use sdl3::{
    Sdl,
    gpu::{Device, ShaderFormat, TextureFormat},
    video::Window,
};

use crate::render::resource_cache::ResourceCache;

pub struct RenderContextConfig {
    pub title: String,
    pub window_width: u32,
    pub window_height: u32,
}

pub struct RenderContext {
    // Declared first so cached GPU resources drop before the device they belong to.
    pub resources: ResourceCache,
    pub sdl_context: Sdl,
    pub window: Window,
    pub device: Device,
    pub swapchain_texture_format: TextureFormat,
    pub window_width: u32,
    pub window_height: u32,
    pub width: u32,
    pub height: u32,
}

impl RenderContext {
    /// # Errors
    pub fn new(config: &RenderContextConfig) -> anyhow::Result<Self> {
        // Init SDL3
        let sdl_context = sdl3::init()?;
        let video_subsystem = sdl_context.video()?;

        // Create window
        let window = video_subsystem
            .window(&config.title, config.window_width, config.window_height)
            .position_centered()
            .build()?;

        // Create GPU device & claim window for the GPU swapchain
        let shader_formats = ShaderFormat::SPIRV;
        let device = Device::new(shader_formats, true)?.with_window(&window)?;
        let swapchain_texture_format = device.get_swapchain_texture_format(&window);

        // Get actual dimensions of the window
        let (width, height) = window.size_in_pixels();

        Ok(Self {
            resources: ResourceCache::default(),
            sdl_context,
            window,
            device,
            swapchain_texture_format,
            window_width: config.window_width,
            window_height: config.window_height,
            width,
            height,
        })
    }
}
