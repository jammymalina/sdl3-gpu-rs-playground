use sdl3::pixels::Color;

/// # Panics
#[must_use]
pub fn to_color_from_f32(rgba: [f32; 4]) -> Color {
    let [r, g, b, a] =
        rgba.map(|channel| cast::u8((channel * 255.0).round().clamp(0.0, 255.0)).unwrap());
    Color::RGBA(r, g, b, a)
}
