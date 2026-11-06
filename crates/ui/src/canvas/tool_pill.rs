//! [`ToolPill`]: a floating pill of tools over the canvas (FigCraft's bottom toolbar), for apps
//! whose tools don't sit in a rail.

use std::hash::Hash;

use egui::accesskit::Role;
use egui::{Area, AsIdSalt, EventFilter, Id, Key, Modifiers, Order, Rect, Sense, Shape, Ui, Vec2, pos2, vec2};

use crate::direction;
use crate::focus::{Region, RegionFocus};
use crate::localized::Labels as _;
use crate::patterns::rail::tool::{SlotMode, slot_button};
use crate::patterns::{LastUsed, ToolSlot, visible_slots};
use crate::primitives::{Surface, SurfaceKind};
use crate::theme::{Tokens, TokensOf};

/// Where the pill sits on the canvas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PillPlace {
    #[default]
    #[allow(missing_docs)]
    BottomCenter,
    #[allow(missing_docs)]
    TopCenter,
}

/// What [`ToolPill::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolPillResponse<T> {
    /// The tool to make current: a clicked button, or a tool chosen from a group's flyout.
    pub select: Option<T>,
    /// The pill's screen rectangle.
    pub rect: Rect,
}

/// The pill: the same [`ToolSlot`]s a [`crate::patterns::ToolRail`] takes, laid out in a row in a
/// rounded popup surface over the canvas, each slot one button with a flyout for its group (the
/// rail's [`crate::patterns::FlyoutButton`]).
///
/// ```no_run
/// use craft_ui::canvas::ToolPill;
/// # fn demo(ui: &mut egui::Ui, canvas: egui::Rect, slots: &[craft_ui::patterns::ToolSlot<&'static str>], current: &'static str) {
/// if let Some(tool) = ToolPill::new("tools").show(ui, canvas, slots, &current).select { /* switch tool */ }
/// # }
/// ```
///
/// - **Placement:** centred at the canvas's bottom (or top) edge, inset by the canvas overlay inset;
///   it never covers the scroll bars. The canvas keeps the pointer everywhere else.
/// - **Keyboard:** a `Toolbar` region for F6 ([`crate::focus::RegionFocus`]); arrows move between
///   buttons, Enter or Space selects, Down or Alt+Down opens a group's flyout. Tool letters are
///   the app's keymap and need nothing here.
/// - **AccessKit:** a `Toolbar` named by [`ToolPillLabels::name`] ("Tools"); the current tool's
///   button is pressed.
#[must_use = "show the pill with `show`"]
pub struct ToolPill {
    id: Id,
    place: PillPlace,
    labels: Option<ToolPillLabels>,
}

impl ToolPill {
    /// A pill keyed `id_salt`, at the bottom centre.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id: Id::unique("craft-ui-tool-pill").with(id_salt), place: PillPlace::default(), labels: None }
    }

    /// Sets where the pill sits on the canvas.
    pub fn place(mut self, place: PillPlace) -> Self {
        self.place = place;
        self
    }

    /// Translated labels; default the installed words.
    pub fn labels(mut self, labels: ToolPillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the pill over `canvas` with `slots`, `current` shown pressed.
    // figcraft: crates/ui-egui/src/toolbar.rs:489 `pill` (shadow, panel, border at the canvas's
    // bottom centre; registered as the toolbar navigation region).
    pub fn show<T: Clone + PartialEq + Hash + Send + Sync + 'static>(
        self,
        ui: &mut Ui,
        canvas: Rect,
        slots: &[ToolSlot<T>],
        current: &T,
    ) -> ToolPillResponse<T> {
        let Self { id, place, labels } = self;
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id);
        let labels = labels.unwrap_or_else(|| ToolPillLabels::of(ui));
        let tokens = Tokens::of(ui);
        let (side, gap, pad) = (tokens.size.rail.button, tokens.space.gap.inline / 2.0, tokens.space.pad.panel);
        let inset = crate::style::scroll::ScrollStyle::new(&tokens).canvas_width + tokens.space.pad.panel;
        let visible = visible_slots(slots, &[]);
        let count = visible.len() as f32;
        let natural = count * side + (count - 1.0).max(0.0) * gap + 2.0 * pad;
        let size = vec2(natural.min((canvas.width() - 2.0 * inset).max(0.0)), side + 2.0 * pad);
        let top = match place {
            PillPlace::BottomCenter => canvas.bottom() - inset - size.y,
            PillPlace::TopCenter => canvas.top() + inset,
        };
        let rect = Rect::from_min_size(pos2(canvas.center().x - size.x / 2.0, top), size);
        if visible.is_empty() {
            return ToolPillResponse { select: None, rect };
        }

        let surface = Surface::style(&tokens, SurfaceKind::Popup);
        let last_id = id.with("last-used");
        let mut last = ctx.data(|d| d.get_temp::<LastUsed<T>>(last_id)).unwrap_or_default();
        let rtl = direction::is_rtl(ui);
        let mut select = None;
        let mut cells: Vec<(Id, bool)> = Vec::new();
        let mut flyout_open = false;
        // The pill takes the press and the drag over it, so the canvas under it gets neither.
        Area::new(id.with("area"))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .sense(Sense::click_and_drag())
            .role(Role::Toolbar)
            .accessible_name(labels.name.as_str())
            .show(&ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                ui.painter().add(Shape::Vec(surface.shapes(rect, ui.pixels_per_point())));
                ui.set_clip_rect(rect);
                for (k, (index, tools)) in visible.iter().enumerate() {
                    let offset = pad + k as f32 * (side + gap);
                    let x = if rtl { rect.right() - offset - side } else { rect.left() + offset };
                    let cell = egui::Rect::from_min_size(pos2(x, rect.top() + pad), Vec2::splat(side));
                    let mode = SlotMode { editing: false, down_opens: true };
                    if let Some(button) = slot_button(ui, &slots[*index], tools, current, &mut last, cell, mode) {
                        if select.is_none() {
                            select = button.select;
                        }
                        flyout_open |= button.open;
                        cells.push((button.response.id, button.response.enabled()));
                    }
                }
            });
        ctx.data_mut(|d| d.insert_temp(last_id, last));
        if !flyout_open {
            arrows(&ctx, rtl, &cells);
        }
        RegionFocus::new(&ctx).region(&ctx, Region::new("toolPill", labels.name.as_str()), rect, cells.first().map(|(id, _)| *id));
        ToolPillResponse { select, rect }
    }
}

/// Left and Right move the focus between the pill's buttons (swapped in a mirrored interface),
/// skipping disabled ones; the keys stay out of egui's geometric focus movement.
fn arrows(ctx: &egui::Context, rtl: bool, cells: &[(Id, bool)]) {
    let Some(at) = cells.iter().position(|(id, _)| ctx.memory(|m| m.has_focus(*id))) else {
        return;
    };
    let filter = EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() };
    ctx.memory_mut(|m| m.set_focus_lock_filter(cells[at].0, filter));
    let (previous, next) = if rtl { (Key::ArrowRight, Key::ArrowLeft) } else { (Key::ArrowLeft, Key::ArrowRight) };
    let forward = ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, next) {
            Some(true)
        } else if input.consume_key(Modifiers::NONE, previous) {
            Some(false)
        } else {
            None
        }
    });
    let Some(forward) = forward else { return };
    let candidates: Vec<usize> = if forward { (at + 1..cells.len()).collect() } else { (0..at).rev().collect() };
    if let Some(to) = candidates.into_iter().find(|to| cells[*to].1) {
        ctx.memory_mut(|m| m.request_focus_with_filter(cells[to].0, filter));
    }
}

crate::words! {
    /// The tool pill's labels, English by default.
    #[non_exhaustive]
    pub struct ToolPillLabels {
        name: "toolPill.name" = "Tools";
    }
}
