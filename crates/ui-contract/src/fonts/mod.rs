//! The font stacks every app installs, their sources (system fonts, served faces, on-demand script coverage), the
//! document faces and what text layout reads of the installed stack. The text engine takes a stack through
//! [`FontHost`].

mod defaults;
pub(crate) mod document;
mod faces;
mod facts;
mod host;
pub(crate) mod on_demand;
pub(crate) mod script;
pub(crate) mod system;

pub(crate) use defaults::deflated_default_definitions;
pub use defaults::{default_stack, emoji_icon_font, noto_emoji};
pub use document::{FaceBytes, FaceState, MAX_DOCUMENT_FACE_BYTES};
pub use faces::{FaceList, RegularWeight, set_weight};
pub use facts::StackFacts;
pub(crate) use facts::{peek_stack_facts, refresh_facts, stack_facts};
pub use host::FontHost;
pub use on_demand::{FontCandidates, MAX_SCRIPT_READS, NoCandidates, OnDemandFonts, SystemCandidates, han_for_locale};
pub use system::system_font_source;
pub use system::ui_face::{SYSTEM_UI_OPTICAL_SIZE, SYSTEM_UI_REGULAR, SYSTEM_UI_SEMIBOLD, SystemUiFace, poll_system_ui_face, system_ui_face};

mod coverage;
mod language;
mod served;
mod stack;

pub use coverage::{Missing, Sample, assert_covers, assert_covers_with_system, missing_glyphs};
pub use craft_fonts_build::{EmbeddedFace, ServedFace};
pub use language::{I18nScripts, LanguageScripts};
pub use served::loader::{FetchDone, FetchWake, FontFetcher};
pub use stack::{CjkRows, Emoji, Family, SEMIBOLD, Stack, SystemFonts, TAB_TITLE, UiFonts};

use craft_theme::FontSlot;
use craft_ui_rt::FontFace;

/// The family name of [`FontRef::signature`]; the stack registers it in every build.
const SIGNATURE_FAMILY: &str = "signature";

/// A font handle: the family a run is set in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontRef(FontFace);

impl FontRef {
    /// The interface face.
    pub fn proportional() -> FontRef {
        FontRef(FontFace::Slot(FontSlot::Ui))
    }

    /// The monospace face.
    pub fn monospace() -> FontRef {
        FontRef(FontFace::Slot(FontSlot::Mono))
    }

    /// The signature face: always a family the stack has. Its chain is the craft-fonts faces of the signature role,
    /// then the proportional chain; without one in the build, the proportional chain.
    pub fn signature() -> FontRef {
        FontRef::named(SIGNATURE_FAMILY)
    }

    /// A family registered under `name`. [`SEMIBOLD`] and [`TAB_TITLE`] name the semibold and condensed slots.
    pub fn named(name: &str) -> FontRef {
        FontRef(match name {
            SEMIBOLD => FontFace::Slot(FontSlot::UiSemibold),
            TAB_TITLE => FontFace::Slot(FontSlot::UiCondensed),
            name => FontFace::Family(name.into()),
        })
    }

    /// The stack family that sets the face.
    pub fn face(&self) -> &FontFace {
        &self.0
    }

    pub(crate) fn into_face(self) -> FontFace {
        self.0
    }

    /// The egui family that lays the face out, for code that still builds egui layout jobs.
    pub fn family(&self) -> egui::FontFamily {
        match &self.0 {
            FontFace::Slot(FontSlot::Ui) => egui::FontFamily::Proportional,
            FontFace::Slot(FontSlot::Mono) => egui::FontFamily::Monospace,
            FontFace::Slot(FontSlot::UiSemibold) => egui::FontFamily::Name(SEMIBOLD.into()),
            FontFace::Slot(FontSlot::UiCondensed) => egui::FontFamily::Name(TAB_TITLE.into()),
            FontFace::Family(name) => egui::FontFamily::Name(name.as_ref().into()),
        }
    }
}

impl From<FontRef> for FontFace {
    fn from(font: FontRef) -> FontFace {
        font.0
    }
}
