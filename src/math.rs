//! Small math primitives built on `glam`, used across the crate.

use glam::{Mat3, Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    #[inline]
    #[must_use]
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    #[inline]
    #[must_use]
    pub fn to_normal_matrix(&self) -> Mat4 {
        let linear = Mat3::from_quat(self.rotation) * Mat3::from_diagonal(self.scale);
        Mat4::from_mat3(linear.inverse().transpose())
    }

    #[inline]
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }
}
