//! Slicers: tiles that filter a table by a column's values (N6.06).

use craft_ui::localized::Labels as _;
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{
    Align2, Color32, CornerRadius, EventFilter, FontId, IdSalt, Key, Modifiers, Rect, Response, Sense, StrokeKind, Ui, UiBuilder, WidgetInfo, pos2,
    vec2,
};

use craft_ui::behavior::{GridKey, GridNav};
use craft_ui::icons::{Icon, IconSize};

use super::style::SheetStyle;

craft_ui::words! {
    /// The slicer's words, English by default (Excel's).
    #[non_exhaustive]
    pub struct SlicerLabels {
        clear: "slicer.clear" = "Clear Filter";
    }
}

/// A slicer's parts.
#[derive(Clone, Debug, PartialEq)]
pub struct SlicerLayout {
    /// The caption header.
    pub header: Rect,
    /// The clear-filter button in the header.
    pub clear: Rect,
    /// One rect per value tile, in order.
    pub tiles: Vec<Rect>,
}

/// The layout of a slicer in `rect` at `zoom` with `n` values in `columns` columns: the header,
/// padding, gap, tile height and clear button from `style` (`sheet.slicer_*`, each × zoom),
/// tiles filling the width.
// gridcraft: crates/ui-egui/src/chartview.rs:400 `slicer_layout`.
pub fn slicer_layout(rect: Rect, zoom: f32, n: usize, columns: u32, style: &SheetStyle) -> SlicerLayout {
    let header_height = style.slicer_header * zoom;
    let padding = style.slicer_padding * zoom;
    let gap = style.slicer_gap * zoom;
    let tile_height = style.slicer_tile * zoom;
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), header_height));
    let button = style.slicer_clear * zoom;
    let clear = Rect::from_min_size(pos2(header.right() - padding - button, header.top() + (header_height - button) / 2.0), vec2(button, button));
    let columns = columns.max(1) as f32;
    let inner_width = (rect.width() - 2.0 * padding).max(1.0);
    let tile_width = ((inner_width - (columns - 1.0) * gap) / columns).max(1.0);
    let tiles = (0..n)
        .map(|i| {
            let (column, row) = ((i as f32) % columns, (i as f32 / columns).floor());
            let min = pos2(rect.left() + padding + column * (tile_width + gap), header.bottom() + padding + row * (tile_height + gap));
            Rect::from_min_size(min, vec2(tile_width, tile_height))
        })
        .collect();
    SlicerLayout { header, clear, tiles }
}

/// What a click on a slicer did.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlicerAction {
    /// Toggle the value of this tile.
    Toggle(String),
    /// Clear the filter.
    Clear,
}

/// Excel's slicer (GridCraft): a card with a soft shadow, the caption in the accent-soft header
/// with the funnel button (tinted while a filter is on), value tiles (on: accent with on-accent
/// text; off: hover with dim text); tiles that overflow the card are left out. A click on a tile
/// toggles its value, on the funnel clears; elsewhere it is the app's (select, drag the object).
/// The card and the funnel button sense click and drag, so a press on them never reaches the grid
/// under the slicer.
/// The slicer is one Tab stop whose arrows move between the tiles in reading order
/// ([`GridNav`] over the tile columns), Space toggles the focused
/// tile as a click does, and the funnel button is a Tab stop of
/// its own named by [`SlicerLabels::clear`]. AccessKit: a `ListBox` of `ListBoxOption`s with
/// `selected`, the focused tile its `active_descendant`. The tiles are the slicer's own (Excel's look); step 5's `Chip` is a removable tag
/// and `ToggleButton` a pressed button, neither of which is a slicer tile.
pub struct Slicer<'a> {
    id_salt: IdSalt,
    caption: &'a str,
    items: &'a [(String, bool)],
    columns: u32,
    filtered: bool,
    zoom: f32,
    labels: Option<SlicerLabels>,
}

impl<'a> Slicer<'a> {
    /// `items`: each value and whether it is on.
    pub fn new(id_salt: impl egui::AsIdSalt, caption: &'a str, items: &'a [(String, bool)]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), caption, items, columns: 1, filtered: false, zoom: 1.0, labels: None }
    }

    /// Uses these label texts instead of the ones read from the active locale's catalog.
    pub fn labels(mut self, labels: SlicerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The number of tile columns; 1 by default.
    pub fn columns(mut self, columns: u32) -> Self {
        self.columns = columns;
        self
    }

    /// Whether a filter is on; the funnel button is then tinted.
    pub fn filtered(mut self, on: bool) -> Self {
        self.filtered = on;
        self
    }

    /// The sheet zoom the slicer sits at.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    // gridcraft: crates/ui-egui/src/chartview.rs:424 `paint_slicers`, `:472` `slicer_click`.
    /// Shows the slicer in `rect` and reports clicks and key presses.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> SlicerOutput {
        let Self { id_salt, caption, items, columns, filtered, zoom, labels } = self;
        let labels = labels.unwrap_or_else(|| SlicerLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id_salt);
        let layout = slicer_layout(rect, zoom, items.len(), columns, &style);
        // Tiles that overflow the card are left out (GridCraft `chartview.rs:442`).
        let shown: Vec<usize> = (0..items.len()).filter(|&i| rect.contains(layout.tiles[i].left_top())).collect();
        let enabled = ui.is_enabled();

        // The AccessKit options are created first so the card, above them, keeps pointer and focus.
        let options = ui.new_child(UiBuilder::new().id_salt("tiles").accessibility_parent(id).max_rect(rect));
        for &i in &shown {
            let option = options.interact(layout.tiles[i], id.with(("tile", i)), Sense::hover());
            option.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, enabled, items[i].1, &items[i].0));
        }
        let card = ui.interact(rect, id, Sense::click_and_drag());
        card.widget_info(|| WidgetInfo::labeled(Role::ListBox, enabled, caption));
        let clear = ui.interact(layout.clear, id.with("clear"), Sense::click_and_drag());
        clear.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &labels.clear));

        let mut actions = Vec::new();
        let active_key = id.with("active");
        let mut active = ctx.data(|d| d.get_temp::<usize>(active_key)).unwrap_or(0);
        let nav = GridNav::wrapped(shown.len(), columns.max(1) as usize).rtl(craft_ui::direction::is_rtl(ui));
        if card.has_focus() && !shown.is_empty() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() });
            });
            let key = ui.input_mut(|input| {
                [Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown, Key::Home, Key::End]
                    .into_iter()
                    .find(|&key| input.consume_key(Modifiers::NONE, key))
            });
            if let Some(step) = key.and_then(GridKey::from_key) {
                active = nav.step(active.min(shown.len() - 1), step, None).index;
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Space)) {
                actions.push(SlicerAction::Toggle(items[shown[active.min(shown.len() - 1)]].0.clone()));
            }
        }
        if card.clicked() && !clear.hovered() {
            let hit = ui.input(|input| input.pointer.interact_pos()).and_then(|p| shown.iter().position(|&i| layout.tiles[i].contains(p)));
            if let Some(at) = hit {
                active = at;
                actions.push(SlicerAction::Toggle(items[shown[at]].0.clone()));
            }
        }
        if clear.clicked() {
            actions.push(SlicerAction::Clear);
        }
        ctx.data_mut(|d| d.insert_temp(active_key, active));
        let focused = (card.has_focus() && !shown.is_empty()).then(|| id.with(("tile", shown[active.min(shown.len() - 1)])).accesskit_id());
        ctx.accesskit_node_builder(id, |node| match focused {
            Some(tile) => node.set_active_descendant(tile),
            None => node.clear_active_descendant(),
        });

        let colors = &tokens.color;
        let radius: CornerRadius = tokens.radius.panel;
        let line = Line::Hairline.egui(&ctx, colors.line.border.to_egui());
        let painter = ui.painter();
        painter.add(tokens.shadow.floating.as_shape(rect, radius));
        painter.rect_filled(rect, radius, colors.surface.raised.to_egui());
        painter.rect_stroke(rect, radius, line, StrokeKind::Inside);
        let header_radius = CornerRadius { sw: 0, se: 0, ..radius };
        painter.rect_filled(layout.header, header_radius, colors.accent.soft.to_egui());
        let strong = tokens.font(style.slicer_caption);
        let caption_font = FontId::new(strong.size * zoom, strong.family);
        let base = tokens.font(style.slicer_text);
        let font = FontId::new(base.size * zoom, base.family);
        let padding = style.slicer_padding * zoom;
        let caption_at = layout.header.left_center() + vec2(padding, 0.0);
        painter.with_clip_rect(Rect::from_min_max(layout.header.min, pos2(layout.clear.left(), layout.header.max.y))).text(
            caption_at,
            Align2::LEFT_CENTER,
            caption,
            caption_font,
            colors.accent.text.to_egui(),
        );
        let funnel = if filtered { colors.accent.solid } else { colors.text.muted }.to_egui();
        craft_ui::icons::paint(ui, layout.clear, Icon::Funnel, IconSize::Sm, funnel);
        let painter = ui.painter();
        for &i in &shown {
            let (text, on) = &items[i];
            let tile = layout.tiles[i];
            let (fill, ink): (Color32, Color32) = if *on {
                (colors.accent.solid.to_egui(), colors.text.on_accent.to_egui())
            } else {
                (colors.state.hover.to_egui(), colors.text.muted.to_egui())
            };
            painter.rect_filled(tile, tokens.radius.control, fill);
            painter.with_clip_rect(tile).text(tile.left_center() + vec2(padding, 0.0), Align2::LEFT_CENTER, text, font.clone(), ink);
        }
        if card.has_focus() && !shown.is_empty() {
            let focus = Line::W2.egui(&ctx, colors.line.focus.to_egui());
            painter.rect_stroke(layout.tiles[shown[active.min(shown.len() - 1)]], tokens.radius.control, focus, StrokeKind::Outside);
        }
        if clear.has_focus() {
            let focus = Line::W2.egui(&ctx, colors.line.focus.to_egui());
            painter.rect_stroke(layout.clear, tokens.radius.control, focus, StrokeKind::Outside);
        }
        SlicerOutput { response: card, actions }
    }
}

/// What the slicer did this pass.
#[must_use]
pub struct SlicerOutput {
    /// The slicer's response.
    pub response: Response,
    /// In order. A click on a tile or the funnel and a key press in one pass report both.
    pub actions: Vec<SlicerAction>,
}
