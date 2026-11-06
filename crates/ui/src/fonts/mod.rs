//! Font definitions and the on-demand script coverage policy; the egui calls sit in `runtime::egui::fonts`.

use crate::runtime::egui::fonts as egui_fonts;
use crate::runtime::layout_memory;
use egui::{Context, FontDefinitions, FontFamily};

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

/// Gives the UI `definitions`, as when on-demand script coverage loads the glyphs of another script. Text
/// measured in this pass used the old metrics, so the pass asks for one discard pass: layout memory and the
/// new metrics agree when the frame is shown.
pub fn load(ctx: &Context, definitions: FontDefinitions) {
    egui_fonts::set_definitions(ctx, definitions);
    layout_memory::ask_discard(ctx);
}
