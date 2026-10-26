use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use sdl3::gpu::Buffer;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct VertexPosition {
    pub position: [f32; 3],
}

impl VertexPosition {
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct VertexAttributes {
    pub normal: [f32; 3],
    _padding0: f32,
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    _padding1: [f32; 2],
}

impl VertexAttributes {
    #[must_use]
    pub const fn new(normal: [f32; 3], tangent: [f32; 4], uv: [f32; 2]) -> Self {
        Self {
            normal,
            _padding0: 0.0,
            tangent,
            uv,
            _padding1: [0.0, 0.0],
        }
    }

    #[must_use]
    pub const fn with_normal(mut self, x: f32, y: f32, z: f32) -> Self {
        self.normal = [x, y, z];
        self
    }

    #[must_use]
    pub const fn with_tangent(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.tangent = [x, y, z, w];
        self
    }

    #[must_use]
    pub const fn with_uv(mut self, u: f32, v: f32) -> Self {
        self.uv = [u, v];
        self
    }
}

const _: () = {
    assert!(std::mem::size_of::<VertexPosition>() == 12);
    assert!(std::mem::align_of::<VertexPosition>() == 4);

    assert!(std::mem::size_of::<VertexAttributes>() == 48);
    assert!(std::mem::align_of::<VertexAttributes>() == 4);
};

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<VertexPosition>,
    pub attributes: Vec<VertexAttributes>,
    pub indices: Vec<u16>,
}

impl MeshData {
    pub fn compute_tangents(&mut self) {
        let vertex_count = self.positions.len();
        if vertex_count == 0 || self.indices.is_empty() {
            return;
        }

        // Temporary accumulation buffers for surface tangents and bitangents
        let mut tan1 = vec![Vec3::ZERO; vertex_count];
        let mut tan2 = vec![Vec3::ZERO; vertex_count];

        // 1. Accumulate triangle tangents across all faces
        for chunk in self.indices.as_chunks::<3>().0 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            let p0 = Vec3::from(self.positions[i0].position);
            let p1 = Vec3::from(self.positions[i1].position);
            let p2 = Vec3::from(self.positions[i2].position);

            let uv0 = Vec2::from(self.attributes[i0].uv);
            let uv1 = Vec2::from(self.attributes[i1].uv);
            let uv2 = Vec2::from(self.attributes[i2].uv);

            let edge1 = p1 - p0;
            let edge2 = p2 - p0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            // Solve: [edge1, edge2]^T = [delta_uv1, delta_uv2]^T * [tangent, bitangent]
            let r = 1.0 / f32::mul_add(delta_uv2.x, -delta_uv1.y, delta_uv1.x * delta_uv2.y);

            // Skip degenerate triangles or UVs that share the exact same coordinates
            if !r.is_finite() {
                continue;
            }

            let s_dir = (edge1 * delta_uv2.y - edge2 * delta_uv1.y) * r;
            let t_dir = (edge2 * delta_uv1.x - edge1 * delta_uv2.x) * r;

            tan1[i0] += s_dir;
            tan1[i1] += s_dir;
            tan1[i2] += s_dir;

            tan2[i0] += t_dir;
            tan2[i1] += t_dir;
            tan2[i2] += t_dir;
        }

        // 2. Gram-Schmidt Orthogonalization and Handedness Calculation
        for i in 0..vertex_count {
            let n = Vec3::from(self.attributes[i].normal);
            let t = tan1[i];

            if t.length_squared() == 0.0 {
                // Fallback for vertices with degenerate / missing tangents
                self.attributes[i].tangent = [1.0, 0.0, 0.0, 1.0];
                continue;
            }

            // Gram-Schmidt orthogonalize: t' = normalize(t - n * dot(n, t))
            let t_ortho = (t - n * n.dot(t)).normalize_or_zero();

            // Calculate handedness (sign of bitangent):
            // In left-handed space, n.cross(t).dot(tan2) correctly preserves the sign
            let w = if n.cross(t).dot(tan2[i]) < 0.0 {
                -1.0
            } else {
                1.0
            };

            self.attributes[i].tangent = [t_ortho.x, t_ortho.y, t_ortho.z, w];
        }
    }
}

pub struct Mesh {
    pub position_buffer: Buffer,
    pub attribute_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u16,
}
