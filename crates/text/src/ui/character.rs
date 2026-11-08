//! [`CharacterControls`] and [`ParagraphControls`]: Photoshop's Character and Paragraph
//! panels as patterns for a panel body, a Properties section or a style options editor. Values
//! are in points (sizes, leading, baseline shift, indents, spacing) and thousandths of an em
//! (kerning, tracking); the length fields show them in the app's type unit ([`DocUnit`]). The
//! controls change nothing themselves: each returns the edit events of the changes the user made
//! this pass.

#[cfg(feature = "font-picker")]
use craft_font_manager::Catalog;
#[cfg(feature = "font-picker")]
use craft_font_manager::picker::{FontFilters, FontPicker, FontPickerSettings};
use std::ops::RangeInclusive;
use std::sync::Arc;

use super::length::{DocUnit, length_edit, length_field};
use craft_behavior::{ColumnWidth, EditEvent, GestureId, IconId, Mixed, NumberCodec};
use craft_theme::{Rgba, Srgba};
use craft_ui::buttons::ToggleButton;
use craft_ui::choice::{Checkbox, Segmented, Swatch};
use craft_ui::contract::{Bind, Entry, GlyphLabel, LabelKind, NoValue, Outcome, Props as _, Show as _, ShowContainer as _, SideLabel};
use craft_ui::form::Field;
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::patterns::form_dialogs::HostRequests;
use craft_ui::pickers::Select;
use craft_ui::runtime::env::{Env, Label, UnitKind, Units};
use craft_ui::text::{NamedOr, NumberField, NumberName, NumberPick, NumberValue as _};
use egui::{Response, Ui};

mod paragraph_dialog;
#[cfg(feature = "screen-testing")]
mod stories;

pub use paragraph_dialog::{
    LineSpacing, ParagraphAlign, ParagraphAnswer, ParagraphDialog, ParagraphDialogWords, ParagraphFormat, ParagraphPreview, ParagraphPreviewStyle,
    SpacingKind, SpecialIndent,
};

craft_ui::words! {
    /// The Character controls' words, English by default (PhotoCraft's tooltips). A control whose label is an icon or a
    /// mark shows only that; the word is its accessible name.
    #[non_exhaustive]
    pub struct CharacterWords {
        character: "character.title" = "Character";
        family: "character.family" = "Font family";
        style: "character.style" = "Style";
        size: "character.size" = "Font size";
        leading: "character.leading" = "Leading";
        auto: "character.auto" = "Auto";
        kerning: "character.kerning" = "Kerning";
        metrics: "character.metrics" = "Metrics";
        optical: "character.optical" = "Optical";
        tracking: "character.tracking" = "Tracking";
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
        standard_ligatures: "character.standardLigatures" = "Standard Ligatures";
        contextual_alternates: "character.contextualAlternates" = "Contextual Alternates";
        discretionary_ligatures: "character.discretionaryLigatures" = "Discretionary Ligatures";
        swash: "character.swash" = "Swash";
        stylistic_alternates: "character.stylisticAlternates" = "Stylistic Alternates";
        titling_alternates: "character.titlingAlternates" = "Titling Alternates";
        ordinals: "character.ordinals" = "Ordinals";
        fractions: "character.fractions" = "Fractions";
        language: "character.language" = "Language";
    }
}

craft_ui::words! {
    /// The Paragraph controls' words, English by default (PhotoCraft's tooltips).
    #[non_exhaustive]
    pub struct ParagraphWords {
        paragraph: "paragraph.title" = "Paragraph";
        alignment: "paragraph.alignment" = "Alignment";
        direction: "paragraph.direction" = "Direction";
        left: "paragraph.left" = "Left align text";
        center: "paragraph.center" = "Center text";
        right: "paragraph.right" = "Right align text";
        justify_left: "paragraph.justifyLeft" = "Justify last left";
        justify_center: "paragraph.justifyCenter" = "Justify last centered";
        justify_right: "paragraph.justifyRight" = "Justify last right";
        justify_all: "paragraph.justifyAll" = "Justify all";
        justify_needs_box: "paragraph.justifyNeedsBox" = "Justification needs a text box; point text has none";
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
/// their names are the matching [`CharacterWords`] fields.
pub const OPENTYPE_FEATURES: [&str; 8] = ["liga", "calt", "dlig", "swsh", "salt", "titl", "ordn", "frac"];

/// The key of the colour request the swatch makes.
pub const COLOR_KEY: &str = "character.color";

/// Auto leading is the font size × 120 % (Photoshop's default).
const AUTO_LEADING: f32 = 1.2;

/// The kerning menu's number rows.
const KERNING_PRESETS: [f32; 14] = [0.0, -100.0, -75.0, -50.0, -25.0, -10.0, -5.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 200.0];

/// The most decimals the menus' number rows show.
const PRESET_DECIMALS: usize = 2;

/// The leading menu's Auto row.
const AUTO_ROW: u32 = 0;
/// The kerning menu's Metrics row.
const METRICS_ROW: u32 = 0;
/// The kerning menu's Optical row.
const OPTICAL_ROW: u32 = 1;

/// The range of the size field, points.
const SIZE_RANGE: RangeInclusive<f64> = 0.1..=1296.0;
/// The range of the leading field, points.
const LEADING_RANGE: RangeInclusive<f64> = 0.1..=5000.0;
/// The range of the baseline shift field, points.
const SHIFT_RANGE: RangeInclusive<f64> = -1296.0..=1296.0;
/// The range of the tracking and kerning fields, thousandths of an em.
const TRACKING_RANGE: RangeInclusive<f64> = -1000.0..=10000.0;
/// The range of the scale fields, percent.
const SCALE_RANGE: RangeInclusive<f64> = 0.0..=1000.0;
/// The range of the indent fields, points.
const INDENT_RANGE: RangeInclusive<f64> = -1296.0..=1296.0;
/// The range of the space fields, points.
const SPACE_RANGE: RangeInclusive<f64> = 0.0..=1296.0;

/// Two columns of the same share, for the rows of two fields.
static TWO_COLUMNS: [ColumnWidth; 2] = [ColumnWidth::fill(1.0), ColumnWidth::fill(1.0)];

/// Leading: automatic (the font size × the paragraph's auto-leading, 120 % by default) or set.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Leading {
    /// Auto.
    Auto,
    /// The leading in document points, the representation of `CharacterValues::size`.
    Fixed(f32),
}

/// Kerning: from the font's pairs, from the glyph shapes, or a value in thousandths of an em
/// (0 is none). The field shows the number; the menu checks Metrics or Optical.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Kerning {
    /// Metrics.
    Metrics,
    /// Optical.
    Optical,
    /// A value in thousandths of an em; 0 is none.
    Manual(f32),
}

/// The names a kerning field holds besides a number.
#[derive(Clone, Copy, Debug, PartialEq)]
enum KerningName {
    Metrics,
    Optical,
}

impl NumberName for KerningName {
    fn id(self) -> u32 {
        match self {
            KerningName::Metrics => METRICS_ROW,
            KerningName::Optical => OPTICAL_ROW,
        }
    }

    fn from_id(id: u32) -> Option<Self> {
        match id {
            METRICS_ROW => Some(KerningName::Metrics),
            OPTICAL_ROW => Some(KerningName::Optical),
            _ => None,
        }
    }
}

impl KerningName {
    fn kerning(self) -> Kerning {
        match self {
            KerningName::Metrics => Kerning::Metrics,
            KerningName::Optical => Kerning::Optical,
        }
    }
}

/// The kerning a kerning field's value stands for; a value that is neither a number nor one of the names is the
/// default, Metrics.
fn kerning_of(value: NamedOr<KerningName>) -> Kerning {
    let named = || value.named().and_then(KerningName::from_id).map(KerningName::kerning);
    value.number().map(|number| Kerning::Manual(number as f32)).or_else(named).unwrap_or(Kerning::Metrics)
}

/// How letters are capitalized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Caps {
    /// Normal.
    #[default]
    Normal,
    /// All caps.
    AllCaps,
    /// Small caps.
    SmallCaps,
}

/// Where the text sits relative to the baseline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BaselinePosition {
    /// Normal.
    #[default]
    Normal,
    /// Superscript.
    Superscript,
    /// Subscript.
    Subscript,
}

/// The character attributes at the type target. `None` in a field: the selection mixes values,
/// and the field shows empty (Photoshop).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CharacterValues {
    /// The family.
    pub family: Option<String>,
    /// The style.
    pub style: Option<String>,
    /// Size in points.
    pub size: Option<f32>,
    /// The leading.
    pub leading: Option<Leading>,
    /// The kerning.
    pub kerning: Option<Kerning>,
    /// Tracking in thousandths of an em.
    pub tracking: Option<f32>,
    /// Percent, 100 is unscaled.
    pub vertical_scale: Option<f32>,
    /// Percent, 100 is unscaled.
    pub horizontal_scale: Option<f32>,
    /// Baseline shift in points; positive raises the text.
    pub baseline_shift: Option<f32>,
    /// The swatch's colour.
    pub color: Option<Rgba>,
    /// The faux bold.
    pub faux_bold: Option<bool>,
    /// The faux italic.
    pub faux_italic: Option<bool>,
    /// The caps.
    pub caps: Option<Caps>,
    /// The position.
    pub position: Option<BaselinePosition>,
    /// The underline.
    pub underline: Option<bool>,
    /// The strikethrough.
    pub strikethrough: Option<bool>,
    /// The [`OPENTYPE_FEATURES`] that are on, by tag.
    pub opentype: Vec<&'static str>,
    /// BCP 47 tag of the text's language (hyphenation, spelling, smart quotes).
    pub language: Option<String>,
}

/// One change from the Character controls.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CharacterChange {
    /// Family.
    Family(String),
    /// A style of the current family; the app keeps the family.
    Style(String),
    /// Size in points.
    Size(f32),
    /// Leading.
    Leading(Leading),
    /// Kerning.
    Kerning(Kerning),
    /// Tracking in thousandths of an em.
    Tracking(f32),
    /// Percent, 100 is unscaled.
    VerticalScale(f32),
    /// Percent, 100 is unscaled.
    HorizontalScale(f32),
    /// Baseline shift in points.
    BaselineShift(f32),
    /// Faux bold.
    FauxBold(bool),
    /// Faux italic.
    FauxItalic(bool),
    /// All Caps and Small Caps are one choice: pressing the pressed one returns `Normal`.
    Caps(Caps),
    /// Superscript and Subscript, the same way.
    Position(BaselinePosition),
    /// Underline.
    Underline(bool),
    /// Strikethrough.
    Strikethrough(bool),
    /// An OpenType feature switched on or off.
    #[non_exhaustive]
    OpenType {
        /// The feature's four-letter tag, such as "liga".
        tag: &'static str,
        /// On.
        on: bool,
    },
    /// Language.
    Language(String),
}

/// What the Character controls did this pass.
#[non_exhaustive]
pub struct CharacterControlsOutput {
    /// The controls' response.
    pub response: Response,
    /// The edit events of this pass, in the order of the controls (usually one; a preset pick and a toggle in the
    /// same pass are two). A scrub or a typing run in a field is a gesture (`Begin`, `Update`, then `Commit`, or
    /// `Cancel` on Esc): the app merges its steps into one history step. A preset, toggle or select is a `Set`. The
    /// app applies them in order.
    pub edits: Vec<EditEvent<CharacterChange>>,
    /// The family under the pointer in the open font list, for a live preview of the selected text (only with
    /// `font-picker`).
    pub preview_family: Option<String>,
    /// The colour swatch was clicked: the app opens its colour picker for this request (key [`COLOR_KEY`]) and sets
    /// the colour. Set for the one pass of the click.
    pub requests: HostRequests,
}

impl Outcome for CharacterControlsOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The Character panel's controls, in Photoshop's rows: family; style; size and leading; kerning and tracking;
/// vertical and horizontal scale; baseline shift and colour; the style toggles (Faux Bold, Faux Italic, All Caps,
/// Small Caps, then Superscript and Subscript when [`superscript`](Self::superscript) is on, Underline,
/// Strikethrough); the OpenType buttons; the language when [`languages`](Self::languages) is given.
///
/// ```ignore
/// let out = CharacterControls::new("Character", &values, &families, &styles).unit(DocUnit::points()).show(ui);
/// for edit in out.edits { gestures.handle(&mut doc, edit, apply_to_selection) }
/// ```
///
/// - Fields fill two columns of the available width with a mark for a label (PhotoCraft's marks: "tT", "A↕", "VA",
///   "↕T", "↔T", "Aª"); a drag on the mark scrubs the value. Size, leading, kerning and tracking have a preset menu in
///   the field (presets are shortcuts, not limits: any value in range can be typed). Ranges in points: size
///   0.1–1296, leading 0.1–5000, baseline shift −1296–1296; tracking and kerning −1000–10000; scales 0–1000 %. The
///   leading menu starts with an Auto row ([`Leading::Auto`]); while leading is Auto the field shows the font size
///   × 120 %. The kerning menu starts with Metrics and Optical; the field is empty while either is the kerning.
/// - Family: craft-font-manager's `FontPicker` with `font-picker` and [`font_picker`](Self::font_picker), else a
///   `Select` of `families`.
/// - Toggles are toggle buttons in one row, pressed while on; Tab reaches each.
/// - A mixed value shows an empty field or an unpressed toggle; typing or pressing sets it for the whole selection.
#[must_use = "show the controls with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
pub struct CharacterControls<'a> {
    label: Label,
    values: &'a CharacterValues,
    families: &'a [String],
    styles: &'a [String],
    /// The unit the length fields show (Preferences › Units & Rulers › Type) and what converts it: the document's
    /// resolution for pixels. Values stay in points. Default points.
    #[prop]
    unit: DocUnit,
    /// Shows the Superscript and Subscript toggles (Photoshop; off by default, PhotoCraft has no superscript).
    #[prop]
    superscript: bool,
    /// Shows the Language select over `(tag, name)` pairs (Photoshop's bottom row).
    #[prop]
    languages: Option<&'a [(String, String)]>,
    #[cfg(feature = "font-picker")]
    picker: Option<(&'a Arc<Catalog>, Bind<'a, FontPickerSettings>, Bind<'a, FontFilters>)>,
}

impl<'a> CharacterControls<'a> {
    /// Controls named `label` for `values`, with the installed `families` and the current family's `styles`.
    pub fn new(label: impl Into<Label>, values: &'a CharacterValues, families: &'a [String], styles: &'a [String]) -> Self {
        CharacterControls {
            label: label.into(),
            values,
            families,
            styles,
            unit: DocUnit::points(),
            superscript: false,
            languages: None,
            #[cfg(feature = "font-picker")]
            picker: None,
        }
    }

    /// The family field is craft-font-manager's font picker over `catalog`, with the app's stars, recents and
    /// filters.
    #[cfg(feature = "font-picker")]
    #[must_use]
    pub fn font_picker(mut self, catalog: &'a Arc<Catalog>, settings: Bind<'a, FontPickerSettings>, filters: Bind<'a, FontFilters>) -> Self {
        self.picker = Some((catalog, settings, filters));
        self
    }

    /// Shows the controls and returns the changes the user made.
    pub fn show(self, ui: &mut Ui) -> CharacterControlsOutput {
        let values = self.values;
        let (families, styles, languages) = (self.families, self.styles, self.languages);
        let (unit, superscript) = (self.unit, self.superscript);
        #[cfg(feature = "font-picker")]
        let mut picker = self.picker;
        let env = Env::of(ui);
        let words = env.words::<CharacterWords>();
        let units = env.units();
        let lengths = Lengths { units: &units, unit };
        let ppi = units.points_per_inch();

        let size_presets: Vec<Entry<NumberPick>> = SIZE_PRESETS
            .iter()
            .map(|&p| {
                let shown = unit.shown(f64::from(p), ppi);
                Entry::new(NumberPick::Number(shown), units.text(unit.kind, shown, 0..=PRESET_DECIMALS))
            })
            .collect();
        let auto_row = Entry::new(NumberPick::Choice(AUTO_ROW), words.auto.clone()).checked(values.leading == Some(Leading::Auto));
        let leading_presets: Vec<Entry<NumberPick>> = [auto_row, Entry::separator()].into_iter().chain(size_presets.iter().cloned()).collect();
        let tracking_presets = number_presets(&units, TRACKING_PRESETS.iter().copied());
        let kerning_presets: Vec<Entry<NumberPick>> = [
            Entry::new(NumberPick::Choice(METRICS_ROW), words.metrics.clone()).checked(values.kerning == Some(Kerning::Metrics)),
            Entry::new(NumberPick::Choice(OPTICAL_ROW), words.optical.clone()).checked(values.kerning == Some(Kerning::Optical)),
            Entry::separator(),
        ]
        .into_iter()
        .chain(KERNING_PRESETS.iter().map(|&p| Entry::new(NumberPick::Number(f64::from(p)), units.number(f64::from(p), 0..=PRESET_DECIMALS))))
        .collect();

        let family_entries = string_entries(families.iter().map(|name| (name.clone(), name.clone())));
        let style_entries = string_entries(styles.iter().map(|name| (name.clone(), name.clone())));
        let language_entries = languages.map(|languages| string_entries(languages.iter().cloned()));
        let leading_value = match values.leading {
            Some(Leading::Fixed(points)) => Some(points),
            Some(Leading::Auto) => values.size.map(|size| size * AUTO_LEADING),
            None => None,
        };
        let kerning_shown = values.kerning.map(|kerning| match kerning {
            Kerning::Manual(value) => NamedOr::Number(f64::from(value)),
            Kerning::Metrics => NamedOr::Named(KerningName::Metrics),
            Kerning::Optical => NamedOr::Named(KerningName::Optical),
        });

        let mut edits: Vec<EditEvent<CharacterChange>> = Vec::new();
        let mut preview_family = None;
        let mut requests = HostRequests::new();

        let shown = Stack::new().fill_width().label(self.label).show(ui, |ui, _| {
            #[cfg(feature = "font-picker")]
            let by_picker = if let Some((catalog, settings, filters)) = picker.as_mut() {
                let family = mixed(values.family.clone());
                let side = SideLabel::new().kind(LabelKind::Hidden);
                let shown = Field::new(words.family.clone()).side(side).show(ui, |ui, _| {
                    let out = FontPicker::new(words.family.clone(), Bind::given(family.clone()), *catalog)
                        .settings(settings.reborrowed())
                        .filters(filters.reborrowed())
                        .show(ui);
                    (out.preview, out.edit.event)
                });
                if let Some((preview, event)) = shown.inner {
                    preview_family = preview;
                    let kept = event.filter(|event| matches!(event, EditEvent::Commit { .. } | EditEvent::Set { .. }));
                    edits.extend(kept.map(|event| event.map(CharacterChange::Family)));
                }
                true
            } else {
                false
            };
            #[cfg(not(feature = "font-picker"))]
            let by_picker = false;
            if !by_picker {
                edits.extend(select_row(ui, &words.family, mixed(values.family.clone()), &family_entries, CharacterChange::Family));
            }
            edits.extend(select_row(ui, &words.style, mixed(values.style.clone()), &style_entries, CharacterChange::Style));

            Grid::new(&TWO_COLUMNS).fill().show(ui, |ui, _| {
                let size = LengthRow {
                    label: words.size.clone(),
                    glyph: GlyphLabel::text("tT"),
                    value: values.size,
                    range: SIZE_RANGE,
                    presets: &size_presets,
                };
                let (event, _) = lengths.row(ui, size);
                edits.extend(number_change(event, CharacterChange::Size));
                let leading = LengthRow {
                    label: words.leading.clone(),
                    glyph: GlyphLabel::text("A↕"),
                    value: leading_value,
                    range: LEADING_RANGE,
                    presets: &leading_presets,
                };
                let (event, picked) = lengths.row(ui, leading);
                edits.extend(number_change(event, |points| CharacterChange::Leading(Leading::Fixed(points))));
                if picked == Some(NumberPick::Choice(AUTO_ROW)) {
                    edits.push(picked_event(CharacterChange::Leading(Leading::Auto), words.leading.as_str()));
                }
            });
            Grid::new(&TWO_COLUMNS).fill().show(ui, |ui, _| {
                let event = lengths.kerning(ui, words.kerning.clone(), GlyphLabel::icon(IconId::new("text_cursor")), kerning_shown, &kerning_presets);
                edits.extend(event.map(|event| event.map(|value| CharacterChange::Kerning(kerning_of(value)))));
                let tracking = PlainRow {
                    label: words.tracking.clone(),
                    glyph: GlyphLabel::text("VA"),
                    value: plain(values.tracking),
                    range: TRACKING_RANGE,
                    percent: false,
                    presets: &tracking_presets,
                };
                let (event, _) = lengths.plain(ui, tracking);
                edits.extend(number_change(event, CharacterChange::Tracking));
            });
            Grid::new(&TWO_COLUMNS).fill().show(ui, |ui, _| {
                let vertical = PlainRow {
                    label: words.vertical_scale.clone(),
                    glyph: GlyphLabel::text("↕T"),
                    value: plain(values.vertical_scale),
                    range: SCALE_RANGE,
                    percent: true,
                    presets: &[],
                };
                let (event, _) = lengths.plain(ui, vertical);
                edits.extend(number_change(event, CharacterChange::VerticalScale));
                let horizontal = PlainRow {
                    label: words.horizontal_scale.clone(),
                    glyph: GlyphLabel::text("↔T"),
                    value: plain(values.horizontal_scale),
                    range: SCALE_RANGE,
                    percent: true,
                    presets: &[],
                };
                let (event, _) = lengths.plain(ui, horizontal);
                edits.extend(number_change(event, CharacterChange::HorizontalScale));
            });
            Grid::new(&TWO_COLUMNS).fill().show(ui, |ui, _| {
                let shift = LengthRow {
                    label: words.baseline_shift.clone(),
                    glyph: GlyphLabel::text("Aª"),
                    value: values.baseline_shift,
                    range: SHIFT_RANGE,
                    presets: &[],
                };
                let (event, _) = lengths.row(ui, shift);
                edits.extend(number_change(event, CharacterChange::BaselineShift));
                let color = mixed(values.color.map(Srgba::from));
                let side = SideLabel::new().kind(LabelKind::Glyph(GlyphLabel::text(words.color.clone())));
                let well = Field::new(words.text_color.clone())
                    .side(side)
                    .show(ui, |ui, _| Swatch::new(words.text_color.clone(), color).picker(COLOR_KEY).fill().show(ui).pick_color);
                if let Some(request) = well.inner.flatten() {
                    requests.request_color(request);
                }
            });

            let caps = values.caps;
            let position = values.position;
            let mut toggles: Vec<Toggle<CharacterChange>> = vec![
                Toggle::new(&words.faux_bold, "text_bold", values.faux_bold, Box::new(CharacterChange::FauxBold)),
                Toggle::new(&words.faux_italic, "text_italic", values.faux_italic, Box::new(CharacterChange::FauxItalic)),
                Toggle::new(
                    &words.all_caps,
                    "text_all_caps",
                    caps.map(|caps| caps == Caps::AllCaps),
                    Box::new(|on| CharacterChange::Caps(if on { Caps::AllCaps } else { Caps::Normal })),
                ),
                Toggle::new(
                    &words.small_caps,
                    "text_small_caps",
                    caps.map(|caps| caps == Caps::SmallCaps),
                    Box::new(|on| CharacterChange::Caps(if on { Caps::SmallCaps } else { Caps::Normal })),
                ),
            ];
            if superscript {
                toggles.push(Toggle::new(
                    &words.superscript,
                    "text_superscript",
                    position.map(|position| position == BaselinePosition::Superscript),
                    Box::new(|on| CharacterChange::Position(if on { BaselinePosition::Superscript } else { BaselinePosition::Normal })),
                ));
                toggles.push(Toggle::new(
                    &words.subscript,
                    "text_subscript",
                    position.map(|position| position == BaselinePosition::Subscript),
                    Box::new(|on| CharacterChange::Position(if on { BaselinePosition::Subscript } else { BaselinePosition::Normal })),
                ));
            }
            toggles.push(Toggle::new(&words.underline, "text_underline", values.underline, Box::new(CharacterChange::Underline)));
            toggles.push(Toggle::new(&words.strikethrough, "text_strikethrough", values.strikethrough, Box::new(CharacterChange::Strikethrough)));
            edits.extend(toggle_row(ui, toggles));

            let names = [
                &words.standard_ligatures,
                &words.contextual_alternates,
                &words.discretionary_ligatures,
                &words.swash,
                &words.stylistic_alternates,
                &words.titling_alternates,
                &words.ordinals,
                &words.fractions,
            ];
            let features: Vec<Toggle<CharacterChange>> = OPENTYPE_FEATURES
                .iter()
                .zip(OPENTYPE_ICONS)
                .zip(names)
                .map(|((&tag, icon), name)| {
                    Toggle::new(name, icon, Some(values.opentype.contains(&tag)), Box::new(move |on| CharacterChange::OpenType { tag, on }))
                })
                .collect();
            edits.extend(toggle_row(ui, features));

            if let Some(entries) = &language_entries {
                edits.extend(select_row(ui, &words.language, mixed(values.language.clone()), entries, CharacterChange::Language));
            }
        });
        CharacterControlsOutput { response: shown.output, edits, preview_family, requests }
    }
}

/// Paragraph alignment, in Photoshop's button order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextAlign {
    /// Left.
    #[default]
    Left,
    /// Center.
    Center,
    /// Right.
    Right,
    /// Justify left.
    JustifyLeft,
    /// Justify center.
    JustifyCenter,
    /// Justify right.
    JustifyRight,
    /// Justify all.
    JustifyAll,
}

impl TextAlign {
    /// Whether the alignment needs a text box: justification has a line width to fill.
    fn needs_box(self) -> bool {
        matches!(self, TextAlign::JustifyLeft | TextAlign::JustifyCenter | TextAlign::JustifyRight | TextAlign::JustifyAll)
    }
}

/// A paragraph's direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParagraphDirection {
    /// From the paragraph's first strong character.
    #[default]
    Automatic,
    /// Left to right.
    LeftToRight,
    /// Right to left.
    RightToLeft,
}

/// The paragraph attributes at the type target; `None` where the selected paragraphs differ.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ParagraphValues {
    /// The align.
    pub align: Option<TextAlign>,
    /// The direction.
    pub direction: Option<ParagraphDirection>,
    /// Indent at the start edge in points (the left in left-to-right text).
    pub start_indent: Option<f32>,
    /// Indent at the end edge in points (the right in left-to-right text).
    pub end_indent: Option<f32>,
    /// First-line indent in points.
    pub first_line_indent: Option<f32>,
    /// Space before the paragraph in points.
    pub space_before: Option<f32>,
    /// Space after the paragraph in points.
    pub space_after: Option<f32>,
    /// The hyphenate.
    pub hyphenate: Option<bool>,
}

/// One change from the Paragraph controls.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ParagraphChange {
    /// Align.
    Align(TextAlign),
    /// Direction.
    Direction(ParagraphDirection),
    /// Indent at the start edge in points.
    StartIndent(f32),
    /// Indent at the end edge in points.
    EndIndent(f32),
    /// First-line indent in points.
    FirstLineIndent(f32),
    /// Space before the paragraph in points.
    SpaceBefore(f32),
    /// Space after the paragraph in points.
    SpaceAfter(f32),
    /// Hyphenate.
    Hyphenate(bool),
}

/// What the Paragraph controls did this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParagraphControlsOutput {
    /// The controls' response.
    pub response: Response,
    /// As [`CharacterControlsOutput::edits`].
    pub edits: Vec<EditEvent<ParagraphChange>>,
}

impl Outcome for ParagraphControlsOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The Paragraph panel's controls: the seven alignment buttons; the three direction buttons; left and right indent;
/// first-line indent; space before and after; Hyphenate.
///
/// - The four justify buttons are disabled on point text ([`point_text`](Self::point_text)): justification needs a
///   paragraph box or area type (Photoshop greys them).
/// - Ranges in points: indents −1296–1296, space before and after 0–1296.
#[must_use = "show the controls with `show`"]
#[derive(craft_ui::contract::Pattern, craft_ui::contract::Props)]
pub struct ParagraphControls<'a> {
    label: Label,
    values: &'a ParagraphValues,
    /// The unit the length fields show. Default points.
    #[prop]
    unit: DocUnit,
    /// Any of the formatted text is point text (no box): the justify buttons are disabled.
    #[prop]
    point_text: bool,
}

impl<'a> ParagraphControls<'a> {
    /// Controls named `label` for the paragraph attributes `values`.
    pub fn new(label: impl Into<Label>, values: &'a ParagraphValues) -> Self {
        ParagraphControls { label: label.into(), values, unit: DocUnit::points(), point_text: false }
    }

    /// Shows the controls and returns the changes the user made.
    pub fn show(self, ui: &mut Ui) -> ParagraphControlsOutput {
        let values = self.values;
        let point_text = self.point_text;
        let env = Env::of(ui);
        let words = env.words::<ParagraphWords>();
        let units = env.units();
        let lengths = Lengths { units: &units, unit: self.unit };

        let aligns: Vec<Entry<TextAlign>> = [
            (TextAlign::Left, "align_left", &words.left),
            (TextAlign::Center, "align_center", &words.center),
            (TextAlign::Right, "align_right", &words.right),
            (TextAlign::JustifyLeft, "align_justify", &words.justify_left),
            (TextAlign::JustifyCenter, "align_justify", &words.justify_center),
            (TextAlign::JustifyRight, "align_justify", &words.justify_right),
            (TextAlign::JustifyAll, "align_justify", &words.justify_all),
        ]
        .into_iter()
        .map(|(align, icon, name)| {
            let entry = Entry::new(align, name.clone()).icon(IconId::new(icon));
            if point_text && align.needs_box() { entry.disabled_reason(words.justify_needs_box.clone()) } else { entry }
        })
        .collect();
        let directions: Vec<Entry<ParagraphDirection>> = [
            (ParagraphDirection::Automatic, "pilcrow", &words.direction_auto),
            (ParagraphDirection::LeftToRight, "pilcrow_right", &words.direction_ltr),
            (ParagraphDirection::RightToLeft, "pilcrow_left", &words.direction_rtl),
        ]
        .into_iter()
        .map(|(direction, icon, name)| Entry::new(direction, name.clone()).icon(IconId::new(icon)))
        .collect();

        let mut edits: Vec<EditEvent<ParagraphChange>> = Vec::new();
        let shown = Stack::new().fill_width().label(self.label).show(ui, |ui, _| {
            let out = Segmented::new(words.alignment.clone(), Bind::given(mixed(values.align)), &aligns).show(ui);
            edits.extend(out.event.map(|event| event.map(ParagraphChange::Align)));
            let out = Segmented::new(words.direction.clone(), Bind::given(mixed(values.direction)), &directions).show(ui);
            edits.extend(out.event.map(|event| event.map(ParagraphChange::Direction)));

            Grid::new(&TWO_COLUMNS).fill().show(ui, |ui, _| {
                let start = LengthRow {
                    label: words.start_indent.clone(),
                    glyph: GlyphLabel::text("→|"),
                    value: values.start_indent,
                    range: INDENT_RANGE,
                    presets: &[],
                };
                let (event, _) = lengths.row(ui, start);
                edits.extend(number_change(event, ParagraphChange::StartIndent));
                let end = LengthRow {
                    label: words.end_indent.clone(),
                    glyph: GlyphLabel::text("|←"),
                    value: values.end_indent,
                    range: INDENT_RANGE,
                    presets: &[],
                };
                let (event, _) = lengths.row(ui, end);
                edits.extend(number_change(event, ParagraphChange::EndIndent));
                let first = LengthRow {
                    label: words.first_line_indent.clone(),
                    glyph: GlyphLabel::text("¶→"),
                    value: values.first_line_indent,
                    range: INDENT_RANGE,
                    presets: &[],
                };
                let (event, _) = lengths.row(ui, first);
                edits.extend(number_change(event, ParagraphChange::FirstLineIndent));
            });
            Grid::new(&TWO_COLUMNS).fill().show(ui, |ui, _| {
                let before = LengthRow {
                    label: words.space_before.clone(),
                    glyph: GlyphLabel::text("↑¶"),
                    value: values.space_before,
                    range: SPACE_RANGE,
                    presets: &[],
                };
                let (event, _) = lengths.row(ui, before);
                edits.extend(number_change(event, ParagraphChange::SpaceBefore));
                let after = LengthRow {
                    label: words.space_after.clone(),
                    glyph: GlyphLabel::text("¶↓"),
                    value: values.space_after,
                    range: SPACE_RANGE,
                    presets: &[],
                };
                let (event, _) = lengths.row(ui, after);
                edits.extend(number_change(event, ParagraphChange::SpaceAfter));
            });
            let out = Checkbox::new(Bind::given(mixed(values.hyphenate))).label(words.hyphenate.clone()).show(ui);
            edits.extend(out.event.map(|event| event.map(ParagraphChange::Hyphenate)));
        });
        ParagraphControlsOutput { response: shown.output, edits }
    }
}

/// The unit the length fields of a pattern show their numbers in.
struct Lengths<'u> {
    units: &'u Units,
    unit: DocUnit,
}

/// A length field of a row: its label, mark, value in points and range in points.
struct LengthRow<'p> {
    label: Label,
    glyph: GlyphLabel,
    value: Option<f32>,
    range: RangeInclusive<f64>,
    presets: &'p [Entry<NumberPick>],
}

/// A plain number field of a row: a count, a percentage, or a value in thousandths.
struct PlainRow<'p> {
    label: Label,
    glyph: GlyphLabel,
    value: Mixed<Option<f64>>,
    range: RangeInclusive<f64>,
    percent: bool,
    presets: &'p [Entry<NumberPick>],
}

impl Lengths<'_> {
    /// A field over a length in points, shown in the unit: its event in points and the preset row picked.
    fn row(&self, ui: &mut Ui, spec: LengthRow<'_>) -> (Option<EditEvent<Option<f64>>>, Option<NumberPick>) {
        let LengthRow { label, glyph, value, range, presets } = spec;
        let points = value.map_or(Mixed::Mixed, |value| Mixed::Same(Some(f64::from(value))));
        let shown = Field::new(label.clone()).side(mark(glyph)).show(ui, |ui, _| {
            let out = length_field(self.units, label.clone(), self.unit, points, range.clone()).presets(presets).scrub_by_label().show(ui);
            (out.edit.event, out.preset)
        });
        match shown.inner {
            Some((event, picked)) => (event.map(|event| length_edit(self.units, self.unit, event)), picked),
            None => (None, None),
        }
    }

    /// The kerning field: a number or the name of a presets row (Metrics, Optical).
    fn kerning(
        &self,
        ui: &mut Ui,
        label: Label,
        glyph: GlyphLabel,
        value: Option<NamedOr<KerningName>>,
        presets: &[Entry<NumberPick>],
    ) -> Option<EditEvent<NamedOr<KerningName>>> {
        let units = self.units;
        let shown = Field::new(label.clone()).side(mark(glyph)).show(ui, |ui, _| {
            let field = NumberField::named(label.clone(), Bind::given(mixed(value))).range(TRACKING_RANGE).presets(presets).scrub_by_label();
            field.codec(NumberCodec::new(units.reader(None), 0)).show(ui).edit.event
        });
        shown.inner.flatten()
    }

    /// A field over a number that is not a length: its event and the preset row picked.
    fn plain(&self, ui: &mut Ui, spec: PlainRow<'_>) -> (Option<EditEvent<Option<f64>>>, Option<NumberPick>) {
        let PlainRow { label, glyph, value, range, percent, presets } = spec;
        let units = self.units;
        let shown = Field::new(label.clone()).side(mark(glyph)).show(ui, |ui, _| {
            let field = NumberField::new(label.clone(), Bind::given(value)).range(range.clone()).presets(presets).scrub_by_label();
            let field = if percent {
                field.unit_kind(UnitKind::Percent).unit(units.symbol(UnitKind::Percent)).step(UnitKind::Percent.step())
            } else {
                field.codec(NumberCodec::new(units.reader(None), 0))
            };
            let out = field.show(ui);
            (out.edit.event, out.preset)
        });
        shown.inner.unwrap_or((None, None))
    }
}

/// A label shown as a mark, whose drag scrubs the field.
fn mark(glyph: GlyphLabel) -> SideLabel {
    SideLabel::new().kind(LabelKind::Glyph(glyph)).scrub(true)
}

/// `value` as a field's value: a missing one is mixed.
fn mixed<T>(value: Option<T>) -> Mixed<T> {
    value.map_or(Mixed::Mixed, Mixed::Same)
}

/// `value` as a plain number field's value: a missing one is mixed.
fn plain(value: Option<f32>) -> Mixed<Option<f64>> {
    value.map_or(Mixed::Mixed, |value| Mixed::Same(Some(f64::from(value))))
}

/// A number field's event as a change; a cleared field is not one.
fn number_change<C>(event: Option<EditEvent<Option<f64>>>, change: impl Fn(f32) -> C) -> Option<EditEvent<C>> {
    let event = event?;
    if matches!(event.value(), Some(None)) {
        return None;
    }
    Some(event.map(|value| change(value.unwrap_or_default() as f32)))
}

/// A menu row that is not a number, as the one-step change it makes.
fn picked_event<C>(change: C, name: &str) -> EditEvent<C> {
    EditEvent::set(GestureId::default(), Mixed::Mixed, change, Arc::from(name), None)
}

/// The rows of a presets menu over groups of numbers, a separator between the groups.
fn number_presets<'g>(units: &Units, groups: impl Iterator<Item = &'g [f32]>) -> Vec<Entry<NumberPick>> {
    let mut rows = Vec::new();
    for (at, group) in groups.enumerate() {
        if at > 0 {
            rows.push(Entry::separator());
        }
        rows.extend(group.iter().map(|&p| Entry::new(NumberPick::Number(f64::from(p)), units.number(f64::from(p), 0..=PRESET_DECIMALS))));
    }
    rows
}

/// Select entries over `(value, text)` pairs.
fn string_entries(pairs: impl Iterator<Item = (String, String)>) -> Vec<Entry<String>> {
    pairs.map(|(value, text)| Entry::new(value, text)).collect()
}

/// A row with a select over `entries`; its pick as a change.
fn select_row<C>(ui: &mut Ui, label: &Label, value: Mixed<String>, entries: &[Entry<String>], change: impl Fn(String) -> C) -> Option<EditEvent<C>> {
    let side = SideLabel::new().kind(LabelKind::Hidden);
    let shown = Field::new(label.clone())
        .side(side)
        .show(ui, |ui, _| Select::new(label.clone(), Bind::given(value.clone()), entries).placeholder("—").show(ui).event);
    shown.inner.flatten().map(|event| event.map(&change))
}

/// One toggle button of a row: its name, icon, state (`None` while the selection mixes) and the change it makes.
struct Toggle<C> {
    name: Label,
    icon: &'static str,
    on: Option<bool>,
    change: Box<dyn Fn(bool) -> C>,
}

impl<C> Toggle<C> {
    fn new(name: &Label, icon: &'static str, on: Option<bool>, change: Box<dyn Fn(bool) -> C>) -> Self {
        Toggle { name: name.clone(), icon, on, change }
    }
}

/// A row of icon toggle buttons; the pressed ones report their change when clicked.
fn toggle_row<C>(ui: &mut Ui, toggles: Vec<Toggle<C>>) -> Vec<EditEvent<C>> {
    let mut edits = Vec::new();
    Row::new().wrap(true).show(ui, |ui, _| {
        for toggle in &toggles {
            let bind = Bind::given(mixed(toggle.on));
            let out = ToggleButton::new(toggle.name.clone(), bind).icon(IconId::new(toggle.icon)).icon_only().show(ui);
            edits.extend(out.event.map(|event| event.map(&toggle.change)));
        }
    });
    edits
}

/// The OpenType buttons' icons in [`OPENTYPE_FEATURES`] order. The sets have no OpenType marks: the three ligature
/// features share the ligature icon, swash and stylistic alternates use the variable-font and similar-font icons,
/// titling the type icon, ordinals the superscript icon and fractions the slash.
const OPENTYPE_ICONS: [&str; 8] = ["ligature", "ligature", "ligature", "font_variable", "font_similar", "text", "text_superscript", "slash"];
