//! `codemod-v4 [--dry-run] <crate dir>...`: applies contract-rt.md section 15.11's rules 1-24 to the `.rs` files below
//! each crate's `src`, `tests`, `benches` and `examples`, and prints the counts per rule and the skipped sites by class.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codemod_v4::{Options, Skip, transform};

fn package_name(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package && let Some(rest) = line.strip_prefix("name") {
            return rest.trim_start().strip_prefix('=').map(|v| v.trim().trim_matches('"').to_owned());
        }
    }
    None
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn main() -> ExitCode {
    let mut dry = false;
    let mut crates = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--dry-run" => dry = true,
            "-h" | "--help" => {
                println!("usage: codemod-v4 [--dry-run] <crate dir>...");
                return ExitCode::SUCCESS;
            }
            _ => crates.push(PathBuf::from(arg)),
        }
    }
    if crates.is_empty() {
        eprintln!("usage: codemod-v4 [--dry-run] <crate dir>...");
        return ExitCode::FAILURE;
    }
    let mut totals: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut skips: BTreeMap<&'static str, Vec<(String, Skip)>> = BTreeMap::new();
    let (mut changed, mut seen, mut failed) = (0usize, 0usize, 0usize);
    for dir in &crates {
        let options = Options { inside_contract: package_name(dir).is_some_and(|n| n == "craft-ui-contract") };
        let mut files = Vec::new();
        for sub in ["src", "tests", "benches", "examples"] {
            rust_files(&dir.join(sub), &mut files);
        }
        for file in files {
            let Ok(src) = std::fs::read_to_string(&file) else { continue };
            seen += 1;
            let shown = file.display().to_string();
            let out = match transform(&src, &options) {
                Ok(out) => out,
                Err(e) => {
                    println!("{shown}: parse error, left alone: {e}");
                    failed += 1;
                    continue;
                }
            };
            if out.text != src {
                changed += 1;
                let counts: Vec<String> = out.counts.iter().map(|(rule, n)| format!("{rule} x{n}")).collect();
                println!("{shown}: {}", counts.join(", "));
                for (rule, n) in &out.counts {
                    *totals.entry(rule).or_default() += n;
                }
                if !dry && let Err(e) = std::fs::write(&file, &out.text) {
                    eprintln!("{shown}: {e}");
                    failed += 1;
                }
            }
            for skip in out.skips {
                skips.entry(skip.class).or_default().push((shown.clone(), skip));
            }
        }
    }
    println!(
        "\n{} {changed} of {seen} files{}",
        if dry { "would change" } else { "changed" },
        if failed > 0 { format!(" ({failed} failed)") } else { String::new() }
    );
    println!("\nEdits per rule:");
    for (rule, n) in &totals {
        println!("  {rule}: {n}");
    }
    println!("\nSkipped sites by class:");
    for (class, sites) in &skips {
        println!("  {class}: {}", sites.len());
        for (file, skip) in sites {
            println!("    {file}:{}: {}", skip.line, skip.detail);
        }
    }
    if failed > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
