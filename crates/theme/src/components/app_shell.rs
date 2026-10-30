//! The app shell: the area behind the document and the zone that reveals hidden chrome. The shell
//! paints nothing else; bars, rails and panels paint themselves.
//!
//! The backdrop is a surface, not text or a mark, so no contrast pair names it: the document's
//! own contrast is the document's. The body-only backdrop is a fixed black that no theme
//! overrides, like the desktops' Close reds of the title bar.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::values;

/// App shell tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AppShellTokens {
    /// Width of the edge reveal zone along the start, end and top window edges in body-only mode.
    pub edge_reveal: f32,
    /// Behind the document in normal mode: `surface.canvas`.
    pub backdrop: Rgba,
    /// Behind the document in body-only mode: black. Fixed.
    pub body_only_backdrop: Rgba,
}

/// An overridable app shell token. Paths: `backdrop`, e.g. `app_shell.backdrop`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AppShellField {
    /// `app_shell.backdrop`
    Backdrop,
}

impl Family for AppShellTokens {
    type Field = AppShellField;
    const FIELDS: &'static [AppShellField] = &[AppShellField::Backdrop];

    fn defaults(s: &Semantic<'_>) -> AppShellTokens {
        AppShellTokens {
            edge_reveal: values::APP_SHELL_EDGE_REVEAL,
            backdrop: s.color.surface.canvas,
            body_only_backdrop: values::APP_SHELL_BODY_ONLY_BACKDROP,
        }
    }

    fn path(_: AppShellField) -> String {
        "backdrop".to_owned()
    }

    fn from_path(path: &str) -> Option<AppShellField> {
        (path == "backdrop").then_some(AppShellField::Backdrop)
    }

    fn kind(_: AppShellField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: AppShellField) -> OverrideValue {
        OverrideValue::Color(self.backdrop)
    }

    fn set(&mut self, _: AppShellField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            self.backdrop = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
