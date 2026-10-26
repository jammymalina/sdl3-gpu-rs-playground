use glam::{EulerRot, Quat, Vec3};

use crate::math::Transform;

#[derive(Clone, Copy, Debug)]
pub struct DirectionalLight {
    pub transform: Transform,
    pub color: Vec3,
    pub intensity: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        // In Left-Handed (+Z forward, +Y up):
        // Pitch of +50° tilts +Z down towards -Y (the ground).
        // Yaw of 45° turns it diagonally.
        let sun_rotation = Quat::from_euler(
            EulerRot::YXZ,
            45.0_f32.to_radians(), // Yaw
            50.0_f32.to_radians(), // Pitch (+50° tilts downward in LH)
            0.0,                   // Roll
        );

        Self {
            transform: Transform {
                position: Vec3::ZERO,
                rotation: sun_rotation,
                scale: Vec3::ONE,
            },
            color: Vec3::new(1.0, 0.97, 0.92),
            intensity: 1.0,
        }
    }
}

impl DirectionalLight {
    #[inline]
    #[must_use]
    pub fn direction_to_light(&self) -> Vec3 {
        -self.transform.forward()
    }

    #[inline]
    #[must_use]
    pub fn ray_direction(&self) -> Vec3 {
        self.transform.forward()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MainLight {
    None,
    Directional(DirectionalLight),
}

impl Default for MainLight {
    fn default() -> Self {
        Self::Directional(DirectionalLight::default())
    }
}

impl MainLight {
    #[must_use]
    pub fn direction_to_light(&self) -> Vec3 {
        match self {
            Self::Directional(sun) => sun.direction_to_light(),
            Self::None => Vec3::Y,
        }
    }

    #[must_use]
    pub fn final_color(&self) -> Vec3 {
        match self {
            Self::Directional(sun) => sun.color * sun.intensity,
            Self::None => Vec3::ZERO,
        }
    }
}
