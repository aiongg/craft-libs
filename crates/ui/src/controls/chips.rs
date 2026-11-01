//! [`ChipField`]: removable chips laid out in lines, with "+N more" where a long list stops.

use std::borrow::Cow;
use std::ops::Range;

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{
    AsIdSalt, Color32, CornerRadius, EventFilter, Id, IdSalt, Key, Modifiers, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, pos2,
    vec2,
};

use super::{ControlSize, TooltipExt as _};
use crate::behavior::{ClaimedKey, GridKey, GridNav, KeyClaim};
use crate::direction;
use crate::localized::words::Counted;
use crate::localized::{Labels as _, Words};
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::Interactive;
use crate::text::{ElideAt, elided_galley};
use crate::theme::{ToEgui, Tokens, TokensOf as _, corner_radius, role_format};

/// The "+N more" button's word; S5C-F9 moves it to the kernel's `counts`.
const MORE: Counted = Counted::new("chips.more", "+{n} more", "+{n} more");

/// Side of the × arms' reach from its centre, and their stroke (`chip.remove_width` and the arms
/// are proposed tokens; S5C-F8 moves them).
const CROSS_ARM: f32 = 3.5;
const CROSS_STROKE: f32 = 1.5;
/// Between the chip's edge and its label, and after a label with no ×.
const PAD_X: f32 = 8.0;
/// Between chips and between lines.
const GAP: f32 = 6.0;

crate::words! {
    /// The chips' words, English by default.
    #[non_exhaustive]
    pub struct ChipLabels {
        /// A chip's remove button, for AccessKit and its tooltip: `{name}` the chip's full name
        /// (LightCraft's "Remove “{name}”").
        remove: "chips.remove" = "Remove “{name}”";
        /// The button for the chips that don't fit: `{n}` how many. A
        /// [`Counted`](crate::localized::words::Counted) word (one and other both "+{n} more" in
        /// English), written with [`Words::counted`](crate::localized::Words::counted); S5C-F9
        /// adds it to the kernel's `counts` and [`COUNTS`](crate::localized::words::COUNTS).
        more: "chips.more" = "+{n} more";
        clear_all: "chips.clearAll" = "Clear All";
        /// A partial chip's accessible name (the asterisk is only drawn): `{name}`.
        partial: "chips.partial" = "{name}, some of the selection";
    }
}

/// One chip: a tag, a keyword, a filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip<'a> {
    label: Cow<'a, str>,
    full_name: Option<Cow<'a, str>>,
    partial: bool,
    removable: bool,
}

impl<'a> Chip<'a> {
    /// A removable chip showing `label`.
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self { label: label.into(), full_name: None, partial: false, removable: true }
    }

    /// The name screen readers and the tooltip give when the label is shortened
    /// ("Places › Europe › Paris" for a chip showing "Paris").
    pub fn full_name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.full_name = Some(name.into());
        self
    }

    /// Only some of the selected items have it: drawn in `text.muted` with an asterisk after the
    /// label (LightCraft's keywords of a multi-selection).
    pub fn partial(mut self, partial: bool) -> Self {
        self.partial = partial;
        self
    }

    /// It has a × at its end (default on).
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }
}

/// What the chips reported this pass.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ChipFieldResponse {
    pub response: Response,
    /// The chip whose × was clicked, or which Delete or Backspace removed: the host removes it.
    pub removed: Option<usize>,
    /// The chip whose name was clicked or activated with Enter or Space.
    pub clicked: Option<usize>,
    /// The chip whose context menu was asked for (secondary click, Menu key, Shift+F10); the host
    /// opens its menu on [`ChipFieldResponse::response`] at the chip.
    pub context_menu: Option<(usize, egui::Rect)>,
    /// Clear All was pressed.
    pub cleared: bool,
}

/// A field of removable chips, after LightCraft's Keywording box and its grid's filter chips.
///
/// ```ignore
/// let chips: Vec<Chip> = keywords.iter().map(|k| Chip::new(&k.name).partial(k.partial)).collect();
/// let out = ChipField::new("keywords", "Keywords", &chips).max_lines(3).show(ui);
/// if let Some(i) = out.removed { app.run("keywords.remove", &keywords[i]) }
/// ```
///
/// - **Look.** Each chip is a pill 22 points high (`chip.height`, the `shape.pill` radius) with a
///   1-point `field.border.rest` edge, the label in the `Label` role 8 points in, and for a
///   removable chip a × target 22 points wide at its end (`chip.remove_width`; arms 3.5, 1.5
///   wide, `text.muted`, `text.normal` under the pointer). Hovering the name fills the chip with
///   `state.hover` at half strength; hovering the × lights only the × end, so the user sees which
///   part a click hits. A label too long for a line is cut short at its end with "…" to fit
///   (services' `text::elided_galley` with `ElideAt::End`), its full name as the tooltip.
/// - **Lines.** Chips flow in lines 6 points apart (`chip.gap`), each moved whole to the next
///   line when it doesn't fit ([`chip_lines`]). With [`max_lines`](Self::max_lines), the chips
///   past the last line are counted on a "+N more" button there, which opens a
///   [`Menu`](crate::menu::Menu) with a row per hidden chip (its full name; choosing it removes
///   it). With [`clear_all`](Self::clear_all), a Clear All button ends the last line (the grid's
///   filter strip).
/// - **Keys.** The field is one Tab stop over the chips (and the "+N more" and Clear All
///   buttons after them) as a grid of the laid-out lines:
///   [`GridNav::ragged`](crate::behavior::GridNav::ragged) of the line lengths with
///   [`rtl`](crate::behavior::GridNav::rtl), the stop from
///   [`GridNav::roving_move`](crate::behavior::GridNav::roving_move) (resting on the first chip).
///   ← and → move along the reading order (mirrored in right-to-left), ↑ and ↓ to the
///   neighbouring line, Home and End to the ends. Delete and Backspace remove the focused chip
///   and the focus moves to the next one (the previous one at the end), Enter and Space click
///   it, the Menu key and Shift+F10 ask for its menu. The focused chip claims Delete,
///   Backspace, Enter and Space ([`KeyClaim`](crate::behavior::KeyClaim)), so the app's Delete
///   command doesn't run while a chip has the focus. The focused chip draws the focus ring.
/// - **AccessKit.** A `List` named by its label; each chip a `ListItem` named by its full name
///   (and [`ChipLabels::partial`]), with a child `Button` [`ChipLabels::remove`].
pub struct ChipField<'a> {
    id_salt: IdSalt,
    label: String,
    chips: &'a [Chip<'a>],
    max_lines: Option<usize>,
    clear_all: bool,
    size: ControlSize,
    enabled: bool,
    labels: Option<ChipLabels>,
}

impl<'a> ChipField<'a> {
    /// A field named `label` (its accessible name) of `chips`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, chips: &'a [Chip<'a>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            chips,
            max_lines: None,
            clear_all: false,
            size: ControlSize::Md,
            enabled: true,
            labels: None,
        }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: ChipLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// At most `lines` lines; the rest go behind "+N more".
    pub fn max_lines(mut self, lines: usize) -> Self {
        self.max_lines = Some(lines);
        self
    }

    /// A Clear All button after the chips.
    pub fn clear_all(mut self, on: bool) -> Self {
        self.clear_all = on;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // lightcraft: crates/ui-egui/src/panels/keywording.rs:80 chip, :243 chip_groups; lightcraft: crates/ui-egui/src/panels/chips.rs:92 show
    // needs: services craft-ui::text::elided_galley (N11.03) for the cut label.
    pub fn show(self, ui: &mut Ui) -> ChipFieldResponse {
        let Self { id_salt, label, chips, max_lines, clear_all, size, enabled, labels } = self;
        let labels = labels.unwrap_or_else(|| ChipLabels::of(ui));
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let enabled = enabled && ui.is_enabled();
        let (height, cross) = match size {
            ControlSize::Sm => (18.0, 18.0),
            ControlSize::Md => (22.0, 22.0),
        };
        let width = ui.available_width();
        let (rtl, forward) = (direction::is_rtl(ui), direction::forward(ui));
        let text_format = |color: Color32| role_format(&tokens, TypeRole::Label, color);
        let (normal, muted) = (tokens.color.text.normal.to_egui(), tokens.color.text.muted.to_egui());

        // What each chip shows and how wide it wants to be.
        let shown_text: Vec<String> = chips.iter().map(|c| if c.partial { format!("{}*", c.label) } else { c.label.to_string() }).collect();
        let wants: Vec<f32> = chips
            .iter()
            .zip(&shown_text)
            .map(|(chip, text)| {
                let label = elided_galley(ui, text, text_format(normal), f32::INFINITY, ElideAt::End).galley.size().x;
                (PAD_X + label + if chip.removable { cross } else { PAD_X }).ceil().min(width.max(cross + PAD_X))
            })
            .collect();
        let button_width = |ui: &Ui, text: &str| {
            let label = elided_galley(ui, text, text_format(normal), f32::INFINITY, ElideAt::End).galley.size().x;
            (2.0 * PAD_X + label).ceil().min(width)
        };
        let clear_width = (clear_all && !chips.is_empty()).then(|| button_width(ui, &labels.clear_all));
        let words = Words::of_ctx(&ctx);
        let more_text = |hidden: usize| words.counted(&ctx, MORE, &labels.more, hidden as u64);

        // How many chips show: all, or as many as leave room for "+N more" in the last line.
        let extras = |hidden: usize| -> Vec<f32> {
            let mut extra = Vec::new();
            if hidden > 0 {
                extra.push(button_width(ui, &more_text(hidden)));
            }
            extra.extend(clear_width);
            extra
        };
        let layout = |shown: usize, hidden: usize| {
            let mut widths = wants[..shown].to_vec();
            widths.extend(extras(hidden));
            let lines = chip_lines(&widths, width, GAP);
            (widths, lines)
        };
        let limit = max_lines.map(|n| n.max(1));
        let mut shown = chips.len();
        let mut lines = layout(shown, 0).1;
        if let Some(limit) = limit.filter(|&limit| lines.len() > limit) {
            // The chips that fit `limit` lines, less as many as the button needs room for.
            let mut count = chip_lines(&wants, width, GAP).get(limit - 1).map_or(chips.len(), |last| last.end);
            loop {
                let candidate = layout(count, chips.len() - count).1;
                if candidate.len() <= limit || count == 0 {
                    shown = count;
                    lines = candidate;
                    break;
                }
                count -= 1;
            }
        }
        let hidden = chips.len() - shown;
        let (widths, _) = layout(shown, hidden);

        let line_count = lines.len();
        let total = if line_count == 0 { 0.0 } else { line_count as f32 * height + (line_count - 1) as f32 * GAP };
        let (_, strip) = ui.allocate_space(vec2(width, total));
        let mut field = ui.new_child(
            UiBuilder::new().id_salt(id_salt).max_rect(strip).layout(direction::row(ui)).accessibility_role(Role::List).accessibility_label(label),
        );

        // Cells in reading order: the shown chips, "+N more", Clear All.
        let more_cell = (hidden > 0).then_some(shown);
        let clear_cell = clear_width.map(|_| shown + usize::from(hidden > 0));
        let ids: Vec<Id> = (0..widths.len())
            .map(|cell| {
                if Some(cell) == more_cell {
                    Interactive::id_in(&field, "more")
                } else if Some(cell) == clear_cell {
                    Interactive::id_in(&field, "clear-all")
                } else {
                    Interactive::id_in(&field, ("chip", cell))
                }
            })
            .collect();
        let pending = field.unique_id().with("focus-after");
        if let Some(index) = ctx.data_mut(|d| d.remove_temp::<usize>(pending))
            && !ids.is_empty()
        {
            ctx.memory_mut(|m| m.request_focus(ids[index.min(ids.len() - 1)]));
        }

        // One Tab stop; the arrows move the focus along the lines.
        let focused = enabled.then(|| ids.iter().position(|&id| ctx.memory(|m| m.has_focus(id)))).flatten();
        let mut stop = focused.unwrap_or(0);
        if let Some(from) = focused {
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(ids[from], EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() })
            });
            let nav = GridNav::ragged(lines.iter().map(|line| line.len())).rtl(rtl);
            ui.input_mut(|i| {
                for (key, grid_key) in [
                    (Key::ArrowRight, GridKey::Right),
                    (Key::ArrowLeft, GridKey::Left),
                    (Key::ArrowDown, GridKey::Down),
                    (Key::ArrowUp, GridKey::Up),
                    (Key::Home, GridKey::Home),
                    (Key::End, GridKey::End),
                ] {
                    if i.consume_key(Modifiers::NONE, key) {
                        stop = nav.step(stop, grid_key, None).index;
                    }
                }
            });
            if stop != from {
                ctx.memory_mut(|m| m.request_focus(ids[stop]));
            }
        }
        if let Some(requested) = ids.iter().position(|&id| ui.input(|i| i.has_accesskit_action_request(id, egui::accesskit::Action::Focus))) {
            stop = requested;
            ctx.memory_mut(|m| m.request_focus(ids[stop]));
        }
        let chip_focused = focused.filter(|&f| f < shown);
        let mut removed = None;
        let mut context_menu = None;
        if let Some(chip) = chip_focused {
            KeyClaim::claim(
                &ctx,
                ids[chip],
                &[Key::Delete, Key::Backspace, Key::Enter, Key::Space].map(|key| ClaimedKey::exact(Modifiers::NONE, key)),
            );
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete) || i.consume_key(Modifiers::NONE, Key::Backspace)) {
                removed = Some(chip);
                // The next chip takes the removed one's place; the last one hands the focus back.
                let target = if chip + 1 < shown { chip } else { chip.saturating_sub(1) };
                ctx.data_mut(|d| d.insert_temp(pending, target));
            }
        }

        let span = |from: f32, length: f32, row: usize| {
            let (a, b) = (direction::start_x(ui, strip) + forward * from, direction::start_x(ui, strip) + forward * (from + length));
            let top = strip.top() + row as f32 * (height + GAP);
            Rect::from_min_max(pos2(a.min(b), top), pos2(a.max(b), top + height))
        };
        let radius = corner_radius(tokens.shape.radius.pill.min(height / 2.0));
        let edge = Stroke::new(1.0, tokens.component.field.border.rest.to_egui());
        let hover_fill = tokens.color.state.hover.to_egui();
        let mut clicked = None;
        let mut cleared = false;
        let mut opened_more = None;
        let mut responses: Vec<Response> = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let mut at = 0.0;
            for cell in line.clone() {
                let rect = span(at, widths[cell], row);
                at += widths[cell] + GAP;
                if cell < shown {
                    let chip = &chips[cell];
                    let name = chip.full_name.as_deref().unwrap_or(&chip.label);
                    let accessible = if chip.partial { craft_i18n::fmt(&labels.partial, &[("name", name)]) } else { name.to_string() };
                    let mut body =
                        Interactive::with_id(ids[cell], Role::ListItem, accessible).corner_radius(radius).state_layer(false).enabled(enabled);
                    if cell != stop {
                        body = body.sense(Sense::CLICK);
                    }
                    let body = body.interact_at(&field, rect).finish(&field);
                    let cross_rect = chip.removable.then(|| {
                        let (end, inner) = (direction::end_x(ui, rect), direction::end_x(ui, rect) - forward * cross);
                        Rect::from_x_y_ranges(end.min(inner)..=end.max(inner), rect.y_range())
                    });
                    let mut cross_response = None;
                    if let Some(cross_rect) = cross_rect {
                        let parts =
                            field.new_child(UiBuilder::new().id_salt(("chip-remove", cell)).max_rect(cross_rect).accessibility_parent(ids[cell]));
                        let remove = craft_i18n::fmt(&labels.remove, &[("name", name)]);
                        let response = Interactive::new(("remove", cell), Role::Button, remove.as_str())
                            .sense(Sense::CLICK)
                            .corner_radius(CornerRadius::ZERO)
                            .state_layer(false)
                            .focus_ring(false)
                            .enabled(enabled)
                            .interact_at(&parts, cross_rect)
                            .finish(&parts);
                        cross_response = Some(response);
                    }
                    let cross_hot = cross_response.as_ref().is_some_and(|r| r.hovered());
                    if field.is_rect_visible(rect) {
                        let painter = field.painter();
                        if body.hovered() && enabled {
                            painter.rect_filled(rect, radius, hover_fill.gamma_multiply(0.5));
                        }
                        if let (true, Some(cross_rect)) = (cross_hot, cross_response.as_ref().map(|r| r.rect)) {
                            let round = if rtl {
                                CornerRadius { nw: radius.nw, sw: radius.sw, ne: 0, se: 0 }
                            } else {
                                CornerRadius { nw: 0, sw: 0, ne: radius.ne, se: radius.se }
                            };
                            painter.rect_filled(cross_rect, round, hover_fill);
                        }
                        painter.add(Shape::rect_stroke(rect, radius, edge, StrokeKind::Inside));
                        let label_room = (widths[cell] - PAD_X - if chip.removable { cross } else { PAD_X }).max(1.0);
                        let color = if !enabled {
                            tokens.color.text.disabled.to_egui()
                        } else if chip.partial {
                            muted
                        } else {
                            normal
                        };
                        let text = elided_galley(&field, &shown_text[cell], text_format(color), label_room, ElideAt::End);
                        let x = if rtl { rect.right() - PAD_X - text.galley.size().x } else { rect.left() + PAD_X };
                        painter.galley(
                            pos2(x, rect.center().y - text.galley.size().y / 2.0).round_to_pixels(field.pixels_per_point()),
                            text.galley,
                            color,
                        );
                        if let Some(r) = cross_response.as_ref() {
                            let c = r.rect.center();
                            let color = if !enabled {
                                tokens.color.text.disabled.to_egui()
                            } else if cross_hot {
                                normal
                            } else {
                                muted
                            };
                            let stroke = Stroke::new(CROSS_STROKE, color);
                            painter.line_segment([c + vec2(-CROSS_ARM, -CROSS_ARM), c + vec2(CROSS_ARM, CROSS_ARM)], stroke);
                            painter.line_segment([c + vec2(-CROSS_ARM, CROSS_ARM), c + vec2(CROSS_ARM, -CROSS_ARM)], stroke);
                        }
                    }
                    if enabled {
                        if cross_response.as_ref().is_some_and(|r| r.clicked()) {
                            removed = Some(cell);
                        } else if body.clicked() {
                            clicked = Some(cell);
                        }
                        let key = body.has_focus()
                            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10));
                        if body.secondary_clicked() || key {
                            context_menu = Some((cell, rect));
                        }
                    }
                    let tooltip = chip.full_name.as_deref().or(Some(&*chip.label)).filter(|_| chip.full_name.is_some());
                    responses.push(match tooltip {
                        Some(text) => body.tooltip(text),
                        None => body,
                    });
                    if let Some(r) = cross_response {
                        responses.push(r);
                    }
                } else {
                    let more = Some(cell) == more_cell;
                    let text = if more { more_text(hidden) } else { labels.clear_all.clone() };
                    let mut button =
                        Interactive::with_id(ids[cell], Role::Button, text.as_str()).corner_radius(radius).state_layer(false).enabled(enabled);
                    if cell != stop {
                        button = button.sense(Sense::CLICK);
                    }
                    let response = button.interact_at(&field, rect).finish(&field);
                    if field.is_rect_visible(rect) {
                        let painter = field.painter();
                        if response.hovered() && enabled {
                            painter.rect_filled(rect, radius, hover_fill.gamma_multiply(0.5));
                        }
                        painter.add(Shape::rect_stroke(rect, radius, edge, StrokeKind::Inside));
                        let color = if enabled { normal } else { tokens.color.text.disabled.to_egui() };
                        let galley = elided_galley(&field, &text, text_format(color), (rect.width() - 2.0 * PAD_X).max(1.0), ElideAt::End).galley;
                        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
                    }
                    if more {
                        let entries: Vec<MenuEntry<usize>> =
                            (shown..chips.len()).map(|i| MenuItem::new(chips[i].full_name.as_deref().unwrap_or(&chips[i].label), i).into()).collect();
                        if enabled {
                            opened_more = PressMenu::new(&entries).show(&field, &response).chosen;
                        }
                    } else if enabled && response.clicked() {
                        cleared = true;
                    }
                    responses.push(response);
                }
            }
        }
        if let Some(index) = opened_more {
            removed = removed.or(Some(index));
        }

        let response = match responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all.with_new_rect(strip),
            None => ui.interact(strip, field.unique_id(), Sense::hover()),
        };
        ChipFieldResponse { response, removed, clicked, context_menu, cleared }
    }
}

/// The chips of each line when chips `widths` wide flow into lines `row_width` wide, `gap`
/// apart: a chip that doesn't fit what is left of a line starts the next; a chip wider than a
/// line has a line of its own. Ranges index `widths`.
// lightcraft: crates/ui-egui/src/panels/keywording.rs:601 lines
pub fn chip_lines(widths: &[f32], row_width: f32, gap: f32) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut used = 0.0;
    for (i, &w) in widths.iter().enumerate() {
        if i > start && used + gap + w > row_width {
            lines.push(start..i);
            start = i;
            used = w;
        } else {
            used = if i == start { w } else { used + gap + w };
        }
    }
    if start < widths.len() {
        lines.push(start..widths.len());
    }
    lines
}
