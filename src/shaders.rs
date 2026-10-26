//! SPIR-V shaders compiled from `shaders/` at build time.
//!
//! Each shader is one `.slang` file containing all of its stages, every stage
//! an entry point annotated with a `[shader("stage")]` attribute. `build.rs`
//! compiles one SPIR-V blob per entry point and generates the module included
//! below in `OUT_DIR`; it also exposes:
//!
//! * [`Shader`] - one compiled entry point (key, source name, stage, Slang
//!   entry-point function name, SPIR-V `bytes`),
//! * [`ALL`] - every compiled entry point,
//!
//! Files without any `[shader("...")]` attribute (e.g. `common_types.slang`)
//! are importable modules: they are only pulled into compilation through other
//! shaders `import`ing them.
//!
//! ```ignore
//! use sdl3::gpu::{ShaderFormat, ShaderStage};
//!
//! let vertex_info = shaders::find_stage("flat_color", "vertex").unwrap();
//! let vertex = gpu
//!     .create_shader()
//!     .with_code(ShaderFormat::SPIRV, vertex_info.bytes, ShaderStage::Vertex)
//!     .with_entrypoint(&vertex_info.entrypoint_cstring()) // e.g. `vertex_main`
//!     .build()?;
//! ```

#![allow(dead_code)]
// The generated code lives in OUT_DIR and is written by `build.rs`, so the
// crate's strict clippy lints don't apply to it.
#![allow(clippy::must_use_candidate, clippy::doc_markdown)]

use std::ffi::CString;

use sdl3::gpu::ShaderStage;

include!(concat!(env!("OUT_DIR"), "/shaders.rs"));

impl Shader {
    /// SDL3 [`ShaderStage`] for this entry point, i.e. the stage classes SDL
    /// GPU's `create_shader` API accepts: `vertex` and `fragment` (`pixel` is
    /// an alias of `fragment`). Other stages (`geometry`, `hull`, `domain`, ...)
    /// compile and are embedded, but SDL GPU pipelines cannot consume them directly.
    #[must_use]
    pub fn sdl_stage(&self) -> Option<ShaderStage> {
        match self.stage {
            "vertex" => Some(ShaderStage::Vertex),
            "fragment" => Some(ShaderStage::Fragment),
            _ => None,
        }
    }

    /// `entrypoint` as a [`CString`], for `...::with_entrypoint(&shader.entrypoint_cstring())`.
    ///
    /// # Panics
    /// Panics if `entrypoint` contained a NUL byte, which cannot happen for
    /// the identifiers emitted by the build script.
    #[must_use]
    pub fn entrypoint_cstring(&self) -> CString {
        CString::new(self.entrypoint).expect("entry point names never contain a NUL byte")
    }
}

/// Canonical Slang stage for any accepted `[shader("...")]` spelling, e.g.
/// `canonical_stage("pixel") == Some("fragment")`. `None` for unknown stages.
#[must_use]
pub fn canonical_stage(stage: &str) -> Option<&'static str> {
    STAGE_ALIASES
        .iter()
        .find_map(|(alias, canonical)| (*alias == stage).then_some(*canonical))
}

/// Look up a compiled entry point by key (`"<name>.<stage>"`, e.g.
/// `get("flat_color.vertex")`). The stage may use any alias (`"flat_color.pixel"`),
/// and a trailing `.spv` is tolerated.
#[must_use]
pub fn get(key: &str) -> Option<&'static Shader> {
    let key = key.strip_suffix(".spv").unwrap_or(key);
    let (name, stage) = key.rsplit_once('.')?;
    find_stage(name, stage)
}

/// Find a compiled entry point by source name and Slang stage, e.g.
/// `find_stage("flat_color", "vertex")`. The stage may use any alias, so
/// `find_stage("flat_color", "pixel")` finds the fragment entry point.
#[must_use]
pub fn find_stage(name: &str, stage: &str) -> Option<&'static Shader> {
    let stage = canonical_stage(stage)?;
    ALL.iter()
        .find(|shader| shader.name == name && shader.stage == stage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_is_an_alias_of_fragment() {
        assert_eq!(canonical_stage("pixel"), Some("fragment"));
        assert_eq!(canonical_stage("fragment"), Some("fragment"));
        assert_eq!(canonical_stage("tesscontrol"), Some("hull"));
        assert_eq!(canonical_stage("bogus"), None);
    }

    #[test]
    fn lookups_accept_aliases() {
        let fragment = find_stage("flat_color", "fragment").expect("flat_color.fragment");

        assert_eq!(find_stage("flat_color", "pixel"), Some(fragment));
        assert_eq!(get("flat_color.fragment"), Some(fragment));
        assert_eq!(get("flat_color.pixel"), Some(fragment));
        assert_eq!(get("flat_color.pixel.spv"), Some(fragment));
        assert_eq!(get("flat_color.bogus"), None);
    }

    #[test]
    fn fragment_stage_maps_to_sdl_fragment() {
        let fragment = find_stage("flat_color", "pixel").expect("flat_color.fragment");

        assert_eq!(fragment.stage, "fragment");
        assert!(matches!(fragment.sdl_stage(), Some(ShaderStage::Fragment)));
    }
}
