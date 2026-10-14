# sdl3-gpu-rs-playground

A small SDL3 GPU (Vulkan) playground in Rust. Shaders are written in
[Slang](https://shader-slang.com) and compiled to SPIR-V automatically by
`cargo build`.

## Shaders

Shader sources live in [`shaders/`](shaders/):

```
shaders/
├── basic.vert.slang   -> compiled to basic.vert.spv
└── basic.frag.slang   -> compiled to basic.frag.spv
```

The recommended convention is `<name>.<stage>.slang`, where `<stage>` is one
of `vert`, `frag` or `comp`. A `<stage>.slang` name (`shaders/gui/vert.slang`)
is also accepted. Either way the stage ends up in the compiled name and the
generated constant (`shaders/gui/vert.slang` -> `GUI_VERT`). Any other
`.slang` file (for example a shared `common.slang`) is treated as an include
and skipped.

Use subdirectories to group shaders by feature, material or render pass — not
to carry the stage:

```
shaders/
├── common/
│   └── lighting.slang        # shared, not compiled on its own
├── opaque/
│   ├── opaque.vert.slang
│   └── opaque.frag.slang
└── sky/
    └── sky.vert.slang
```

Keeping the stage as the filename suffix (rather than naming files
`vert.slang`/`frag.slang`) means the compiled `.spv` is self-describing, the
build script can infer the stage without guessing, and the shader name stays
in the artifact. It also matches the `Name.vert.hlsl` convention used by
SDL_shadercross examples.

`build.rs` runs the Slang compiler (`slangc`) for every shader, embeds the
resulting SPIR-V into the crate, and regenerates `$OUT_DIR/shaders.rs` (wired up
by `src/shaders.rs`). No manual step is required — editing a shader and running
`cargo build` recompiles it.

```rust
use sdl3::gpu::{ShaderFormat, ShaderStage};

let vertex_shader = device
    .create_shader()
    .with_code(ShaderFormat::SPIRV, shaders::BASIC_VERT, ShaderStage::Vertex)
    .with_entrypoint(c"main")
    .build()?;
```

`src/shaders.rs` exposes `BASIC_VERT` / `BASIC_FRAG` constants plus
`get("basic.vert")` and `ALL`.

### Requirements

`slangc` must be available. `build.rs` looks for it in this order:

1. the `SLANGC` environment variable,
2. `slangc` on `PATH`,
3. well-known install locations such as `/opt/shader-slang-bin/bin/slangc`
   (Arch `shader-slang-bin`) and `/opt/shader-slang/bin/slangc`.

On Arch, install it with:

```sh
pacman -S shader-slang-bin
```

### Writing shaders for SDL3 GPU

Shaders must follow SDL3's resource-binding convention, which differs per
stage ([SDL_CreateGPUShader](https://wiki.libsdl.org/SDL3/SDL_CreateGPUShader)):

| Stage    | Samplers / storage textures / storage buffers | Uniform buffers |
| -------- | --------------------------------------------- | --------------- |
| Vertex   | set `0`                                       | set `1`         |
| Fragment | set `2`                                       | set `3`         |

Bindings start at `0` and must be consecutive, with samplers before storage
textures before storage buffers. Slang requires explicit
`[[vk::binding(binding, set)]]` attributes for SPIR-V (D3D `register`/`space`
does not map to descriptor sets).

Vertex inputs/outputs and fragment outputs use `[[vk::location(n)]]`. The entry
point must be named `main`, which is what the Rust side passes as the
entrypoint.

`build.rs` compiles with `-matrix-layout-column-major` and
`-fvk-use-gl-layout`, so a `glam::Mat4` (column-major) can be uploaded as-is and
used with `mul(matrix, vector)`.
