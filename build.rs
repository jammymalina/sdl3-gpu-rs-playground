//! Build-time Slang → SPIR-V compilation for SDL3's GPU API.
//!
//! Every `<name>.<stage>.slang` file under [`SHADER_DIR`] is compiled with the
//! Slang compiler (`slangc`) into SPIR-V and embedded into the crate through a
//! generated `shaders.rs` in `OUT_DIR` (wired up by `src/shaders.rs`).
//!
//! `slangc` is located through, in order:
//!
//! 1. the `SLANGC` environment variable,
//! 2. `slangc` on `PATH`,
//! 3. well-known install locations (Arch's `shader-slang-bin`, the Vulkan SDK).
//!
//! The recognised stage suffixes are `vert`, `frag` and `comp`; any other
//! `.slang` file is treated as a shared include and skipped (but still watched
//! for changes).

// This is build tooling, not library code - opt out of the crate's strict
// pedantic/nursery clippy lints.
#![allow(clippy::pedantic, clippy::nursery)]

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Directory (relative to the crate root) that holds the shader sources.
const SHADER_DIR: &str = "shaders";

/// Recognised `<name>.<stage>.slang` suffixes mapped to `slangc -stage` values.
const STAGES: &[(&str, &str)] = &[
    ("vert", "vertex"),
    ("frag", "fragment"),
    ("comp", "compute"),
];

/// Well-known locations for `slangc` when it is not on `PATH`.
const SLANGC_HINTS: &[&str] = &[
    // Arch AUR: shader-slang-bin
    "/opt/shader-slang-bin/bin/slangc",
    // Arch AUR: shader-slang
    "/opt/shader-slang/bin/slangc",
    // Vulkan SDK
    "/usr/local/bin/slangc",
];

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shader_dir = manifest_dir.join(SHADER_DIR);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={SHADER_DIR}");
    println!("cargo:rerun-if-env-changed=SLANGC");

    let slangc = find_slangc().unwrap_or_else(|| {
        panic!(
            "could not find the Slang compiler (`slangc`).\n\
             Install it (on Arch: `pacman -S shader-slang-bin`) or set the `SLANGC` \
             environment variable to the binary's path.\n\
             Searched `PATH` and: {}",
            SLANGC_HINTS.join(", ")
        );
    });

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let spv_dir = out_dir.join("shaders");
    fs::create_dir_all(&spv_dir).expect("failed to create shader output directory");

    // Debug builds keep debug info and skip optimization; release builds optimize.
    let debug = env::var("PROFILE").as_deref() != Ok("release");

    let mut sources = Vec::new();
    collect_sources(&shader_dir, &mut sources);
    sources.sort();

    let mut compiled = Vec::new();
    for src in &sources {
        let Some((key, stage)) = classify(src, &shader_dir) else {
            continue;
        };
        println!("cargo:rerun-if-changed={}", src.display());

        let out = spv_dir.join(format!("{key}.spv"));
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).expect("failed to create shader output subdirectory");
        }
        compile(&slangc, src, &out, stage, debug, &shader_dir);

        // Path is relative to the generated `shaders.rs` (which lives in OUT_DIR).
        let spv_rel = format!("shaders/{key}.spv");
        compiled.push((key, spv_rel));
    }

    if compiled.is_empty() {
        println!(
            "cargo:warning=no shaders found in `{SHADER_DIR}/` \
             (expected files named `<name>.<vert|frag|comp>.slang`)"
        );
    }

    check_unique_idents(&compiled);
    write_manifest(&out_dir, &compiled);
}

/// Fail with a clear message if two shaders would generate the same constant
/// (for example `basic.vert.slang` and `basic/vert.slang`).
fn check_unique_idents(compiled: &[(String, String)]) {
    let mut idents: Vec<(String, &str)> = compiled
        .iter()
        .map(|(key, _)| (ident(key), key.as_str()))
        .collect();
    idents.sort();
    for pair in idents.windows(2) {
        if pair[0].0 == pair[1].0 {
            panic!(
                "shader name collision: `{}` and `{}` both compile to the identifier `{}`",
                pair[0].1, pair[1].1, pair[0].0
            );
        }
    }
}

/// Recursively collect every `.slang` file below `dir`.
fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "slang") {
            out.push(path);
        }
    }
}

/// Locate the shader entry point and stage from a source path.
///
/// Two layouts are supported:
///
/// * `shaders/basic.vert.slang` -> `("basic.vert", "vertex")`
/// * `shaders/basic/vert.slang` -> `("basic/vert", "vertex")`
///
/// The first form keeps the stage (and therefore the compiled `.spv` name)
/// self-describing and is the recommended convention. Both forms are accepted
/// so either layout compiles; every other `.slang` file (e.g. a shared
/// `common.slang`) is treated as an include and skipped.
fn classify(src: &Path, shader_dir: &Path) -> Option<(String, &'static str)> {
    let rel = src.strip_prefix(shader_dir).ok()?;
    let stem = rel.to_str()?.strip_suffix(".slang")?;

    // `<name>.<stage>.slang`: the stage is the final extension.
    if let Some((_, stage_key)) = stem.rsplit_once('.')
        && let Some(stage) = stage_from_key(stage_key)
    {
        return Some((stem.replace('\\', "/"), stage));
    }

    // `<stage>.slang`: the stage is the file name itself.
    let file = stem.rsplit(['/', '\\']).next()?;
    let stage = stage_from_key(file)?;
    Some((stem.replace('\\', "/"), stage))
}

/// Map a stage key (e.g. `vert`) to its Slang `-stage` value.
fn stage_from_key(key: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find_map(|(suffix, stage)| (*suffix == key).then_some(*stage))
}

fn find_slangc() -> Option<PathBuf> {
    if let Some(explicit) = env::var_os("SLANGC") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Some(path_var) = env::var_os("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join("slangc");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    SLANGC_HINTS
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

fn compile(slangc: &Path, src: &Path, out: &Path, stage: &str, debug: bool, include_dir: &Path) {
    let mut cmd = Command::new(slangc);
    cmd.arg(src)
        // SPIR-V is what the Vulkan backend of SDL3 GPU consumes.
        .args(["-target", "spirv"])
        // SDL3 passes `main` as the entrypoint; see `src/shaders.rs`.
        .args(["-entry", "main", "-stage", stage])
        // Allow `import`/`#include` of sibling shaders.
        .arg("-I")
        .arg(include_dir)
        // SDL_GPU wants column-major matrices (matches `glam::Mat4`) ...
        .arg("-matrix-layout-column-major")
        // ... and GL/std layouts for storage buffers.
        .arg("-fvk-use-gl-layout")
        .args(["-o"])
        .arg(out);

    if debug {
        cmd.args(["-g", "-O0"]);
    } else {
        cmd.arg("-O3");
    }

    let output = cmd
        .output()
        .unwrap_or_else(|err| panic!("failed to run `{}`: {err}", slangc.display()));

    if !output.status.success() {
        panic!(
            "Slang failed to compile `{}`:\n{}{}",
            src.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

/// Generate `$OUT_DIR/shaders.rs` with one `&[u8]` constant per shader.
fn write_manifest(out_dir: &Path, compiled: &[(String, String)]) {
    let mut rs = String::from("// @generated by build.rs - do not edit.\n\n");

    for (key, rel) in compiled {
        let _ = writeln!(
            rs,
            "/// `{key}` (compiled with [slangc](https://shader-slang.com)).\npub const {}: &[u8] = include_bytes!(r#\"{rel}\"#);\n",
            ident(key),
        );
    }

    let _ = writeln!(
        rs,
        "/// Every embedded shader as `(key, spv_bytes)`, where `key` is the path \
         relative to `shaders/` without the `.slang` extension (e.g. `\"basic.vert\"`).\npub const ALL: &[(&str, &[u8])] = &[",
    );
    for (key, _) in compiled {
        let _ = writeln!(rs, "    ({key:?}, {}),", ident(key));
    }
    let _ = writeln!(
        rs,
        "];\n\n/// Look up an embedded shader by key. A trailing `.spv` is optional.\npub fn get(name: &str) -> Option<&'static [u8]> {{\n    let name = name.strip_suffix(\".spv\").unwrap_or(name);\n    ALL.iter().find(|(key, _)| *key == name).map(|(_, bytes)| *bytes)\n}}",
    );

    fs::write(out_dir.join("shaders.rs"), rs).expect("failed to write generated shaders.rs");
}

/// `basic.vert` -> `BASIC_VERT` (valid Rust const identifier).
fn ident(key: &str) -> String {
    let mut ident = String::with_capacity(key.len());
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            ident.push(ch.to_ascii_uppercase());
        } else {
            ident.push('_');
        }
    }
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }
    ident
}
