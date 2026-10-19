//! Cameras: a [`Camera`] trait plus orthographic and perspective
//! implementations.
//!
//! A camera owns a [`Transform`] (where it sits in the world) plus
//! projection state. After changing any projection parameter, call
//! [`Camera::update_projection_matrix`] to refresh the matrices, then
//! read them back through the trait's accessors to feed the renderer.
//!
//! Coordinate conventions follow the SDL3 GPU API: the GPU API is
//! left-handed (D3D12/Metal style) with NDC x/y in [-1, 1], NDC depth in
//! [0, 1] and +Z forward in view space, so with an identity [`Transform`]
//! the camera looks down +Z. SDL converts per-backend (e.g. Vulkan) behind
//! the scenes, so no flipping is needed here.

use glam::{Mat4, Vec3};

use crate::math::Transform;

/// A camera that owns its projection state and produces the matrices the
/// renderer needs.
///
/// Mutate the camera's parameters, call
/// [`Camera::update_projection_matrix`], then read
/// [`Camera::projection_matrix`], [`Camera::projection_matrix_inverse`] and
/// [`Camera::view_matrix`] — or the derived [`Camera::world_to_clip`] when
/// the renderer wants the matrix product precombined.
pub trait Camera {
    /// Recompute the projection matrices from the camera's current
    /// parameters.
    fn update_projection_matrix(&mut self);

    /// The camera's world [`Transform`]: position, rotation and scale.
    #[must_use]
    fn transform(&self) -> &Transform;

    /// Mutable access to the camera's world [`Transform`]; only affects
    /// [`Camera::view_matrix`], not the projection matrices.
    fn transform_mut(&mut self) -> &mut Transform;

    /// The projection matrix, mapping camera space to clip space.
    #[must_use]
    fn projection_matrix(&self) -> Mat4;

    /// The inverse of [`Camera::projection_matrix`], mapping clip space back
    /// to camera space.
    #[must_use]
    fn projection_matrix_inverse(&self) -> Mat4;

    /// The view matrix: the inverse of the camera's world [`Transform`],
    /// mapping world space to camera space.
    #[must_use]
    fn view_matrix(&self) -> Mat4 {
        self.transform().to_matrix().inverse()
    }

    /// The world-to-clip matrix as a single product:
    /// [`Camera::projection_matrix`] * [`Camera::view_matrix`].
    #[must_use]
    fn world_to_clip(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Maps a world-space `point` into camera (view) space.
    #[must_use]
    fn world_to_view(&self, point: Vec3) -> Vec3 {
        // The view matrix is affine, so the homogeneous w stays 1.
        (self.view_matrix() * point.extend(1.0)).truncate()
    }

    /// The world-space direction the camera looks along.
    ///
    /// Left-handed convention as per SDL3 GPU: with an identity [`Transform`]
    /// the camera looks down +Z. Derived fresh from the [`Transform`]
    /// rotation, so it never goes stale.
    #[must_use]
    fn forward(&self) -> Vec3 {
        self.transform().rotation * Vec3::Z
    }

    /// Rotates the camera so [`Camera::forward`] points at a world-space
    /// `target`, keeping `position` unchanged and using +Y up.
    ///
    /// Degenerate inputs (looking at the camera's own position, or at a
    /// point straight above/below) fall back to a slightly nudged direction
    /// instead of producing NaNs.
    fn look_at(&mut self, target: Vec3) {
        let mut dir = target - self.transform().position;
        if dir.length_squared() == 0.0 {
            // `target == position`: look down +Z, the forward direction at
            // the singularity in this convention.
            dir = Vec3::Z;
        }
        // Parallel to the +Y up vector: nudge in Z before normalizing.
        if dir.x == 0.0 && dir.z == 0.0 {
            dir.z += f32::EPSILON;
        }
        let dir = dir.normalize();
        // The view rotation maps world space to view space, so the camera's
        // world rotation is its inverse.
        self.transform_mut().rotation =
            glam::camera::lh::view::look_to_quat(dir, Vec3::Y).inverse();
    }

    /// Sizes the camera to a centered `width` x `height` viewport and
    /// immediately recomputes the projection matrices — the helper to
    /// call from a window-resize handler.
    ///
    /// Each camera maps the size onto its own projection parameters: an
    /// [`OrthographicCamera`] stores it as frustum edges; a
    /// [`PerspectiveCamera`] as an aspect ratio.
    fn set_view_size(&mut self, width: f32, height: f32);

    /// `true` if a world-space point lies inside the camera's frustum.
    ///
    /// Transforms the point to clip space and tests it against the NDC cube
    /// (NDC depth in [0, 1]), so no plane extraction is needed.
    /// Extend this with mesh/bounds culling later; for now it is a strict
    /// point test.
    #[must_use]
    fn is_point_visible(&self, point: Vec3) -> bool {
        let clip = self.world_to_clip() * point.extend(1.0);
        let w = clip.w;
        clip.x.abs() <= w && clip.y.abs() <= w && (0.0..=w).contains(&clip.z)
    }
}

/// Frustum edges for an [`OrthographicCamera`], in world units, listed in
/// the order (`left`, `right`, `top`, `bottom`, `near`, `far`).
///
/// `zoom` shrinks the effective frustum towards its center without touching
/// the edges themselves: `zoom = 2` magnifies everything 2x.
#[derive(Clone, Copy, Debug)]
pub struct OrthographicCameraConfig {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub near: f32,
    pub far: f32,
    pub zoom: f32,
}

impl Default for OrthographicCameraConfig {
    fn default() -> Self {
        Self {
            left: -1.0,
            right: 1.0,
            top: 1.0,
            bottom: -1.0,
            near: 0.1,
            far: 2000.0,
            zoom: 1.0,
        }
    }
}

/// An orthographic camera.
///
/// The box from `left` to `right`, `bottom` to `top` and `near` to `far`
/// in front of the camera, oriented by the camera's [`Transform`] (with an
/// identity transform, left-handed +Z forward with +Y up).
#[derive(Clone, Copy, Debug)]
pub struct OrthographicCamera {
    config: OrthographicCameraConfig,
    transform: Transform,
    projection_matrix: Mat4,
    projection_matrix_inverse: Mat4,
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        // Bake the default config's matrices immediately so the camera is
        // usable without any explicit `update_projection_matrix` call.
        let mut camera = Self {
            config: OrthographicCameraConfig::default(),
            transform: Transform::default(),
            projection_matrix: Mat4::IDENTITY,
            projection_matrix_inverse: Mat4::IDENTITY,
        };
        camera.update_projection_matrix();
        camera
    }
}

impl Camera for OrthographicCamera {
    fn update_projection_matrix(&mut self) {
        // SDL3 GPU is left-handed (D3D12/Metal style) with NDC x/y in
        // [-1, 1], NDC depth in [0, 1] and +Z forward in view space —
        // exactly glam's `directx` builder. glam has no SDL preset; the
        // same convention is shared by DirectX and WebGPU (hence the
        // `webgpu` alias on that module). glam orders the vertical edges
        // (bottom, top), while the config lists them top before bottom.
        //
        // `zoom` shrinks the effective viewport towards the frustum's
        // center before building the matrix.
        let c = self.config;
        let (zoomed_left, zoomed_right) = {
            let width = (c.right - c.left) / (2.0 * c.zoom);
            let center = f32::midpoint(c.left, c.right);
            (center - width, center + width)
        };
        let (zoomed_bottom, zoomed_top) = {
            let height = (c.top - c.bottom) / (2.0 * c.zoom);
            let center = f32::midpoint(c.bottom, c.top);
            (center - height, center + height)
        };

        self.projection_matrix = glam::camera::lh::proj::directx::orthographic(
            zoomed_left,
            zoomed_right,
            zoomed_bottom,
            zoomed_top,
            c.near,
            c.far,
        );
        self.projection_matrix_inverse = self.projection_matrix.inverse();
    }

    #[inline]
    fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    #[inline]
    fn projection_matrix_inverse(&self) -> Mat4 {
        self.projection_matrix_inverse
    }

    #[inline]
    fn transform(&self) -> &Transform {
        &self.transform
    }

    #[inline]
    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    /// Resizes the frustum to a centered `width` x `height` viewport and
    /// immediately recomputes the projection matrices — the helper to call
    /// from a window-resize handler.
    fn set_view_size(&mut self, width: f32, height: f32) {
        self.config.set_view_size(width, height);
        self.update_projection_matrix();
    }
}

impl OrthographicCameraConfig {
    /// Sizes the frustum to a centered `width` x `height` viewport, keeping
    /// the near/far planes and `zoom`. Call
    /// [`Camera::update_projection_matrix`] afterwards to apply — or use the
    /// one-call [`Camera::set_view_size`] wrapper.
    pub fn set_view_size(&mut self, width: f32, height: f32) {
        self.left = -0.5 * width;
        self.right = 0.5 * width;
        self.bottom = -0.5 * height;
        self.top = 0.5 * height;
    }
}

impl OrthographicCamera {
    /// Creates a camera with the given frustum; the projection matrices are
    /// computed immediately so the camera is ready to render.
    #[must_use]
    pub fn new(config: OrthographicCameraConfig) -> Self {
        let mut camera = Self {
            config,
            transform: Transform::default(),
            projection_matrix: Mat4::IDENTITY,
            projection_matrix_inverse: Mat4::IDENTITY,
        };
        camera.update_projection_matrix();
        camera
    }

    /// The frustum parameters. Mutate them through [`Self::config_mut`],
    /// then call [`Camera::update_projection_matrix`] to apply.
    #[inline]
    #[must_use]
    pub const fn config(&self) -> &OrthographicCameraConfig {
        &self.config
    }

    /// Mutable access to the frustum parameters. Call
    /// [`Camera::update_projection_matrix`] afterwards to apply.
    #[inline]
    pub const fn config_mut(&mut self) -> &mut OrthographicCameraConfig {
        &mut self.config
    }
}

/// Frustum parameters for a [`PerspectiveCamera`], listed in the order
/// (`vertical_fov`, `aspect_ratio`, `near`, `far`).
///
/// * `vertical_fov` — the full vertical angle between the bottom and top
///   frustum planes, in radians, in `(0, π)`
/// * `aspect_ratio` — viewing width over height
/// * `near`, `far` — clip plane distances along the camera's forward axis,
///   in world units, both positive with `near < far`
///
/// `zoom` narrows the effective field of view without touching the
/// parameters themselves: `zoom = 2` magnifies everything 2x.
#[derive(Clone, Copy, Debug)]
pub struct PerspectiveCameraConfig {
    pub vertical_fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub zoom: f32,
}

impl Default for PerspectiveCameraConfig {
    fn default() -> Self {
        Self {
            vertical_fov: std::f32::consts::FRAC_PI_3, // 60°
            aspect_ratio: 1.0,
            near: 0.1,
            far: 2000.0,
            zoom: 1.0,
        }
    }
}

/// A perspective camera.
///
/// The frustum is a pyramid with its apex at the camera: `vertical_fov`
/// radians between the bottom and top planes, `aspect_ratio` scaled
/// horizontally, extending from the `near` plane out to the `far` plane,
/// oriented by the camera's [`Transform`] (with an identity transform,
/// left-handed +Z forward with +Y up). Unlike an [`OrthographicCamera`],
/// objects shrink with distance.
#[derive(Clone, Copy, Debug)]
pub struct PerspectiveCamera {
    config: PerspectiveCameraConfig,
    transform: Transform,
    projection_matrix: Mat4,
    projection_matrix_inverse: Mat4,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        // Bake the default config's matrices immediately so the camera is
        // usable without any explicit `update_projection_matrix` call.
        let mut camera = Self {
            config: PerspectiveCameraConfig::default(),
            transform: Transform::default(),
            projection_matrix: Mat4::IDENTITY,
            projection_matrix_inverse: Mat4::IDENTITY,
        };
        camera.update_projection_matrix();
        camera
    }
}

impl Camera for PerspectiveCamera {
    fn update_projection_matrix(&mut self) {
        // Same SDL3 GPU convention as the orthographic camera: glam's
        // `directx` builder gives a left-handed projection with NDC x/y in
        // [-1, 1] and NDC depth in [0, 1].
        //
        // `zoom` narrows the effective field of view before building the
        // matrix: it divides the focal length, the tangent of the
        // half-angle, matching the orthographic camera's zoom behavior.
        let c = self.config;
        let vertical_fov = 2.0 * (f32::tan(c.vertical_fov / 2.0) / c.zoom).atan();
        self.projection_matrix = glam::camera::lh::proj::directx::perspective(
            vertical_fov,
            c.aspect_ratio,
            c.near,
            c.far,
        );
        self.projection_matrix_inverse = self.projection_matrix.inverse();
    }

    #[inline]
    fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    #[inline]
    fn projection_matrix_inverse(&self) -> Mat4 {
        self.projection_matrix_inverse
    }

    #[inline]
    fn transform(&self) -> &Transform {
        &self.transform
    }

    #[inline]
    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn set_view_size(&mut self, width: f32, height: f32) {
        // A zero height (e.g. a minimized window) would make the aspect
        // ratio undefined; keep the previous aspect ratio in that case.
        if height != 0.0 {
            self.config.aspect_ratio = width / height;
        }
        self.update_projection_matrix();
    }
}

impl PerspectiveCameraConfig {
    /// Sets [`Self::aspect_ratio`] to `width / height`, keeping the
    /// near/far planes and `zoom`. Call
    /// [`Camera::update_projection_matrix`] afterwards to apply — the
    /// [`Camera::set_view_size`] implementation on [`PerspectiveCamera`]
    /// wraps the two together.
    pub fn set_view_size(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }
}

impl PerspectiveCamera {
    /// Creates a camera with the given frustum; the projection matrices are
    /// computed immediately so the camera is ready to render.
    #[must_use]
    pub fn new(config: PerspectiveCameraConfig) -> Self {
        let mut camera = Self {
            config,
            transform: Transform::default(),
            projection_matrix: Mat4::IDENTITY,
            projection_matrix_inverse: Mat4::IDENTITY,
        };
        camera.update_projection_matrix();
        camera
    }

    /// The frustum parameters. Mutate them through [`Self::config_mut`],
    /// then call [`Camera::update_projection_matrix`] to apply.
    #[inline]
    #[must_use]
    pub const fn config(&self) -> &PerspectiveCameraConfig {
        &self.config
    }

    /// Mutable access to the frustum parameters. Call
    /// [`Camera::update_projection_matrix`] afterwards to apply.
    #[inline]
    pub const fn config_mut(&mut self) -> &mut PerspectiveCameraConfig {
        &mut self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn default_perspective_projects_screen_center_between_planes() {
        let camera = PerspectiveCamera::default();
        let clipped = camera.world_to_clip() * Vec3::Z.extend(1.0);
        let ndc = clipped.truncate() / clipped.w;
        assert_abs_diff_eq!(ndc.x, 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(ndc.y, 0.0, epsilon = 1e-5);
        assert!(
            (0.0..=1.0).contains(&ndc.z),
            "NDC depth outside [0, 1]: {}",
            ndc.z
        );
    }

    #[test]
    fn perspective_respects_plane_distances() {
        let camera = PerspectiveCamera::default();
        let visible = Vec3::Z;
        let behind_near = Vec3::Z * (camera.config().near / 2.0);
        let beyond_far = Vec3::Z * (camera.config().far * 2.0);
        assert!(camera.is_point_visible(visible));
        assert!(!camera.is_point_visible(behind_near));
        assert!(!camera.is_point_visible(beyond_far));
    }

    #[test]
    fn perspective_is_point_visible_respects_horizontal_extent() {
        // With the default square aspect, the frustum's half-opening at the
        // screen center is `tan(fov / 2)` per unit of depth (`1 / √3` for
        // the default 60° fov), so points well inside are visible and
        // further out beyond it are not.
        let camera = PerspectiveCamera::default();
        assert!(camera.is_point_visible(Vec3::new(0.5, 0.0, 1.0)));
        assert!(!camera.is_point_visible(Vec3::new(1.1, 0.0, 1.0)));
    }

    #[test]
    fn perspective_set_view_size_adjusts_aspect_ratio() {
        let mut camera = PerspectiveCamera::default();
        camera.set_view_size(16.0, 9.0);
        assert_abs_diff_eq!(camera.config().aspect_ratio, 16.0 / 9.0, epsilon = 1e-6);
    }

    #[test]
    fn zero_height_viewport_keeps_previous_aspect_ratio() {
        let mut camera = PerspectiveCamera::default();
        camera.set_view_size(16.0, 9.0);
        camera.set_view_size(0.0, 0.0);
        assert_abs_diff_eq!(camera.config().aspect_ratio, 16.0 / 9.0, epsilon = 1e-6);
    }

    #[test]
    fn identity_transform_forward_is_plus_z() {
        // SDL3 GPU convention: +Z points forward with identity transforms.
        assert_abs_diff_eq!(
            OrthographicCamera::default().forward(),
            Vec3::Z,
            epsilon = 1e-6
        );
        assert_abs_diff_eq!(
            PerspectiveCamera::default().forward(),
            Vec3::Z,
            epsilon = 1e-6
        );
    }

    #[test]
    fn look_at_tracks_target() {
        let mut camera = OrthographicCamera::default();
        camera.look_at(Vec3::new(5.0, 0.0, 0.0));
        assert_abs_diff_eq!(camera.forward(), Vec3::X, epsilon = 1e-5);
    }

    #[test]
    fn look_at_degenerate_target_avoids_nans() {
        let mut camera = OrthographicCamera::default();
        camera.look_at(Vec3::new(5.0, 0.0, 0.0));
        // A target at the camera's own position: the degenerate case.
        camera.look_at(camera.transform().position);
        let forward = camera.forward();
        assert_abs_diff_eq!(forward, Vec3::Z, epsilon = 1e-5);
    }

    #[test]
    fn zoom_2x_doubles_the_focal_length() {
        // Doubling the zoom divisor halves the tangent of the half-angle,
        // which doubles the focal length (the projection matrix's y scale)
        // — the perspective sibling of the orthographic frustum shrink.
        let wide = PerspectiveCamera::default();
        let zoomed = PerspectiveCamera::new(PerspectiveCameraConfig {
            zoom: 2.0,
            ..PerspectiveCameraConfig::default()
        });
        let focal = |matrix: Mat4| matrix.to_cols_array()[5]; // column 1, row 1
        let ratio = focal(zoomed.projection_matrix()) / focal(wide.projection_matrix());
        assert_abs_diff_eq!(ratio, 2.0, epsilon = 1e-5);
    }
}
