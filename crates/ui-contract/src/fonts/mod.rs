//! Font definitions and the on-demand script coverage policy; the egui calls sit in `runtime::egui::fonts`.

mod defaults;
pub(crate) mod document;
mod faces;
mod facts;
mod host;
pub(crate) mod on_demand;
pub(crate) mod script;
pub(crate) mod system;

pub use defaults::{deflated_default_definitions, emoji_icon_font, noto_emoji};
pub use document::{FaceBytes, FaceState, MAX_DOCUMENT_FACE_BYTES};
pub use faces::{FaceList, RegularWeight, set_weight};
pub use facts::StackFacts;
pub(crate) use facts::{peek_stack_facts, refresh_facts, stack_facts};
pub use host::FontHost;
pub use on_demand::{FontCandidates, MAX_SCRIPT_READS, NoCandidates, OnDemandFonts, SystemCandidates, han_for_locale, on_demand_registered};
pub use system::system_font_provider;
pub use system::ui_face::{SYSTEM_UI_OPTICAL_SIZE, SYSTEM_UI_REGULAR, SYSTEM_UI_SEMIBOLD, SystemUiFace, poll_system_ui_face, system_ui_face};

mod coverage;
mod language;
mod served;
mod stack;

pub use coverage::{Missing, Sample, assert_covers, assert_covers_with_system, missing_glyphs};
pub use craft_fonts_build::{EmbeddedFace, ServedFace};
pub use language::{I18nScripts, LanguageScripts};
pub use served::loader::{FetchDone, FetchWake, FontFetcher};
pub use served::loading;
pub use stack::{CjkRows, Emoji, Family, SEMIBOLD, Stack, SystemFonts, TAB_TITLE, UiFonts};

use crate::runtime::egui::fonts as egui_fonts;
use crate::runtime::frame;
use crate::runtime::layout_memory;
use egui::{Context, FontDefinitions, FontFamily};

/// The egui family name of [`FontRef::signature`]; the stack registers it in every build.
const SIGNATURE_FAMILY: &str = "signature";

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

    /// The signature face: always a family egui knows. Its chain is the craft-fonts faces of the signature role,
    /// then the proportional chain; without one in the build, the proportional chain.
    pub fn signature() -> FontRef {
        FontRef::named(SIGNATURE_FAMILY)
    }

    /// A family registered under `name`.
    pub fn named(name: &str) -> FontRef {
        FontRef(FontFamily::Name(name.into()))
    }

    pub(crate) fn from_family(family: FontFamily) -> FontRef {
        FontRef(family)
    }

    /// The egui family that lays the face out, for a layout job an app builds itself: the open paragraph of a text
    /// editor, whose galley its `TextEditorHost` lays out. Content painters lay document runs out through `DocText`,
    /// which takes the `FontRef` itself.
    pub fn family(&self) -> FontFamily {
        self.0.clone()
    }
}

/// Gives the UI `definitions`, as when on-demand script coverage loads the glyphs of another script. Text
/// measured in this pass used the old metrics, so the pass asks for one discard pass: layout memory and the
/// new metrics agree when the frame is shown. Between frames nothing was measured with the old metrics, and
/// the next frame measures with the new ones. What text layout reads of the definitions follows them: framework text
/// in a semibold role draws in the [`SEMIBOLD`] family only when `definitions` have it, else in the proportional one.
pub fn load(ctx: &Context, definitions: FontDefinitions) {
    load_with(ctx, definitions, Vec::new());
}

/// [`load`], with `providers` serving the characters the faces lack to the probe that measures the definitions' line
/// ratio: a stack's own providers.
pub(crate) fn load_with(ctx: &Context, definitions: FontDefinitions, providers: Vec<std::sync::Arc<dyn egui::FontProvider>>) {
    egui_fonts::set_definitions(ctx, definitions);
    if frame::in_pass(ctx) {
        layout_memory::ask_discard(ctx);
    }
}
