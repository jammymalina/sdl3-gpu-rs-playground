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
    #[must_use]
    pub const fn from_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    #[must_use]
    pub const fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    #[must_use]
    pub const fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    #[must_use]
    pub const fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub const fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec3::splat(scale);
        self
    }

    /// Moves the transform by `delta` in parent space.
    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
    }

    /// Applies `delta` on top of the current rotation, around world/parent axes.
    pub fn rotate(&mut self, delta: Quat) {
        self.rotation = (delta * self.rotation).normalize();
    }

    /// Rotates around the parent X axis by `radians`.
    pub fn rotate_x(&mut self, radians: f32) {
        self.rotate(Quat::from_rotation_x(radians));
    }

    /// Rotates around the parent Y axis by `radians`.
    pub fn rotate_y(&mut self, radians: f32) {
        self.rotate(Quat::from_rotation_y(radians));
    }

    /// Rotates around the parent Z axis by `radians`.
    pub fn rotate_z(&mut self, radians: f32) {
        self.rotate(Quat::from_rotation_z(radians));
    }

    #[inline]
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    #[inline]
    #[must_use]
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

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
