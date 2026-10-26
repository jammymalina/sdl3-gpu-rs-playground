//! Build-time Slang → SPIR-V compilation for SDL3's GPU API.
//!
//! Every `.slang` file under [`SHADER_DIR`] that declares at least one
//! `[shader("stage")]` entry point is treated as a shader: all stages of a
//! shader live together in the one file (e.g. `vertex_main` + `geom_main` +
//! `fragment_main` in `flat_color.slang`), and `slangc` is run once per entry
//! point with the outermost `[shader("...")]` attribute picking the stage and
//! the annotated function picking the `-entry` name.
//!
//! Files without any `[shader("...")]` attribute are importable modules (e.g.
//! `module common_types;` + `import common_types;`): they are never compiled
//! on their own (but still watched for changes) and are consumed via Slang's
//! `import`. Module members are private by default, so mark everything you
//! hand out as `public`.
//!
//! `slangc` is located through, in order:
//!
//! 1. the `SLANGC` environment variable,
//! 2. `slangc` on `PATH`,
//! 3. well-known install locations (see [`SLANGC_HINTS`]).

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

/// Recognised `[shader("...")]` stage values, mapped to canonical
/// `slangc -stage` names (frequent aliases included).
const STAGES: &[(&str, &str)] = &[
    ("vertex", "vertex"),
    ("hull", "hull"),
    ("tesscontrol", "hull"),
    ("domain", "domain"),
    ("tesseval", "domain"),
    ("geometry", "geometry"),
    ("fragment", "fragment"),
    ("pixel", "fragment"),
    ("compute", "compute"),
    ("raygeneration", "raygeneration"),
    ("intersection", "intersection"),
    ("anyhit", "anyhit"),
    ("closesthit", "closesthit"),
    ("miss", "miss"),
    ("callable", "callable"),
    ("mesh", "mesh"),
    ("amplification", "amplification"),
    ("task", "amplification"),
];

/// Well-known locations for `slangc` when it is not on `PATH`.
const SLANGC_HINTS: &[&str] = &[
    // Arch AUR: shader-slang-bin
    "/opt/shader-slang-bin/bin/slangc",
    // Arch AUR: shader-slang
    "/opt/shader-slang/bin/slangc",
    // Manual install
    "/usr/local/bin/slangc",
];

/// One `[shader("stage")]` entry point discovered in a source file.
struct EntryPoint {
    /// Stage string taken verbatim from the attribute, e.g. `vertex` or `pixel`.
    stage: String,
    /// Name of the annotated function, e.g. `vertex_main`.
    name: String,
}

/// One compiled entry point, recorded for the generated manifest.
struct Compiled {
    /// Manifest key: `<path below shaders/ without extension>.<stage>`.
    key: String,
    /// Path of the source file below `shaders/` without extension.
    name: String,
    /// Canonical `slangc -stage` name.
    stage: &'static str,
    /// Name of the annotated entry-point function.
    entrypoint: String,
    /// Path of the compiled `.spv`, relative to the generated manifest.
    spv_rel: String,
}

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
    // Start from a clean slate so blobs of deleted or renamed shaders don't linger.
    let spv_dir = out_dir.join("shaders");
    let _ = fs::remove_dir_all(&spv_dir);
    fs::create_dir_all(&spv_dir).expect("failed to create shader output directory");

    // Debug builds keep debug info and skip optimization; release builds optimize.
    let debug = env::var("PROFILE").as_deref() != Ok("release");

    let mut sources = Vec::new();
    collect_sources(&shader_dir, &mut sources);
    sources.sort();

    let mut compiled: Vec<Compiled> = Vec::new();
    for src in &sources {
        println!("cargo:rerun-if-changed={}", src.display());

        let text =
            fs::read_to_string(src).unwrap_or_else(|err| panic!("failed to read `{}`: {err}", src.display()));
        // Module (its file declares no shader stages) - compiled indirectly
        // through the shaders that `import` it.
        let entry_points = entry_points(&text);
        if entry_points.is_empty() {
            continue;
        }

        // Key prefix: path below `shaders/` without the `.slang` extension,
        // with `/` separators (e.g. `lights/point_lit`).
        let rel = src.strip_prefix(&shader_dir).expect("source under SHADER_DIR");
        let stem = rel
            .to_string_lossy()
            .strip_suffix(".slang")
            .expect("collect_sources only returns .slang files")
            .replace('\\', "/");

        for entry_point in entry_points {
            let Some(stage) = stage_from_key(&entry_point.stage) else {
                panic!(
                    "`{}` declares an unknown stage in `[shader(\"{}\")]`.\n\
                     Valid stages: {}",
                    src.display(),
                    entry_point.stage,
                    STAGES.iter().map(|(key, _)| *key).collect::<Vec<_>>().join(", ")
                );
            };

            // One SPIR-V file per (shader, stage): `shaders/flat_color.vertex.spv`.
            let key = format!("{stem}.{stage}");
            let out = spv_dir.join(format!("{key}.spv"));
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).expect("failed to create shader output subdirectory");
            }
            compile(&slangc, src, &out, stage, &entry_point.name, debug, &shader_dir);

            // Path is relative to the generated `shaders.rs` (which lives in OUT_DIR).
            compiled.push(Compiled {
                key: key.clone(),
                name: stem.clone(),
                stage,
                entrypoint: entry_point.name.clone(),
                spv_rel: format!("shaders/{key}.spv"),
            });
        }
    }

    if compiled.is_empty() {
        println!(
            "cargo:warning=no shader entry points found in `{SHADER_DIR}/` \
             (expected `[shader(\"...\")]`-annotated functions; files without \
             them are treated as importable modules)"
        );
    }

    check_unique_idents(&compiled);
    write_manifest(&out_dir, &compiled);
}

/// Map a `[shader("...")]` stage to its canonical `slangc -stage` value.
fn stage_from_key(key: &str) -> Option<&'static str> {
    STAGES.iter().find_map(|(suffix, stage)| (*suffix == key).then_some(*stage))
}

/// Fail with a clear message if two entry points would generate the same
/// constant (for example two `[shader("vertex")]` functions in one file).
fn check_unique_idents(compiled: &[Compiled]) {
    let mut idents: Vec<(String, &str)> =
        compiled.iter().map(|c| (ident(&c.key), c.key.as_str())).collect();
    idents.sort();
    for pair in idents.windows(2) {
        if pair[0].0 == pair[1].0 {
            panic!("shader key collision: `{}` overwrites with the same identifier", pair[0].1);
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

/// Discover every `[shader("stage")]` entry point in a Slang source file.
///
/// Scans the (comment-stripped) source for the attribute and then walks
/// forward to the first `(` outside of any further `[...]` annotation; the
/// identifier immediately before that paren is the entry-point function:
///
/// ```text
/// [shader("vertex")] VertexOutput vertex_main() { ... }
/// [shader("geometry")]
/// [maxvertexcount(3)]
/// void geom_main(triangle VertexOutput in[3], inout TriangleStream<GeomOutput> out) { ... }
/// ```
///
/// An attribute not followed by a function signature (e.g. on a `struct`) is
/// ignored; a file without any `[shader("...")]` attribute is an importable
/// module and gets no entry points.
fn entry_points(src: &str) -> Vec<EntryPoint> {
    let text = strip_comments(src);
    let bytes = text.as_bytes();

    let mut entries = Vec::new();
    let mut pos = 0;

    while let Some(rel) = text[pos..].find("[shader(") {
        let attr = pos + rel;

        // The stage is the string literal inside the attribute.
        let Some(q0) = text[attr..].find('"').map(|rel| attr + rel) else {
            break;
        };
        let Some(q1) = text[q0 + 1..].find('"').map(|rel| q0 + 1 + rel) else {
            break;
        };
        let stage = text[q0 + 1..q1].to_owned();

        // Walk from the attribute end to the entry function: skip comments,
        // whitespace and any further `[...]` annotations, and stop at the
        // first paren at attribute-nesting level zero.
        let mut scan = q1 + 1;
        let mut end = scan;
        let mut depth = 0usize;
        let mut stop = false;
        while scan < bytes.len() {
            match bytes[scan] {
                b'[' => depth += 1,
                b']' => depth = depth.saturating_sub(1),
                b'(' if depth == 0 => {
                    end = scan;
                    stop = true;
                    break;
                }
                // `{` or `;` first means the attribute was not attached to a
                // function definition (e.g. a struct); ignore it.
                b'{' | b';' if depth == 0 => break,
                _ => {}
            }
            scan += 1;
        }

        if stop {
            // Entry name: identifier immediately before the paren, ignoring
            // whitespace between the name and the argument list.
            let mut name_end = end;
            while name_end > 0 && bytes[name_end - 1].is_ascii_whitespace() {
                name_end -= 1;
            }
            let mut name_start = name_end;
            while name_start > 0
                && (bytes[name_start - 1].is_ascii_alphanumeric() || bytes[name_start - 1] == b'_')
            {
                name_start -= 1;
            }
            if name_start < name_end {
                entries.push(EntryPoint {
                    stage,
                    name: text[name_start..name_end].to_owned(),
                });
            }
        }

        pos = q1 + 1;
    }

    // Deterministic manifest order no matter how the author ordered stages.
    entries.sort_by(|a, b| (&a.stage, &a.name).cmp(&(&b.stage, &b.name)));
    entries.dedup_by(|a, b| a.stage == b.stage && a.name == b.name);
    entries
}

/// Remove `// ...` and `/* ... */` comments so attribute scanning never treats
/// commented-out shader annotations as entries. String literals are copied
/// verbatim, so a `//` inside `"http://..."` is not mistaken for a comment.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                out.push('"');
                while let Some(c) = chars.next() {
                    out.push(c);
                    match c {
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                out.push(escaped);
                            }
                        }
                        '"' | '\n' => break,
                        _ => {}
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                // Keep the newline so error positions stay aligned.
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next(); // consume '*'
                loop {
                    match chars.next() {
                        Some('*') if chars.peek() == Some(&'/') => {
                            chars.next();
                            break;
                        }
                        Some('\n') => out.push('\n'),
                        Some(_) => {}
                        None => break, // unterminated block comment
                    }
                }
            }
            _ => out.push(ch),
        }
    }

    out
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

fn compile(slangc: &Path, src: &Path, out: &Path, stage: &str, entrypoint: &str, debug: bool, include_dir: &Path) {
    let mut cmd = Command::new(slangc);
    cmd.arg(src)
        // SPIR-V is what the Vulkan backend of SDL3 GPU consumes.
        .args(["-target", "spirv"])
        // The entry point function name from its `[shader("...")]` attribute.
        .args(["-entry", entrypoint, "-stage", stage])
        // Allow `import`/`#include` of sibling shaders and modules.
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
            "Slang failed to compile `{}` (stage `{stage}`, entry `{entrypoint}`):\n{}{}",
            src.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    // Surface successful-build diagnostics (e.g. `E39029`) in cargo's output.
    for line in String::from_utf8_lossy(&output.stderr).lines().filter(|l| !l.trim().is_empty()) {
        println!("cargo:warning={line}");
    }
}

/// Generate `$OUT_DIR/shaders.rs`: one `&[u8]` constant per compiled entry
/// point plus a `Shader` metadata struct and the `ALL` table for wiring the
/// bytes, entry-point names and stages into the Rust side.
fn write_manifest(out_dir: &Path, compiled: &[Compiled]) {
    let mut rs = String::from("// @generated by build.rs - do not edit.\n\n");

    rs.push_str(
        "/// One compiled shader entry point: its metadata and SPIR-V bytes.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         pub struct Shader {\n\
         \x20   /// `<name>.<stage>`, e.g. `\"flat_color.vertex\"`.\n\
         \x20   pub key: &'static str,\n\
         \x20   /// Path of the source file below `shaders/` without extension, e.g.\n\
         \x20   /// `\"flat_color\"` or `\"lights/point_lit\"`.\n\
         \x20   pub name: &'static str,\n\
         \x20   /// Canonical Slang stage (`[shader(\"pixel\")]` is stored as `\"fragment\"`),\n\
         \x20   /// e.g. `\"vertex\"`, `\"geometry\"`, `\"fragment\"`.\n\
         \x20   pub stage: &'static str,\n\
         \x20   /// Name of the annotated entry-point function.\n\
         \x20   pub entrypoint: &'static str,\n\
         \x20   /// Compiled SPIR-V bytes.\n\
         \x20   pub bytes: &'static [u8],\n\
         }\n\n",
    );

    for c in compiled {
        let _ = writeln!(
            rs,
            "/// `{key}`: SPIR-V, compiled from `{name}.slang` (stage `{stage}`).\n\
             pub const {}: &[u8] = include_bytes!(r#\"{}\"#);\n",
            ident(&c.key),
            c.spv_rel,
            key = c.key,
            name = c.name,
            stage = c.stage,
        );
    }

    rs.push_str(
        "/// Every accepted `[shader(\"...\")]` stage spelling and the canonical stage it maps to\n\
         /// (e.g. `(\"pixel\", \"fragment\")`).\n\
         pub static STAGE_ALIASES: &[(&str, &str)] = &[\n",
    );
    for (alias, canonical) in STAGES {
        let _ = writeln!(rs, "    ({alias:?}, {canonical:?}),");
    }
    rs.push_str("];\n\n");

    rs.push_str("/// Every compiled shader entry point.\npub static ALL: &[Shader] = &[\n");
    for c in compiled {
        let _ = writeln!(
            rs,
            "    Shader {{ key: {key:?}, name: {name:?}, stage: {stage:?}, \
             entrypoint: {entrypoint:?}, bytes: {ident} }},",
            key = c.key,
            name = c.name,
            stage = c.stage,
            entrypoint = c.entrypoint,
            ident = ident(&c.key),
        );
    }
    rs.push_str("];\n");

    fs::write(out_dir.join("shaders.rs"), rs).expect("failed to write generated shaders.rs");
}

/// `flat_color.vertex` -> `FLAT_COLOR_VERTEX` (valid Rust const identifier).
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
