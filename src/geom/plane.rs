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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use glam::Vec3;

    #[test]
    fn default_plane_has_4_vertices_and_6_indices() {
        let mesh = plane(PlaneConfig::default());

        assert_eq!(mesh.positions.len(), 4);
        assert_eq!(mesh.attributes.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
    }

    #[test]
    fn default_plane_spans_unit_bounds() {
        let mesh = plane(PlaneConfig::default());

        let mut mins = [f32::INFINITY; 3];
        let mut maxs = [f32::NEG_INFINITY; 3];

        for vertex in &mesh.positions {
            for (axis, &value) in vertex.position.iter().enumerate() {
                mins[axis] = mins[axis].min(value);
                maxs[axis] = maxs[axis].max(value);
            }
        }

        // The plane lies on its local z = 0 plane, centred on the origin.
        let expected_mins = [-0.5, -0.5, 0.0];
        let expected_maxs = [0.5, 0.5, 0.0];

        for axis in 0..3 {
            assert_abs_diff_eq!(mins[axis], expected_mins[axis], epsilon = 1e-6);
            assert_abs_diff_eq!(maxs[axis], expected_maxs[axis], epsilon = 1e-6);
        }
    }

    #[test]
    fn uvs_are_upright_and_unmirrored() {
        // The engine's downward-growing texture v must put `v = 0` on the
        // plane's top edge (+Y in world space), and `u` must run
        // left-to-right as seen from the default viewpoint (which looks
        // along +Z, so screen-right is +X):
        //     u = 0.5 + x,  v = 0.5 - y
        let mesh = plane(PlaneConfig::default());

        for (vertex, attributes) in mesh.positions.iter().zip(&mesh.attributes) {
            let position = Vec3::from(vertex.position);

            assert_abs_diff_eq!(attributes.uv[0], 0.5 + position.x, epsilon = 1e-6);
            assert_abs_diff_eq!(attributes.uv[1], 0.5 - position.y, epsilon = 1e-6);
        }
    }

    #[test]
    fn uvs_cover_the_full_range() {
        let mesh = plane(PlaneConfig::default());

        let mut min_u = f32::INFINITY;
        let mut max_u = f32::NEG_INFINITY;
        let mut min_v = f32::INFINITY;
        let mut max_v = f32::NEG_INFINITY;

        for attributes in &mesh.attributes {
            min_u = min_u.min(attributes.uv[0]);
            max_u = max_u.max(attributes.uv[0]);
            min_v = min_v.min(attributes.uv[1]);
            max_v = max_v.max(attributes.uv[1]);
        }

        assert_abs_diff_eq!(min_u, 0.0, epsilon = 1e-6);
        assert_abs_diff_eq!(max_u, 1.0, epsilon = 1e-6);
        assert_abs_diff_eq!(min_v, 0.0, epsilon = 1e-6);
        assert_abs_diff_eq!(max_v, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn triangle_winding_faces_the_default_camera() {
        // The plane's normal is -Z and the engine's default viewpoint sits
        // at -Z looking along +Z, so every triangle's cross-product normal
        // must have a negative z: front-facing under D3D-style
        // clockwise-in-screen-space rasterizer state.
        let mesh = plane(PlaneConfig::default());

        for &[a, b, c] in mesh.indices.as_chunks::<3>().0 {
            let pa = Vec3::from(mesh.positions[usize::from(a)].position);
            let pb = Vec3::from(mesh.positions[usize::from(b)].position);
            let pc = Vec3::from(mesh.positions[usize::from(c)].position);

            let face_normal = (pb - pa).cross(pc - pa);
            assert!(face_normal.z < 0.0, "triangle {a} {b} {c} faces away");
        }
    }

    #[test]
    fn subdivided_plane_has_expected_element_counts() {
        let mesh = plane(PlaneConfig {
            width_segments: 2,
            height_segments: 3,
            ..PlaneConfig::default()
        });

        let expected_vertices = (2 + 1) * (3 + 1);
        let expected_indices = 2 * 3 * 6;

        assert_eq!(mesh.positions.len(), expected_vertices);
        assert_eq!(mesh.attributes.len(), expected_vertices);
        assert_eq!(mesh.indices.len(), expected_indices);
    }
}
