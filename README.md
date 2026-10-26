# sdl3-gpu-rs-playground

A small SDL3 GPU (Vulkan) playground in Rust. Shaders are written in
[Slang](https://shader-slang.com), compiled to SPIR-V automatically by
`cargo build`.

## Shaders

Shader sources live in [`shaders/`](shaders/). Every shader is a **single
`.slang` file containing all of its stages** — no `.frag`/`.vert` file
splitting anywhere; `.slang` is the only extension used.

Each stage is an entry-point function annotated with its stage:

```slang
// shaders/flat_color.slang
import common_types;

ConstantBuffer<SceneData> scene : register(b0);
ConstantBuffer<ObjectData> object : register(b1);
ConstantBuffer<BasicColorData> color : register(b2);

struct VertexInput { /* ... */ }
struct VertexOutput { /* ... */ }

[shader("vertex")]
VertexOutput vertex_main(VertexInput input) { /* ... */ }

[shader("fragment")]
float4 frag_main(VertexOutput input) : SV_Target { /* ... */ }
```

Entry-point names are free-form (they don't have to be `main`); `build.rs`
reads them straight out of the file.

Directories are purely organizational — subdirectories group shaders by
feature or render pass, they don't carry the stage and don't affect the
pipeline:

```
shaders/
├── common_types.slang   # module: shared types, no entry points
├── flat_color.slang     # vertex + fragment (all stages in one file)
└── lit/                 # just grouping (illustrative: not in the repo yet)
    └── blinn.slang
```

### Importable modules

A `.slang` file without any `[shader("...")]` attribute is a **module** —
like `shaders/common_types.slang`:

```slang
module common_types;

public struct SceneData
{
    public float4x4 view_projection;
    /* ... */
};
```

Modules are never compiled on their own; shaders pull them in with
`import common_types;` (`build.rs` passes `-I shaders` to `slangc`). Slang
module members are private by default, so mark everything that crosses a
module boundary `public` — the struct declarations and their fields alike.

### Build pipeline

`build.rs` scans every `shaders/**.slang` file, finds all
`[shader("...")]`-annotated functions, and runs `slangc` once per entry point
with the stage from the attribute and the annotated function as `-entry`:

```sh
slangc shaders/flat_color.slang -target spirv \
    -entry vertex_main -stage vertex \
    -I shaders -matrix-layout-column-major -fvk-use-gl-layout
```

* one SPIR-V blob per shader-stage: `$OUT_DIR/shaders/flat_color.vertex.spv`
  and `flat_color.fragment.spv` for the file above. The key and file name always
  use the **canonical** stage, so `[shader("pixel")]` produces `*.fragment.spv`,
* `-g -O0` in debug builds, `-O3` in release builds,
* `slangc` warnings from successful compiles are forwarded to cargo as
  `cargo:warning`s (e.g. `E39029`, see below),
* the blobs are embedded into the crate through the generated
  `$OUT_DIR/shaders.rs` which `src/shaders.rs` includes. Editing a `.slang`
  file and rerunning `cargo build` recompiles — no manual step. Blobs of deleted
  or renamed shaders are removed on each build.

Accepted `[shader("...")]` stage names and their canonical stage:

| Attribute value                      | Canonical stage |
| ------------------------------------ | --------------- |
| `vertex`                             | `vertex`        |
| `fragment`, `pixel`                  | `fragment`      |
| `geometry`                           | `geometry`      |
| `hull`, `tesscontrol`                | `hull`          |
| `domain`, `tesseval`                 | `domain`        |
| `compute`                            | `compute`       |
| `mesh`                               | `mesh`          |
| `amplification`, `task`              | `amplification` |
| `raygeneration`, `intersection`, `anyhit`, `closesthit`, `miss`, `callable` | same name |

Any spelling in this table is accepted everywhere a stage is taken: in the
attribute, in `shaders::find_stage` / `shaders::get`, and in
`shaders::canonical_stage`.

### Using the compiled shaders from Rust

Each compiled entry point is exposed in `shaders::ALL` as a `Shader { key,
name, stage, entrypoint, bytes }` record, keyed by
`<path below shaders/ without extension>.<stage>`:

| Source file      | Entry point      | Key                  | Constant            |
| ---------------- | ---------------- | -------------------- | ------------------- |
| `flat_color.slang` | `vertex_main` | `flat_color.vertex`   | `FLAT_COLOR_VERTEX`   |
| `flat_color.slang` | `frag_main`   | `flat_color.fragment` | `FLAT_COLOR_FRAGMENT` |

```rust
use anyhow::Context;
use sdl3::gpu::ShaderFormat;

let vs = shaders::find_stage("flat_color", "vertex")
    .context("flat_color.vertex missing from the manifest")?;
let vertex_shader = device
    .create_shader()
    .with_code(
        ShaderFormat::SPIRV,
        vs.bytes,
        vs.sdl_stage().context("vertex stage has no SDL ShaderStage")?,
    )
    .with_entrypoint(&vs.entrypoint_cstring())
    .build()?;
```

`find_stage` and `get` return `Option<&'static Shader>`, so convert `None`
into an error with `.context(...)` (or `ok_or`) as above.

* `shaders::find_stage("flat_color", "vertex")` — by source name and stage,
  any stage spelling from the table above works (`"pixel"` finds the fragment entry),
* `shaders::get("flat_color.vertex")` — by key (`<name>.<canonical stage>`),
  also accepts an alias or a trailing `.spv`,
* `shaders::canonical_stage("pixel")` — alias to canonical stage (`Some("fragment")`),
* `shaders::FLAT_COLOR_VERTEX`, `shaders::FLAT_COLOR_FRAGMENT` — plain byte
  constants also exist per compiled entry point,
* `Shader::stage` is always the canonical stage; `Shader::sdl_stage()` maps
  `vertex` and `fragment` to SDL3's `ShaderStage` and returns `None` for
  anything else. Stages SDL GPU's shader API can't consume (`geometry`, `hull`,
  `domain`, `mesh`, ...) still compile and are embedded, but no pipeline can be
  built from them through `create_shader`.

### Requirements

`slangc` must be available. `build.rs` looks for it in this order:

1. the `SLANGC` environment variable,
2. `slangc` on `PATH`,
3. well-known install locations, in order:
   `/opt/shader-slang-bin/bin/slangc` (Arch `shader-slang-bin`),
   `/opt/shader-slang/bin/slangc` (Arch `shader-slang`), and
   `/usr/local/bin/slangc` (manual install).

On Arch, install it with:

```sh
pacman -S shader-slang-bin
```

### Writing shaders for SDL3 GPU

SDL3's resource-binding convention, which differs per stage
([SDL_CreateGPUShader](https://wiki.libsdl.org/SDL3/SDL_CreateGPUShader)):

| Stage    | Samplers / storage textures / storage buffers | Uniform buffers |
| -------- | --------------------------------------------- | --------------- |
| Vertex   | set `0`                                       | set `1`         |
| Fragment | set `2`                                       | set `3`         |

Bindings start at `0` and must be consecutive, with samplers before storage
textures before storage buffers. The HLSL-style `register(bN)` spelling
compiles (with `-fvk-use-gl-layout`); with this toolchain a `bN` cbuffer on
the vertex stage maps to SDL3's vertex uniform slot `N` (fragment stage:
fragment uniform slot `N`) and `slangc` emits an `E39029` warning since no
explicit `[[vk::binding(binding, set)]]` is given — add one if you need
tighter control over descriptor sets and bindings.

Vertex inputs/outputs and fragment outputs use `[[vk::location(n)]]`. Entry
points can be named anything — the build script discovers them, so the Rust
side never hard-codes entry-point names.

`build.rs` compiles with `-matrix-layout-column-major` and
`-fvk-use-gl-layout`, so a `glam::Mat4` (column-major) can be uploaded as-is
and used with `mul(matrix, vector)`.
