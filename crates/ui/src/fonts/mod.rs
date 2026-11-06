//! Font definitions and the on-demand script coverage policy; the egui calls sit in `runtime::egui::fonts`.

use egui::FontFamily;

/// A font handle: the family a run is set in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontRef(FontFamily);

impl FontRef {
    /// The interface face.
    pub fn proportional() -> FontRef {
        FontRef(FontFamily::Proportional)
    }

    /// The monospace face.
    pub fn monospace() -> FontRef {
        FontRef(FontFamily::Monospace)
    }

    /// A family registered under `name`.
    pub fn named(name: &str) -> FontRef {
        FontRef(FontFamily::Name(name.into()))
    }

    pub(crate) fn family(&self) -> FontFamily {
        self.0.clone()
    }
}
