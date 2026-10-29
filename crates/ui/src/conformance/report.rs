//! [`Report`]: the keyed side-by-side text of a failed check.

use std::fmt::{self, Write as _};

use super::compare::Diff;

/// Rows shown per section before `… N more`.
const MAX_ROWS: usize = 30;
const MISSING: &str = "<missing>";

/// A group of keys that differ between the reference and the app.
#[derive(Clone, Debug)]
pub(super) struct Section {
    /// `style (dark)`, `tokens (light)`, `context`.
    pub title: String,
    pub diffs: Vec<Diff>,
}

/// What differed in one case between the reference appearance and the app's, with the
/// differences the app allowed and hints at their likely cause. Displays as the keyed
/// side-by-side text.
#[derive(Clone, Debug)]
pub struct Report(Box<Details>);

impl Report {
    pub(super) fn new(details: Details) -> Self {
        Report(Box::new(details))
    }

    pub(super) fn details_mut(&mut self) -> &mut Details {
        &mut self.0
    }

    /// The name of the case that failed.
    pub fn case(&self) -> &str {
        &self.0.case
    }

    /// The keys that differ, as `section: key`, for tests of the report itself.
    pub fn keys(&self) -> impl Iterator<Item = String> + '_ {
        self.0.sections.iter().flat_map(|s| s.diffs.iter().map(move |d| format!("{}: {}", s.title, d.key)))
    }

    /// The header and the first `rows` lines after it, for a summary over many cases.
    pub(super) fn excerpt(&self, rows: usize) -> String {
        let text = self.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let mut out: Vec<String> = lines.iter().take(rows + 1).map(|l| (*l).to_owned()).collect();
        if lines.len() > rows + 1 {
            out.push(format!("… {} more lines", lines.len() - rows - 1));
        }
        out.join("\n")
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Details {
    pub(super) case: String,
    pub(super) mode: String,
    pub(super) sections: Vec<Section>,
    /// `(key prefix, reason, differing keys)` of every allowance that hid a difference.
    pub(super) allowed: Vec<(String, String, usize)>,
    /// `(key prefix, reason)` of allowances that matched no difference.
    pub(super) unused: Vec<(String, String)>,
    pub(super) hints: Vec<String>,
}

impl fmt::Display for Details {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "conformance failed: {} ({})", self.case, self.mode)?;
        for section in &self.sections {
            section.write(f)?;
        }
        if !self.allowed.is_empty() {
            writeln!(f, "allowed ({}):", self.allowed.len())?;
            for (prefix, reason, keys) in &self.allowed {
                writeln!(f, "  {prefix}  {reason:?}  ({keys} key{})", if *keys == 1 { "" } else { "s" })?;
            }
        }
        for (prefix, reason) in &self.unused {
            writeln!(f, "unused allowance: {prefix}  {reason:?} (matches no difference; remove it)")?;
        }
        for hint in &self.hints {
            writeln!(f, "hint: {hint}")?;
        }
        Ok(())
    }
}

impl Section {
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.diffs.is_empty() {
            return writeln!(f, "{}: same", self.title);
        }
        let n = self.diffs.len();
        writeln!(f, "{}: {n} difference{}", self.title, if n == 1 { "" } else { "s" })?;
        let shown = &self.diffs[..n.min(MAX_ROWS)];
        let value = |v: &Option<String>| v.clone().unwrap_or_else(|| MISSING.to_owned());
        let key_width = shown.iter().map(|d| d.key.chars().count()).max().unwrap_or(0);
        let reference_width = shown.iter().map(|d| value(&d.reference).chars().count()).max().unwrap_or(0).max("^ reference".len());
        let mut out = String::new();
        for d in shown {
            let _ = writeln!(out, "  {:key_width$}  {:reference_width$}  {}", d.key, value(&d.reference), value(&d.app));
        }
        let _ = writeln!(out, "  {:key_width$}  {:reference_width$}  ^ app", "", "^ reference");
        f.write_str(&out)?;
        if n > MAX_ROWS {
            writeln!(f, "  … {} more", n - MAX_ROWS)?;
        }
        Ok(())
    }
}
