//! `FullWindowZoomBar`: the zoom strip under a full-window canvas.

use super::super::words::DialogWords;
use super::view::{FullWindowView, FullWindowZoomKeys};
use crate::buttons::IconButton;
use crate::contract::{Bind, Entry, NoValue, Outcome, Props as _, Show as _, ShowContainer as _, clicked};
use crate::display::{Elide, Progress, Shade, Text};
use crate::layout_components::Row;
use crate::pickers::Select;
use crate::runtime::env::{Env, Label, UnitKind};
use crate::style::TypeRole;
use craft_behavior::IconId;
use craft_theme::ControlSize;
use egui::{Response, Ui};

/// The select's value for Fit in View.
const FIT: i64 = 0;

/// The menu's zooms in tenths of a percent: 12.5 %, 25 %, 50 %, 100 %, 200 %, 400 %.
const MENU: [i64; 6] = [125, 250, 500, 1000, 2000, 4000];

/// A zoom in tenths of a percent, the select's value for it.
fn tenths(percent: f64) -> i64 {
    (percent * 10.0).round() as i64
}

/// What the zoom bar reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct FullWindowZoomBarOutput {
    /// The strip's response.
    pub response: Response,
    /// The view after a button or a pick of this pass; `None` when the zoom did not change.
    pub view: Option<FullWindowView>,
}

impl Outcome for FullWindowZoomBarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The strip under a full-window canvas: Zoom Out, Zoom In, a zoom select, then a progress bar while the preview
/// renders, or a note.
///
/// - **Buttons.** Zoom Out and Zoom In step the view's ladder (`FullWindowZoomKeys::{Out, In}`); each is disabled
///   where a step would not move.
/// - **Select.** "Zoom": Fit in View, 12.5 %, 25 %, 50 %, 100 %, 200 % and 400 %; a zoom off the menu (stepped to
///   300 %) is listed as itself in its place among them. A fitted view shows Fit in View.
/// - **Progress or note.** While [`progress`](Self::progress) is set, a progress bar (indeterminate for
///   `Some(None)`); otherwise the [`note`](Self::note) in the faint shade, caption type, cut with an ellipsis.
/// - The bar keeps no view: it shows the one given and reports the next, which the caller adopts.
#[derive(crate::contract::Pattern)]
#[must_use = "show the bar with `show`"]
pub struct FullWindowZoomBar {
    view: FullWindowView,
    progress: Option<Option<f32>>,
    note: Option<Label>,
}

impl FullWindowZoomBar {
    /// A zoom bar for `view`, the same view the canvas above it shows.
    pub fn new(view: &FullWindowView) -> Self {
        FullWindowZoomBar { view: *view, progress: None, note: None }
    }

    /// The preview's progress: `None` shows the note, `Some(None)` an indeterminate bar, `Some(Some(f))` a bar at `f`
    /// (0 to 1). `FullWindowStatus::progress` gives it.
    pub fn progress(mut self, progress: Option<Option<f32>>) -> Self {
        self.progress = progress;
        self
    }

    /// Text after the select while no progress shows: "Preview 120 ms".
    pub fn note(mut self, note: impl Into<Label>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Shows the bar.
    pub fn show(self, ui: &mut Ui) -> FullWindowZoomBarOutput {
        let env = Env::of(ui);
        let (words, units) = (env.words::<DialogWords>(), env.units());
        let view = self.view;
        let mut next: Option<FullWindowView> = None;
        let row = Row::new().show(ui, |ui, _| {
            for (icon, label, command, dir) in
                [("minus", &words.full_window_zoom_out, FullWindowZoomKeys::Out, -1), ("plus", &words.full_window_zoom_in, FullWindowZoomKeys::In, 1)]
            {
                let button = IconButton::new(IconId::new(icon)).label(label.clone()).size(ControlSize::Sm).enabled(view.can_step(dir)).show(ui);
                if clicked(&button) {
                    let mut stepped = view;
                    if stepped.apply(command) {
                        next = Some(stepped);
                    }
                }
            }
            let current = if view.is_fit() { FIT } else { tenths(view.zoom_percent()) };
            let mut zooms = MENU.to_vec();
            if current != FIT && !zooms.contains(&current) {
                zooms.push(current);
                zooms.sort_unstable();
            }
            let mut entries = vec![Entry::new(FIT, words.full_window_fit_in_view.clone())];
            entries.extend(zooms.into_iter().map(|zoom| Entry::new(zoom, Label::from(units.text(UnitKind::Percent, zoom as f64 / 10.0, 0..=1)))));
            let picked = Select::new(words.full_window_zoom.clone(), Bind::given(current), &entries).size(ControlSize::Sm).show(ui);
            if let Some(&tenths) = picked.value() {
                let mut chosen = view;
                let changed = if tenths == FIT { chosen.apply(FullWindowZoomKeys::Fit) } else { chosen.set_device_zoom(tenths as f64 / 1000.0) };
                if changed {
                    next = Some(chosen);
                }
            }
            match (self.progress, &self.note) {
                (Some(Some(fraction)), _) => {
                    Progress::new(fraction).label(words.full_window_updating.clone()).show(ui);
                }
                (Some(None), _) => {
                    Progress::indeterminate().label(words.full_window_updating.clone()).show(ui);
                }
                (None, Some(note)) if !note.as_str().is_empty() => {
                    Text::new(note.clone()).role(TypeRole::Caption).shade(Shade::Faint).elide(Elide::End).show(ui);
                }
                _ => {}
            }
        });
        FullWindowZoomBarOutput { response: row.output, view: next }
    }
}
