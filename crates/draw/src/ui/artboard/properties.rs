//! [`ArtboardProperties`]: an artboard's section in the Properties panel (task SH-T8).

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};
use kurbo::Rect;

use super::bar::SIDE;
use super::{ArtboardBackground, ArtboardItem, ArtboardLabels, ArtboardPreset};
use craft_ui::composites::{PropertyGroup, PropertyLabel, PropertyRow, SectionHeader};
use craft_ui::controls::{NumberField, Select, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, UnitKind, Units};

/// The range of X and Y, pixels.
const OFFSET: std::ops::RangeInclusive<f64> = -300_000.0..=300_000.0;

/// The background dropdown's rows.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Choice {
    White,
    Black,
    Transparent,
    Other,
}

/// What [`ArtboardProperties::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app applies the changes and answers the colour request"]
pub struct ArtboardPropertiesOutput {
    /// The board's new rect from the preset, W, H, X or Y (`Set`, or a caption scrub's gesture).
    pub rect: Option<EditEvent<Rect>>,
    /// A background picked from the dropdown (White, Black, Transparent, or Other… once its colour
    /// came back).
    pub background: Option<ArtboardBackground>,
    /// Other… was chosen (or its swatch clicked): the app opens its color picker with this request
    /// (titled "Artboard Background Color") and sets `Custom(color)` from the answer.
    pub pick_color: Option<ColorRequest>,
}

/// The Artboard section: Size preset, W and H (1–300,000 px), X and Y (±300,000 px, all four with
/// their pixel unit), Background
/// (White, Black, Transparent, Other… with a swatch of the custom colour). A collapsible
/// `SectionHeader` titled "Artboard"; every field a Tab stop in reading order; the grid mirrors
/// right to left.
pub struct ArtboardProperties<'a> {
    id_salt: IdSalt,
    board: &'a ArtboardItem,
    presets: &'a [ArtboardPreset],
    labels: Option<ArtboardLabels>,
}

impl<'a> ArtboardProperties<'a> {
    /// The section of `board`, with the app's `presets`.
    pub fn new(id_salt: impl AsIdSalt, board: &'a ArtboardItem, presets: &'a [ArtboardPreset]) -> Self {
        ArtboardProperties { id_salt: IdSalt::new(id_salt), board, presets, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ArtboardLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draw the section and return what the user changed.
    pub fn show(self, ui: &mut Ui) -> ArtboardPropertiesOutput {
        // pc: crates/ui-egui/src/canvas/artboard_ui.rs:129-205 `properties` (Artboard :137, X :166,
        // Y :167, the background dropdown :174-184, Other… :182).
        // new: `SectionHeader`, `NumberField`, `Select`, `Swatch`; `ColorRequest` as `NewGuideDialog`.
        let labels = self.labels.clone().unwrap_or_else(|| ArtboardLabels::of(ui));
        let units = Units::of(ui);
        let numbers = units.numbers();
        let pixels = units.unit(UnitKind::Pixels);
        let id = ui.make_persistent_id(self.id_salt);
        let mut out = ArtboardPropertiesOutput::default();
        let rect = self.board.rect.abs();
        let (width, height) = (rect.width(), rect.height());
        let background = self.board.background;
        SectionHeader::new((id, "section"), labels.artboard.as_str()).show(ui, |ui| {
            PropertyGroup::new((id, "rows")).show(ui, |ui| {
                let mut choice = self.presets.iter().position(|p| p.size.width == width && p.size.height == height);
                PropertyRow::word(labels.size.as_str()).show(ui, |ui, _| {
                    let mut select = Select::new((id, "preset"), &labels.preset, &mut choice).option(None, labels.custom.as_str());
                    for (i, preset) in self.presets.iter().enumerate() {
                        select = select.option(Some(i), preset.name.as_str());
                    }
                    let pick = select.show(ui);
                    if let Some(EditEvent::Commit { value: Some(i), modifiers, .. } | EditEvent::Set { value: Some(i), modifiers, .. }) = pick.event
                        && let Some(preset) = self.presets.get(i)
                    {
                        let moved = Rect::new(rect.x0, rect.y0, rect.x0 + preset.size.width, rect.y0 + preset.size.height);
                        out.rect = Some(EditEvent::set(moved, modifiers));
                    }
                    pick
                });
                let field = |ui: &mut Ui, salt: &str, name: &str, value: f64, range: &std::ops::RangeInclusive<f64>| {
                    NumberField::new((id, salt), name, value, numbers).range(range.clone()).decimals(0..=0).unit(pixels.clone()).show(ui)
                };
                let (w, h) = PropertyRow::pair(PropertyLabel::glyph(labels.width.as_str()), PropertyLabel::glyph(labels.height.as_str()))
                    .show(ui, |ui, _| field(ui, "w", &labels.width_name, width, &SIDE), |ui, _| field(ui, "h", &labels.height_name, height, &SIDE))
                    .inner;
                let resized = |size: (f64, f64)| Rect::new(rect.x0, rect.y0, rect.x0 + size.0, rect.y0 + size.1);
                let (x, y) = PropertyRow::pair(PropertyLabel::glyph(labels.x.as_str()), PropertyLabel::glyph(labels.y.as_str()))
                    .show(ui, |ui, _| field(ui, "x", &labels.x, rect.x0, &OFFSET), |ui, _| field(ui, "y", &labels.y, rect.y0, &OFFSET))
                    .inner;
                let moved = |at: (f64, f64)| Rect::new(at.0, at.1, at.0 + width, at.1 + height);
                out.rect = out
                    .rect
                    .take()
                    .or(w.event.map(|e| e.map(|v| resized((v, height)))))
                    .or(h.event.map(|e| e.map(|v| resized((width, v)))))
                    .or(x.event.map(|e| e.map(|v| moved((v, rect.y0)))))
                    .or(y.event.map(|e| e.map(|v| moved((rect.x0, v)))));

                let custom = match background {
                    ArtboardBackground::Custom(c) => Some(c),
                    _ => None,
                };
                let request = |color: Option<Srgba>| {
                    ColorRequest::new("artboard-background", color.unwrap_or(Srgba::rgb(1.0, 1.0, 1.0))).title(labels.background_color.as_str())
                };
                PropertyRow::word(labels.background.as_str()).show(ui, |ui, _| {
                    let mut now = match background {
                        ArtboardBackground::White => Choice::White,
                        ArtboardBackground::Black => Choice::Black,
                        ArtboardBackground::Transparent => Choice::Transparent,
                        ArtboardBackground::Custom(_) => Choice::Other,
                    };
                    let pick = ui
                        .horizontal(|ui| {
                            let pick = Select::new((id, "background"), &labels.background, &mut now)
                                .option(Choice::White, labels.white.as_str())
                                .option(Choice::Black, labels.black.as_str())
                                .option(Choice::Transparent, labels.transparent.as_str())
                                .option(Choice::Other, labels.other.as_str())
                                .show(ui);
                            if let Some(color) = custom
                                && ui.add(Swatch::new((id, "swatch"), labels.background_color.as_str(), color)).clicked()
                            {
                                out.pick_color = Some(request(Some(color)));
                            }
                            pick
                        })
                        .inner;
                    if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = pick.event {
                        match value {
                            Choice::White => out.background = Some(ArtboardBackground::White),
                            Choice::Black => out.background = Some(ArtboardBackground::Black),
                            Choice::Transparent => out.background = Some(ArtboardBackground::Transparent),
                            Choice::Other => out.pick_color = Some(request(custom)),
                        }
                    }
                    pick
                });
            });
        });
        out
    }
}
