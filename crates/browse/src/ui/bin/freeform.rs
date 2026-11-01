//! The Freeform view: cards placed by the user, stacks and snapping.

use std::collections::HashMap;
use std::fmt::Debug;

use craft_theme::TypeRole;
use craft_ui::behavior::{ListDrag, ListSelection, Selection, SelectionChange, SelectionMode, SelectionPick};
use craft_ui::canvas::textures::{Priority, TextureKey, ThumbRequest};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::list::RenameEnd;
use craft_ui::list::{ContextMenuRequest, DropMark, paint_drop_mark};
use craft_ui::localized::SharedLabels as _;
use craft_ui::primitives::{FocusRing, Interactive};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::accesskit::{Action, Role};
use egui::containers::scroll_area::ScrollSource;
use egui::{
    CornerRadius, DragAndDrop, EventFilter, Id, Key, Modifiers, PointerButton, Pos2, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

use super::icon::{ASPECT, CardList, entries_of};
use super::list::{drop_items, inside_any, item_of, open_event};
use super::{BinEvent, BinLabels, BinModel, BinState, MediaBin, ThumbPair};
use crate::ui::grid::rename::{self, Rename};
use crate::ui::grid::{
    BadgeVisibility, TileContent, TileFace, TileLabels, TileLook, TileSource as _, TileStyle, TileZoom, ZoomStep, card_size, paint_tile,
};

/// A card in the Freeform view, where the host stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct FreeformCard<K> {
    /// The item or bin.
    pub key: K,
    /// Its top-left corner in the view, in points.
    pub pos: Vec2,
    /// The stack it belongs to (cards of one stack move together); the stack's first card shows
    /// the "×N" badge.
    pub stack: Option<u32>,
}

/// What the Freeform view reads and edits.
pub(super) struct FreeformInput<'a, K: TextureKey, M: ?Sized> {
    pub id: Id,
    pub model: &'a M,
    pub state: &'a BinState<K>,
    pub selection: &'a mut Selection<K>,
    pub thumbnails: Option<ThumbPair<'a, K>>,
    pub cards: &'a [FreeformCard<K>],
    pub snap: Option<f32>,
    pub zoom_range: (f32, f32),
    pub default_size: f32,
    pub labels: &'a BinLabels,
    pub rename: Option<K>,
}

/// A drag under way: the cards it moves (the selection and the grabbed card's stack, in display
/// order), where each started, and where the pointer was pressed. It lives in egui memory, so a
/// drag that ends in Esc leaves nothing behind and the cards are drawn at the host's positions.
#[derive(Clone)]
struct Drag<K> {
    keys: Vec<K>,
    from: Vec<Vec2>,
    grabbed: usize,
    press: Pos2,
}

impl<K> Drag<K> {
    /// How far the cards have moved with the pointer at `pointer`: the grabbed card snaps to the
    /// grid and the rest keep their distance from it. Positions count from the start edge, so
    /// the pointer's x counts backwards when `rtl`.
    fn delta(&self, pointer: Pos2, snap: Option<f32>, rtl: bool) -> Vec2 {
        let start = self.from[self.grabbed];
        let mut by = pointer - self.press;
        if rtl {
            by.x = -by.x;
        }
        let target = start + by;
        let snapped = match snap {
            Some(step) if step > 0.0 => vec2((target.x / step).round() * step, (target.y / step).round() * step),
            _ => target,
        };
        snapped - start
    }
}

/// What the pointer did on a card, handled after the cards are drawn.
enum Hit {
    Click(Modifiers),
    Double(Modifiers),
    Menu(Option<Pos2>),
    DragStart(Modifiers),
}

/// A marquee under way: where it began (from the content's origin) and the selection it began
/// with. It lives in egui memory.
#[derive(Clone)]
struct Marquee<K> {
    start: Vec2,
    base: Selection<K>,
    additive: bool,
    cancelled: bool,
}

/// An arrow key's direction on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// The card the lead moves to for an arrow key: the nearest centre in that direction, with the
/// cross distance counting double so a key keeps to its row or column. `rects` are the cards as
/// the screen shows them.
pub(super) fn neighbour(rects: &[Rect], from: usize, dir: Dir) -> Option<usize> {
    let here = rects.get(from)?.center();
    rects
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != from)
        .filter_map(|(at, rect)| {
            let d = rect.center() - here;
            let (along, across) = match dir {
                Dir::Right => (d.x, d.y),
                Dir::Left => (-d.x, d.y),
                Dir::Down => (d.y, d.x),
                Dir::Up => (-d.y, d.x),
            };
            (along > 0.0).then_some((at, along + 2.0 * across.abs()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(at, _)| at)
}

/// Where a card at `pos` (counted from the start edge of a content `area`) is on the screen.
pub(super) fn card_at(area: Rect, pos: Vec2, cell: Vec2, rtl: bool) -> Rect {
    let x = if rtl { area.right() - pos.x - cell.x } else { area.left() + pos.x };
    Rect::from_min_size(pos2(x, area.top() + pos.y), cell)
}

/// A change of selection when `new` differs from `old`.
fn change_of<K: Clone + PartialEq>(old: &Selection<K>, pick: SelectionPick, row: Option<K>, new: Selection<K>) -> Option<SelectionChange<K>> {
    (new != *old).then_some(SelectionChange { pick, row, selection: new })
}

/// The point-size steps of a Cmd/Ctrl + arrow nudge (Photoshop's: 1, with Shift 10).
const NUDGE: f32 = 1.0;
const NUDGE_BIG: f32 = 10.0;

/// An arrow key press this pass, with what it asks of the cards.
enum Press {
    /// The lead moves (Shift extends the selection).
    Lead { dir: Dir, extend: bool },
    /// The lead jumps to the first or last card.
    Edge { last: bool, extend: bool },
    /// The selected cards move by this many points.
    Nudge(Vec2),
}

/// Takes this pass's arrow, Home and End presses from the input.
fn take_press(ctx: &egui::Context) -> Option<Press> {
    const ARROWS: [(Key, Dir, Vec2); 4] = [
        (Key::ArrowLeft, Dir::Left, vec2(-1.0, 0.0)),
        (Key::ArrowRight, Dir::Right, vec2(1.0, 0.0)),
        (Key::ArrowUp, Dir::Up, vec2(0.0, -1.0)),
        (Key::ArrowDown, Dir::Down, vec2(0.0, 1.0)),
    ];
    ctx.input_mut(|i| {
        for (key, dir, unit) in ARROWS {
            if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, key) {
                return Some(Press::Nudge(unit * NUDGE_BIG));
            }
            if i.consume_key(Modifiers::COMMAND, key) {
                return Some(Press::Nudge(unit * NUDGE));
            }
            if i.consume_key(Modifiers::SHIFT, key) {
                return Some(Press::Lead { dir, extend: true });
            }
            if i.consume_key(Modifiers::NONE, key) {
                return Some(Press::Lead { dir, extend: false });
            }
        }
        for (key, last) in [(Key::Home, false), (Key::End, true)] {
            if i.consume_key(Modifiers::SHIFT, key) {
                return Some(Press::Edge { last, extend: true });
            }
            if i.consume_key(Modifiers::NONE, key) {
                return Some(Press::Edge { last, extend: false });
            }
        }
        None
    })
}

/// Draws the Freeform view: the shown bin's cards (the Icon view's order and search) at the
/// host's [`FreeformCard`] positions in a scroll area; a card without a position takes the next
/// free slot of a flow layout, and while searching every card does. Positions count from the
/// start edge, so a right-to-left view mirrors them.
///
/// The cards are the grid's (`paint_tile`: bins with the folder face, the stack's "×N" pill) in
/// a list box with one Tab stop: the lead card is its active descendant. Arrows move the lead
/// to the nearest card in that direction (Shift extends, Home and End jump); Cmd/Ctrl + arrows
/// move the selected cards 1 pt, with Shift 10 pt, as `FreeformMoved`. A drag from the
/// background is a marquee (Esc puts the selection back).
///
/// A drag moves the selection and the grabbed card's stack together, snapping the grabbed card to
/// `snap`; Esc (egui's drag cancel) ends it with the cards where the host has them; a release in
/// the view reports `FreeformMoved { keys, to }` once, or `Drop { into: Some(bin) }` over a bin
/// card; a release outside the view reports nothing, so a footer action can take the payload. A
/// drag from another list released in the view is a drop into the bin it is over, else into the
/// shown bin; items already there report nothing. Click, Shift range, Cmd toggle, double click,
/// Enter, Delete, the context menu and Cmd/Ctrl + / − / 0 are as in the Icon view. F2 on the lead and
/// `MediaBin::start_rename` open the grid's rename field in the card's caption (`Rename` on Enter or
/// focus loss when the text changed; Esc cancels).
pub(super) fn freeform_view<K, M>(ui: &mut Ui, input: FreeformInput<'_, K, M>) -> (Response, Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:1014 (Freeform: cards, snap lines, stacks, Esc cancels).
    let FreeformInput { id, model, state, selection, thumbnails, cards, snap, zoom_range, default_size, labels, rename: requested } = input;
    let ctx = ui.ctx().clone();
    let tokens = Tokens::of(ui);
    let style = TileStyle::style(&tokens);
    let bin_style = MediaBin::<'_, K, M>::style(&tokens);
    let tile_labels = TileLabels::shared(ui);
    let rtl = direction::is_rtl(ui);
    let searching = !state.search.trim().is_empty();
    let (entries, _) = entries_of(model, state);
    let mut list = CardList::new(model, entries);
    let len = list.keys.len();

    let side = state.settings.icon_size;
    let picture_height = side / ASPECT;
    let caption_height = 2.0 * (tokens.typography.get(TypeRole::Caption).line_height + style.caption_gap);
    let cell = card_size(side, ASPECT, caption_height);

    // Positions: the host's, else the next slot of a flow layout.
    let by_key: HashMap<&K, &FreeformCard<K>> = cards.iter().map(|card| (&card.key, card)).collect();
    let columns = (((ui.available_width() - style.gap) / (cell.x + style.gap)).floor() as usize).max(1);
    let mut unplaced = 0usize;
    let mut positions: Vec<Vec2> = Vec::with_capacity(len);
    let mut stack_of: Vec<Option<u32>> = Vec::with_capacity(len);
    for key in &list.keys {
        let card = if searching { None } else { by_key.get(key).copied() };
        stack_of.push(card.and_then(|card| card.stack));
        positions.push(match card {
            Some(card) => card.pos,
            None => {
                let at = unplaced;
                unplaced += 1;
                vec2(style.gap + (at % columns) as f32 * (cell.x + style.gap), style.gap + (at / columns) as f32 * (cell.y + style.gap))
            }
        });
    }
    let mut sizes: HashMap<u32, usize> = HashMap::new();
    for stack in stack_of.iter().flatten() {
        *sizes.entry(*stack).or_default() += 1;
    }
    let mut counted: Vec<u32> = Vec::new();
    list.stacks = stack_of
        .iter()
        .map(|stack| {
            let stack = (*stack)?;
            let size = *sizes.get(&stack)?;
            (size > 1 && !counted.contains(&stack)).then(|| {
                counted.push(stack);
                size
            })
        })
        .collect();

    let box_id = id.with("freeform");
    let drag_slot = id.with("drag");
    let marquee_slot = id.with("marquee");
    let mut drag: Option<Drag<K>> = ctx.data(|d| d.get_temp::<Option<Drag<K>>>(drag_slot)).flatten();
    let mut marquee: Option<Marquee<K>> = ctx.data(|d| d.get_temp::<Option<Marquee<K>>>(marquee_slot)).flatten();
    let rename_slot = id.with("rename");
    let mut renaming: Option<Rename<K>> = ctx.data(|d| d.get_temp::<Option<Rename<K>>>(rename_slot)).flatten();
    let pointer = ctx.input(|i| i.pointer.latest_pos());
    let delta = drag.as_ref().zip(pointer).map_or(Vec2::ZERO, |(drag, pointer)| drag.delta(pointer, snap, rtl));
    let moved = |from: Vec2| (from + delta).max(Vec2::ZERO);

    let rules = ListSelection::new(SelectionMode::Multiple);
    let mut events: Vec<BinEvent<K>> = Vec::new();

    // The keys of the focused list box, handled before the cards are drawn so they show them.
    let focused = ctx.memory(|m| m.has_focus(box_id));
    let geometry = Geometry { positions: &positions, cell, rtl };
    let (reveal, menu_key) = if focused { handle_keys(&ctx, box_id, &list, &geometry, selection, &mut events) } else { (None, false) };

    let f2 = focused && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F2));
    let wanted = requested.or_else(|| f2.then(|| selection.lead.clone()).flatten());
    begin_rename(model, wanted.filter(|key| list.keys.contains(key)), &mut renaming, &mut events);

    let mut hits: Vec<(usize, Hit)> = Vec::new();
    let mut bin_rects: Vec<(K, Rect)> = Vec::new();
    let mut lead_tile: Option<Id> = None;
    let mut rename_shown: Option<rename::Outcome> = None;
    let extent = positions.iter().fold(Vec2::ZERO, |far, pos| far.max(*pos + cell)) + vec2(style.gap, style.gap);
    let grabbed_key = drag.as_ref().map(|d| d.keys[d.grabbed].clone());
    let is_dragged = |key: &K| drag.as_ref().is_some_and(|d| d.keys.contains(key));
    let order: Vec<usize> = (0..len).filter(|&at| !is_dragged(&list.keys[at])).chain((0..len).filter(|&at| is_dragged(&list.keys[at]))).collect();

    let builder = UiBuilder::new()
        .scope_id(box_id)
        .sense(Sense::CLICK | Sense::FOCUSABLE)
        .accessibility_role(Role::ListBox)
        .accessibility_label(labels.freeform_view.as_str())
        .layout(egui::Layout::top_down(egui::Align::Min));
    let scope = ui.scope_builder(builder, |ui| {
        ScrollArea::both().id_salt((id, "scroll")).auto_shrink(false).scroll_source(ScrollSource::SCROLL_BAR | ScrollSource::MOUSE_WHEEL).show(
            ui,
            |ui| {
                let available = ui.available_size();
                let (area, background) = ui.allocate_exact_size(extent.max(available), Sense::click_and_drag());
                let modifiers = ui.input(|i| i.modifiers);
                let ppp = ui.pixels_per_point();

                let band = marquee_step(&ctx, box_id, &mut marquee, &background, drag.is_some(), area, &list.keys, &geometry, selection, &mut events);

                for &at in &order {
                    let key = &list.keys[at];
                    let dragged = is_dragged(key);
                    let pos = if dragged { moved(positions[at]) } else { positions[at] };
                    let cell_rect = card_at(area, pos, cell, rtl);
                    if reveal == Some(at) {
                        ui.scroll_to_rect(cell_rect, None);
                    }
                    let open = renaming.as_ref().filter(|open| open.key == *key);
                    if !ui.is_rect_visible(cell_rect) && grabbed_key.as_ref() != Some(key) && open.is_none() {
                        continue;
                    }
                    let is_bin = list.bins[at];
                    let mut content = TileContent::default();
                    list.content(at, &mut content);
                    let selected = selection.contains(key);
                    let label = match &content.detail {
                        Some(detail) if !is_bin => format!("{}, {detail}", content.name),
                        _ => content.name.clone(),
                    };
                    let tile = id.with(("card", key));
                    let response = Interactive::with_id(tile, Role::ListBoxOption, label)
                        .sense(Sense::CLICK | Sense::DRAG)
                        .selected(selected)
                        .position_in_set(at)
                        .state_layer(false)
                        .focus_ring(false)
                        .interact_at(ui, cell_rect)
                        .finish(ui);
                    ctx.accesskit_node_builder(tile, |node| {
                        node.set_size_of_set(len);
                        node.add_action(Action::Focus);
                        node.add_action(Action::ShowContextMenu);
                    });
                    let is_lead = selection.lead.as_ref() == Some(key);
                    if is_lead {
                        lead_tile = Some(tile);
                    }
                    let picture = Rect::from_min_size(cell_rect.min, vec2(side, picture_height));
                    let caption = Rect::from_min_max(pos2(picture.left(), picture.bottom()), cell_rect.max);
                    let texture = thumbnails
                        .filter(|_| !is_bin)
                        .map(|(thumbs, source)| (thumbs, thumbs.state(&ctx, source, key, &ThumbRequest::for_cell(side, ppp), Priority::VISIBLE)));
                    let face = if is_bin { TileFace::Folder { glyph: bin_style.bin_icon } } else { TileFace::Picture };
                    let look = TileLook {
                        content: &content,
                        picture,
                        caption: open.is_none().then_some(caption),
                        selected,
                        hovered: response.hovered() && drag.is_none(),
                        lead_focused: focused && is_lead,
                        checked: None,
                        badges: BadgeVisibility::Never,
                        scrub: None,
                        rtl,
                        response: &response,
                        face,
                    };
                    paint_tile::<K>(
                        ui,
                        &style,
                        &tile_labels,
                        &look,
                        texture.as_ref().map(|(thumbs, _)| *thumbs),
                        texture.as_ref().map(|(_, state)| state),
                    );
                    if let Some(open) = open {
                        rename_shown = Some(rename::show(ui, tile, caption, &tile_labels.rename, open));
                    } else if drag.is_none() {
                        response.clone().on_hover_text(content.name);
                    }
                    if is_bin && !dragged {
                        bin_rects.push((key.clone(), cell_rect));
                    }

                    if response.double_clicked() {
                        hits.push((at, Hit::Double(modifiers)));
                    } else if response.clicked() || ui.input(|i| i.has_accesskit_action_request(tile, Action::Focus)) {
                        hits.push((at, Hit::Click(modifiers)));
                    }
                    if response.secondary_clicked() {
                        hits.push((at, Hit::Menu(response.interact_pointer_pos())));
                    }
                    if response.drag_started_by(PointerButton::Primary) {
                        hits.push((at, Hit::DragStart(modifiers)));
                    }
                }
                if let Some(rect) = band {
                    let painter = ui.painter().with_clip_rect(ui.clip_rect());
                    painter.rect_filled(rect, 0.0, style.marquee_fill);
                    painter.rect_stroke(rect, 0.0, Stroke::new(style.ring_width, style.marquee_line), StrokeKind::Inside);
                }
                (background, area)
            },
        )
    });
    if let Some(shown) = rename_shown {
        settle_rename(model, shown, &mut renaming, &mut events);
    }
    let response = scope.response;
    let scroll = scope.inner;
    let (background, area) = scroll.inner;
    let view = scroll.inner_rect;
    let cell_of = |at: usize| card_at(area, positions[at], cell, rtl);

    // Pointer hits, with the selection rules of the other views.
    if !hits.is_empty() || background.clicked() {
        ctx.memory_mut(|m| m.request_focus(box_id));
    }
    for (at, hit) in hits {
        let key = list.keys[at].clone();
        match hit {
            Hit::Click(modifiers) => {
                if let Some(change) = rules.click(selection, &list, at, modifiers) {
                    apply(selection, change, &mut events);
                }
            }
            Hit::Double(modifiers) => events.push(open_event(model, key, modifiers)),
            Hit::Menu(pointer) => {
                let was_selected = selection.contains(&key);
                if !was_selected && let Some(change) = rules.click(selection, &list, at, Modifiers::NONE) {
                    apply(selection, change, &mut events);
                }
                events.push(BinEvent::ContextMenu(ContextMenuRequest {
                    row: Some(key),
                    selected: was_selected,
                    anchor: cell_of(at),
                    pointer,
                    toggle: None,
                }));
            }
            Hit::DragStart(modifiers) => {
                if !selection.contains(&key) {
                    let pick = if modifiers.command { modifiers } else { Modifiers::NONE };
                    if let Some(change) = rules.click(selection, &list, at, pick) {
                        apply(selection, change, &mut events);
                    }
                }
                let stack = stack_of[at];
                let moving: Vec<usize> =
                    (0..len).filter(|&i| selection.contains(&list.keys[i]) || (stack.is_some() && stack_of[i] == stack)).collect();
                let grabbed = moving.iter().position(|&i| i == at).unwrap_or(0);
                let keys: Vec<K> = moving.iter().map(|&i| list.keys[i].clone()).collect();
                let press = ctx.input(|i| i.pointer.press_origin()).or(pointer).unwrap_or(area.min);
                drag = Some(Drag { from: moving.iter().map(|&i| positions[i]).collect(), keys: keys.clone(), grabbed, press });
                let payload = ListDrag { list: id, items: keys, grabbed: key.clone(), label: item_of(model, &key).name };
                events.push(BinEvent::Drag(payload.clone()));
                DragAndDrop::set_payload(&ctx, payload);
            }
        }
    }
    if background.clicked() {
        let modifiers = ctx.input(|i| i.modifiers);
        if !modifiers.command
            && !modifiers.shift
            && let Some(change) = rules.click_empty(selection)
        {
            apply(selection, change, &mut events);
        }
    }
    if background.secondary_clicked() {
        events.push(BinEvent::ContextMenu(ContextMenuRequest {
            row: None,
            selected: false,
            anchor: view,
            pointer: background.interact_pointer_pos(),
            toggle: None,
        }));
    }
    if menu_key {
        let lead = selection.lead.as_ref().and_then(|lead| list.keys.iter().position(|key| key == lead));
        events.push(BinEvent::ContextMenu(ContextMenuRequest {
            row: lead.map(|at| list.keys[at].clone()),
            selected: lead.is_some(),
            anchor: lead.map_or(view, cell_of),
            pointer: None,
            toggle: None,
        }));
    }

    // Cmd/Ctrl + / − / 0 zoom.
    if focused || response.hovered() {
        let zoom = TileZoom::Range { min: zoom_range.0, max: zoom_range.1, step: super::footer::ZOOM_STEP, default: default_size };
        events.extend(zoom_event(&ctx, zoom, side));
    }

    // The drag: a release in the view commits (into a bin card, or the new positions), one outside
    // the view or Esc ends it.
    let (released, down) = ctx.input(|i| (i.pointer.any_released(), i.pointer.any_down()));
    let mut dropped = false;
    if let Some(live) = drag.take() {
        let has_payload = DragAndDrop::payload::<ListDrag<K>>(&ctx).is_some_and(|p| p.list == id);
        if released || !down {
            if let Some(pointer) = pointer.filter(|p| view.contains(*p)) {
                let moved_by = live.delta(pointer, snap, rtl);
                let bin = bin_rects.iter().find(|(key, rect)| rect.contains(pointer) && !inside_any(model, key, &live.keys));
                if let Some((bin, _)) = bin {
                    dropped = true;
                    if let Some(items) = drop_items(model, Some(bin), &live.keys) {
                        events.push(BinEvent::Drop { items, into: Some(bin.clone()) });
                    }
                } else if moved_by != Vec2::ZERO {
                    let to = live.from.iter().map(|from| (*from + moved_by).max(Vec2::ZERO)).collect();
                    events.push(BinEvent::FreeformMoved { keys: live.keys, to });
                }
            }
        } else if has_payload {
            if let Some(snapped) = pointer.map(|p| live.delta(p, snap, rtl)).filter(|_| snap.is_some()) {
                let card = card_at(area, (live.from[live.grabbed] + snapped).max(Vec2::ZERO), cell, rtl);
                let line = Stroke::new(style.ring_hover_width, style.ring_hover);
                let painter = ui.painter().with_clip_rect(view);
                painter.vline(card.left(), area.y_range(), line);
                painter.hline(area.x_range(), card.top(), line);
            }
            if let Some(pointer) = pointer
                && let Some((_, rect)) = bin_rects.iter().find(|(key, rect)| rect.contains(pointer) && !inside_any(model, key, &live.keys))
            {
                paint_drop_mark(ui, *rect, DropMark::Into);
            }
            ctx.request_repaint();
            drag = Some(live);
        }
    }

    if !dropped {
        let pointer = pointer.filter(|p| view.contains(*p));
        foreign_drop(ui, model, id, state.shown.as_ref(), pointer, released, &bin_rects, &mut events);
    }
    ctx.data_mut(|d| {
        d.insert_temp(drag_slot, drag);
        d.insert_temp(marquee_slot, marquee);
        d.insert_temp(rename_slot, renaming);
    });

    ctx.accesskit_node_builder(box_id, |node| {
        node.set_size_of_set(len);
        node.set_multiselectable();
        match lead_tile.filter(|_| focused) {
            Some(tile) => node.set_active_descendant(tile.accesskit_id()),
            None => node.clear_active_descendant(),
        }
    });
    FocusRing::paint_if(ui, response.rect, CornerRadius::ZERO, focused && len == 0);
    (response, events)
}

/// Where the cards are, counted from the start edge.
struct Geometry<'a> {
    positions: &'a [Vec2],
    cell: Vec2,
    rtl: bool,
}

impl Geometry<'_> {
    fn rect(&self, area: Rect, at: usize) -> Rect {
        card_at(area, self.positions[at], self.cell, self.rtl)
    }
}

/// The focused list box's keys: arrows, Home, End, Space, Enter, Delete, Cmd/Ctrl + arrows and
/// the menu keys. Returns the card to scroll into view and whether the context menu was asked
/// for.
fn handle_keys<K, M>(
    ctx: &egui::Context,
    box_id: Id,
    list: &CardList<'_, K, M>,
    geometry: &Geometry<'_>,
    selection: &mut Selection<K>,
    events: &mut Vec<BinEvent<K>>,
) -> (Option<usize>, bool)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    let (len, cell, rtl, positions) = (list.keys.len(), geometry.cell, geometry.rtl, geometry.positions);
    let rules = ListSelection::new(SelectionMode::Multiple);
    let mut reveal = None;
    ctx.memory_mut(|m| m.set_focus_lock_filter(box_id, EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }));
    let lead = selection.lead.as_ref().and_then(|lead| list.keys.iter().position(|key| key == lead));
    let mut select = |to: usize, extend: bool, selection: &mut Selection<K>, events: &mut Vec<BinEvent<K>>| {
        if let Some(change) = rules.click(selection, list, to, Modifiers { shift: extend, ..Modifiers::NONE }) {
            apply(selection, change, events);
        }
        reveal = Some(to);
    };
    match take_press(ctx) {
        Some(Press::Lead { dir, extend }) => {
            let screen: Vec<Rect> = positions.iter().map(|pos| card_at(Rect::from_min_size(Pos2::ZERO, Vec2::ZERO), *pos, cell, rtl)).collect();
            let to = lead.map_or((len > 0).then_some(0), |from| neighbour(&screen, from, dir));
            if let Some(to) = to {
                select(to, extend && lead.is_some(), selection, events);
            }
        }
        Some(Press::Edge { last, extend }) => {
            if let Some(to) = if last { len.checked_sub(1) } else { (len > 0).then_some(0) } {
                select(to, extend, selection, events);
            }
        }
        Some(Press::Nudge(by)) => {
            let by = vec2(if rtl { -by.x } else { by.x }, by.y);
            let moving: Vec<usize> = (0..len).filter(|&at| selection.contains(&list.keys[at])).collect();
            if !moving.is_empty() {
                let keys = moving.iter().map(|&at| list.keys[at].clone()).collect();
                let to = moving.iter().map(|&at| (positions[at] + by).max(Vec2::ZERO)).collect();
                events.push(BinEvent::FreeformMoved { keys, to });
            }
        }
        None => {}
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space))
        && let Some(change) = rules.toggle_lead(selection, list)
    {
        apply(selection, change, events);
    }
    let menu = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10));
    let modifiers = ctx.input(|i| i.modifiers);
    if ctx.input(|i| i.key_pressed(Key::Enter))
        && let Some(lead) = lead
    {
        events.push(open_event(list.model, list.keys[lead].clone(), modifiers));
    }
    if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && !selection.is_empty() {
        events.push(BinEvent::Remove(selection.items.clone()));
    }
    (reveal, menu)
}

/// The marquee: a drag from the background selects the cards it touches, live; Esc puts the
/// selection back. Returns the band to draw.
#[allow(clippy::too_many_arguments)]
fn marquee_step<K: TextureKey>(
    ctx: &egui::Context,
    box_id: Id,
    marquee: &mut Option<Marquee<K>>,
    background: &Response,
    card_drag: bool,
    area: Rect,
    keys: &[K],
    geometry: &Geometry<'_>,
    selection: &mut Selection<K>,
    events: &mut Vec<BinEvent<K>>,
) -> Option<Rect> {
    let modifiers = ctx.input(|i| i.modifiers);
    let primary_down = ctx.input(|i| i.pointer.primary_down());
    let interact = ctx.input(|i| i.pointer.interact_pos());
    if marquee.is_none()
        && !card_drag
        && background.drag_started_by(PointerButton::Primary)
        && let Some(p) = interact
    {
        ctx.memory_mut(|m| m.request_focus(box_id));
        *marquee = Some(Marquee { start: p - area.min, base: selection.clone(), additive: modifiers.shift || modifiers.command, cancelled: false });
    }
    let mut live = marquee.take()?;
    if !live.cancelled && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        live.cancelled = true;
        let base = live.base.clone();
        if let Some(change) = change_of(selection, SelectionPick::Replace, base.lead.clone(), base) {
            apply(selection, change, events);
        }
    }
    let mut band = None;
    if let Some(p) = interact.filter(|_| !live.cancelled) {
        let rect = Rect::from_two_pos(area.min + live.start, p);
        let touched: Vec<K> = (0..keys.len()).filter(|&at| geometry.rect(area, at).intersects(rect)).map(|at| keys[at].clone()).collect();
        let mut items = if live.additive { live.base.items.clone() } else { Vec::new() };
        for key in &touched {
            if !items.contains(key) {
                items.push(key.clone());
            }
        }
        let lead = touched.last().cloned().or_else(|| live.base.lead.clone().filter(|_| live.additive));
        let anchor = touched.first().cloned().or_else(|| live.base.anchor.clone().filter(|_| live.additive));
        if let Some(change) = change_of(selection, SelectionPick::Range, lead.clone(), Selection { items, lead, anchor }) {
            apply(selection, change, events);
        }
        band = Some(rect);
    }
    if primary_down {
        *marquee = Some(live);
    }
    band
}

/// A drag from another list: into the bin card it is over, else into the shown bin.
#[allow(clippy::too_many_arguments)]
fn foreign_drop<K, M>(
    ui: &Ui,
    model: &M,
    id: Id,
    shown: Option<&K>,
    pointer: Option<Pos2>,
    released: bool,
    bins: &[(K, Rect)],
    events: &mut Vec<BinEvent<K>>,
) where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    let ctx = ui.ctx();
    let (Some(payload), Some(pointer)) = (DragAndDrop::payload::<ListDrag<K>>(ctx).filter(|p| p.list != id), pointer) else { return };
    let over = bins.iter().find(|(key, rect)| rect.contains(pointer) && !inside_any(model, key, &payload.items));
    if released {
        DragAndDrop::clear_payload(ctx);
        let into = over.map(|(key, _)| key.clone());
        if let Some(items) = drop_items(model, into.as_ref().or(shown), &payload.items) {
            events.push(BinEvent::Drop { items, into });
        }
    } else if let Some((_, rect)) = over {
        paint_drop_mark(ui, *rect, DropMark::Into);
    }
}

/// Opens the rename of `key` with its whole name selected, ending another open one first.
fn begin_rename<K, M>(model: &M, key: Option<K>, renaming: &mut Option<Rename<K>>, events: &mut Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    let Some(key) = key else { return };
    if renaming.as_ref().is_some_and(|open| open.key == key) {
        return;
    }
    if let Some(open) = renaming.take() {
        end_rename(model, open, RenameEnd::Blur, events);
    }
    *renaming = Some(Rename { text: item_of(model, &key).name, key, fresh: true });
}

/// Takes what the rename field showed: the text so far, and the end if it came.
fn settle_rename<K, M>(model: &M, shown: rename::Outcome, renaming: &mut Option<Rename<K>>, events: &mut Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    let Some(mut open) = renaming.take() else { return };
    open.text = shown.text;
    open.fresh = false;
    match shown.end {
        Some(end) => end_rename(model, open, end, events),
        None => *renaming = Some(open),
    }
}

/// Closes a rename: a changed, non-empty text is reported unless Esc ended it.
fn end_rename<K, M>(model: &M, open: Rename<K>, end: RenameEnd, events: &mut Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    if end != RenameEnd::Escape && !open.text.is_empty() && open.text != item_of(model, &open.key).name {
        events.push(BinEvent::Rename { key: open.key, text: open.text });
    }
}

/// The zoom step Cmd/Ctrl + / − / 0 asks for this pass, as the footer slider's event.
fn zoom_event<K>(ctx: &egui::Context, zoom: TileZoom, side: f32) -> Option<BinEvent<K>> {
    let step = ctx.input(|i| {
        if !i.modifiers.command {
            None
        } else if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
            Some(ZoomStep::In)
        } else if i.key_pressed(Key::Minus) {
            Some(ZoomStep::Out)
        } else if i.key_pressed(Key::Num0) {
            Some(ZoomStep::Reset)
        } else {
            None
        }
    })?;
    let modifiers = ctx.input(|i| i.modifiers);
    Some(BinEvent::Zoom(EditEvent::set(f64::from(zoom.step(side, step)), modifiers)))
}

/// Sets the selection a change gives and reports it.
fn apply<K: Clone>(selection: &mut Selection<K>, change: SelectionChange<K>, events: &mut Vec<BinEvent<K>>) {
    *selection = change.selection.clone();
    events.push(BinEvent::Selection(change));
}
