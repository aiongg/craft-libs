//! The Freeform view: cards placed by the user, stacks and snapping.

use std::collections::HashMap;
use std::fmt::Debug;

use craft_theme::TypeRole;
use craft_ui::behavior::{ListDrag, ListSelection, Selection, SelectionChange, SelectionMode};
use craft_ui::canvas::textures::{Priority, TextureKey, TextureState, ThumbPaint, ThumbRequest, Thumbnails};
use craft_ui::list::{ContextMenuRequest, DropMark, paint_drop_mark};
use craft_ui::localized::SharedLabels as _;
use craft_ui::primitives::{FocusRing, Interactive};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};
use egui::accesskit::Role;
use egui::containers::scroll_area::ScrollSource;
use egui::text::LayoutJob;
use egui::{
    Color32, CornerRadius, DragAndDrop, Id, Key, Modifiers, PointerButton, Pos2, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
    pos2, vec2,
};

use super::icon::{ASPECT, CardList, entries_of, paint_bin_face};
use super::list::{inside_any, item_of, open_event};
use super::{BinEvent, BinModel, BinState, BinStyle, ThumbPair};
use crate::ui::grid::{TileLabels, TileStyle, TileZoom, card_size, fit_name};

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
    /// grid and the rest keep their distance from it.
    fn delta(&self, pointer: Pos2, snap: Option<f32>) -> Vec2 {
        let start = self.from[self.grabbed];
        let target = start + (pointer - self.press);
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

/// Draws the Freeform view: the shown bin's cards (the Icon view's order and search) at the
/// host's [`FreeformCard`] positions in a scroll area; a card without a position takes the next
/// free slot of a flow layout, and while searching every card does.
///
/// A drag moves the selection and the grabbed card's stack together, snapping the grabbed card to
/// `snap`; Esc (egui's drag cancel) ends it with the cards where the host has them; a release in
/// the view reports `FreeformMoved { keys, to }` once, or `Drop { into: Some(bin) }` over a bin
/// card; a release outside the view reports nothing, so a footer action can take the payload. The
/// first card of a stack shows "×N". Click, Shift range, Cmd toggle, double click, Enter, Delete,
/// the context menu and Cmd/Ctrl + / − / 0 are as in the Icon view.
pub(super) fn freeform_view<K, M>(ui: &mut Ui, input: FreeformInput<'_, K, M>) -> (Response, Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:1014 (Freeform: cards, snap lines, stacks, Esc cancels).
    let FreeformInput { id, model, state, selection, thumbnails, cards, snap, zoom_range, default_size } = input;
    let ctx = ui.ctx().clone();
    let tokens = Tokens::of(ui);
    let style = TileStyle::of(&tokens);
    let bin_style = BinStyle::of(&tokens);
    let tile_labels = TileLabels::shared(ui);
    let searching = !state.search.trim().is_empty();
    let (entries, _) = entries_of(model, state);
    let mut list = CardList::new(model, entries);

    let side = state.settings.icon_size;
    let picture_size = vec2(side, side / ASPECT);
    let caption_height = {
        let line = ui.painter().layout_job(LayoutJob::single_section("Ag".to_owned(), role_format(&tokens, TypeRole::Caption, Color32::PLACEHOLDER)));
        2.0 * (line.size().y + style.caption_gap)
    };
    let cell = card_size(side, ASPECT, caption_height);

    // Positions: the host's, else the next slot of a flow layout.
    let by_key: HashMap<&K, &FreeformCard<K>> = cards.iter().map(|card| (&card.key, card)).collect();
    let columns = (((ui.available_width() - style.gap) / (cell.x + style.gap)).floor() as usize).max(1);
    let mut unplaced = 0usize;
    let mut positions: Vec<Vec2> = Vec::with_capacity(list.keys.len());
    let mut stack_of: Vec<Option<u32>> = Vec::with_capacity(list.keys.len());
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

    let drag_slot = id.with("drag");
    let mut drag: Option<Drag<K>> = ctx.data(|d| d.get_temp::<Option<Drag<K>>>(drag_slot)).flatten();
    let pointer = ctx.input(|i| i.pointer.latest_pos());
    let delta = drag.as_ref().zip(pointer).map_or(Vec2::ZERO, |(drag, pointer)| drag.delta(pointer, snap));
    let moved = |from: Vec2| (from + delta).max(Vec2::ZERO);

    let mut events: Vec<BinEvent<K>> = Vec::new();
    let mut hits: Vec<(usize, Hit)> = Vec::new();
    let mut bin_rects: Vec<(K, Rect)> = Vec::new();
    let mut focused: Option<usize> = None;
    let extent = positions.iter().fold(Vec2::ZERO, |far, pos| far.max(*pos + cell)) + vec2(style.gap, style.gap);
    let grabbed_key = drag.as_ref().map(|d| d.keys[d.grabbed].clone());
    let is_dragged = |key: &K| drag.as_ref().is_some_and(|d| d.keys.contains(key));
    let order: Vec<usize> =
        (0..list.keys.len()).filter(|&at| !is_dragged(&list.keys[at])).chain((0..list.keys.len()).filter(|&at| is_dragged(&list.keys[at]))).collect();

    let scroll = ScrollArea::both()
        .id_salt((id, "scroll"))
        .auto_shrink(false)
        .scroll_source(ScrollSource::SCROLL_BAR | ScrollSource::MOUSE_WHEEL)
        .show(ui, |ui| {
            let available = ui.available_size();
            let (area, background) = ui.allocate_exact_size(extent.max(available), Sense::click_and_drag());
            let modifiers = ui.input(|i| i.modifiers);
            let ppp = ui.pixels_per_point();
            let placeholder = Thumbnails::<K>::style(&tokens).placeholder;
            for &at in &order {
                let key = &list.keys[at];
                let dragged = is_dragged(key);
                let pos = if dragged { moved(positions[at]) } else { positions[at] };
                let top_left = area.min + pos;
                let cell_rect = Rect::from_min_size(top_left, cell);
                if !ui.is_rect_visible(cell_rect) && grabbed_key.as_ref() != Some(key) {
                    continue;
                }
                let is_bin = list.bins[at];
                let item = item_of(model, key);
                let selected = selection.contains(key);
                let label = match &item.duration {
                    Some(detail) if !is_bin => format!("{}, {detail}", item.name),
                    _ => item.name.clone(),
                };
                let response = Interactive::with_id(id.with(("card", key)), Role::ListBoxOption, label)
                    .sense(Sense::click_and_drag())
                    .selected(selected)
                    .position_in_set(at)
                    .state_layer(false)
                    .focus_ring(false)
                    .interact_at(ui, cell_rect)
                    .finish(ui);
                let pic = Rect::from_min_size(top_left, picture_size);
                let painter = ui.painter();
                if is_bin {
                    paint_bin_face::<K>(ui, pic, &bin_style);
                    if !dragged {
                        bin_rects.push((key.clone(), cell_rect));
                    }
                } else {
                    let texture = thumbnails
                        .map(|(thumbs, source)| (thumbs, thumbs.state(&ctx, source, key, &ThumbRequest::for_cell(side, ppp), Priority::VISIBLE)));
                    let drawn =
                        texture.as_ref().is_some_and(|(thumbs, state)| thumbs.paint(painter, pic, state, &ThumbPaint::default().checker(false)));
                    if !drawn && !matches!(texture, Some((_, TextureState::Empty))) {
                        painter.rect_filled(pic, 0.0, placeholder);
                    }
                    if let Some((from, to)) = item.range {
                        let bar = Rect::from_min_max(
                            pos2(pic.left() + from * pic.width(), pic.bottom() - style.range_bar),
                            pos2(pic.left() + to * pic.width(), pic.bottom()),
                        );
                        painter.rect_filled(bar, 0.0, style.range);
                    }
                }
                if selected {
                    painter.rect_stroke(pic, 0.0, Stroke::new(style.ring_width, style.ring_selected), StrokeKind::Outside);
                    painter.rect_stroke(pic.expand(style.ring_width), 0.0, Stroke::new(style.ring_halo_width, style.ring_halo), StrokeKind::Outside);
                } else if response.hovered() {
                    painter.rect_stroke(pic, 0.0, Stroke::new(style.ring_hover_width, style.ring_hover), StrokeKind::Outside);
                }
                FocusRing::paint_if(ui, pic.expand(style.ring_width + style.ring_halo_width), CornerRadius::ZERO, response.has_focus());
                if let Some(count) = list.stacks[at] {
                    let text = tile_labels.stack.replace("{n}", &count.to_string());
                    let galley = painter.layout_job(LayoutJob::single_section(text, role_format(&tokens, TypeRole::Caption, style.badge_text)));
                    let pill = Rect::from_min_size(
                        pos2(pic.right() - style.inset - galley.size().x - style.inset, pic.top() + style.inset),
                        vec2(galley.size().x + style.inset, style.badge_height.max(galley.size().y)),
                    );
                    painter.rect_filled(pill, CornerRadius::from(pill.height() / 2.0), style.scrim);
                    painter.galley(pill.center() - galley.size() / 2.0, galley, style.badge_text);
                }
                // The caption: the name (shortened in the middle), then the duration or the offline text.
                let offline = item.status != crate::ui::grid::TileStatus::Normal;
                let name_color = if offline {
                    bin_style.offline
                } else if selected {
                    style.caption_selected
                } else {
                    style.caption
                };
                let mut y = pic.bottom() + style.caption_gap;
                let detail = match &item.status {
                    crate::ui::grid::TileStatus::Offline(text) => {
                        Some(text.clone()).filter(|text| !text.is_empty()).or_else(|| Some(tile_labels.offline.clone()))
                    }
                    crate::ui::grid::TileStatus::Unreadable => Some(tile_labels.unreadable.clone()),
                    crate::ui::grid::TileStatus::Failed => Some(tile_labels.failed.clone()),
                    crate::ui::grid::TileStatus::Normal => item.duration.clone().filter(|_| !is_bin),
                };
                for (text, color) in [(Some(item.name.clone()), name_color), (detail, style.detail)] {
                    let Some(text) = text else { continue };
                    let layout =
                        |text: &str| painter.layout_job(LayoutJob::single_section(text.to_owned(), role_format(&tokens, TypeRole::Caption, color)));
                    let shown = fit_name(&text, |candidate| layout(candidate).size().x <= side);
                    let galley = layout(&shown);
                    let height = galley.size().y;
                    painter.galley(pos2(top_left.x + (side - galley.size().x) / 2.0, y), galley, color);
                    y += height;
                }

                if response.has_focus() {
                    focused = Some(at);
                }
                if response.double_clicked() {
                    hits.push((at, Hit::Double(modifiers)));
                } else if response.clicked() {
                    hits.push((at, Hit::Click(modifiers)));
                }
                if response.secondary_clicked() {
                    hits.push((at, Hit::Menu(response.interact_pointer_pos())));
                }
                if response.drag_started_by(PointerButton::Primary) {
                    hits.push((at, Hit::DragStart(modifiers)));
                }
            }
            (background, area)
        });
    let (background, area) = scroll.inner;
    let view = scroll.inner_rect;
    let response = background.clone();

    // Pointer hits, with the selection rules of the other views.
    let rules = ListSelection::new(SelectionMode::Multiple);
    let enter = ctx.input(|i| i.key_pressed(Key::Enter));
    for (at, hit) in hits {
        let key = list.keys[at].clone();
        match hit {
            Hit::Click(_) if enter && focused == Some(at) => {}
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
                let anchor = Rect::from_min_size(area.min + positions[at], cell);
                events.push(BinEvent::ContextMenu(ContextMenuRequest { row: Some(key), selected: was_selected, anchor, pointer, toggle: None }));
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
                    (0..list.keys.len()).filter(|&i| selection.contains(&list.keys[i]) || (stack.is_some() && stack_of[i] == stack)).collect();
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
        ctx.memory_mut(|m| m.request_focus(background.id));
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

    // Keys: Enter opens the focused card, Delete removes the selection, Cmd/Ctrl + / − / 0 zoom.
    let has_focus = focused.is_some() || background.has_focus();
    if has_focus {
        let modifiers = ctx.input(|i| i.modifiers);
        if enter && let Some(at) = focused {
            events.push(open_event(model, list.keys[at].clone(), modifiers));
        }
        if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && !selection.is_empty() {
            events.push(BinEvent::Remove(selection.items.clone()));
        }
    }
    if has_focus || background.hovered() {
        let zoom = TileZoom::Range { min: zoom_range.0, max: zoom_range.1, step: super::footer::ZOOM_STEP, default: default_size };
        let step = ctx.input(|i| {
            if !i.modifiers.command {
                None
            } else if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                Some(crate::ui::grid::ZoomStep::In)
            } else if i.key_pressed(Key::Minus) {
                Some(crate::ui::grid::ZoomStep::Out)
            } else if i.key_pressed(Key::Num0) {
                Some(crate::ui::grid::ZoomStep::Reset)
            } else {
                None
            }
        });
        if let Some(step) = step {
            let modifiers = ctx.input(|i| i.modifiers);
            events.push(BinEvent::Zoom(craft_ui::edit::EditEvent::set(f64::from(zoom.step(side, step)), modifiers)));
        }
    }

    // The drag: a release in the view commits (into a bin card, or the new positions), one outside
    // the view or Esc ends it.
    if let Some(live) = drag.take() {
        let (released, down) = ctx.input(|i| (i.pointer.any_released(), i.pointer.any_down()));
        let has_payload = DragAndDrop::payload::<ListDrag<K>>(&ctx).is_some_and(|p| p.list == id);
        if released || !down {
            if let Some(pointer) = pointer.filter(|p| view.contains(*p)) {
                let moved_by = live.delta(pointer, snap);
                let bin = bin_rects.iter().find(|(key, rect)| rect.contains(pointer) && !inside_any(model, key, &live.keys));
                if let Some((bin, _)) = bin {
                    events.push(BinEvent::Drop { items: live.keys, into: Some(bin.clone()) });
                } else if moved_by != Vec2::ZERO {
                    let to = live.from.iter().map(|from| (*from + moved_by).max(Vec2::ZERO)).collect();
                    events.push(BinEvent::FreeformMoved { keys: live.keys, to });
                }
            }
        } else if has_payload {
            if let Some(snapped) = pointer.map(|p| live.delta(p, snap)).filter(|_| snap.is_some()) {
                let card = Rect::from_min_size(area.min + (live.from[live.grabbed] + snapped).max(Vec2::ZERO), cell);
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
    ctx.data_mut(|d| d.insert_temp(drag_slot, drag));
    (response, events)
}

/// Sets the selection a change gives and reports it.
fn apply<K: Clone>(selection: &mut Selection<K>, change: SelectionChange<K>, events: &mut Vec<BinEvent<K>>) {
    *selection = change.selection.clone();
    events.push(BinEvent::Selection(change));
}
