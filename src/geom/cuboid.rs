#[derive(Clone, Copy, Debug)]
pub struct CuboidConfig {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub width_segments: u16,
    pub height_segments: u16,
    pub depth_segments: u16,
}

impl Default for CuboidConfig {
    fn default() -> Self {
        Self {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
            width_segments: 1,
            height_segments: 1,
            depth_segments: 1,
        }
    }
}
