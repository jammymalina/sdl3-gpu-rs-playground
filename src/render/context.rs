use sdl3::gpu::Device;

pub struct RenderContext {
    pub device: Device,
}

impl RenderContext {
    #[must_use]
    pub const fn new(device: Device) -> Self {
        Self { device }
    }
}
