//! Design-system lint for Craft apps.
//!
//! The clippy half is the template `clippy.toml` entries ([`CLIPPY_TEMPLATE`]),
//! the check that an app's `clippy.toml` carries them ([`check_clippy`]) and the
//! writer that merges them in ([`write_clippy`]). Apps call these from their
//! `xtask`; the binary exposes the same operations.
//!
//! The syntax half is [`check`]: it parses the configured directories and reports
//! literal sizes and named `Color32` constants as a [`Report`].

mod clippy;
mod config;
mod markers;
mod report;
mod rules;

use std::path::{Path, PathBuf};

pub use clippy::{ClippyDrift, DriftKind, check_clippy, clippy_drift, merge_clippy, write_clippy};
pub use config::{Config, Exemption};
pub use markers::{Marked, analyze_with_markers};
pub use report::{Finding, Report, Severity};
pub use rules::{Analysis, RULES, analyze_source};

/// The `disallowed-methods` and `disallowed-types` entries every app's `clippy.toml` carries.
pub const CLIPPY_TEMPLATE: &str = include_str!("../clippy.template.toml");

/// The `[workspace.lints.clippy]` lines an app adds to its workspace `Cargo.toml`.
pub const LINTS_TEMPLATE: &str = include_str!("../lints.template.toml");

/// Checks the configured paths (relative to `root`) and returns the findings.
///
/// Test files and `#[cfg(test)]` code are skipped unless the config includes them.
/// `// design: allow(<reason>)` markers and `[[exempt]]` paths silence findings; see [`analyze_with_markers`].
/// A file that does not parse is a `parse` finding; an unreadable path is an error.
pub fn check(config: &Config, root: &Path) -> Result<Report, Error> {
    let mut report = Report::default();
    for entry in &config.paths {
        let base = root.join(entry);
        let mut files = Vec::new();
        collect_rust_files(&base, &mut files)?;
        for file in files {
            if !config.include_tests && is_test_path(file.strip_prefix(&base).unwrap_or(&file)) {
                continue;
            }
            let src = std::fs::read_to_string(&file).map_err(|source| Error::Io { path: file.clone(), source })?;
            report.files += 1;
            let area = area_of(&base, &file);
            let shown = file.strip_prefix(root).unwrap_or(&file).to_owned();
            let exempt = |rule: &str| config.exempt.iter().any(|e| e.covers(&shown, rule));
            let marked = analyze_with_markers(&src, config.include_tests, config.deny_unused, &exempt);
            report.markers += marked.markers;
            for mut finding in marked.findings {
                finding.path.clone_from(&shown);
                finding.area.clone_from(&area);
                report.findings.push(finding);
            }
        }
    }
    report.exemptions = config.exempt.len();
    report.findings.sort_by(|a, b| (&a.path, a.line, a.column).cmp(&(&b.path, b.line, b.column)));
    Ok(report)
}

/// Test files: `*_test.rs`, `*_tests.rs`, `tests.rs`, and anything under `tests/` or `benches/`.
fn is_test_path(relative: &Path) -> bool {
    let in_test_dir = relative.parent().is_some_and(|p| p.components().any(|c| matches!(c.as_os_str().to_str(), Some("tests" | "benches"))));
    let name = relative.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    in_test_dir || name == "tests.rs" || name.ends_with("_test.rs") || name.ends_with("_tests.rs")
}

fn area_of(base: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(base).unwrap_or(file);
    let mut parts = relative.components();
    match (parts.next(), parts.next()) {
        (Some(first), Some(_)) => first.as_os_str().to_string_lossy().into_owned(),
        _ => "(root)".to_owned(),
    }
}

/// Every `.rs` file under `path` (or `path` itself), sorted, skipping hidden directories and `target`.
fn collect_rust_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), Error> {
    let io = |source| Error::Io { path: path.to_owned(), source };
    if path.is_file() {
        out.push(path.to_owned());
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path).map_err(io)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>().map_err(io)?;
    entries.sort();
    for entry in entries {
        let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if entry.is_dir() {
            if !name.starts_with('.') && name != "target" {
                collect_rust_files(&entry, out)?;
            }
        } else if entry.extension().is_some_and(|e| e == "rs") {
            out.push(entry);
        }
    }
    Ok(())
}

/// Why a command could not run to completion.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read or written.
    Io { path: std::path::PathBuf, source: std::io::Error },
    /// A TOML file does not parse or does not have the expected shape.
    Toml { path: std::path::PathBuf, message: String },
    /// The app's `clippy.toml` lacks or differs from template entries.
    ClippyDrift(Vec<ClippyDrift>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Toml { path, message } => write!(f, "{}: {message}", path.display()),
            Self::ClippyDrift(drift) => {
                for d in drift {
                    writeln!(f, "{d}")?;
                }
                write!(
                    f,
                    "design-lint: clippy.toml is out of step with the template in {} entries; run `craft-design-lint clippy --write`",
                    drift.len()
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
