use crate::mesh::{MeshData, VertexAttributes, VertexPosition};

#[derive(Clone, Copy, Debug)]
pub struct PlaneConfig {
    pub width: f32,
    pub height: f32,
    pub width_segments: u16,
    pub height_segments: u16,
}

impl Default for PlaneConfig {
    fn default() -> Self {
        Self {
            width: 1.0,
            height: 1.0,
            width_segments: 1,
            height_segments: 1,
        }
    }
}

#[must_use]
pub fn plane(config: PlaneConfig) -> MeshData {
    let width_half = config.width / 2.0;
    let height_half = config.height / 2.0;

    let grid_x = (config.width_segments, config.width_segments + 1);
    let grid_y = (config.height_segments, config.height_segments + 1);

    let segment_width = config.width / f32::from(config.width_segments);
    let segment_height = config.height / f32::from(config.height_segments);

    let num_vertices = (grid_x.1 * grid_y.1) as usize;
    let mut positions = Vec::with_capacity(num_vertices);
    let mut attributes = Vec::with_capacity(num_vertices);

    for iy in 0..grid_y.1 {
        let y = f32::mul_add(f32::from(iy), segment_height, -height_half);

        for ix in 0..grid_x.1 {
            let x = f32::mul_add(f32::from(ix), segment_width, -width_half);

            positions.push(VertexPosition::new(x, -y, 0.0));
            attributes.push(
                VertexAttributes::default()
                    .with_normal(0.0, 0.0, -1.0)
                    .with_uv(
                        f32::from(ix) / f32::from(grid_x.0),
                        f32::from(iy) / f32::from(grid_y.0),
                    ),
            );
        }
    }

    let mut indices = Vec::with_capacity(num_vertices * 6);

    for iy in 0..grid_y.0 {
        for ix in 0..grid_x.0 {
            let a = ix + grid_x.1 * iy;
            let b = ix + grid_x.1 * (iy + 1);
            let c = (ix + 1) + grid_x.1 * (iy + 1);
            let d = (ix + 1) + grid_x.1 * iy;

            indices.extend_from_slice(&[a, d, b, b, d, c]);
        }
    }

    MeshData {
        positions,
        attributes,
        indices,
    }
}
