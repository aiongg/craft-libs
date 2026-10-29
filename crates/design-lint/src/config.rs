//! `design-lint.toml`: which directories the checker reads.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::Error;

/// What to check. Paths are relative to the root passed to [`crate::check`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Config {
    /// Directories or files to check.
    pub paths: Vec<PathBuf>,
    /// Also check test files and `#[cfg(test)]` code.
    pub include_tests: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    lint: RawLint,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLint {
    paths: Vec<PathBuf>,
    #[serde(default, rename = "include-tests")]
    include_tests: bool,
}

impl Config {
    /// A config that checks `paths` and skips tests.
    pub fn new(paths: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self { paths: paths.into_iter().map(Into::into).collect(), include_tests: false }
    }

    /// Reads a `design-lint.toml` file.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io { path: path.to_owned(), source })?;
        Self::parse(path, &text)
    }

    /// Parses config text; `origin` names the file in errors.
    pub fn parse(origin: &Path, text: &str) -> Result<Self, Error> {
        let raw: Raw = toml::from_str(text).map_err(|e| Error::Toml { path: origin.to_owned(), message: e.to_string() })?;
        Ok(Self { paths: raw.lint.paths, include_tests: raw.lint.include_tests })
    }
}
