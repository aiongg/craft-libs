//! [`MaskViewControls`]: the View Mode block of Select and Mask's Properties panel.

use craft_theme::TypeRole;
use craft_ui::controls::{Checkbox, Kbd, Select, Slider};
use craft_ui::direction;
use craft_ui::localized::{Labels, Units};
use craft_ui::primitives::{Content, Text};
use egui::{AsIdSalt, IdSalt, KeyboardShortcut, Modifiers, Response, Ui};

use super::{MaskViewChange, MaskViewLabels, MaskViewMode, MaskViewState};

/// What [`MaskViewControls::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MaskViewResponse {
    /// The block's outer response.
    pub response: Response,
    /// The change the user made this pass, already applied to the state.
    pub changed: Option<MaskViewChange>,
}

/// The View Mode controls: a "View" row with a `Select` named "View Mode" and the seven views (each
/// option shows its key letter as its shortcut, `Kbd`), the current view's Opacity slider in percent (0..=100, only
/// for the four views that have one), the Show Edge and Show Original checkboxes, and the hint line
/// in the caption role. Remember Settings and the Refine Edge Brush controls are the workspace's.
///
/// Keyboard: each control is a Tab stop; the drop-down opens with Space or Enter. AccessKit: the
/// drop-down is a combo box with the views as options, the slider a number with "%", the
/// checkboxes are checkboxes. RTL mirrors the row.
pub struct MaskViewControls<'a> {
    id_salt: IdSalt,
    state: &'a mut MaskViewState,
    labels: Option<MaskViewLabels>,
}

impl<'a> MaskViewControls<'a> {
    /// The controls of `state`, under `id_salt`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut MaskViewState) -> Self {
        Self { id_salt: IdSalt::new(id_salt), state, labels: None }
    }

    /// Translated words; without them [`MaskViewLabels::of`] the context.
    pub fn labels(mut self, labels: MaskViewLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the controls in `ui`'s layout.
    pub fn show(self, ui: &mut Ui) -> MaskViewResponse {
        let Self { id_salt, state, labels } = self;
        let labels = labels.unwrap_or_else(|| MaskViewLabels::of(ui));
        let units = Units::of(ui);
        let mut changed = None;
        let outer = ui.scope(|ui| {
            direction::horizontal(ui, |ui| {
                ui.add(Text::new(labels.view.as_str()).role(TypeRole::Label).content(Content::Muted));
                let mut picked = state.mode;
                let mut select = Select::new((&id_salt, "mode"), labels.view_mode.as_str(), &mut picked);
                for mode in MaskViewMode::ALL {
                    let key = KeyboardShortcut::new(Modifiers::NONE, mode.key());
                    select = select.option(mode, mode.label(&labels)).option_shortcut(Kbd::new(&key));
                }
                let _ = select.show(ui);
                if picked != state.mode {
                    let change = MaskViewChange::Mode(picked);
                    state.apply(change);
                    changed = Some(change);
                }
            });
            let mode = state.mode;
            if let Some(percent) = state.opacity(mode) {
                let out = Slider::new((&id_salt, "opacity", mode.id()), labels.opacity.as_str(), percent, 0.0..=100.0)
                    .step(1.0)
                    .value_text(units.percent(f64::from(percent)))
                    .show(ui);
                if let Some(value) = out.event.as_ref().and_then(|e| e.value()) {
                    let value = (*value as f32).round();
                    if Some(value) != state.opacity(mode) {
                        let change = MaskViewChange::Opacity(mode, value);
                        state.apply(change);
                        changed = Some(change);
                    }
                }
            }
            let mut edge = state.show_edge;
            let _ = Checkbox::bool(labels.show_edge.as_str(), &mut edge).id_salt((&id_salt, "edge")).show(ui);
            if edge != state.show_edge {
                let change = MaskViewChange::ShowEdge(edge);
                state.apply(change);
                changed = Some(change);
            }
            let mut original = state.show_original;
            let _ = Checkbox::bool(labels.show_original.as_str(), &mut original).id_salt((&id_salt, "original")).show(ui);
            if original != state.show_original {
                let change = MaskViewChange::ShowOriginal(original);
                state.apply(change);
                changed = Some(change);
            }
            ui.add(Text::new(labels.hint.as_str()).role(TypeRole::Caption).content(Content::Muted));
        });
        MaskViewResponse { response: outer.response, changed }
    }
}
