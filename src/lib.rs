//! # `sdl3_gpu_rs_playground`
//!
//! A scene-graph 3D library in Rust, built on SDL3's GPU API,
//! with shaders written in [Slang](https://shader-slang.com) and compiled to
//! SPIR-V at build time.
//!
//! The crate is a library; runnable demos live under `examples/` (e.g.
//! `cargo run --example rainbow_triangle`).

pub mod assets;
pub mod camera;
pub mod geom;
pub mod math;
pub mod mesh;
pub mod render;
pub mod scene;
pub mod shaders;
