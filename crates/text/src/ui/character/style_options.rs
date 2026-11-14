//! `TextStyleOptions`: the body of the Character Style and Paragraph Style options dialogs, a name and one include
//! check with a field per row.

use craft_behavior::EditEvent;
use craft_ui::contract::{NoValue, Outcome};
use craft_ui::patterns::form_dialogs::HostRequests;
use craft_ui::runtime::env::Label;
use egui::{Response, Ui};

use super::{CharacterChange, CharacterValues, ParagraphChange, ParagraphValues};
use crate::ui::length::DocUnit;

#[cfg(feature = "font-picker")]
use super::FontPickerSource;

craft_ui::words! {
    /// The words of the style options editor, English by default: the three short alignment names its rows show.
    #[non_exhaustive]
    pub struct TextStyleOptionsWords {
        left: "styleOptions.left" = "Left";
        center: "styleOptions.center" = "Center";
        right: "styleOptions.right" = "Right";
    }
}

/// The key of the colour request the colour well makes.
pub const STYLE_COLOR_KEY: &str = "textStyle.color";

/// An attribute a style may define; the rows of the options editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StyleAttribute {
    /// Family.
    Font,
    /// Size.
    Size,
    /// Leading.
    Leading,
    /// Tracking.
    Tracking,
    /// Baseline shift.
    BaselineShift,
    /// Colour.
    Color,
    /// Faux bold.
    FauxBold,
    /// Faux italic.
    FauxItalic,
    /// Underline.
    Underline,
    /// Strikethrough.
    Strikethrough,
    /// Alignment.
    Align,
    /// First-line indent.
    FirstLineIndent,
    /// Start indent.
    StartIndent,
    /// End indent.
    EndIndent,
    /// Space before.
    SpaceBefore,
    /// Space after.
    SpaceAfter,
    /// Hyphenate.
    Hyphenate,
}

impl StyleAttribute {
    /// The rows of a character style.
    pub const CHARACTER: [StyleAttribute; 10] = [
        StyleAttribute::Font,
        StyleAttribute::Size,
        StyleAttribute::Leading,
        StyleAttribute::Tracking,
        StyleAttribute::BaselineShift,
        StyleAttribute::Color,
        StyleAttribute::FauxBold,
        StyleAttribute::FauxItalic,
        StyleAttribute::Underline,
        StyleAttribute::Strikethrough,
    ];

    /// The rows of a paragraph style.
    pub const PARAGRAPH: [StyleAttribute; 7] = [
        StyleAttribute::Align,
        StyleAttribute::FirstLineIndent,
        StyleAttribute::StartIndent,
        StyleAttribute::EndIndent,
        StyleAttribute::SpaceBefore,
        StyleAttribute::SpaceAfter,
        StyleAttribute::Hyphenate,
    ];
}

/// The values the options editor shows: the style's name, its attribute values, and which attributes it defines.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StyleOptionValues {
    /// The style's name.
    pub name: String,
    /// The character attributes (a paragraph style's character part too).
    pub character: CharacterValues,
    /// The paragraph attributes; `None` for a character style.
    pub paragraph: Option<ParagraphValues>,
    /// The attributes the style defines; the others are inherited.
    pub included: Vec<StyleAttribute>,
}

impl StyleOptionValues {
    /// A style named `name` with `character` values, no paragraph part and no attribute included.
    pub fn new(name: impl Into<String>, character: CharacterValues) -> Self {
        StyleOptionValues { name: name.into(), character, paragraph: None, included: Vec::new() }
    }

    /// Whether the style defines `attribute`.
    pub fn includes(&self, attribute: StyleAttribute) -> bool {
        self.included.contains(&attribute)
    }
}

/// One change from the options editor.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StyleOptionsChange {
    /// The style's new name.
    Rename(String),
    /// An include check switched.
    Include {
        /// The attribute.
        attribute: StyleAttribute,
        /// Whether the style defines it now.
        on: bool,
    },
    /// A character field.
    Character(CharacterChange),
    /// A paragraph field.
    Paragraph(ParagraphChange),
}

/// What the options editor did this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct TextStyleOptionsOutput {
    /// The editor's response.
    pub response: Response,
    /// The edit events of this pass.
    pub edits: Vec<EditEvent<StyleOptionsChange>>,
    /// The colour well was clicked (key [`STYLE_COLOR_KEY`]).
    pub requests: HostRequests,
    /// The editor asks to close.
    pub closed: bool,
}

impl Outcome for TextStyleOptionsOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The options of a character or paragraph style: a name field, then one include check and field per row of `rows`.
/// A row that is not included shows the inherited value and takes no input; every length row is in the type unit.
#[must_use = "show the editor with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
#[allow(dead_code)] // P3(panels-w13): pn.C52
pub struct TextStyleOptions<'a> {
    label: Label,
    values: &'a StyleOptionValues,
    rows: &'a [StyleAttribute],
    families: &'a [String],
    styles: &'a [String],
    /// The unit the length fields show. Default points.
    #[prop]
    unit: DocUnit,
    /// The name field takes input.
    #[prop]
    renamable: bool,
    #[cfg(feature = "font-picker")]
    picker: Option<FontPickerSource<'a>>,
}

impl<'a> TextStyleOptions<'a> {
    /// The editor named `label` over `values`, with the `rows` to show, the installed `families` and the family's
    /// `styles`.
    pub fn new(
        label: impl Into<Label>,
        values: &'a StyleOptionValues,
        rows: &'a [StyleAttribute],
        families: &'a [String],
        styles: &'a [String],
    ) -> Self {
        TextStyleOptions {
            label: label.into(),
            values,
            rows,
            families,
            styles,
            unit: DocUnit::points(),
            renamable: true,
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

    /// Shows the editor and returns the changes the user made.
    pub fn show(self, _ui: &mut Ui) -> TextStyleOptionsOutput {
        todo!("pn.C52")
    }
}
