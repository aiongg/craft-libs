//! [`CharacterControls`] and [`ParagraphControls`] (F14.04): Photoshop's Character and Paragraph
//! panels as controls for a panel body, a Properties section or a style options editor. Values
//! are in points (sizes, leading, baseline shift, indents, spacing) and thousandths of an em
//! (kerning, tracking); the fields show them in the app's type unit ([`TypeUnit`]). The controls
//! change nothing themselves: each returns the one change the user made this pass.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(feature = "font-picker")]
use std::sync::Arc;

#[cfg(feature = "font-picker")]
use craft_font_manager::Catalog;
#[cfg(feature = "font-picker")]
use craft_font_manager::picker::{FontFilters, FontPickerSettings};
use egui::{Color32, Ui};

craft_ui::words! {
    /// The Character controls' words, English by default (PhotoCraft's tooltips).
    #[non_exhaustive]
    pub struct CharacterLabels {
        character: "character.title" = "Character";
        family: "character.family" = "Font family";
        style: "character.style" = "Style";
        size: "character.size" = "Font size";
        size_presets: "character.sizePresets" = "Font size presets";
        leading: "character.leading" = "Leading (set to the font size × auto-leading when Auto)";
        leading_presets: "character.leadingPresets" = "Leading presets";
        auto: "character.auto" = "Auto";
        kerning: "character.kerning" = "Kerning: Metrics, Optical or a value in 1/1000 em (Alt+←/→ at the caret)";
        metrics: "character.metrics" = "Metrics";
        optical: "character.optical" = "Optical";
        tracking: "character.tracking" = "Tracking (1/1000 em)";
        tracking_presets: "character.trackingPresets" = "Tracking presets";
        vertical_scale: "character.verticalScale" = "Vertical scale";
        horizontal_scale: "character.horizontalScale" = "Horizontal scale";
        baseline_shift: "character.baselineShift" = "Baseline shift";
        color: "character.color" = "Color";
        text_color: "character.textColor" = "Text color";
        faux_bold: "character.fauxBold" = "Faux Bold";
        faux_italic: "character.fauxItalic" = "Faux Italic";
        all_caps: "character.allCaps" = "All Caps";
        small_caps: "character.smallCaps" = "Small Caps";
        superscript: "character.superscript" = "Superscript";
        subscript: "character.subscript" = "Subscript";
        underline: "character.underline" = "Underline";
        strikethrough: "character.strikethrough" = "Strikethrough";
        standard_ligatures: "type.openType.standardLigatures" = "Standard Ligatures";
        contextual_alternates: "type.openType.contextualAlternates" = "Contextual Alternates";
        discretionary_ligatures: "type.openType.discretionaryLigatures" = "Discretionary Ligatures";
        swash: "type.openType.swash" = "Swash";
        stylistic_alternates: "type.openType.stylisticAlternates" = "Stylistic Alternates";
        titling_alternates: "type.openType.titlingAlternates" = "Titling Alternates";
        ordinals: "type.openType.ordinals" = "Ordinals";
        fractions: "type.openType.fractions" = "Fractions";
        language: "character.language" = "Language";
        points: "character.unitPoints" = "pt";
        pixels: "character.unitPixels" = "px";
        millimeters: "character.unitMillimeters" = "mm";
    }
}

craft_ui::words! {
    /// The Paragraph controls' words, English by default (PhotoCraft's tooltips).
    #[non_exhaustive]
    pub struct ParagraphLabels {
        paragraph: "paragraph.title" = "Paragraph";
        left: "paragraph.left" = "Left align text";
        center: "paragraph.center" = "Center text";
        right: "paragraph.right" = "Right align text";
        justify_left: "paragraph.justifyLeft" = "Justify last left";
        justify_center: "paragraph.justifyCenter" = "Justify last centered";
        justify_right: "paragraph.justifyRight" = "Justify last right";
        justify_all: "paragraph.justifyAll" = "Justify all";
        direction_auto: "paragraph.directionAuto" = "Automatic paragraph direction";
        direction_ltr: "paragraph.directionLtr" = "Left-to-right paragraph direction";
        direction_rtl: "paragraph.directionRtl" = "Right-to-left paragraph direction";
        start_indent: "paragraph.startIndent" = "Indent left margin";
        end_indent: "paragraph.endIndent" = "Indent right margin";
        first_line_indent: "paragraph.firstLineIndent" = "Indent first line";
        space_before: "paragraph.spaceBefore" = "Add space before paragraph";
        space_after: "paragraph.spaceAfter" = "Add space after paragraph";
        hyphenate: "paragraph.hyphenate" = "Hyphenate";
    }
}

/// The font size menu's presets, in points (PhotoCraft's `SIZE_PRESETS`, Photoshop's list); the
/// leading menu uses them too.
pub const SIZE_PRESETS: [f32; 17] = [6.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 24.0, 30.0, 36.0, 48.0, 60.0, 72.0, 150.0, 300.0];

/// The tracking menu's presets in thousandths of an em, in groups split by separators
/// (PhotoCraft's `TRACKING_PRESETS`).
pub const TRACKING_PRESETS: [&[f32]; 3] = [&[-100.0, -75.0, -50.0, -25.0, -10.0, -5.0], &[0.0], &[5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 200.0]];

/// The OpenType buttons' feature tags in Photoshop's order (PhotoCraft's `OPENTYPE_BUTTONS`);
/// their names are the matching [`CharacterLabels`] fields.
pub const OPENTYPE_FEATURES: [&str; 8] = ["liga", "calt", "dlig", "swsh", "salt", "titl", "ordn", "frac"];

/// The unit the size fields show (Preferences › Units & Rulers › Type). Values stay in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TypeUnit {
    #[default]
    Points,
    /// Pixels at the document's resolution.
    Pixels {
        ppi: f32,
    },
    Millimeters,
}

impl TypeUnit {
    /// Shown units per point: 1, `ppi / 72` (`ppi` at least 1), 25.4 / 72.
    // pc: crates/ui-egui/src/tools/type_tool/defaults.rs:62 `type_unit`.
    pub fn per_point(self) -> f32 {
        todo!()
    }

    /// The unit's word in `labels` (`points`, `pixels`, `millimeters`; English "pt", "px", "mm");
    /// [`ParagraphControls`] uses the same words.
    // pc: crates/ui-egui/src/tools/type_tool/defaults.rs:62 `type_unit`.
    pub fn suffix(self, labels: &CharacterLabels) -> &str {
        todo!()
    }
}

/// Leading: automatic (the font size × the paragraph's auto-leading, 120 % by default) or set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Leading {
    Auto,
    Points(f32),
}

/// Kerning: from the font's pairs, from the glyph shapes, or a value in thousandths of an em
/// (0 is none). The field shows "Metrics", "Optical" or the rounded number.
// pc: crates/ui-egui/src/tools/type_tool/options_bar.rs:228 `kerning_label`, :240 `parse_kerning`
// (Metrics / Optical in any case, or a number in -1000..=10000).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kerning {
    Metrics,
    Optical,
    Manual(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Caps {
    #[default]
    Normal,
    AllCaps,
    SmallCaps,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BaselinePosition {
    #[default]
    Normal,
    Superscript,
    Subscript,
}

/// The character attributes at the type target. `None` in a field: the selection mixes values,
/// and the field shows empty (Photoshop).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterValues {
    pub family: Option<String>,
    pub style: Option<String>,
    pub size: Option<f32>,
    pub leading: Option<Leading>,
    pub kerning: Option<Kerning>,
    pub tracking: Option<f32>,
    /// Percent, 100 is unscaled.
    pub vertical_scale: Option<f32>,
    pub horizontal_scale: Option<f32>,
    pub baseline_shift: Option<f32>,
    /// The swatch's colour, converted by the app for display.
    pub color: Option<Color32>,
    pub faux_bold: Option<bool>,
    pub faux_italic: Option<bool>,
    pub caps: Option<Caps>,
    pub position: Option<BaselinePosition>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    /// The [`OPENTYPE_FEATURES`] that are on, by tag.
    pub opentype: Vec<&'static str>,
    /// BCP 47 tag of the text's language (hyphenation, spelling, smart quotes).
    pub language: Option<String>,
}

/// One change from the Character controls.
#[derive(Clone, Debug, PartialEq)]
pub enum CharacterChange {
    Family(String),
    /// A style of the current family; the app keeps the family.
    Style(String),
    Size(f32),
    Leading(Leading),
    Kerning(Kerning),
    Tracking(f32),
    VerticalScale(f32),
    HorizontalScale(f32),
    BaselineShift(f32),
    FauxBold(bool),
    FauxItalic(bool),
    /// All Caps and Small Caps are one choice: pressing the pressed one returns `Normal`.
    Caps(Caps),
    /// Superscript and Subscript, the same way.
    Position(BaselinePosition),
    Underline(bool),
    Strikethrough(bool),
    OpenType {
        tag: &'static str,
        on: bool,
    },
    Language(String),
}

/// What the Character controls did this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterResponse {
    pub change: Option<CharacterChange>,
    /// The family under the pointer in the open font list, for a live preview of the selected
    /// text (`FontPickerResponse::preview`; only with `font-picker`).
    pub preview_family: Option<String>,
    /// The colour swatch was clicked: the app opens its colour picker and sets the colour.
    pub pick_color: bool,
    /// A field is being dragged (scrubbed): the app merges this drag's changes into one history
    /// step (Photoshop's single step per scrub).
    // pc: crates/ui-egui/src/tools/type_tool/defaults.rs:71 (the scrub coalesce key).
    pub scrubbing: bool,
}

/// The Character panel's controls, in Photoshop's rows: family; style; size and leading; kerning
/// and tracking; vertical and horizontal scale; baseline shift and colour; the style toggles
/// (Faux Bold, Faux Italic, All Caps, Small Caps, then Superscript and Subscript when
/// [`superscript`](Self::superscript) is on, Underline, Strikethrough); the OpenType buttons; the
/// language when [`languages`](Self::languages) is given.
///
/// ```ignore
/// let r = CharacterControls::new(&values, &families, &styles).unit(TypeUnit::Points).show(ui);
/// if let Some(change) = r.change { apply_to_selection(change) }
/// ```
///
/// - Fields fill two columns of the available width with an icon label each (PhotoCraft's glyph
///   labels: "tT", "A↕", "VA", "↕T", "↔T", "Aª"); size, leading and tracking have a preset menu in
///   the field (presets are shortcuts, not limits: any value in range can be typed). Ranges in
///   points: size 0.1–1296, leading 0.1–5000, baseline shift −1296–1296; tracking −1000–10000;
///   scales 0–1000 %. Kerning takes Metrics, Optical or a number.
/// - Family: craft-font-manager's `FontPicker` with `font-picker` and
///   [`font_picker`](Self::font_picker), else a `Select` of `families`.
/// - Toggles are toggle buttons in one row, pressed while on; Tab reaches each.
/// - A mixed value shows an empty field or an unpressed toggle; typing or pressing sets it for the
///   whole selection.
pub struct CharacterControls<'a> {
    values: &'a CharacterValues,
    families: &'a [String],
    styles: &'a [String],
    unit: TypeUnit,
    superscript: bool,
    languages: Option<&'a [(String, String)]>,
    #[cfg(feature = "font-picker")]
    picker: Option<(&'a Arc<Catalog>, &'a mut FontPickerSettings, &'a mut FontFilters)>,
    labels: Option<CharacterLabels>,
}

impl<'a> CharacterControls<'a> {
    /// Controls for `values`, with the installed `families` and the current family's `styles`.
    pub fn new(values: &'a CharacterValues, families: &'a [String], styles: &'a [String]) -> Self {
        CharacterControls {
            values,
            families,
            styles,
            unit: TypeUnit::Points,
            superscript: false,
            languages: None,
            #[cfg(feature = "font-picker")]
            picker: None,
            labels: None,
        }
    }

    pub fn unit(mut self, unit: TypeUnit) -> Self {
        self.unit = unit;
        self
    }

    /// Show the Superscript and Subscript toggles (Photoshop; off by default, PhotoCraft has no
    /// superscript).
    pub fn superscript(mut self, on: bool) -> Self {
        self.superscript = on;
        self
    }

    /// Show the Language select over `(tag, name)` pairs (Photoshop's bottom row).
    pub fn languages(mut self, languages: &'a [(String, String)]) -> Self {
        self.languages = Some(languages);
        self
    }

    /// The family field is craft-font-manager's font picker over `catalog`, with the app's stars,
    /// recents and filters.
    #[cfg(feature = "font-picker")]
    pub fn font_picker(mut self, catalog: &'a Arc<Catalog>, settings: &'a mut FontPickerSettings, filters: &'a mut FontFilters) -> Self {
        self.picker = Some((catalog, settings, filters));
        self
    }

    pub fn labels(mut self, labels: CharacterLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pc: crates/ui-egui/src/tools/type_tool/panel.rs:215 `type_sections` (character half), :149
    // `metric_field`, :25 `opentype_row`; options_bar.rs:228 `kerning_label`.
    pub fn show(self, ui: &mut Ui) -> CharacterResponse {
        todo!()
    }
}

/// Paragraph alignment, in Photoshop's button order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    JustifyLeft,
    JustifyCenter,
    JustifyRight,
    JustifyAll,
}

/// A paragraph's direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParagraphDirection {
    /// From the paragraph's first strong character.
    #[default]
    Automatic,
    LeftToRight,
    RightToLeft,
}

/// The paragraph attributes at the type target; `None` where the selected paragraphs differ.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphValues {
    pub align: Option<TextAlign>,
    pub direction: Option<ParagraphDirection>,
    pub start_indent: Option<f32>,
    pub end_indent: Option<f32>,
    pub first_line_indent: Option<f32>,
    pub space_before: Option<f32>,
    pub space_after: Option<f32>,
    pub hyphenate: Option<bool>,
}

/// One change from the Paragraph controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParagraphChange {
    Align(TextAlign),
    Direction(ParagraphDirection),
    StartIndent(f32),
    EndIndent(f32),
    FirstLineIndent(f32),
    SpaceBefore(f32),
    SpaceAfter(f32),
    Hyphenate(bool),
}

/// What the Paragraph controls did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParagraphResponse {
    pub change: Option<ParagraphChange>,
    /// As [`CharacterResponse::scrubbing`].
    pub scrubbing: bool,
}

/// The Paragraph panel's controls: the seven alignment buttons; the three direction buttons; left
/// and right indent; first-line indent; space before and after; Hyphenate.
///
/// - The four justify buttons are disabled on point text ([`point_text`](Self::point_text)):
///   justification needs a paragraph box or area type (Photoshop greys them).
/// - Ranges in points: indents −1296–1296, space before and after 0–1296.
pub struct ParagraphControls<'a> {
    values: &'a ParagraphValues,
    unit: TypeUnit,
    point_text: bool,
    labels: Option<ParagraphLabels>,
}

impl<'a> ParagraphControls<'a> {
    pub fn new(values: &'a ParagraphValues) -> Self {
        ParagraphControls { values, unit: TypeUnit::Points, point_text: false, labels: None }
    }

    pub fn unit(mut self, unit: TypeUnit) -> Self {
        self.unit = unit;
        self
    }

    /// Any of the formatted text is point text (no box): the justify buttons are disabled.
    pub fn point_text(mut self, on: bool) -> Self {
        self.point_text = on;
        self
    }

    pub fn labels(mut self, labels: ParagraphLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pc: crates/ui-egui/src/tools/type_tool/panel.rs:215 `type_sections` (paragraph half);
    // options_bar.rs:356 `direction_items`.
    pub fn show(self, ui: &mut Ui) -> ParagraphResponse {
        todo!()
    }
}
