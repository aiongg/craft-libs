//! Design-system lint for Craft apps.
//!
//! The clippy half is the template `clippy.toml` entries ([`CLIPPY_TEMPLATE`]),
//! the check that an app's `clippy.toml` carries them ([`check_clippy`]) and the
//! writer that merges them in ([`write_clippy`]). Apps call these from their
//! `xtask`; the binary exposes the same operations.

mod clippy;

pub use clippy::{ClippyDrift, DriftKind, check_clippy, clippy_drift, merge_clippy, write_clippy};

/// The `disallowed-methods` and `disallowed-types` entries every app's `clippy.toml` carries.
pub const CLIPPY_TEMPLATE: &str = include_str!("../clippy.template.toml");

/// The `[workspace.lints.clippy]` lines an app adds to its workspace `Cargo.toml`.
pub const LINTS_TEMPLATE: &str = include_str!("../lints.template.toml");

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
