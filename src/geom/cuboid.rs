use crate::mesh::{MeshData, VertexAttributes, VertexPosition};
use glam::Vec3;

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

#[derive(Clone, Copy)]
struct Plane {
    u_axis: Vec3,
    v_axis: Vec3,
    n_axis: Vec3,
    size_u: f32,
    size_v: f32,
    size_n: f32,
    grid_x: u16,
    grid_y: u16,
}

fn build_plane(plane: Plane, vertex_base: u16, mesh: &mut MeshData) {
    let grid_x = (plane.grid_x, plane.grid_x + 1);
    let grid_y = (plane.grid_y, plane.grid_y + 1);

    let segment_u = plane.size_u / f32::from(grid_x.0);
    let segment_v = plane.size_v / f32::from(grid_y.0);

    // Corner of the face at `ix = iy = 0`: half a size back along each
    // spanning axis and half a size out along the normal.
    let origin = plane.u_axis * (-plane.size_u * 0.5)
        + plane.v_axis * (-plane.size_v * 0.5)
        + plane.n_axis * (plane.size_n * 0.5);
    let du = plane.u_axis * segment_u;
    let dv = plane.v_axis * segment_v;
    let normal = plane.n_axis;

    // Generate vertices, normals and uvs.
    //
    // `u_axis`/`v_axis` are unit vectors, so the position is exactly the
    // bilinear evaluation `origin + du * ix + dv * iy`, with `origin`
    // already including the half-size offsets along all three axes.
    for iy in 0..grid_y.1 {
        let y = f32::from(iy) * segment_v;
        let v = f32::from(iy) / f32::from(grid_y.0);

        for ix in 0..grid_x.1 {
            let x = f32::from(ix) * segment_u;
            let position = origin + du * x + dv * y;

            mesh.positions
                .push(VertexPosition::new(position.x, position.y, position.z));
            mesh.attributes.push(
                VertexAttributes::default()
                    .with_normal(normal.x, normal.y, normal.z)
                    .with_uv(f32::from(ix) / f32::from(grid_x.0), v),
            );
        }
    }

    // Indices: two triangles (six indices) per segment.
    for iy in 0..grid_y.0 {
        for ix in 0..grid_x.0 {
            let a = vertex_base + ix + grid_x.1 * iy;
            let b = vertex_base + ix + grid_x.1 * (iy + 1);
            let c = vertex_base + (ix + 1) + grid_x.1 * (iy + 1);
            let d = vertex_base + (ix + 1) + grid_x.1 * iy;

            mesh.indices.extend_from_slice(&[a, d, b, b, d, c]);
        }
    }
}

#[must_use]
pub fn cuboid(config: CuboidConfig) -> MeshData {
    let c = &config;

    let planes = [
        Plane {
            u_axis: Vec3::Z,
            v_axis: -Vec3::Y,
            n_axis: Vec3::X,
            size_u: c.depth,
            size_v: c.height,
            size_n: c.width,
            grid_x: c.depth_segments,
            grid_y: c.height_segments,
        }, // px
        Plane {
            u_axis: -Vec3::Z,
            v_axis: -Vec3::Y,
            n_axis: -Vec3::X,
            size_u: c.depth,
            size_v: c.height,
            size_n: c.width,
            grid_x: c.depth_segments,
            grid_y: c.height_segments,
        }, // nx
        Plane {
            u_axis: Vec3::X,
            v_axis: Vec3::Z,
            n_axis: -Vec3::Y,
            size_u: c.width,
            size_v: c.depth,
            size_n: c.height,
            grid_x: c.width_segments,
            grid_y: c.depth_segments,
        }, // py
        Plane {
            u_axis: Vec3::X,
            v_axis: -Vec3::Z,
            n_axis: Vec3::Y,
            size_u: c.width,
            size_v: c.depth,
            size_n: c.height,
            grid_x: c.width_segments,
            grid_y: c.depth_segments,
        }, // ny
        Plane {
            u_axis: -Vec3::X,
            v_axis: -Vec3::Y,
            n_axis: Vec3::Z,
            size_u: c.width,
            size_v: c.height,
            size_n: c.depth,
            grid_x: c.width_segments,
            grid_y: c.height_segments,
        }, // pz
        Plane {
            u_axis: Vec3::X,
            v_axis: -Vec3::Y,
            n_axis: -Vec3::Z,
            size_u: c.width,
            size_v: c.height,
            size_n: c.depth,
            grid_x: c.width_segments,
            grid_y: c.height_segments,
        }, // nz
    ];

    let total_vertices: usize = planes
        .iter()
        .map(|plane| (usize::from(plane.grid_x) + 1) * (usize::from(plane.grid_y) + 1))
        .sum();

    let total_indices: usize = planes
        .iter()
        .map(|plane| usize::from(plane.grid_x) * usize::from(plane.grid_y) * 6)
        .sum();

    let mut mesh = MeshData {
        positions: Vec::with_capacity(total_vertices),
        attributes: Vec::with_capacity(total_vertices),
        indices: Vec::with_capacity(total_indices),
    };

    let mut vertex_base = 0_u16;

    for plane in planes {
        build_plane(plane, vertex_base, &mut mesh);
        vertex_base += (plane.grid_x + 1) * (plane.grid_y + 1);
    }

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn default_cuboid_has_24_vertices_and_36_indices() {
        let mesh = cuboid(CuboidConfig::default());

        assert_eq!(mesh.positions.len(), 24);
        assert_eq!(mesh.attributes.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn default_cuboid_spans_unit_cube() {
        let mesh = cuboid(CuboidConfig::default());

        let mut mins = [f32::INFINITY; 3];
        let mut maxs = [f32::NEG_INFINITY; 3];

        for vertex in &mesh.positions {
            for (axis, &value) in vertex.position.iter().enumerate() {
                mins[axis] = mins[axis].min(value);
                maxs[axis] = maxs[axis].max(value);
            }
        }

        for axis in 0..3 {
            assert_abs_diff_eq!(mins[axis], -0.5, epsilon = 1e-6);
            assert_abs_diff_eq!(maxs[axis], 0.5, epsilon = 1e-6);
        }
    }

    #[test]
    fn faces_lie_on_their_planes() {
        let mesh = cuboid(CuboidConfig::default());

        for (vertex, attributes) in mesh.positions.iter().zip(&mesh.attributes) {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(attributes.normal);

            // The vertex lies on its face plane, half a size out along the
            // normal.
            let offset = position.dot(normal);
            assert_abs_diff_eq!(offset, 0.5, epsilon = 1e-6);
        }
    }

    #[test]
    fn vertex_normals_are_outward_unit_axes() {
        let mesh = cuboid(CuboidConfig::default());

        for (vertex, attributes) in mesh.positions.iter().zip(&mesh.attributes) {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(attributes.normal);

            // Exactly one component is non-zero.
            let axis_count = [normal.x, normal.y, normal.z]
                .iter()
                .filter(|&&component| component != 0.0)
                .count();
            assert_eq!(axis_count, 1, "normal {normal:?} is not a unit axis");

            // The normal points away from the centre.
            assert!(
                normal.dot(position) > 0.0,
                "normal {normal:?} at {position:?} points inward"
            );
        }
    }

    #[test]
    fn triangle_winding_faces_outward() {
        let mesh = cuboid(CuboidConfig::default());

        for &[a, b, c] in mesh.indices.as_chunks::<3>().0 {
            let pa = Vec3::from(mesh.positions[usize::from(a)].position);
            let pb = Vec3::from(mesh.positions[usize::from(b)].position);
            let pc = Vec3::from(mesh.positions[usize::from(c)].position);

            let centre = (pa + pb + pc) / 3.0;
            let face_normal = (pb - pa).cross(pc - pa);

            assert!(
                face_normal.dot(centre) > 0.0,
                "triangle {a} {b} {c} winds inward"
            );
        }
    }

    #[test]
    fn uvs_cover_the_full_range() {
        let mesh = cuboid(CuboidConfig::default());

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
    fn side_face_uvs_are_upright_and_unmirrored() {
        let mesh = cuboid(CuboidConfig::default());

        for (vertex, attributes) in mesh.positions.iter().zip(&mesh.attributes) {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(attributes.normal);

            // Only the four vertical faces: their outward normal lies in the
            // x/z plane, so the screen-right direction when viewed from
            // outside is `r = (-n.z, 0, n.x)` and screen-up is +Y.
            if normal.y == 0.0 {
                let right = Vec3::new(-normal.z, 0.0, normal.x);

                assert_abs_diff_eq!(attributes.uv[0], 0.5 + position.dot(right), epsilon = 1e-6);
                assert_abs_diff_eq!(attributes.uv[1], 0.5 - position.y, epsilon = 1e-6);
            }
        }
    }

    #[test]
    fn minus_z_face_grid_matches_plane() {
        // The -Z face is geometrically equivalent to `crate::geom::plane`
        // (same facing, same grid), just offset half the depth along -Z.
        // Its uv layout must therefore be the same as the plane's: same
        // (x, y) per grid cell and identical uvs.
        let plane_mesh = crate::geom::plane::plane(crate::geom::plane::PlaneConfig::default());
        let mesh = cuboid(CuboidConfig::default());

        // 1x1x1 faces build 4 vertices each, so the last face spans 20..24.
        for (offset, plane_vertex) in plane_mesh.positions.iter().enumerate() {
            let cuboid_position = Vec3::from(mesh.positions[20 + offset].position);
            let expected = Vec3::new(
                plane_vertex.position[0],
                plane_vertex.position[1],
                plane_vertex.position[2] - 0.5,
            );

            assert_abs_diff_eq!(cuboid_position, expected, epsilon = 1e-6);

            let cuboid_attributes = mesh.attributes[20 + offset];
            let plane_attributes = plane_mesh.attributes[offset];

            assert_abs_diff_eq!(
                cuboid_attributes.uv[0],
                plane_attributes.uv[0],
                epsilon = 1e-6
            );
            assert_abs_diff_eq!(
                cuboid_attributes.uv[1],
                plane_attributes.uv[1],
                epsilon = 1e-6
            );
        }
    }

    #[test]
    fn subdivided_cuboid_has_expected_element_counts() {
        let mesh = cuboid(CuboidConfig {
            width_segments: 2,
            height_segments: 3,
            depth_segments: 4,
            ..CuboidConfig::default()
        });

        // px/nx: (d+1)(h+1) each; py/ny: (w+1)(d+1) each; pz/nz: (w+1)(h+1).
        let expected_vertices = 2 * (5 * 4) + 2 * (3 * 5) + 2 * (3 * 4);
        let expected_indices = 2 * (4 * 3 * 6) + 2 * (2 * 4 * 6) + 2 * (2 * 3 * 6);

        assert_eq!(mesh.positions.len(), expected_vertices);
        assert_eq!(mesh.attributes.len(), expected_vertices);
        assert_eq!(mesh.indices.len(), expected_indices);
    }
}
