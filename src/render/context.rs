use sdl3::{
    Sdl,
    gpu::{Device, TextureFormat},
    properties::{Properties, Setter},
    sys,
    video::Window,
};

use crate::render::resource_cache::ResourceCache;

pub const DEPTH_FORMAT: TextureFormat = TextureFormat::D32Float;

/// `VK_MAKE_API_VERSION(0, 1, 2, 0)`. Vulkan 1.2 is the first version that
/// allows SPIR-V 1.5 shaders.
const VULKAN_API_VERSION_1_2: u32 = (1 << 22) | (2 << 12);

fn create_device(debug_mode: bool) -> anyhow::Result<Device> {
    let props = Properties::new()?;
    let mut vulkan = sys::gpu::SDL_GPUVulkanOptions {
        vulkan_api_version: VULKAN_API_VERSION_1_2,
        ..Default::default()
    };

    props.set("SDL.gpu.device.create.shaders.spirv", true)?;
    props.set("SDL.gpu.device.create.debugmode", debug_mode)?;
    props.set(
        "SDL.gpu.device.create.vulkan.options",
        std::ptr::from_mut(&mut vulkan),
    )?;

    Ok(Device::new_with_properties(props)?)
}

pub struct RenderContextConfig {
    pub title: String,
    pub window_width: u32,
    pub window_height: u32,
}

pub struct RenderContext {
    // Fields drop in declaration order, so each one must come before anything it depends on:
    // cached resources -> device (releases the window claim and swapchain) -> window -> SDL.
    pub resources: ResourceCache,
    pub device: Device,
    pub swapchain_texture_format: TextureFormat,
    pub window: Window,
    pub sdl_context: Sdl,
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
            .resizable()
            .build()?;

        // Create GPU device & claim window for the GPU swapchain
        let device = create_device(true)?.with_window(&window)?;
        let swapchain_texture_format = device.get_swapchain_texture_format(&window);

        Ok(Self {
            resources: ResourceCache::default(),
            device,
            swapchain_texture_format,
            window,
            sdl_context,
        })
    }
}
