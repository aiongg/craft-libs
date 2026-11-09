#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::report::contrast_report;

fn graphite() -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == "craft.graphite").unwrap().clone()
}

fn keys(theme: &ThemeDef) -> Vec<(Option<String>, Mode, Contrast, Size)> {
    contrast_report(theme)
        .iter()
        .map(|r| (r.app.as_ref().map(ToString::to_string), r.appearance.mode, r.appearance.contrast, r.appearance.size))
        .collect()
}

