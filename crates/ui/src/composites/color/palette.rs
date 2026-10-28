//! [`SwatchPalette`]: a grid of colour wells that is a radio group.

use craft_theme::{ControlSize, Srgba};
use egui::accesskit::Role;
use egui::{Align, AsIdSalt, IdSalt, Layout, Rect, Sense, Ui, UiBuilder, Vec2};

use crate::behavior::RadioSelection;
use crate::controls::{Swatch, TooltipExt as _};
use crate::direction;
use crate::edit::{EditPolicy, EditResponse, Gesture};

/// A grid of [`Swatch`]es from the host's list of named colours, wrapping to the width, that
/// chooses one colour as a radio group does:
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::SwatchPalette;
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.9, 0.3, 0.3);
/// let swatches = [("Red", Srgba::rgb(0.9, 0.3, 0.3)), ("Blue", Srgba::rgb(0.2, 0.4, 0.9))];
/// let picked = SwatchPalette::new("palette", "Swatches", color, swatches).show(ui);
/// if let Some(c) = picked.event.as_ref().and_then(|e| e.value()) {
///     color = *c;
/// }
/// # });
/// ```
///
/// - The swatch equal to `color` (compared in 8 bits, the first when several are) is selected.
///   Wells keep [`Swatch`]'s own room round them (6 pt between two), so the selection ring and
///   the focus ring show; the name is the tooltip.
/// - A click, Enter or Space on a swatch, or a screen reader's click, sets its colour as one
///   [`Set`](crate::edit::EditEvent::Set). Keys and focus are [`RadioSelection`]'s: every swatch
///   is a Tab stop and the arrows move the focus between the swatches (Left and Right along a
///   row, Up and Down between rows) without selecting.
/// - AccessKit: a `RadioGroup` named `label` with the number of swatches; each swatch a
///   `RadioButton` named by its name, with the hex colour as its value.
#[must_use = "show the palette with `show`"]
pub struct SwatchPalette {
    id_salt: IdSalt,
    label: String,
    color: Srgba,
    swatches: Vec<(String, Srgba)>,
    size: ControlSize,
    enabled: bool,
}

impl SwatchPalette {
    /// A palette named `label` (its accessible name) of `swatches`, each a name and a colour,
    /// choosing the host's `color`.
    pub fn new<S: Into<String>>(
        id_salt: impl AsIdSalt,
        label: impl Into<String>,
        color: Srgba,
        swatches: impl IntoIterator<Item = (S, Srgba)>,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            color,
            swatches: swatches.into_iter().map(|(name, color)| (name.into(), color)).collect(),
            size: ControlSize::Md,
            enabled: true,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether the palette takes input; a disabled one draws its swatches faded.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the palette and returns this frame's edit: a [`Set`](crate::edit::EditEvent::Set) of
    /// the colour clicked. The response covers the swatches and is
    /// [changed](egui::Response::changed) with it.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Srgba> {
        let id = ui.make_persistent_id(self.id_salt);
        let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) }.with_main_wrap(true);
        let builder = UiBuilder::new().scope_id(id).layout(layout).accessibility_role(Role::RadioGroup).accessibility_label(self.label.clone());
        let enabled = self.enabled && ui.is_enabled();
        let mut gesture = Gesture::load(ui, id, &self.color, EditPolicy::default()).enabled(enabled);
        let inner = ui.scope_builder(builder, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let ids: Vec<_> = (0..self.swatches.len()).map(|i| (Swatch::id_in(ui, (id, i)), enabled)).collect();
            let selected = self.swatches.iter().position(|(_, c)| c.to_rgba() == self.color.to_rgba());
            let mut radios = RadioSelection::new(ids, selected);
            radios.report_size(ui, id);
            radios.click_from_input(ui);
            let chosen = radios.selected().filter(|_| radios.changed());
            if let Some(i) = chosen {
                gesture.set(self.swatches[i].1);
            }
            let shown = chosen.or(selected);
            let mut covered: Option<egui::Response> = None;
            for (i, (name, color)) in self.swatches.iter().enumerate() {
                let swatch =
                    Swatch::new((id, i), name.as_str(), color.to_rgba()).size(self.size).radio(shown == Some(i)).position_in_set(i).enabled(enabled);
                let response = ui.add(swatch).tooltip(name.as_str());
                covered = Some(match covered {
                    Some(all) => all.union(response),
                    None => response,
                });
            }
            (covered, chosen.is_some())
        });
        let (covered, changed) = inner.inner;
        let mut response =
            covered.unwrap_or_else(|| ui.interact(Rect::from_min_size(inner.response.rect.min, Vec2::ZERO), id.with("swatches"), Sense::hover()));
        if changed {
            response.mark_changed();
        }
        EditResponse::new(response, gesture.finish())
    }
}
