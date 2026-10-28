//! Embeds the candidate interface faces of craft-fonts when `CRAFT_FONTS_DIR` points at a checkout
//! (`fonts/ui-candidates-manifest.txt`; see craft-fonts' `docs/integration.md`). Without it the
//! gallery builds with an empty list and the face specimen says so.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_DIR");
    let mut src = String::from("pub static UI_CANDIDATES: &[Candidate] = &[\n");
    if let Some(dir) = std::env::var_os("CRAFT_FONTS_DIR").map(PathBuf::from) {
        match candidates(&dir) {
            Ok(entries) => src.push_str(&entries),
            Err(e) => println!("cargo::warning=building without the candidate faces: CRAFT_FONTS_DIR={}: {e}", dir.display()),
        }
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("ui_candidates.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

/// One `Candidate { .. }` initialiser per manifest line.
fn candidates(dir: &Path) -> Result<String, String> {
    let manifest = dir.join("fonts/ui-candidates-manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let fields: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, ..] = fields.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let _ = writeln!(out, "    Candidate {{ family: {family:?}, style: {style:?}, bytes: include_bytes!({:?}) }},", path.display().to_string());
    }
    Ok(out)
}
