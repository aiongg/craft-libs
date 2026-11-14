//! `TypeBar`: the type tool's options bar, the family, style, size, alignment, colour and panels buttons in one
//! strip.

use craft_behavior::EditEvent;
use craft_ui::contract::{NoValue, Outcome};
use craft_ui::patterns::form_dialogs::HostRequests;
use egui::{Response, Ui};

use crate::ui::character::{CharacterChange, CharacterValues, ParagraphChange, ParagraphValues};
use crate::ui::length::DocUnit;
use crate::ui::type_options::{TextAntiAlias, TextOrientation};

#[cfg(feature = "font-picker")]
use crate::ui::character::FontPickerSource;

/// One change from the type bar.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TypeBarChange {
    /// A character field.
    Character(CharacterChange),
    /// A paragraph field.
    Paragraph(ParagraphChange),
    /// The anti-aliasing method.
    AntiAlias(TextAntiAlias),
    /// The text orientation.
    Orientation(TextOrientation),
}

/// What the type bar did this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct TypeBarOutput {
    /// The bar's response.
    pub response: Response,
    /// The edit events of this pass.
    pub edits: Vec<EditEvent<TypeBarChange>>,
    /// The family under the pointer in the open font list, for a live preview.
    pub preview_family: Option<String>,
    /// The family the pointer rested on in the open font list.
    pub rested_family: Option<String>,
    /// The colour well was clicked.
    pub requests: HostRequests,
    /// The Warp Text button was pressed.
    pub warp: bool,
    /// The Character and Paragraph panels button was pressed.
    pub panels: bool,
}

impl Outcome for TypeBarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The options bar of the type tool: the family and style, the size, the anti-aliasing menu, the alignment buttons,
/// the colour well, Warp Text and the panels button. Show it as children of the app's `Bar::new(label,
/// BarKind::Options)`. The alignment buttons show the vertical glyphs when `orientation` is `Some(Vertical)`.
#[must_use = "show the bar with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
#[allow(dead_code)] // P3(tools-w5): tl.T20
pub struct TypeBar<'a> {
    character: &'a CharacterValues,
    paragraph: &'a ParagraphValues,
    families: &'a [String],
    styles: &'a [String],
    /// The unit the length fields show. Default points.
    #[prop]
    unit: DocUnit,
    /// The text's orientation; `None` hides the orientation toggle.
    #[prop]
    orientation: Option<TextOrientation>,
    /// The anti-aliasing method; `None` hides the menu.
    #[prop]
    anti_alias: Option<TextAntiAlias>,
    /// The methods the anti-aliasing menu offers.
    #[prop]
    anti_alias_methods: &'a [TextAntiAlias],
    /// Warp Text is available.
    #[prop]
    can_warp: bool,
    #[cfg(feature = "font-picker")]
    picker: Option<FontPickerSource<'a>>,
}

impl<'a> TypeBar<'a> {
    /// The bar over `character` and `paragraph` values, with the installed `families` and the family's `styles`.
    pub fn new(character: &'a CharacterValues, paragraph: &'a ParagraphValues, families: &'a [String], styles: &'a [String]) -> Self {
        TypeBar {
            character,
            paragraph,
            families,
            styles,
            unit: DocUnit::points(),
            orientation: None,
            anti_alias: None,
            anti_alias_methods: &TextAntiAlias::BAR,
            can_warp: false,
            #[cfg(feature = "font-picker")]
            picker: None,
        }
    }

    /// The family field is craft-font-manager's font picker over `source`.
    #[cfg(feature = "font-picker")]
    pub fn font_picker(mut self, source: FontPickerSource<'a>) -> Self {
        self.picker = Some(source);
        self
    }

    /// Shows the bar and returns the changes the user made.
    pub fn show(self, _ui: &mut Ui) -> TypeBarOutput {
        todo!("tl.T20")
    }
}
