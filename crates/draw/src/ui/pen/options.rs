//! The pen's Path Options: the rubber band, the path colour and thickness
//! ([`PathOptionsSettings`]), and the gear popover that edits them ([`PathOptionsPopover`]).

use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_theme::{Rgba, Srgba};
use craft_ui::composites::{FieldControl as _, FormGrid};
use craft_ui::controls::{Checkbox, IconButton, NumberField, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose};
use craft_ui::theme::{Tokens, TokensOf as _};

craft_ui::words! {
    /// The words of [`PathOptionsPopover`], English by default (Photoshop's Pen gear popover).
    #[non_exhaustive]
    pub struct PathOptionsLabels {
        /// The gear button's accessible name and tooltip, and the popover's title.
        title: "pathOptions.title" = "Path Options";
        /// The thickness number field.
        thickness: "pathOptions.thickness" = "Thickness";
        /// The colour swatch button, and the colour picker's title.
        color: "pathOptions.color" = "Color";
        /// The rubber band checkbox.
        rubber_band: "pathOptions.rubberBand" = "Rubber Band";
    }
}

/// The [`ColorRequest::key`] of the path colour the popover asks for.
pub const PATH_OPTIONS_COLOR_KEY: &str = "pathOptions.color";

/// The pen's Path Options, persisted by the app with its preferences (Photoshop's Pen gear
/// popover; VectorCraft's two rubber band preferences map to the one setting plus a
/// [`PenDraft::path_options`](super::PenDraft::path_options) per tool). Serialised in camelCase;
/// unknown fields are ignored and missing ones take their defaults.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PathOptionsSettings {
    /// The Pen's rubber band from the last anchor to the pointer. Off by default, as Photoshop
    /// (PhotoCraft always draws it; its users turn this on once). The Curvature mode always draws
    /// its preview curve.
    pub rubber_band: bool,
    /// The path colour as sRGB bytes; `None` uses `overlay.line`. Paths with a colour of their
    /// own (a layer colour) keep it.
    pub color: Option<[u8; 3]>,
    /// The path line's width in points, within [`THICKNESS_RANGE`](Self::THICKNESS_RANGE);
    /// `None` uses `bezier.line` for paths and `bezier.draft_line` for the draft.
    pub thickness: Option<f32>,
}

impl PathOptionsSettings {
    /// The thicknesses the popover offers, in points.
    pub const THICKNESS_RANGE: RangeInclusive<f32> = 0.5..=8.0;
}

/// What [`PathOptionsPopover::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PathOptionsPopoverResponse {
    /// The gear button.
    pub response: Response,
    /// The settings changed: the checkbox is a `Set`, the thickness field its own gesture (a typed
    /// value a `Set`, a scrub `Begin`, `Update`, `Commit`). The value is the whole new
    /// [`PathOptionsSettings`]; they are preferences, so the host stores the latest value and makes
    /// no undo step.
    pub event: Option<EditEvent<PathOptionsSettings>>,
    /// The colour swatch was clicked: the app opens its colour picker keyed
    /// [`PATH_OPTIONS_COLOR_KEY`] and, on OK, stores the colour in
    /// [`PathOptionsSettings::color`] itself.
    pub pick_color: Option<ColorRequest>,
}

/// The Pen's gear button in the options bar and its Path Options popover (Photoshop's): a
/// "Thickness" number field (points, [`PathOptionsSettings::THICKNESS_RANGE`], empty while the
/// token's width is used), a "Color" swatch button showing the path colour (`overlay.line` while
/// unset), and a "Rubber Band" checkbox.
///
/// ```no_run
/// use craft_draw::ui::pen::{PathOptionsPopover, PathOptionsSettings};
/// # fn demo(ui: &mut egui::Ui, settings: &mut PathOptionsSettings, open: &mut bool) {
/// let r = PathOptionsPopover::new("pen-path-options", settings).show(ui, open);
/// if let Some(value) = r.event.as_ref().and_then(|e| e.value()) {
///     *settings = *value;
/// }
/// # }
/// ```
///
/// - The gear is an `IconButton` (`Icon::Settings`) named [`PathOptionsLabels::title`]; a click
///   or Enter toggles the popover (craft-ui `Popover`, opening below it). Focus moves into the
///   popover; Tab goes Thickness, Color, Rubber Band; Esc closes it and returns focus to the gear;
///   a press outside closes it.
/// - AccessKit: the popover is a group named [`PathOptionsLabels::title`] holding a number field,
///   a button and a checkbox with their names.
/// - The popover reports every change at once; there is no OK button.
pub struct PathOptionsPopover<'a> {
    id_salt: IdSalt,
    settings: &'a PathOptionsSettings,
    labels: Option<PathOptionsLabels>,
}

impl<'a> PathOptionsPopover<'a> {
    /// The gear and popover for the host's `settings`.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a PathOptionsSettings) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: PathOptionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the gear button to `ui` and, while `open`, the popover under it. The host owns `open`.
    pub fn show(self, ui: &mut Ui, open: &mut bool) -> PathOptionsPopoverResponse {
        // new: Photoshop's Pen gear popover (Thickness, Color, Rubber Band); PhotoCraft has no
        // such options (step7-pen lead call 12). Built from `IconButton`, `Popover`,
        // `NumberField` (unit "pt"), `Checkbox` and a swatch `Button` asking through
        // `ColorRequest::new(PATH_OPTIONS_COLOR_KEY, ..).title(labels.color)`.
        let labels = self.labels.unwrap_or_else(|| PathOptionsLabels::of(ui));
        let settings = *self.settings;
        let id = ui.make_persistent_id(self.id_salt);

        let response = ui.add(IconButton::new(Icon::Settings, labels.title.as_str()).id_salt((id, "gear")).selected(*open));
        if response.clicked() {
            *open = !*open;
        }

        let numbers = LocaleNumbers::of(ui);
        let tokens = Tokens::of(ui);
        let shown: Rgba = settings.color.map_or(tokens.color.overlay.line, |[r, g, b]| Rgba::rgb(r, g, b));
        let (mut event, mut pick_color) = (None, None);
        Popover::new((id, "popover"), PopoverAnchor::Control(response.clone()))
            .close(PopoverClose::default().swallow_outside(true))
            .toggle(response.id)
            .label(labels.title.as_str())
            .show(ui, open, |ui, _| {
                let mut rubber_band = settings.rubber_band;
                FormGrid::new((id, "grid")).show(ui, |form| {
                    let range = PathOptionsSettings::THICKNESS_RANGE;
                    let edit = form
                        .row(&labels.thickness, |ui, _| {
                            NumberField::optional((id, "thickness"), labels.thickness.as_str(), settings.thickness.map(f64::from), &numbers)
                                .range(range)
                                .step(0.5)
                                .decimals(0..=1)
                                .unit(Unit::new("{n} pt"))
                                .show_optional(ui)
                        })
                        .inner;
                    event = edit.event.map(|e| e.map(|v| PathOptionsSettings { thickness: v.map(|v| v as f32), ..settings }));
                    let swatch = form.row(&labels.color, |ui, _| ui.add(Swatch::new((id, "color"), labels.color.as_str(), shown)).own_name()).inner;
                    if swatch.control().clicked() {
                        pick_color = Some(ColorRequest::new(PATH_OPTIONS_COLOR_KEY, Srgba::from(shown)).title(labels.color.as_str()));
                    }
                    let check =
                        form.unlabelled(|ui| Checkbox::bool(labels.rubber_band.as_str(), &mut rubber_band).id_salt((id, "rubber")).show(ui)).inner;
                    if event.is_none() {
                        event = check.event.map(|e| e.map(|v| PathOptionsSettings { rubber_band: v, ..settings }));
                    }
                });
            });
        PathOptionsPopoverResponse { response, event, pick_color }
    }
}
