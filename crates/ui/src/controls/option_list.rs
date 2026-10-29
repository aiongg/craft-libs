//! What [`Select`](super::Select) and [`ComboBox`](super::ComboBox) share about their popup list
//! of options: the rows and how they are drawn, the press that opens the list and may choose on
//! its release, the focus the list keeps for its owner, the chevron at the box's end, and the
//! state a list keeps between passes.

use std::sync::Arc;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, Galley, Id, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, ViewportId, pos2, vec2};

use super::{ListboxStyle, TextFieldStyle, TooltipExt as _, icon_size};
use crate::behavior::Listbox;
use crate::direction;
use crate::icons::{self, Icon};
use crate::primitives::Interactive;
use crate::theme::{Tokens, TokensOf, role_format};

/// The id of option `option`'s row in the list of the control `owner`.
pub(super) fn option_id(owner: Id, option: usize) -> Id {
    owner.with(("option", option))
}

/// The option rows of one pass of a list.
pub(super) struct OptionRows<'r> {
    style: &'r ListboxStyle,
    size: ControlSize,
    owner: Id,
    /// Every row keeps room for an icon.
    icons: bool,
    /// The control is enabled.
    enabled: bool,
    /// The press that opened the list is still down, or ends in this pass.
    pressing: bool,
    hover: Hover,
    pointer_moved: bool,
    released: bool,
}

/// What the pointer over a row does, by what the control's value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Hover {
    /// It makes the row active, so Enter chooses it: a list whose options are the only values
    /// ([`Select`](super::Select)), as macOS pop-up menus choose the highlighted row.
    Activates,
    /// It draws the hover fill and leaves the keyboard's active option as it was: a list of
    /// suggestions for typed text that is the value ([`ComboBox`](super::ComboBox)), which a row
    /// the pointer passes over must not replace on Enter.
    Highlights,
}

/// One option's row.
pub(super) struct OptionRow<'t> {
    pub option: usize,
    pub text: &'t str,
    pub icon: Option<Icon>,
    pub chosen: bool,
    pub enabled: bool,
    /// A hover tooltip on the row; it is also the row's accessible description.
    pub tooltip: Option<&'t str>,
}

/// What a row did in its pass.
pub(super) struct RowOutcome {
    pub rect: Rect,
    /// A click, or the release of the press that opened the list, chose it.
    pub picked: bool,
    /// It was the active option when it was drawn.
    pub active: bool,
    /// The pointer is over the row and the row can be chosen.
    pub pointed: bool,
}

impl<'r> OptionRows<'r> {
    pub(super) fn new(ui: &Ui, style: &'r ListboxStyle, size: ControlSize, owner: Id, icons: bool, enabled: bool, pressing: bool) -> Self {
        let (pointer_moved, released) = ui.input(|i| (i.pointer.delta() != Vec2::ZERO, i.pointer.primary_released()));
        Self { style, size, owner, icons, enabled, pressing, hover: Hover::Activates, pointer_moved, released }
    }

    /// What the pointer over a row does; [`Hover::Activates`] by default.
    pub(super) fn hover(mut self, hover: Hover) -> Self {
        self.hover = hover;
        self
    }

    /// Shows `row`: a WAI-ARIA option, selected while chosen. The pointer over it does as
    /// [`hover`](Self::hover) says, when it moves onto it or drags over it with the opening press;
    /// a click or that press's release over it picks it.
    pub(super) fn show(&self, ui: &mut Ui, listbox: &mut Listbox, row: &OptionRow<'_>) -> RowOutcome {
        let enabled = self.enabled && row.enabled;
        let size = vec2(ui.available_width().max(self.width(ui, row.text)), self.style.height);
        let response = Interactive::with_id(option_id(self.owner, row.option), Role::ListBoxOption, row.text)
            .sense(Sense::CLICK)
            .selected(row.chosen)
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, size)
            .finish(ui);
        let pointed = enabled && response.contains_pointer() && (self.pointer_moved || self.pressing);
        if pointed && self.hover == Hover::Activates {
            listbox.set_active(Some(row.option));
        }
        let picked = enabled && (response.clicked() || (self.pressing && self.released && response.contains_pointer()));
        let active = listbox.active() == Some(row.option);
        let lit = active || (self.hover == Hover::Highlights && enabled && response.contains_pointer());
        let look =
            RowLook { text: row.text, icon: row.icon, icons: self.icons, chosen: row.chosen, lit, focus: active && listbox.keyboard(), enabled };
        paint_row(ui, self.style, self.size, response.rect, &look);
        let pointed = enabled && response.contains_pointer();
        let rect = response.rect;
        if let Some(text) = row.tooltip {
            response.tooltip(text);
        }
        RowOutcome { rect, picked, active, pointed }
    }

    /// The width a row needs: its text, the check mark and the icon with their gaps, and the
    /// padding.
    fn width(&self, ui: &Ui, text: &str) -> f32 {
        let s = self.style;
        let galley = text_galley(ui, s.label, text, Color32::PLACEHOLDER, f32::INFINITY);
        2.0 * s.pad_x + text_offset(s, self.icons) + galley.size().x
    }
}

/// How many rows of `row_height` the list in `ui` shows at once: a page for Page Up and Down.
pub(super) fn rows_in_view(ui: &Ui, row_height: f32) -> usize {
    (ui.clip_rect().height() / row_height).floor().max(1.0) as usize
}

/// How one row is drawn.
struct RowLook<'r> {
    text: &'r str,
    icon: Option<Icon>,
    icons: bool,
    chosen: bool,
    /// Active, or under the pointer where the pointer only highlights: the active fill
    /// (`accent.solid`) and text. The chosen option has them too.
    lit: bool,
    /// The active row is the keyboard's focus cue.
    focus: bool,
    enabled: bool,
}

/// From a row's reading start to its text, inside the padding: the check mark and, when the list
/// has icons, an icon, each followed by the gap.
fn text_offset(s: &ListboxStyle, icons: bool) -> f32 {
    (s.icon + s.gap) * if icons { 2.0 } else { 1.0 }
}

fn paint_row(ui: &Ui, s: &ListboxStyle, size: ControlSize, rect: Rect, look: &RowLook<'_>) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let ppp = ui.pixels_per_point();
    let filled = (look.lit || look.chosen) && look.enabled;
    if filled {
        ui.painter().rect_filled(rect.round_to_pixels(ppp), s.radius, s.active_fill);
    }
    if look.focus && look.enabled {
        ui.painter().add(RectShape::stroke(rect.round_to_pixels(ppp), s.radius, s.focus, StrokeKind::Inside));
    }
    let (text, mark) = match () {
        () if !look.enabled => (s.text_disabled, s.text_disabled),
        () if filled => (s.active_text, s.active_text),
        () => (s.text, s.mark),
    };
    let forward = direction::forward(ui);
    let start = direction::start_x(ui, rect) + forward * s.pad_x;
    let center = |from_start: f32| pos2(start + forward * (from_start + s.icon / 2.0), rect.center().y);
    let square = |from_start: f32| Rect::from_center_size(center(from_start), vec2(s.icon, s.icon)).round_to_pixels(ppp);
    if look.chosen {
        icons::paint(ui, square(0.0), Icon::Check, icon_size(size), mark);
    }
    if let Some(icon) = look.icon.filter(|_| look.icons) {
        icons::paint(ui, square(s.icon + s.gap), icon, icon_size(size), text);
    }
    let offset = text_offset(s, look.icons);
    let galley = text_galley(ui, s.label, look.text, text, f32::INFINITY);
    let x = start + forward * offset;
    let x = if direction::is_rtl(ui) { x - galley.size().x } else { x };
    let pos = pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp);
    ui.painter().galley(pos, galley, text);
}

/// `text` in `role` and `color` on one line, cut with an ellipsis past `width`.
pub(super) fn text_galley(ui: &Ui, role: TypeRole, text: &str, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), role_format(&Tokens::of(ui), role, color));
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    ui.painter().layout_job(job)
}

/// The square of the chevron at the reading end of a field-like box over `rect`.
pub(super) fn chevron(ui: &Ui, f: &TextFieldStyle, rect: Rect) -> Rect {
    let x = direction::end_x(ui, rect) - direction::forward(ui) * (f.pad_x + f.icon / 2.0);
    Rect::from_center_size(pos2(x, rect.center().y), vec2(f.icon, f.icon)).round_to_pixels(ui.pixels_per_point())
}

/// Half the width of a drawn chevron, and its half rise, in points (PhotoCraft's `chevron_icon`,
/// `widgets/panel.rs:149-155`).
const CHEVRON_HALF_WIDTH: f32 = 3.2;
const CHEVRON_STROKE: f32 = 1.3;
/// The chevron is the text colour at this strength.
const CHEVRON_STRENGTH: f32 = 0.8;

/// Paints the chevron of a field-like box over `rect`: two strokes in `color` at 0.8 strength.
pub(super) fn paint_chevron(ui: &Ui, f: &TextFieldStyle, rect: Rect, color: Color32) {
    let c = chevron(ui, f, rect).center();
    let s = CHEVRON_HALF_WIDTH;
    let stroke = Stroke::new(CHEVRON_STROKE, color.gamma_multiply(CHEVRON_STRENGTH));
    ui.painter().add(Shape::line(vec![c + vec2(-s, -s * 0.5), c + vec2(0.0, s * 0.5), c + vec2(s, -s * 0.5)], stroke));
}

/// Whether the primary button went down on `response`'s widget in this pass: a press, or a click
/// whose press and release came in one pass. The list opens on the press, so the same press can
/// drag onto an option and choose it on release.
pub(crate) fn pressed_on(ui: &Ui, response: &Response) -> bool {
    ui.input(|i| i.pointer.primary_pressed()) && (response.is_pointer_button_down_on() || response.clicked())
}

/// Whether the primary button is up by the end of this pass: the press that opened a list ended.
pub(crate) fn released(ui: &Ui) -> bool {
    ui.input(|i| i.pointer.primary_released() || !i.pointer.primary_down())
}

/// Whether the press that opened the list `popup` ended in this pass away from the list and from
/// `field`, after a drag: the list closes without a choice. A release over either, or a click,
/// leaves it open.
pub(super) fn released_away(ui: &Ui, popup: Id, field: Rect) -> bool {
    let pos = ui.input(|i| i.pointer.interact_pos());
    let in_list = ui.memory(|m| m.area_rect(popup)).zip(pos).is_some_and(|(r, p)| r.contains(p));
    let on_field = pos.is_some_and(|p| field.contains(p));
    !in_list && !on_field && !ui.input(|i| i.pointer.any_click())
}

/// Gives the focus back to `owner` (the widget that had it when the pass began) when a press
/// inside the open list `popup` took it away: egui drops the focus on a click elsewhere, and a
/// press on a disabled option, a group header or the list's edge chooses nothing.
pub(super) fn keep_focus(ui: &Ui, popup: Id, owner: Option<Id>) {
    let pressed_in = ui.input(|i| (i.pointer.any_pressed() || i.pointer.any_click()).then(|| i.pointer.interact_pos()).flatten());
    let in_list = ui.memory(|m| m.area_rect(popup)).zip(pressed_in).is_some_and(|(r, p)| r.contains(p));
    if let Some(owner) = owner
        && in_list
        && ui.memory(|m| m.focused()).is_none()
    {
        ui.memory_mut(|m| m.request_focus(owner));
    }
}

/// A list's state as its control stored it, with the viewport and pass that stored it.
#[derive(Clone)]
struct Kept<S> {
    viewport: ViewportId,
    pass: u64,
    state: S,
}

/// The state stored under `key` by a control drawn in the last pass of this viewport (or earlier
/// in this one, as [`Gesture::load`](crate::edit::Gesture::load) counts). A control missing from a
/// pass has been removed: its list starts closed when it is drawn again.
pub(super) fn load_kept<S: Clone + Send + Sync + 'static>(ui: &Ui, key: Id) -> Option<S> {
    let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
    ui.data(|d| d.get_temp::<Kept<S>>(key)).filter(|k| k.viewport == viewport && k.pass + 1 >= pass).map(|k| k.state)
}

/// Stores `state` under `key` for [`load_kept`].
pub(super) fn store_kept<S: Clone + Send + Sync + 'static>(ui: &Ui, key: Id, state: S) {
    let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
    ui.data_mut(|d| d.insert_temp(key, Kept { viewport, pass, state }));
}

#[cfg(test)]
mod tests {
    use egui::{Context, RawInput, Shape};

    use super::*;
    use crate::controls::Select;
    use crate::theme::{Base, install};

    fn rect_fills(shapes: &[egui::epaint::ClippedShape]) -> Vec<Color32> {
        shapes.iter().filter_map(|c| if let Shape::Rect(r) = &c.shape { Some(r.fill) } else { None }).collect()
    }

    /// The chosen option and the lit row have the active fill, a plain row has none; a closed box's
    /// chevron is two strokes of the text colour at 0.8 strength.
    #[test]
    fn the_chosen_and_the_lit_rows_have_the_active_fill_and_the_chevron_is_drawn_as_a_line() {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut style = None;
        let mut out = ctx.run_ui(RawInput::default(), |ui| {
            let s = Select::style(&crate::theme::Tokens::of(ui), ControlSize::Md);
            let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(120.0, s.list.height));
            let row = |chosen, lit| RowLook { text: "Normal", icon: None, icons: false, chosen, lit, focus: false, enabled: true };
            paint_row(ui, &s.list, ControlSize::Md, rect, &row(false, false));
            paint_row(ui, &s.list, ControlSize::Md, rect.translate(vec2(0.0, 40.0)), &row(true, false));
            paint_row(ui, &s.list, ControlSize::Md, rect.translate(vec2(0.0, 80.0)), &row(false, true));
            paint_chevron(ui, &s.field, rect.translate(vec2(0.0, 120.0)), s.field.text);
            style = Some(s);
        });
        out.textures_delta.clear();
        let s = style.unwrap_or_else(|| panic!("no style"));
        assert_eq!(rect_fills(&out.shapes), [s.list.active_fill, s.list.active_fill], "the chosen row and the lit row, not the plain one");
        assert_eq!(s.list.radius, egui::CornerRadius::ZERO);
        let chevron = out.shapes.iter().find_map(|c| if let Shape::Path(p) = &c.shape { Some(p.clone()) } else { None });
        let chevron = chevron.unwrap_or_else(|| panic!("no chevron path"));
        assert_eq!(chevron.points.len(), 3);
        assert_eq!(chevron.stroke, egui::epaint::PathStroke::new(1.3, s.field.text.gamma_multiply(0.8)));
    }
}
