//! [`ChipField`]: removable chips laid out in lines, with "+N more" where a long list stops.

use std::borrow::Cow;
use std::ops::Range;

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{
    AsIdSalt, Color32, Context, CornerRadius, Id, IdSalt, Key, Modifiers, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2,
};

use super::{ControlSize, TooltipExt as _};
use crate::behavior::{ClaimedKey, GridNav, KeyClaim};
use crate::direction;
use crate::localized::words::counts;
use crate::localized::{Labels as _, Words};
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::Interactive;
use crate::text::{ElideAt, elided_galley};
use crate::theme::{ToEgui, Tokens, TokensOf as _, UiTokens, corner_radius, role_format};

crate::words! {
    /// The chips' words, English by default.
    #[non_exhaustive]
    pub struct ChipFieldLabels {
        /// A chip's remove button, for AccessKit and its tooltip: `{name}` the chip's full name
        /// (LightCraft's "Remove “{name}”").
        remove: "chipField.remove" = "Remove “{name}”";
        /// The button for the chips that don't fit: `{n}` how many. A
        /// [`Counted`](crate::localized::words::Counted) word (one and other both "+{n} more" in
        /// English), written with [`Words::counted`](crate::localized::Words::counted); its
        /// forms are in the kernel's [`COUNTS`](crate::localized::words::COUNTS).
        more: "chipField.more" = "+{n} more";
        clear_all: "chipField.clearAll" = "Clear All";
        /// A partial chip's accessible name (the asterisk is only drawn): `{name}`.
        partial: "chipField.partial" = "{name}, some of the selection";
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
///   the state layer; hovering the × lights only the × end (with the state layer's pressed fill
///   too), so the user sees which part a click hits. A label too long for a line is cut short at its end with "…" to fit
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
///   [`GridNav::roving_move_in`](crate::behavior::GridNav::roving_move_in) (resting on the first chip).
///   ← and → move along the reading order (mirrored in right-to-left), ↑ and ↓ to the
///   neighbouring line, Home and End to the ends. Delete and Backspace remove the focused chip
///   and the focus moves to the next one (the previous one at the end), Enter and Space click
///   it, the Menu key and Shift+F10 ask for its menu. The focused chip claims Delete,
///   Backspace, Enter and Space ([`KeyClaim`](crate::behavior::KeyClaim)), so the app's Delete
///   command doesn't run while a chip has the focus. The focused chip draws the focus ring.
/// - **AccessKit.** A `List` named by its label; each chip a `ListItem` named by its full name
///   (and [`ChipFieldLabels::partial`]), with a child `Button` [`ChipFieldLabels::remove`].
pub struct ChipField<'a> {
    id_salt: IdSalt,
    label: String,
    chips: &'a [Chip<'a>],
    max_lines: Option<usize>,
    clear_all: bool,
    size: ControlSize,
    enabled: bool,
    labels: Option<ChipFieldLabels>,
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
    pub fn labels(mut self, labels: ChipFieldLabels) -> Self {
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
    pub fn show(self, ui: &mut Ui) -> ChipFieldResponse {
        let labels = self.labels.clone().unwrap_or_else(|| ChipFieldLabels::of(ui));
        let tokens = Tokens::of(ui);
        let enabled = self.enabled && ui.is_enabled();
        let look = Look::new(ui, &tokens, &labels, self.size, enabled);
        let arrangement = self.arrange(ui, &look);
        let Arrangement { shown, hidden, ref widths, ref lines, clear } = arrangement;
        let ctx = ui.ctx().clone();

        let total = if lines.is_empty() { 0.0 } else { lines.len() as f32 * look.height + (lines.len() - 1) as f32 * look.gap };
        let (_, strip) = ui.allocate_space(vec2(look.width, total));
        let mut field = ui.new_child(
            UiBuilder::new()
                .id_salt(self.id_salt)
                .max_rect(strip)
                .layout(direction::row(ui))
                .accessibility_role(Role::List)
                .accessibility_label(self.label.as_str()),
        );

        // Cells in reading order: the shown chips, "+N more", Clear All.
        let more_cell = (hidden > 0).then_some(shown);
        let clear_cell = clear.then_some(shown + usize::from(hidden > 0));
        let ids: Vec<Id> = (0..widths.len())
            .map(|cell| match cell {
                _ if Some(cell) == more_cell => Interactive::id_in(&field, "more"),
                _ if Some(cell) == clear_cell => Interactive::id_in(&field, "clear-all"),
                _ => Interactive::id_in(&field, ("chip", cell)),
            })
            .collect();
        let pending = field.unique_id().with("focus-after");
        if let Some(index) = ctx.data_mut(|d| d.remove_temp::<usize>(pending))
            && !ids.is_empty()
        {
            ctx.memory_mut(|m| m.request_focus(ids[index.min(ids.len() - 1)]));
        }
        let focused = enabled.then(|| ids.iter().position(|&id| ctx.memory(|m| m.has_focus(id)))).flatten();
        let nav = GridNav::ragged(lines.iter().map(Range::len)).rtl(look.rtl);
        let (stop, _) = GridNav::roving_move_in(ui, &ids, &nav, None, enabled);

        let mut out = Outcome::default();
        if let Some(chip) = focused.filter(|&f| f < shown) {
            KeyClaim::claim(
                &ctx,
                ids[chip],
                &[Key::Delete, Key::Backspace, Key::Enter, Key::Space].map(|key| ClaimedKey::exact(Modifiers::NONE, key)),
            );
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete) || i.consume_key(Modifiers::NONE, Key::Backspace)) {
                out.removed = Some(chip);
                // The next chip takes the removed one's place; the last one hands the focus back.
                ctx.data_mut(|d| d.insert_temp(pending, if chip + 1 < shown { chip } else { chip.saturating_sub(1) }));
            }
        }

        let mut responses: Vec<Response> = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let mut at = 0.0;
            for cell in line.clone() {
                let rect = look.span(ui, strip, at, widths[cell], row);
                at += widths[cell] + look.gap;
                let cell_sense = if cell == stop { Sense::click() } else { Sense::CLICK };
                if cell < shown {
                    let part = ChipPart {
                        chip: &self.chips[cell],
                        text: &arrangement_text(&self.chips[cell]),
                        cell,
                        id: ids[cell],
                        rect,
                        sense: cell_sense,
                    };
                    responses.extend(part.show(&mut field, &look, &mut out));
                } else {
                    let more = Some(cell) == more_cell;
                    let text = if more { more_text(&ctx, &labels, hidden) } else { labels.clear_all.clone() };
                    let response = button(&mut field, &look, ids[cell], rect, cell_sense, &text);
                    if more {
                        let entries: Vec<MenuEntry<usize>> = (shown..self.chips.len())
                            .map(|i| MenuItem::new(self.chips[i].full_name.as_deref().unwrap_or(&self.chips[i].label), i).into())
                            .collect();
                        if enabled && let Some(i) = PressMenu::new(&entries).show(&field, &response).chosen {
                            out.removed = out.removed.or(Some(i));
                        }
                    } else if enabled && response.clicked() {
                        out.cleared = true;
                    }
                    responses.push(response);
                }
            }
        }

        let response = match responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all.with_new_rect(strip),
            None => ui.interact(strip, field.unique_id(), Sense::hover()),
        };
        ChipFieldResponse { response, removed: out.removed, clicked: out.clicked, context_menu: out.context_menu, cleared: out.cleared }
    }

    /// Which chips show and how the cells flow into lines: all of them, or with
    /// [`max_lines`](Self::max_lines) as many as leave room for "+N more" in the last line.
    fn arrange(&self, ui: &Ui, look: &Look<'_>) -> Arrangement {
        let chips = self.chips;
        let wants: Vec<f32> = chips
            .iter()
            .map(|chip| {
                let label = look.text_width(ui, &arrangement_text(chip));
                (look.pad_x + label + if chip.removable { look.cross } else { look.pad_x }).ceil().min(look.width.max(look.cross + look.pad_x))
            })
            .collect();
        let clear_width = (self.clear_all && !chips.is_empty()).then(|| look.button_width(ui, &look.labels.clear_all));
        let ctx = ui.ctx();
        let layout = |shown: usize, hidden: usize| {
            let mut widths = wants[..shown].to_vec();
            if hidden > 0 {
                widths.push(look.button_width(ui, &more_text(ctx, look.labels, hidden)));
            }
            widths.extend(clear_width);
            let lines = chip_lines(&widths, look.width, look.gap);
            (widths, lines)
        };
        let mut shown = chips.len();
        let (mut widths, mut lines) = layout(shown, 0);
        if let Some(limit) = self.max_lines.map(|n| n.max(1)).filter(|&limit| lines.len() > limit) {
            // The chips that fit `limit` lines, less as many as the button needs room for.
            shown = chip_lines(&wants, look.width, look.gap).get(limit - 1).map_or(chips.len(), |last| last.end);
            loop {
                (widths, lines) = layout(shown, chips.len() - shown);
                if lines.len() <= limit || shown == 0 {
                    break;
                }
                shown -= 1;
            }
        }
        Arrangement { shown, hidden: chips.len() - shown, widths, lines, clear: clear_width.is_some() }
    }
}

/// What a chip shows: its label, with an asterisk after a partial one's.
fn arrangement_text(chip: &Chip<'_>) -> String {
    if chip.partial { format!("{}*", chip.label) } else { chip.label.to_string() }
}

/// "+N more" for `hidden` chips.
fn more_text(ctx: &Context, labels: &ChipFieldLabels, hidden: usize) -> String {
    Words::of_ctx(ctx).counted(ctx, counts::MORE, &labels.more, hidden as u64)
}

/// The field's cells: which chips show, "+N more" after them when some don't, Clear All.
struct Arrangement {
    shown: usize,
    hidden: usize,
    widths: Vec<f32>,
    lines: Vec<Range<usize>>,
    clear: bool,
}

/// What the cells of one pass report.
#[derive(Default)]
struct Outcome {
    removed: Option<usize>,
    clicked: Option<usize>,
    context_menu: Option<(usize, Rect)>,
    cleared: bool,
}

/// Sizes and colours of one pass.
struct Look<'a> {
    tokens: &'a UiTokens,
    labels: &'a ChipFieldLabels,
    height: f32,
    /// Width of the × target.
    cross: f32,
    /// Between a chip's edge and its label, and after a label with no ×.
    pad_x: f32,
    /// Between chips and between lines.
    gap: f32,
    /// Reach and stroke width of the × arms.
    cross_arm: f32,
    cross_stroke: f32,
    width: f32,
    enabled: bool,
    rtl: bool,
    radius: CornerRadius,
    edge: Stroke,
    normal: Color32,
    muted: Color32,
}

impl<'a> Look<'a> {
    fn new(ui: &Ui, tokens: &'a UiTokens, labels: &'a ChipFieldLabels, size: ControlSize, enabled: bool) -> Self {
        let chip = &tokens.component.chip;
        let height = match size {
            ControlSize::Sm => chip.height.sm,
            ControlSize::Md => chip.height.md,
        };
        let color = |c: craft_theme::Rgba| c.to_egui();
        Self {
            tokens,
            labels,
            height,
            cross: chip.remove_width.min(height),
            pad_x: chip.pad_x,
            gap: chip.gap,
            cross_arm: chip.cross_arm,
            cross_stroke: chip.cross_stroke,
            width: ui.available_width(),
            enabled,
            rtl: direction::is_rtl(ui),
            radius: corner_radius(tokens.shape.radius.pill.min(height / 2.0)),
            edge: Stroke::new(1.0, color(tokens.component.field.border.rest)),
            normal: color(tokens.color.text.normal),
            muted: color(tokens.color.text.muted),
        }
    }

    /// The corners of the × end: the chip's on its outer side.
    fn cross_radius(&self) -> CornerRadius {
        let r = self.radius;
        if self.rtl { CornerRadius { nw: r.nw, sw: r.sw, ne: 0, se: 0 } } else { CornerRadius { nw: 0, sw: 0, ne: r.ne, se: r.se } }
    }

    fn format(&self, color: Color32) -> egui::text::TextFormat {
        role_format(self.tokens, TypeRole::Label, color)
    }

    fn text_width(&self, ui: &Ui, text: &str) -> f32 {
        elided_galley(ui, text, self.format(self.normal), f32::INFINITY, ElideAt::End).galley.size().x
    }

    fn button_width(&self, ui: &Ui, text: &str) -> f32 {
        (2.0 * self.pad_x + self.text_width(ui, text)).ceil().min(self.width)
    }

    fn text_color(&self, muted: bool) -> Color32 {
        match () {
            () if !self.enabled => self.tokens.color.text.disabled.to_egui(),
            () if muted => self.muted,
            () => self.normal,
        }
    }

    /// The span of line `row` from `from` to `from + length`, along the reading direction.
    fn span(&self, ui: &Ui, strip: Rect, from: f32, length: f32, row: usize) -> Rect {
        let forward = direction::forward(ui);
        let (a, b) = (direction::start_x(ui, strip) + forward * from, direction::start_x(ui, strip) + forward * (from + length));
        let top = strip.top() + row as f32 * (self.height + self.gap);
        Rect::from_min_max(pos2(a.min(b), top), pos2(a.max(b), top + self.height))
    }
}

/// A pill button ("+N more", Clear All) over `rect`.
fn button(field: &mut Ui, look: &Look<'_>, id: Id, rect: Rect, sense: Sense, text: &str) -> Response {
    let response = Interactive::with_id(id, Role::Button, text)
        .sense(sense)
        .corner_radius(look.radius)
        .enabled(look.enabled)
        .interact_at(field, rect)
        .finish(field);
    if field.is_rect_visible(rect) {
        let painter = field.painter();
        painter.add(Shape::rect_stroke(rect, look.radius, look.edge, StrokeKind::Inside));
        let color = look.text_color(false);
        let galley = elided_galley(field, text, look.format(color), (rect.width() - 2.0 * look.pad_x).max(1.0), ElideAt::End).galley;
        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    }
    response
}

/// One chip: its body over `rect` and, for a removable one, the × at its end.
struct ChipPart<'a> {
    chip: &'a Chip<'a>,
    text: &'a str,
    cell: usize,
    id: Id,
    rect: Rect,
    sense: Sense,
}

impl ChipPart<'_> {
    /// Senses, paints and reports the chip; returns its responses.
    fn show(self, field: &mut Ui, look: &Look<'_>, out: &mut Outcome) -> Vec<Response> {
        let Self { chip, text, cell, id, rect, sense } = self;
        let name = chip.full_name.as_deref().unwrap_or(&chip.label);
        let accessible = if chip.partial { craft_i18n::fmt(&look.labels.partial, &[("name", name)]) } else { name.to_string() };
        let body = Interactive::with_id(id, Role::ListItem, accessible)
            .sense(sense)
            .corner_radius(look.radius)
            .enabled(look.enabled)
            .interact_at(field, rect)
            .finish(field);
        let cross = chip.removable.then(|| {
            let forward = direction::forward(field);
            let (end, inner) = (direction::end_x(field, rect), direction::end_x(field, rect) - forward * look.cross);
            let cross_rect = Rect::from_x_y_ranges(end.min(inner)..=end.max(inner), rect.y_range());
            let parts = field.new_child(UiBuilder::new().id_salt(("chip-remove", cell)).max_rect(cross_rect).accessibility_parent(id));
            let remove = craft_i18n::fmt(&look.labels.remove, &[("name", name)]);
            Interactive::new(("remove", cell), Role::Button, remove.as_str())
                .sense(Sense::CLICK)
                .corner_radius(look.cross_radius())
                .focus_ring(false)
                .enabled(look.enabled)
                .interact_at(&parts, cross_rect)
                .finish(&parts)
        });
        if field.is_rect_visible(rect) {
            self.paint(field, look, cross.as_ref(), text);
        }
        if look.enabled {
            if cross.as_ref().is_some_and(Response::clicked) {
                out.removed = Some(cell);
            } else if body.clicked() {
                out.clicked = Some(cell);
            }
            let key = body.has_focus() && field.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10));
            if body.secondary_clicked() || key {
                out.context_menu = Some((cell, rect));
            }
        }
        let body = if chip.full_name.is_some() { body.tooltip(name) } else { body };
        std::iter::once(body).chain(cross).collect()
    }

    fn paint(&self, field: &Ui, look: &Look<'_>, cross: Option<&Response>, text: &str) {
        let (chip, rect) = (self.chip, self.rect);
        let painter = field.painter();
        let cross_hot = cross.is_some_and(Response::hovered);
        painter.add(Shape::rect_stroke(rect, look.radius, look.edge, StrokeKind::Inside));
        let room = (rect.width() - look.pad_x - if chip.removable { look.cross } else { look.pad_x }).max(1.0);
        let color = look.text_color(chip.partial);
        let galley = elided_galley(field, text, look.format(color), room, ElideAt::End).galley;
        let x = if look.rtl { rect.right() - look.pad_x - galley.size().x } else { rect.left() + look.pad_x };
        painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(field.pixels_per_point()), galley, color);
        if let Some(cross) = cross {
            let c = cross.rect.center();
            let stroke = Stroke::new(look.cross_stroke, if cross_hot && look.enabled { look.normal } else { look.text_color(true) });
            painter.line_segment([c + vec2(-look.cross_arm, -look.cross_arm), c + vec2(look.cross_arm, look.cross_arm)], stroke);
            painter.line_segment([c + vec2(-look.cross_arm, look.cross_arm), c + vec2(look.cross_arm, -look.cross_arm)], stroke);
        }
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
