//! [`FullWindowZoomBar`]: Filter Gallery's zoom bar under a full-window canvas.

use std::sync::Arc;

use egui::{AsIdSalt, Frame, IdSalt, Panel, Response, Ui, UiBuilder};

use super::labels::FullWindowLabels;
use super::style::FullWindowStyle;
use super::view::{FullWindowView, FullWindowZoom};
use crate::controls::{IconButton, Progress, Select};
use crate::direction;
use crate::icons::Icon;
use crate::localized::{LocaleNumbers, SharedLabels as _, Unit};
use crate::primitives::{Content, Text};
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::{ControlSize, TypeRole};

/// What [`FullWindowZoomBar::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FullWindowZoomBarResponse {
    /// The bar's strip.
    pub response: Response,
    /// The view's zoom or fit changed this pass (a button, the select).
    pub changed: bool,
}

/// The strip under a full-window canvas: Zoom Out, Zoom In, a zoom select, then a progress bar
/// while the preview renders or a note. Filter Gallery's preview and Content-Aware Fill's Preview
/// panel show it; the other canvases show the zoom in the title strip only.
///
/// ```ignore
/// FullWindowZoomBar::new("gallery-zoom", &mut view)
///     .progress(preview.progress())
///     .note(format!("Preview {ms} ms"))
///     .show(ui);
/// let canvas = FullWindowCanvas::new("gallery-view", &mut view).source(&preview_image).show(ui);
/// ```
///
/// - **Place.** An `egui::Panel` at the bottom of what is left of the dialog's body (or of a side
///   column's body), `full_window.zoom_bar.height` tall on the side fill (`full_window.side.fill`), padded by
///   `full_window.zoom_bar.pad`. Call it after the side columns and before the canvas it serves.
/// - **Buttons.** Zoom Out and Zoom In [`IconButton`](crate::controls::IconButton)s step the
///   view's ladder ([`FullWindowView::step`]); each is disabled at its end of the range.
/// - **Select.** A [`Select`](crate::controls::Select) "Zoom" of Fit in View, 12.5 %, 25 %, 50 %,
///   100 %, 200 % and 400 %; a zoom off the menu (stepped to 300 %) is listed as itself in its
///   place among them, as PhotoCraft lists it. The fitted view shows Fit in View.
/// - **Progress or note.** While [`progress`](Self::progress) is set, a
///   [`Progress`](crate::controls::Progress) bar `full_window.progress.width` ×
///   `full_window.progress.height` (indeterminate for `Some(None)`); otherwise the
///   [`note`](Self::note) in `text.faint`, `caption` role, cut with an ellipsis.
/// - **Keyboard and AccessKit.** The buttons and the select are Tab stops in reading order; the
///   strip mirrors in a right-to-left interface.
#[must_use = "show the bar with `show`"]
pub struct FullWindowZoomBar<'a> {
    id_salt: IdSalt,
    view: &'a mut FullWindowView,
    progress: Option<Option<f32>>,
    note: Option<String>,
    labels: Option<FullWindowLabels>,
}

impl<'a> FullWindowZoomBar<'a> {
    /// A zoom bar for `view`, the same view the canvas under it shows.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut FullWindowView) -> Self {
        Self { id_salt: IdSalt::new(id_salt), view, progress: None, note: None, labels: None }
    }

    /// The preview's progress: `None` shows the note, `Some(None)` an indeterminate bar,
    /// `Some(Some(f))` a bar at `f` (0 to 1). [`PreviewStatus::progress`](super::PreviewStatus::progress)
    /// gives it.
    pub fn progress(mut self, progress: Option<Option<f32>>) -> Self {
        self.progress = progress;
        self
    }

    /// Text after the select while no progress shows (translated by the app): Filter Gallery's
    /// "Preview 120 ms".
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The words (default: the installed words).
    pub fn labels(mut self, labels: FullWindowLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the bar in `ui` (the bottom of the dialog's body) and applies what the user did to
    /// the view.
    pub fn show(self, ui: &mut Ui) -> FullWindowZoomBarResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:673-700 (the strip 30 high on `t.dock`,
        // padding 10 × 3, buttons "minus" and "plus" at 22, the select of Fit in View and six
        // zooms with the off-menu value inserted in order, the 90 × 6 progress bar, else
        // "Preview N ms" in `t.text_faint` at 11 pt).
        let Self { id_salt, view, progress, note, labels } = self;
        let tokens = Tokens::of(ui);
        let style = FullWindowStyle::of(&tokens);
        let labels = labels.map_or_else(|| FullWindowLabels::shared(ui), Arc::new);
        let numbers = LocaleNumbers::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let mut changed = false;

        let shown = Panel::bottom(id.with("panel"))
            .exact_size(style.zoom_bar_height)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE.fill(style.zoom_bar_fill))
            .show(ui, |ui| {
                let room = ui.max_rect().shrink2(style.zoom_bar_pad);
                let strip = UiBuilder::new().max_rect(room).layout(direction::row(ui)).id_salt("strip");
                ui.scope_builder(strip, |ui| {
                    // The ladder's two ends: a button is disabled where a step would not move.
                    let can = |dir: i32| {
                        let mut probe = *view;
                        probe.step(dir)
                    };
                    let (can_out, can_in) = (can(-1), can(1));
                    for (icon, label, dir) in [(Icon::Minus, &labels.zoom_out, -1), (Icon::Plus, &labels.zoom_in, 1)] {
                        let button = IconButton::new(icon, label.as_str()).size(ControlSize::Sm).enabled(if dir > 0 { can_in } else { can_out });
                        if ui.add(button).clicked() {
                            changed |= view.apply(if dir > 0 { FullWindowZoom::In } else { FullWindowZoom::Out });
                        }
                    }

                    // The select's value: 0 for Fit in View, else the zoom in tenths of a percent.
                    let mut value = if view.is_fit() { FIT } else { tenths(view.zoom_percent()) };
                    let mut zooms = MENU.to_vec();
                    if value != FIT && !zooms.contains(&value) {
                        zooms.push(value);
                        zooms.sort_unstable();
                    }
                    let unit = Unit::new("{n}%");
                    let mut select = Select::new(id.with("zoom"), labels.zoom.as_str(), &mut value).size(ControlSize::Sm).fit(true);
                    select = select.option(FIT, labels.fit_in_view.as_str());
                    for zoom in zooms {
                        let text = format!("{}{}{}", unit.prefix(), numbers.format(zoom as f64 / 10.0, 0..=1), unit.suffix());
                        select = select.option(zoom, text);
                    }
                    let edit = select.show(ui);
                    if let Some(picked) = edit.event.as_ref().and_then(|event| event.value()).copied() {
                        if picked == FIT {
                            changed |= view.apply(FullWindowZoom::Fit);
                        } else if view.is_fit() || picked != tenths(view.zoom_percent()) {
                            view.set_zoom(picked as f64 / 1000.0);
                            changed = true;
                        }
                    }

                    match progress {
                        Some(Some(fraction)) => {
                            ui.add(Progress::new(fraction).width(style.progress_width).label(labels.updating.as_str()));
                        }
                        Some(None) => {
                            ui.add(Progress::indeterminate().width(style.progress_width).label(labels.updating.as_str()));
                        }
                        None => {
                            if let Some(note) = note.as_deref().filter(|n| !n.is_empty()) {
                                let text = Text::new(note).role(TypeRole::Caption).content(Content::Faint).truncate();
                                ui.add(text.max_width(ui.available_width()));
                            }
                        }
                    }
                });
            });
        FullWindowZoomBarResponse { response: shown.response, changed }
    }
}

/// The select's value for Fit in View.
const FIT: i64 = 0;

/// The menu's zooms in tenths of a percent: 12.5 %, 25 %, 50 %, 100 %, 200 %, 400 %.
const MENU: [i64; 6] = [125, 250, 500, 1000, 2000, 4000];

/// `percent` in tenths, the select's value for a zoom.
fn tenths(percent: f64) -> i64 {
    (percent * 10.0).round() as i64
}
