//! [`CharacterControls`] and [`ParagraphControls`] (F14.04): Photoshop's Character and Paragraph
//! panels as controls for a panel body, a Properties section or a style options editor. Values
//! are in points (sizes, leading, baseline shift, indents, spacing) and thousandths of an em
//! (kerning, tracking); the fields show them in the app's type unit ([`TypeUnit`]). The controls
//! change nothing themselves: each returns the one change the user made this pass.

#[cfg(feature = "font-picker")]
use std::sync::Arc;

#[cfg(feature = "font-picker")]
use craft_font_manager::Catalog;
#[cfg(feature = "font-picker")]
use craft_font_manager::picker::{FontFilters, FontPicker, FontPickerSettings};
use craft_ui::controls::expr::{self, ExprUnits};
use craft_ui::controls::{CheckState, Checkbox, ControlSize, IconButton, NumberField, Select, Swatch, TextField};
use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy};
use craft_ui::icons::Icon;
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::load::Bytes;
use egui::{Color32, ImageSource, Ui, vec2};

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
        kerning_presets: "character.kerningPresets" = "Kerning presets";
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
        match self {
            TypeUnit::Points => 1.0,
            TypeUnit::Pixels { ppi } => ppi.max(1.0) / 72.0,
            TypeUnit::Millimeters => 25.4 / 72.0,
        }
    }

    /// The unit's word in `labels` (`points`, `pixels`, `millimeters`; English "pt", "px", "mm");
    /// [`ParagraphControls`] uses the same words.
    // pc: crates/ui-egui/src/tools/type_tool/defaults.rs:62 `type_unit`.
    pub fn suffix(self, labels: &CharacterLabels) -> &str {
        match self {
            TypeUnit::Points => &labels.points,
            TypeUnit::Pixels { .. } => &labels.pixels,
            TypeUnit::Millimeters => &labels.millimeters,
        }
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
    /// A field gesture is under way (a scrub, or a typing run before its Enter): the app merges
    /// the changes of consecutive passes into one history step (Photoshop's single step per
    /// scrub). The pass that ends the gesture carries the last change with this `false`; Esc
    /// reports the value from before the gesture.
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
///   scales 0–1000 %. Kerning takes Metrics, Optical or a number. The leading field's Auto
///   preset (and a typed or scrubbed 0) is [`Leading::Auto`]; while leading is Auto the field
///   shows the font size × 120 %.
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
        let labels = self.labels.unwrap_or_else(|| CharacterLabels::of(ui));
        let values = self.values;
        let scale = f64::from(self.unit.per_point());
        let (length, context) = self.unit.length();
        let suffix = self.unit.suffix(&labels).to_owned();
        let template = format!("{{n}} {suffix}");
        let size_presets: Vec<(String, f32)> = SIZE_PRESETS.iter().map(|&p| (preset_label(f64::from(p) * scale, &suffix), p)).collect();
        let mut leading_presets = vec![(labels.auto.clone(), 0.0)];
        leading_presets.extend(size_presets.iter().cloned());
        let tracking_presets: Vec<(String, f32)> =
            TRACKING_PRESETS.iter().flat_map(|group| group.iter()).map(|&p| (preset_label(f64::from(p), ""), p)).collect();
        let mut pass = Pass::default();
        let auto_leading = values.size.map(|size| size * AUTO_LEADING);

        ui.push_id("craft_text::character", |ui| {
            cell(ui, |ui| {
                #[cfg(feature = "font-picker")]
                if let Some((catalog, settings, filters)) = self.picker {
                    let mut family = values.family.clone().unwrap_or_default();
                    let width = ui.available_width();
                    let r = FontPicker::new("family", catalog, &mut family).settings(&mut *settings).filters(filters).width(width).show(ui);
                    if let Some(picked) = r.picked {
                        settings.note_recent(&picked);
                        pass.change = Some(CharacterChange::Family(picked));
                    }
                    pass.preview = r.preview;
                    return;
                }
                if let Some(family) = choose(
                    ui,
                    "family",
                    &labels.family,
                    values.family.as_deref(),
                    self.families.len(),
                    |i| self.families[i].clone(),
                    |i| self.families[i].clone(),
                ) {
                    pass.change = Some(CharacterChange::Family(family));
                }
            });
            cell(ui, |ui| {
                if let Some(style) = choose(
                    ui,
                    "style",
                    &labels.style,
                    values.style.as_deref(),
                    self.styles.len(),
                    |i| self.styles[i].clone(),
                    |i| self.styles[i].clone(),
                ) {
                    pass.change = Some(CharacterChange::Style(style));
                }
            });

            let metric = |salt, glyph, name: &str, value: Option<f32>, range: (f32, f32)| Metric {
                salt,
                glyph,
                name: name.to_owned(),
                value,
                range,
                scale,
                unit: Some(length),
                context,
                suffix: suffix.clone(),
                template: template.clone(),
                presets: Vec::new(),
            };
            let plain = |salt, glyph, name: &str, value: Option<f32>, range: (f32, f32), suffix: &str| Metric {
                salt,
                glyph,
                name: name.to_owned(),
                value,
                range,
                scale: 1.0,
                unit: None,
                context,
                suffix: suffix.to_owned(),
                template: if suffix.is_empty() { "{n}".to_owned() } else { format!("{{n}}{suffix}") },
                presets: Vec::new(),
            };

            ui.columns(2, |cols| {
                let size = Metric { presets: size_presets.clone(), ..metric("size", "tT", &labels.size, values.size, (0.1, 1296.0)) };
                if let Some(moved) = cell(&mut cols[0], |ui| field(ui, size)) {
                    pass.moved(moved, CharacterChange::Size);
                }
                let leading_value = match values.leading {
                    Some(Leading::Points(p)) => Some(p),
                    Some(Leading::Auto) => auto_leading,
                    None => None,
                };
                let leading = Metric { presets: leading_presets.clone(), ..metric("leading", "A↕", &labels.leading, leading_value, (0.0, 5000.0)) };
                if let Some(moved) = cell(&mut cols[1], |ui| field(ui, leading)) {
                    pass.moved(
                        moved,
                        |p| if p <= 0.0 { CharacterChange::Leading(Leading::Auto) } else { CharacterChange::Leading(Leading::Points(p)) },
                    );
                }
            });
            ui.columns(2, |cols| {
                if let Some(kerning) = cell(&mut cols[0], |ui| kerning_field(ui, &labels, values.kerning)) {
                    pass.change = Some(CharacterChange::Kerning(kerning));
                }
                let tracking = Metric {
                    presets: tracking_presets.clone(),
                    ..plain("tracking", "VA", &labels.tracking, values.tracking, (-1000.0, 10000.0), "")
                };
                if let Some(moved) = cell(&mut cols[1], |ui| field(ui, tracking)) {
                    pass.moved(moved, CharacterChange::Tracking);
                }
            });
            ui.columns(2, |cols| {
                let vertical = plain("vertical_scale", "↕T", &labels.vertical_scale, values.vertical_scale, (0.0, 1000.0), "%");
                if let Some(moved) = cell(&mut cols[0], |ui| field(ui, vertical)) {
                    pass.moved(moved, CharacterChange::VerticalScale);
                }
                let horizontal = plain("horizontal_scale", "↔T", &labels.horizontal_scale, values.horizontal_scale, (0.0, 1000.0), "%");
                if let Some(moved) = cell(&mut cols[1], |ui| field(ui, horizontal)) {
                    pass.moved(moved, CharacterChange::HorizontalScale);
                }
            });
            ui.columns(2, |cols| {
                let shift = metric("baseline_shift", "Aª", &labels.baseline_shift, values.baseline_shift, (-1296.0, 1296.0));
                if let Some(moved) = cell(&mut cols[0], |ui| field(ui, shift)) {
                    pass.moved(moved, CharacterChange::BaselineShift);
                }
                cell(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Text::new(&labels.color).content(Content::Muted));
                        let color = values.color.unwrap_or(Color32::TRANSPARENT);
                        let width = ui.available_width();
                        let swatch = Swatch::new("color", &labels.text_color, color)
                            .size(ControlSize::Sm)
                            .dimensions(width, ui.spacing().interact_size.y)
                            .mixed(values.color.is_none());
                        pass.pick_color |= ui.add(swatch).clicked();
                    });
                });
            });

            let caps = values.caps;
            let position = values.position;
            let styles: [(&str, &str, &str, bool, CharacterChange); 4] = [
                (
                    "faux_bold",
                    FAUX_BOLD,
                    &labels.faux_bold,
                    values.faux_bold == Some(true),
                    CharacterChange::FauxBold(values.faux_bold != Some(true)),
                ),
                (
                    "faux_italic",
                    FAUX_ITALIC,
                    &labels.faux_italic,
                    values.faux_italic == Some(true),
                    CharacterChange::FauxItalic(values.faux_italic != Some(true)),
                ),
                (
                    "all_caps",
                    ALL_CAPS,
                    &labels.all_caps,
                    caps == Some(Caps::AllCaps),
                    CharacterChange::Caps(if caps == Some(Caps::AllCaps) { Caps::Normal } else { Caps::AllCaps }),
                ),
                (
                    "small_caps",
                    SMALL_CAPS,
                    &labels.small_caps,
                    caps == Some(Caps::SmallCaps),
                    CharacterChange::Caps(if caps == Some(Caps::SmallCaps) { Caps::Normal } else { Caps::SmallCaps }),
                ),
            ];
            let decorations: [(&str, &str, &str, bool, CharacterChange); 2] = [
                (
                    "underline",
                    UNDERLINE,
                    &labels.underline,
                    values.underline == Some(true),
                    CharacterChange::Underline(values.underline != Some(true)),
                ),
                (
                    "strikethrough",
                    STRIKETHROUGH,
                    &labels.strikethrough,
                    values.strikethrough == Some(true),
                    CharacterChange::Strikethrough(values.strikethrough != Some(true)),
                ),
            ];
            let positions: [(&str, &str, &str, bool, CharacterChange); 2] = [
                (
                    "superscript",
                    SUPERSCRIPT,
                    &labels.superscript,
                    position == Some(BaselinePosition::Superscript),
                    CharacterChange::Position(if position == Some(BaselinePosition::Superscript) {
                        BaselinePosition::Normal
                    } else {
                        BaselinePosition::Superscript
                    }),
                ),
                (
                    "subscript",
                    SUBSCRIPT,
                    &labels.subscript,
                    position == Some(BaselinePosition::Subscript),
                    CharacterChange::Position(if position == Some(BaselinePosition::Subscript) {
                        BaselinePosition::Normal
                    } else {
                        BaselinePosition::Subscript
                    }),
                ),
            ];
            let row = styles.into_iter().chain(self.superscript.then_some(positions).into_iter().flatten()).chain(decorations);
            buttons(ui, row, |_| true, &mut pass);

            let names = [
                &labels.standard_ligatures,
                &labels.contextual_alternates,
                &labels.discretionary_ligatures,
                &labels.swash,
                &labels.stylistic_alternates,
                &labels.titling_alternates,
                &labels.ordinals,
                &labels.fractions,
            ];
            let features = OPENTYPE_FEATURES.iter().zip(OPENTYPE_GLYPHS).zip(names).map(|((&tag, svg), name)| {
                let on = values.opentype.contains(&tag);
                (tag, svg, name.as_str(), on, CharacterChange::OpenType { tag, on: !on })
            });
            buttons(ui, features, |_| true, &mut pass);

            if let Some(languages) = self.languages {
                cell(ui, |ui| {
                    let current = values.language.as_deref();
                    let tags = |i: usize| languages[i].0.clone();
                    if let Some(tag) = choose(ui, "language", &labels.language, current, languages.len(), tags, |i| languages[i].1.clone()) {
                        pass.change = Some(CharacterChange::Language(tag));
                    }
                });
            }
        });
        CharacterResponse { change: pass.change, preview_family: pass.preview, pick_color: pass.pick_color, scrubbing: pass.scrubbing }
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
        let char_labels = CharacterLabels::of(ui);
        let labels = self.labels.unwrap_or_else(|| ParagraphLabels::of(ui));
        let values = self.values;
        let scale = f64::from(self.unit.per_point());
        let (length, context) = self.unit.length();
        let suffix = self.unit.suffix(&char_labels).to_owned();
        let template = format!("{{n}} {suffix}");
        let mut pass = Pass::default();
        let metric = |salt, glyph, name: &str, value: Option<f32>, range: (f32, f32)| Metric {
            salt,
            glyph,
            name: name.to_owned(),
            value,
            range,
            scale,
            unit: Some(length),
            context,
            suffix: suffix.clone(),
            template: template.clone(),
            presets: Vec::new(),
        };

        ui.push_id("craft_text::paragraph", |ui| {
            use TextAlign::*;
            let aligns = [
                ("left", ALIGN_LEFT, &labels.left, Left),
                ("center", ALIGN_CENTER, &labels.center, Center),
                ("right", ALIGN_RIGHT, &labels.right, Right),
                ("justify_left", JUSTIFY_LEFT, &labels.justify_left, JustifyLeft),
                ("justify_center", JUSTIFY_CENTER, &labels.justify_center, JustifyCenter),
                ("justify_right", JUSTIFY_RIGHT, &labels.justify_right, JustifyRight),
                ("justify_all", JUSTIFY_ALL, &labels.justify_all, JustifyAll),
            ];
            let row = aligns
                .into_iter()
                .map(|(salt, svg, name, align)| (salt, svg, name.as_str(), values.align == Some(align), ParagraphChange::Align(align)));
            buttons(
                ui,
                row,
                |change| !(self.point_text && matches!(change, ParagraphChange::Align(JustifyLeft | JustifyCenter | JustifyRight | JustifyAll))),
                &mut pass,
            );

            use ParagraphDirection::*;
            let directions = [
                ("direction_auto", DIRECTION_AUTO, &labels.direction_auto, Automatic),
                ("direction_ltr", DIRECTION_LTR, &labels.direction_ltr, LeftToRight),
                ("direction_rtl", DIRECTION_RTL, &labels.direction_rtl, RightToLeft),
            ];
            let row = directions
                .into_iter()
                .map(|(salt, svg, name, d)| (salt, svg, name.as_str(), values.direction == Some(d), ParagraphChange::Direction(d)));
            buttons(ui, row, |_| true, &mut pass);

            ui.columns(2, |cols| {
                let start = metric("start_indent", "→|", &labels.start_indent, values.start_indent, (-1296.0, 1296.0));
                if let Some(moved) = cell(&mut cols[0], |ui| field(ui, start)) {
                    pass.moved(moved, ParagraphChange::StartIndent);
                }
                let end = metric("end_indent", "|←", &labels.end_indent, values.end_indent, (-1296.0, 1296.0));
                if let Some(moved) = cell(&mut cols[1], |ui| field(ui, end)) {
                    pass.moved(moved, ParagraphChange::EndIndent);
                }
            });
            ui.columns(2, |cols| {
                let first = metric("first_line_indent", "¶→", &labels.first_line_indent, values.first_line_indent, (-1296.0, 1296.0));
                if let Some(moved) = cell(&mut cols[0], |ui| field(ui, first)) {
                    pass.moved(moved, ParagraphChange::FirstLineIndent);
                }
            });
            ui.columns(2, |cols| {
                let before = metric("space_before", "↑¶", &labels.space_before, values.space_before, (0.0, 1296.0));
                if let Some(moved) = cell(&mut cols[0], |ui| field(ui, before)) {
                    pass.moved(moved, ParagraphChange::SpaceBefore);
                }
                let after = metric("space_after", "¶↓", &labels.space_after, values.space_after, (0.0, 1296.0));
                if let Some(moved) = cell(&mut cols[1], |ui| field(ui, after)) {
                    pass.moved(moved, ParagraphChange::SpaceAfter);
                }
            });
            cell(ui, |ui| {
                let mut state = values.hyphenate.map_or(CheckState::Mixed, CheckState::from);
                let edit = Checkbox::new(&labels.hyphenate, &mut state).size(ControlSize::Sm).id_salt("hyphenate").show(ui);
                if let Some(state) = edit.committed() {
                    pass.change = Some(ParagraphChange::Hyphenate(*state == CheckState::Checked));
                }
            });
        });
        ParagraphResponse { change: pass.change, scrubbing: pass.scrubbing }
    }
}

/// Auto leading is the font size × 120 % (Photoshop's default).
const AUTO_LEADING: f32 = 1.2;

impl TypeUnit {
    /// The unit as the length unit its typed text converts through.
    fn length(self) -> (LengthUnit, UnitContext) {
        match self {
            TypeUnit::Points => (LengthUnit::Points, UnitContext::default()),
            TypeUnit::Pixels { ppi } => (LengthUnit::Pixels, UnitContext { pixels_per_inch: f64::from(ppi.max(1.0)), ..UnitContext::default() }),
            TypeUnit::Millimeters => (LengthUnit::Millimetres, UnitContext::default()),
        }
    }
}

/// What the controls collected this pass. Only the last change is kept: one control is used per
/// pass.
struct Pass<C> {
    change: Option<C>,
    scrubbing: bool,
    preview: Option<String>,
    pick_color: bool,
}

impl<C> Default for Pass<C> {
    fn default() -> Self {
        Pass { change: None, scrubbing: false, preview: None, pick_color: false }
    }
}

impl<C> Pass<C> {
    fn moved(&mut self, moved: Moved, change: impl FnOnce(f32) -> C) {
        self.change = Some(change(moved.value));
        self.scrubbing |= moved.live;
    }
}

/// A field's new value in points (or its own unit), and whether its gesture goes on.
struct Moved {
    value: f32,
    live: bool,
}

/// A number field with a glyph label: `value` and `range` are in the model's unit, shown
/// multiplied by `scale`.
struct Metric {
    salt: &'static str,
    glyph: &'static str,
    name: String,
    value: Option<f32>,
    range: (f32, f32),
    scale: f64,
    unit: Option<LengthUnit>,
    context: UnitContext,
    suffix: String,
    template: String,
    /// `(menu text, model value)`.
    presets: Vec<(String, f32)>,
}

/// A preset as the menu shows it: at most two decimals, then the unit.
fn preset_label(shown: f64, suffix: &str) -> String {
    let number = format!("{shown:.2}");
    let number = number.trim_end_matches('0').trim_end_matches('.');
    let number = if number == "-0" { "0" } else { number };
    if suffix.is_empty() { number.to_owned() } else { format!("{number} {suffix}") }
}

/// One row cell: a justified row of the available width, so a field fills its column.
fn cell<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let size = vec2(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, direction::row(ui).with_main_justify(true), add).inner
}

fn field(ui: &mut Ui, m: Metric) -> Option<Moved> {
    let numbers = LocaleNumbers::of(ui);
    let scale = m.scale;
    let presets: Vec<(String, f64)> = m.presets.iter().map(|(text, value)| (text.clone(), f64::from(*value) * scale)).collect();
    let units = ExprUnits { unit: m.unit, suffix: &m.suffix, context: m.context };
    let read = |text: &str| if m.suffix.is_empty() { expr::parse(text) } else { expr::parse_with_units(text, &units) };
    let (low, high) = (f64::from(m.range.0) * scale, f64::from(m.range.1) * scale);
    let shown = m.value.map_or(low, |v| f64::from(v) * scale).clamp(low, high);
    let before = ui.make_persistent_id((m.salt, "before"));
    let mut number = NumberField::new(m.salt, &m.name, shown, &numbers)
        .range(low..=high)
        .decimals(0..=2)
        .unit(Unit::new(&m.template))
        .expressions(true)
        .or_parse(read)
        .scrub_label(m.glyph)
        .mixed(m.value.is_none())
        .size(ControlSize::Sm);
    if !presets.is_empty() {
        number = number.presets(&presets);
    }
    // A picked preset is the model's own number: the field's rounded text would give 11.99 for 12 pt in mm.
    let model =
        |shown: f64| m.presets.iter().find(|(_, p)| (f64::from(*p) * scale - shown).abs() <= 0.0051).map_or((shown / scale) as f32, |(_, p)| *p);
    match number.show(ui).event? {
        EditEvent::Begin { value, .. } => {
            ui.data_mut(|d| d.insert_temp(before, m.value));
            value.map(|v| Moved { value: model(v), live: true })
        }
        EditEvent::Update { value, .. } => Some(Moved { value: model(value), live: true }),
        EditEvent::Set { value, .. } => Some(Moved { value: model(value), live: false }),
        EditEvent::Commit { value, changed, .. } => changed.then(|| Moved { value: model(value), live: false }),
        EditEvent::Cancel { .. } => ui.data(|d| d.get_temp::<Option<f32>>(before)).flatten().map(|value| Moved { value, live: false }),
        _ => None,
    }
}

/// A select over `count` choices, `current` selected (empty while the selection mixes values).
/// Returns the value of the one picked.
fn choose(
    ui: &mut Ui,
    salt: &'static str,
    name: &str,
    current: Option<&str>,
    count: usize,
    value: impl Fn(usize) -> String,
    text: impl Fn(usize) -> String,
) -> Option<String> {
    let mut selected = match current {
        Some(current) => (0..count).map(&value).find(|v| v == current).unwrap_or_else(|| current.to_owned()),
        None => String::new(),
    };
    let edit = Select::new(salt, name, &mut selected).rows(count, value, text).placeholder("—").size(ControlSize::Sm).show(ui);
    edit.committed().cloned()
}

/// The kerning text: Metrics, Optical or the rounded value.
fn kerning_text(labels: &CharacterLabels, kerning: Option<Kerning>) -> String {
    match kerning {
        None => String::new(),
        Some(Kerning::Metrics) => labels.metrics.clone(),
        Some(Kerning::Optical) => labels.optical.clone(),
        Some(Kerning::Manual(v)) => format!("{}", v.round() as i64),
    }
}

/// Metrics or Optical in any case (the installed word or the English one), or a number in
/// -1000..=10000.
fn parse_kerning(text: &str, labels: &CharacterLabels) -> Option<Kerning> {
    let text = text.trim();
    let is = |word: &str, english: &str| {
        text.eq_ignore_ascii_case(word) || text.eq_ignore_ascii_case(english) || text.to_lowercase() == word.to_lowercase()
    };
    if is(&labels.metrics, "metrics") {
        return Some(Kerning::Metrics);
    }
    if is(&labels.optical, "optical") {
        return Some(Kerning::Optical);
    }
    expr::parse(text).filter(|v| v.is_finite() && (-1000.0..=10000.0).contains(v)).map(|v| Kerning::Manual(v.round() as f32))
}

const KERNING_PRESETS: [f32; 14] = [0.0, -100.0, -75.0, -50.0, -25.0, -10.0, -5.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 200.0];

/// Photoshop's kerning combo: a text field that takes Metrics, Optical or a number, and a menu of
/// presets.
fn kerning_field(ui: &mut Ui, labels: &CharacterLabels, kerning: Option<Kerning>) -> Option<Kerning> {
    ui.horizontal(|ui| kerning_row(ui, labels, kerning)).inner
}

fn kerning_row(ui: &mut Ui, labels: &CharacterLabels, kerning: Option<Kerning>) -> Option<Kerning> {
    let mut chosen = None;
    ui.add(Text::new("VA").content(Content::Muted));
    let side = IconButton::style(&Tokens::of(ui), ControlSize::Sm).side;
    let shown = kerning_text(labels, kerning);
    let width = (ui.available_width() - side).max(0.0);
    ui.spacing_mut().item_spacing.x = 0.0;
    let edit = TextField::new("kerning", &labels.kerning, &shown)
        .placeholder("—")
        .size(ControlSize::Sm)
        .policy(EditPolicy::default().on_release_only(true))
        .width(width)
        .show(ui);
    if let Some(EditEvent::Commit { value, changed: true, .. }) = &edit.event {
        chosen = parse_kerning(value, labels);
    }
    let button = ui.add(IconButton::new(Icon::ChevronDown, &labels.kerning_presets).size(ControlSize::Sm).id_salt("kerning_presets"));
    let words = [labels.metrics.clone(), labels.optical.clone()];
    let entries: Vec<MenuEntry<usize>> = words
        .iter()
        .cloned()
        .chain(KERNING_PRESETS.iter().map(|p| format!("{p}")))
        .enumerate()
        .map(|(i, text)| MenuItem::new(text, i).into())
        .collect();
    let at = button.clicked().then_some(OpenAt::Rect(button.rect));
    let menu = ContextMenu::new(&entries).size(ControlSize::Sm).own_gestures(false).open_at(at).show(ui, &button);
    if let Some(i) = menu.chosen {
        chosen = Some(match i {
            0 => Kerning::Metrics,
            1 => Kerning::Optical,
            _ => Kerning::Manual(KERNING_PRESETS[i - 2]),
        });
    }
    chosen
}

/// A row of icon toggle buttons: `(id, drawing, name, pressed, change)`. A button for which
/// `allowed(change)` is false is disabled. The pressed one reports its change when clicked.
fn buttons<'a, C>(
    ui: &mut Ui,
    items: impl IntoIterator<Item = (&'static str, &'static str, &'a str, bool, C)>,
    allowed: impl Fn(&C) -> bool,
    pass: &mut Pass<C>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (salt, svg, name, on, change) in items {
            let source = ImageSource::Bytes { uri: format!("bytes://craft-text/character/{salt}.svg").into(), bytes: Bytes::Static(svg.as_bytes()) };
            let button = IconButton::new(source, name).size(ControlSize::Sm).selected(on).enabled(allowed(&change)).id_salt(salt);
            if ui.add(button).clicked() {
                pass.change = Some(change);
            }
        }
    });
}

/// A 24 × 24 drawing of strokes in white, tinted by the button.
macro_rules! glyph {
    ($width:literal, $path:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width=""#,
            $width,
            r#"" stroke-linecap="round" stroke-linejoin="round"><path d=""#,
            $path,
            r#""/></svg>"#
        )
    };
}

// The type-style marks: a T with the mark of the style. The crate has no icons for them and
// carries no font in the drawing, so each is a few strokes.
const FAUX_BOLD: &str = glyph!("3.6", "M6 5h12M12 5v14");
const FAUX_ITALIC: &str = glyph!("2", "M9 5h10M15.5 5l-3.5 14");
const ALL_CAPS: &str = glyph!("2", "M2.5 8h9M7 8v10M12.5 8h9M17 8v10");
const SMALL_CAPS: &str = glyph!("2", "M3 5h11M8.5 5v14M14.5 11h7M18 11v8");
const SUPERSCRIPT: &str = glyph!("2", "M3 9h10M8 9v11M16 3h5M18.5 3v6");
const SUBSCRIPT: &str = glyph!("2", "M3 4h10M8 4v11M16 15h5M18.5 15v6");
const UNDERLINE: &str = glyph!("2", "M6 4h12M12 4v12M5 20h14");
const STRIKETHROUGH: &str = glyph!("2", "M6 4h12M12 4v16M4 12h16");

/// The OpenType marks in [`OPENTYPE_FEATURES`] order: fi, st, ct with its ligature arc, a swash A,
/// aa, a titling T, 1st and ½.
const OPENTYPE_GLYPHS: [&str; 8] = [
    glyph!("1.8", "M9 19V8a3 3 0 0 1 3-3M6 11h7M16 11v8M16 7v.01"),
    glyph!("1.8", "M10 10c-1-1.5-5-1.5-5 .8s5 1.7 5 4S5.500 18 4 16M15 6v10a2 2 0 0 0 3 .8M12 10h7"),
    glyph!("1.8", "M10 10a4 4 0 1 0 0 7M16 6v9a2 2 0 0 0 3 1M13 10h7M8 8Q12 3 17 6"),
    glyph!("1.8", "M5 17L10 5l5 12M7 13h6M3 21q9-3 18-1"),
    glyph!("1.8", "M10 14a3 3 0 1 0-6 0 3 3 0 1 0 6 0M10 11v6M21 14a3 3 0 1 0-6 0 3 3 0 1 0 6 0M21 11v6"),
    glyph!("2.4", "M5 5h14M12 5v14M8 19h8"),
    glyph!("1.8", "M4 11l2-2v10M12 15h7M15.500 12v6a1.500 1.500 0 0 0 2 .5M10 7h6"),
    glyph!("1.8", "M5 9l2-2v8M18 5L7 19M13 17a2 2 0 0 1 4 0c0 2-4 2-4 4h4"),
];

// Alignment: four lines whose lengths show the mode (PhotoCraft's `align_glyph`).
const ALIGN_LEFT: &str = glyph!("1.6", "M4 5h16M4 9.700h9.900M4 14.300h16M4 19h9.900");
const ALIGN_CENTER: &str = glyph!("1.6", "M4 5h16M7 9.700h10M4 14.300h16M7 19h10");
const ALIGN_RIGHT: &str = glyph!("1.6", "M4 5h16M10 9.700h10M4 14.300h16M10 19h10");
const JUSTIFY_LEFT: &str = glyph!("1.6", "M4 5h16M4 9.700h16M4 14.300h16M4 19h8.800");
const JUSTIFY_CENTER: &str = glyph!("1.6", "M4 5h16M4 9.700h16M4 14.300h16M7.600 19h8.800");
const JUSTIFY_RIGHT: &str = glyph!("1.6", "M4 5h16M4 9.700h16M4 14.300h16M11.200 19h8.800");
const JUSTIFY_ALL: &str = glyph!("1.6", "M4 5h16M4 9.700h16M4 14.300h16M4 19h16");

// Direction: a pilcrow over an arrow the way the paragraph runs; Auto points both ways.
const DIRECTION_LTR: &str = glyph!("1.8", "M13 3v9M16 3v9M16 3h-4.500a3.200 3.200 0 0 0 0 6.400H13M4 19h16M17 16l3 3-3 3");
const DIRECTION_RTL: &str = glyph!("1.8", "M13 3v9M16 3v9M16 3h-4.500a3.200 3.200 0 0 0 0 6.400H13M4 19h16M7 16l-3 3 3 3");
const DIRECTION_AUTO: &str = glyph!("1.8", "M13 3v9M16 3v9M16 3h-4.500a3.200 3.200 0 0 0 0 6.400H13M4 19h16M17 16l3 3-3 3M7 16l-3 3 3 3");
