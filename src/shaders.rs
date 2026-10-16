//! SPIR-V shaders compiled from `shaders/` at build time.
//!
//! `build.rs` compiles every `<name>.<stage>.slang` file with the Slang
//! compiler and generates this module in `OUT_DIR`. It exposes one `&[u8]`
//! constant per shader (e.g. `BASIC_VERT`), a `get(name)` lookup, and `ALL`.
//!
//! ```ignore
//! use sdl3::gpu::{ShaderFormat, ShaderStage};
//!
//! let vertex = gpu
//!     .create_shader()
//!     .with_code(ShaderFormat::SPIRV, shaders::BASIC_VERT, ShaderStage::Vertex)
//!     .with_entrypoint(c"main")
//!     .build()?;
//! ```

#![allow(dead_code)]
// The generated code lives in OUT_DIR and is written by `build.rs`, so the
// crate's strict clippy lints don't apply to it.
#![allow(clippy::must_use_candidate, clippy::doc_markdown)]

include!(concat!(env!("OUT_DIR"), "/shaders.rs"));
