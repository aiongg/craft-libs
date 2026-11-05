//! One pass of the ruler: gestures, keys and screen reader requests, then drawing.

use craft_ui::behavior::{ClaimedKey, KeyClaim, nudge};
use craft_ui::canvas::{INCH_SIXTEENTHS, RulerStyle, RulerUnits, Rulers, TickRule, Ticks};
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, fmt};
use craft_ui::primitives::{FocusRing, Interactive, Sensed};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::accesskit::{Action, ActionData, Role};
use egui::{CornerRadius, CursorIcon, EventFilter, Id, Key, Modifiers, PointerButton, Rect, Response, Sense, Stroke, Ui, WidgetInfo};

use super::items::{Item, moved, place, without};
use super::paint::{Geometry, Shapes, background, column_grip, column_marker, ticks};
use super::{
    ColumnBorder, RulerValues, SAME_POSITION, TabAlign, TabLeader, TabStop, TextRuler, TextRulerLabels, TextRulerResponse, dropped_off, snap_stop,
};

type Edit = Gesture<RulerValues>;

/// The keys the focused ruler keeps: the arrows move a marker rather than the focus.
const KEY_FILTER: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

/// The keys of the stop list: Page Down and Page Up pick, Delete and Backspace clear (con §9).
const NEXT_KEY: ClaimedKey = ClaimedKey::exact(Modifiers::NONE, Key::PageDown);
const PREVIOUS_KEY: ClaimedKey = ClaimedKey::exact(Modifiers::NONE, Key::PageUp);
const REMOVE_KEY: ClaimedKey = ClaimedKey::exact(Modifiers::NONE, Key::Delete);
const BACK_KEY: ClaimedKey = ClaimedKey::exact(Modifiers::NONE, Key::Backspace);

/// The opacity of a stop dragged off the ruler.
const FADED: f32 = 0.5;

/// The tick rule of a unit: Word's sixteenths for inches, the units' own decimal table otherwise.
fn tick_rule(unit: LengthUnit) -> TickRule<'static> {
    if unit == LengthUnit::Inches { TickRule::Inches(&INCH_SIXTEENTHS) } else { TickRule::Metric }
}

fn ruler_units(ui: &Ui, unit: LengthUnit) -> RulerUnits {
    let per_doc = UnitContext::default().convert(Length { value: 1.0, unit: LengthUnit::Points }, unit).unwrap_or(1.0);
    RulerUnits {
        per_doc,
        ticks: Ticks::Decimal { whole: false },
        numbers: (*LocaleNumbers::of(ui)).clone(),
        unit: Unit::new(&format!("{{n}} {}", unit.suffix())),
        decimals: unit.decimals(),
    }
}

/// The distance a snapping drag moves by: the ruler's smallest tick, in points.
fn snap_step(units: &RulerUnits, rule: TickRule<'_>, style: &RulerStyle, scale: f32) -> f32 {
    let ticks = Rulers::ticks(units, rule, style, f64::from(scale), 0.0..=0.0);
    let gap = ticks.windows(2).find_map(|pair| match pair {
        [a, b] => Some(b.value - a.value),
        _ => None,
    });
    match gap.map(|minor| minor / units.per_doc) {
        Some(points) if points.is_finite() && points > 0.0 => points as f32,
        _ => SAME_POSITION,
    }
}

/// `position` snapped to `step` (unless `free`), kept within `bounds`.
fn snap_within(position: f32, step: f32, bounds: (f32, f32), free: bool) -> f32 {
    let snapped = if free || !step.is_finite() || step <= 0.0 { position } else { (position / step).round() * step };
    snapped.clamp(bounds.0, bounds.1)
}

/// What a drag on the ruler is doing.
#[derive(Clone, Copy, Debug)]
struct Drag {
    /// The item being moved; a stop's index follows it through the sorted stops.
    item: Item,
    /// The stop's index when the drag began (its widget's identity).
    grab: usize,
    /// The pointer is far enough off the ruler to clear the stop on release.
    off: bool,
    /// A new stop being placed, not an existing one moved.
    placing: bool,
}

#[derive(Clone, Debug, Default)]
struct State {
    picked: Option<Item>,
    /// The `selected` the app gave last, to see it change.
    seen: Option<usize>,
    drag: Option<Drag>,
    /// While a table column border is dragged: the borders as they were when it began.
    columns: Option<Vec<f32>>,
}

#[derive(Default)]
struct Outcome {
    selected: Option<usize>,
    open_tabs_dialog: bool,
    column: Option<EditEvent<ColumnBorder>>,
}

struct Pass {
    id: Id,
    g: Geometry,
    style: RulerStyle,
    shapes: Shapes,
    units: RulerUnits,
    rule: TickRule<'static>,
    labels: TextRulerLabels,
    /// The snap step, points.
    step: f32,
    /// The kind of stop a click sets.
    kind: TabAlign,
}

pub(super) fn show(ruler: TextRuler<'_>, ui: &mut Ui, strip: Rect) -> TextRulerResponse {
    let TextRuler { id_salt, values, width, unit, kind, scale, right_to_left, selected, columns, labels } = ruler;
    let tokens = Tokens::of(ui);
    let style = Rulers::style(&tokens);
    let width = if width.is_finite() { width.max(0.0) } else { 0.0 };
    let fit = if width > 0.0 { strip.width() / width } else { 1.0 };
    let scale = scale.filter(|scale| scale.is_finite() && *scale > 0.0).unwrap_or(fit).max(f32::EPSILON);
    let units = ruler_units(ui, unit);
    let pass = Pass {
        id: ui.make_persistent_id(id_salt),
        g: Geometry { strip, scale, right_to_left, width },
        step: snap_step(&units, tick_rule(unit), &style, scale),
        rule: tick_rule(unit),
        shapes: Shapes::of(&tokens),
        kind: kind.as_deref().copied().unwrap_or_default(),
        labels: labels.unwrap_or_else(|| TextRulerLabels::of(ui)),
        units,
        style,
    };
    let mut state: State = ui.data(|d| d.get_temp(pass.id)).unwrap_or_default();
    if selected != state.seen {
        state.picked = selected.map(Item::Stop).or(state.picked);
        state.seen = selected;
    }

    let response = ui.interact(strip, pass.id, Sense::click_and_drag());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(Role::Group, enabled, pass.labels.ruler.as_str()));
    let mut gesture = Gesture::load(ui, pass.id, values, EditPolicy::default()).cancel_on_focus_loss(true).event_filter(KEY_FILTER);
    if !gesture.is_active() {
        state.drag = None;
    }
    let before = gesture.shown().clone();
    let items = pass.register(ui, &state, &before);
    let mut out = Outcome::default();
    for (item, sensed) in &items {
        pass.item(ui, &mut gesture, &mut state, &mut out, *item, sensed.response());
    }
    pass.on_strip(ui, &mut gesture, &mut state, &mut out, &response);
    pass.keys(ui, &mut gesture, &mut state, response.has_focus());
    let borders = pass.column_borders(ui, &mut state, &mut out, columns);
    let shown = gesture.shown().clone();
    pass.paint(ui, &state, &shown, &borders, response.has_focus());
    for (item, sensed) in items {
        pass.report(ui, item, sensed, &before, &shown);
    }
    if let Some(kind) = kind {
        pass.selector(ui, kind);
    }
    ui.data_mut(|d| d.insert_temp(pass.id, state));
    TextRulerResponse { edit: gesture.finish(), selected: out.selected, open_tabs_dialog: out.open_tabs_dialog, column: out.column, response }
}

impl Pass {
    fn name(&self, item: Item, values: &RulerValues) -> &str {
        match item {
            Item::FirstLine => &self.labels.first_line_indent,
            Item::Hanging => &self.labels.hanging_indent,
            Item::LeftBox => &self.labels.left_indent,
            Item::Right => &self.labels.right_indent,
            Item::Stop(at) => self.tab_name(values.stops.get(at).map(|stop| stop.align).unwrap_or_default()),
        }
    }

    fn tab_name(&self, kind: TabAlign) -> &str {
        match kind {
            TabAlign::Left => &self.labels.left_tab,
            TabAlign::Center => &self.labels.center_tab,
            TabAlign::Right => &self.labels.right_tab,
            TabAlign::Decimal => &self.labels.decimal_tab,
            TabAlign::Bar => &self.labels.bar_tab,
        }
    }

    /// The box the item occupies on the strip.
    fn rect(&self, item: Item, values: &RulerValues) -> Rect {
        let x = self.g.x(item.position(values, self.g.width));
        let strip = self.g.strip;
        match item {
            Item::FirstLine => self.shapes.first_line(strip, x),
            Item::Hanging => self.shapes.hanging(strip, x),
            Item::LeftBox => self.shapes.left_box(strip, x),
            Item::Right => self.shapes.right(strip, x),
            Item::Stop(_) => self.shapes.stop(strip, x),
        }
    }

    fn step_or_default(&self) -> f32 {
        if self.step > 0.0 { self.step } else { SAME_POSITION }
    }

    /// The items' widgets, stops first so the markers keep the pointer where they overlap one. A
    /// stop being dragged is the only stop registered, under the id it had when the drag began, so
    /// it keeps the drag while it passes other stops and changes its index.
    fn register(&self, ui: &Ui, state: &State, values: &RulerValues) -> Vec<(Item, Sensed)> {
        let stops: Vec<(Item, usize)> = match state.drag {
            Some(Drag { placing: true, .. }) => Vec::new(),
            Some(Drag { item: item @ Item::Stop(_), grab, .. }) => vec![(item, grab)],
            _ => (0..values.stops.len()).map(|at| (Item::Stop(at), at)).collect(),
        };
        let markers = [Item::LeftBox, Item::Hanging, Item::FirstLine, Item::Right].map(|item| (item, 0));
        stops
            .into_iter()
            .chain(markers)
            .map(|(item, key)| {
                let id = self.id.with((item_key(item), key));
                let sensed = Interactive::with_id(id, Role::Slider, self.name(item, values))
                    .sense(Sense::click_and_drag() - Sense::FOCUSABLE)
                    .state_layer(false)
                    .focus_ring(false)
                    .interact_at(ui, self.rect(item, values));
                (item, sensed)
            })
            .collect()
    }

    fn item(&self, ui: &Ui, gesture: &mut Edit, state: &mut State, out: &mut Outcome, item: Item, r: &Response) {
        if r.hovered() || r.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if r.drag_started_by(PointerButton::Primary) {
            let grab = if let Item::Stop(at) = item { at } else { 0 };
            gesture.begin_with_value();
            state.drag = Some(Drag { item, grab, off: false, placing: false });
            state.picked = Some(item);
            ui.memory_mut(|m| m.request_focus(self.id));
        } else if r.dragged_by(PointerButton::Primary) {
            self.drag_item(ui, gesture, state, r);
        }
        if r.drag_stopped() {
            self.end_drag(ui, gesture, state);
        }
        if r.clicked_by(PointerButton::Primary) {
            state.picked = Some(item);
            ui.memory_mut(|m| m.request_focus(self.id));
            if let Item::Stop(at) = item {
                out.selected = Some(at);
            }
        }
        if r.double_clicked() && matches!(item, Item::Stop(_)) {
            out.open_tabs_dialog = true;
        }
        self.requests(ui, gesture, state, item, r);
    }

    /// The pointer's movement since the press and the modifiers held, for a drag in progress.
    fn pointer(&self, ui: &Ui, r: &Response) -> Option<(egui::Pos2, egui::Pos2, Modifiers)> {
        let pointer = r.interact_pointer_pos()?;
        let origin = ui.input(|i| i.pointer.press_origin())?;
        Some((pointer, origin, ui.input(|i| i.modifiers)))
    }

    fn drag_item(&self, ui: &Ui, gesture: &mut Edit, state: &mut State, r: &Response) {
        let (Some(mut drag), Some(anchor), Some((pointer, origin, modifiers))) = (state.drag, gesture.anchor().cloned(), self.pointer(ui, r)) else {
            return;
        };
        if drag.placing {
            return;
        }
        let from = if let Item::Stop(_) = drag.item { Item::Stop(drag.grab) } else { drag.item };
        let start = from.position(&anchor, self.g.width);
        let target = snap_stop(start + (pointer.x - origin.x) / self.g.scale * self.g.sign(), self.step, self.g.width, modifiers.alt);
        let (value, now) = moved(&anchor, from, target, self.g.width);
        drag.off = matches!(from, Item::Stop(_)) && dropped_off(pointer.y, self.g.strip);
        if !drag.off {
            drag.item = now;
            gesture.update(value, modifiers);
        }
        state.drag = Some(drag);
    }

    /// Release: a stop off the ruler goes (a stop being placed never arrives).
    fn end_drag(&self, ui: &Ui, gesture: &mut Edit, state: &mut State) {
        let Some(drag) = state.drag.take() else { return };
        if drag.off
            && let Some(anchor) = gesture.anchor().cloned()
        {
            let value = if drag.placing { anchor } else { without(&anchor, drag.grab) };
            gesture.update(value, ui.input(|i| i.modifiers));
        }
        gesture.commit();
    }

    /// A press on the empty column places a stop of the selector's kind; a click does too.
    fn on_strip(&self, ui: &Ui, gesture: &mut Edit, state: &mut State, out: &mut Outcome, r: &Response) {
        let column = self.g.column();
        let stop_at = |x: f32, modifiers: Modifiers| TabStop {
            position: snap_stop(self.g.at(x), self.step, self.g.width, modifiers.alt),
            align: self.kind,
            leader: TabLeader::None,
        };
        if r.drag_started_by(PointerButton::Primary) && ui.input(|i| i.pointer.press_origin()).is_some_and(|p| column.contains(p)) {
            gesture.begin_with_value();
            state.drag = Some(Drag { item: Item::Stop(0), grab: 0, off: false, placing: true });
            ui.memory_mut(|m| m.request_focus(self.id));
        }
        if r.dragged_by(PointerButton::Primary)
            && let Some(mut drag) = state.drag.filter(|drag| drag.placing)
            && let Some(anchor) = gesture.anchor().cloned()
            && let Some(pointer) = r.interact_pointer_pos()
        {
            let modifiers = ui.input(|i| i.modifiers);
            let (value, at) = place(&anchor, stop_at(pointer.x, modifiers));
            drag.off = dropped_off(pointer.y, self.g.strip);
            if !drag.off {
                drag.item = Item::Stop(at);
                gesture.update(value, modifiers);
            }
            state.drag = Some(drag);
        }
        if r.drag_stopped() && state.drag.is_some_and(|drag| drag.placing) {
            self.end_drag(ui, gesture, state);
        }
        if r.clicked_by(PointerButton::Primary)
            && let Some(pointer) = r.interact_pointer_pos().filter(|p| column.contains(*p))
        {
            let (value, at) = place(gesture.shown(), stop_at(pointer.x, ui.input(|i| i.modifiers)));
            gesture.set(value);
            state.picked = Some(Item::Stop(at));
            out.selected = Some(at);
        }
    }

    /// Page Down and Page Up pick the next and previous marker or stop; the arrows move the
    /// picked one a snap step (Shift ten); Delete and Backspace clear a picked stop.
    fn keys(&self, ui: &Ui, gesture: &mut Edit, state: &mut State, focused: bool) {
        if !focused {
            return;
        }
        KeyClaim::claim(ui.ctx(), self.id, &[REMOVE_KEY, BACK_KEY]);
        if gesture.is_active() {
            return;
        }
        let (down, up, delete) = ui.input_mut(|i| {
            let removed = REMOVE_KEY.consume(i) | BACK_KEY.consume(i);
            (NEXT_KEY.consume(i), PREVIOUS_KEY.consume(i), removed)
        });
        let current = gesture.shown().clone();
        if down || up {
            state.picked = pick(&Item::all(&current), state.picked, down);
        }
        let Some(item) = state.picked else { return };
        if delete && let Item::Stop(at) = item {
            gesture.set(without(&current, at));
            let left = current.stops.len().saturating_sub(1);
            state.picked = left.checked_sub(1).map(|last| Item::Stop(at.min(last)));
        } else if let Some(delta) = nudge::read(ui, f64::from(self.step_or_default()), 10.0 * f64::from(self.step_or_default())) {
            let to = item.position(&current, self.g.width) + delta.x as f32 * self.g.sign();
            let (value, now) = moved(&current, item, to, self.g.width);
            gesture.set(value);
            state.picked = Some(now);
        }
    }

    /// A screen reader's Increment, Decrement and SetValue on an item.
    fn requests(&self, ui: &Ui, gesture: &mut Edit, state: &mut State, item: Item, r: &Response) {
        let (increments, decrements, value) = ui.input(|i| {
            let count = |action| i.num_accesskit_action_requests(r.id, action);
            let value = i.accesskit_action_requests(r.id, Action::SetValue).filter_map(|request| match request.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v as f32),
                _ => None,
            });
            (count(Action::Increment), count(Action::Decrement), value.last())
        });
        let current = gesture.shown().clone();
        let at = item.position(&current, self.g.width);
        let to = match value {
            Some(value) => value,
            None if increments != decrements => at + self.step_or_default() * (increments as f32 - decrements as f32),
            None => return,
        };
        let (value, now) = moved(&current, item, to, self.g.width);
        gesture.set(value);
        state.picked = Some(now);
    }

    fn paint(&self, ui: &Ui, state: &State, shown: &RulerValues, borders: &[f32], focused: bool) {
        let painter = ui.painter_at(self.g.strip);
        let (style, shapes, g) = (&self.style, &self.shapes, &self.g);
        background(&painter, style, g);
        let items = Item::all(shown);
        let column_boxes: Vec<Rect> = borders.iter().map(|at| column_marker(shapes, g.strip, g.x(*at))).collect();
        let keep_clear: Vec<Rect> = items.iter().map(|item| self.rect(*item, shown)).chain(column_boxes.iter().copied()).collect();
        ticks(&painter, style, g, &self.units, self.rule, &keep_clear);
        let base = g.strip.max.y;
        for (at, stop) in shown.stops.iter().enumerate() {
            let off = state.drag.is_some_and(|drag| drag.off && drag.item == Item::Stop(at));
            let stroke = Stroke::new(style.marker.width, if off { style.marker.color.gamma_multiply(FADED) } else { style.marker.color });
            shapes.glyph(&painter, g.x(stop.position), base, stop.align, g.right_to_left, stroke);
        }
        for marker in &column_boxes {
            column_grip(&painter, style, *marker);
        }
        if state.columns.is_some() && ui.input(|i| i.modifiers.alt) {
            self.widths(&painter, borders);
        }
        let at = |item: Item| g.x(item.position(shown, g.width));
        shapes.indents(&painter, style, g.strip, [at(Item::FirstLine), at(Item::LeftBox), at(Item::Right)]);
        if let Some(item) = state.picked.filter(|item| items.contains(item)) {
            FocusRing::paint_if(ui, self.rect(item, shown), CornerRadius::ZERO, focused);
        }
        if let Some(drag) = state.drag.filter(|drag| !drag.off) {
            self.tip(ui, drag.item, shown);
        }
    }

    fn tip(&self, ui: &Ui, item: Item, shown: &RulerValues) {
        let value = self.units.format_value(f64::from(item.position(shown, self.g.width)), 0.0);
        let text = fmt(&self.labels.tip, &[("name", self.name(item, shown)), ("value", value.as_str())]);
        egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), self.id.with("tip"), egui::PopupAnchor::Pointer).show(|ui| ui.label(text));
    }

    /// The table column borders (WordCraft `table_ruler.rs:columns`): a grip on each, dragged
    /// along the strip. The columns right of a dragged border move with it, so the borders are
    /// returned as the drag leaves them. The drag is one gesture per border.
    fn column_borders(&self, ui: &Ui, state: &mut State, out: &mut Outcome, host: &[f32]) -> Vec<f32> {
        let base = state.columns.clone().filter(|start| start.len() == host.len()).unwrap_or_else(|| host.to_vec());
        let bounds = {
            let (a, b) = (self.g.at(self.g.strip.min.x), self.g.at(self.g.strip.max.x));
            (a.min(b), a.max(b))
        };
        let mut dragged: Option<(usize, f32)> = None;
        for (index, position) in base.iter().copied().enumerate() {
            let id = self.id.with(("column", index));
            let sensed = Interactive::with_id(id, Role::Splitter, self.labels.table_column.as_str())
                .sense(Sense::DRAG)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, column_marker(&self.shapes, self.g.strip, self.g.x(position)));
            let r = sensed.response().clone();
            if r.hovered() || r.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
            }
            let mut gesture =
                Gesture::load(ui, id, &ColumnBorder { index, position }, EditPolicy::default()).cancel_on_focus_loss(true).focus_owners(&[self.id]);
            if r.drag_started_by(PointerButton::Primary) {
                gesture.begin_with_value();
                state.columns = Some(host.to_vec());
                ui.memory_mut(|m| m.request_focus(self.id));
            } else if r.dragged_by(PointerButton::Primary)
                && let Some(anchor) = gesture.anchor().copied()
                && let Some((pointer, origin, modifiers)) = self.pointer(ui, &r)
            {
                let to = anchor.position + (pointer.x - origin.x) / self.g.scale * self.g.sign();
                gesture.update(ColumnBorder { index, position: snap_within(to, self.step, bounds, modifiers.alt) }, modifiers);
            }
            if r.drag_stopped() {
                gesture.commit();
            }
            if let (true, Some(anchor)) = (gesture.is_active(), gesture.anchor()) {
                dragged = Some((index, gesture.shown().position - anchor.position));
            }
            out.column = gesture.finish().or(out.column.take());
            let at = f64::from(position);
            sensed
                .value(self.units.format_value(at, 0.0))
                .numeric(at, f64::from(bounds.0)..=f64::from(bounds.1), f64::from(self.step_or_default()))
                .finish(ui);
            if !r.dragged() {
                r.on_hover_text(self.labels.table_column.as_str());
            }
        }
        if dragged.is_none() {
            state.columns = None;
        }
        let mut shown = base;
        if let Some((index, delta)) = dragged {
            for border in shown.iter_mut().skip(index) {
                *border += delta;
            }
        }
        shown
    }

    /// While Alt is held in a border's drag, each column's width between its borders, over the
    /// strip (WordCraft `table_ruler.rs:widths`).
    fn widths(&self, painter: &egui::Painter, borders: &[f32]) {
        let style = &self.style;
        for pair in borders.windows(2) {
            let [left, right] = pair else { continue };
            let text = self.units.format_value(f64::from((right - left).abs()), 0.0);
            let galley = painter.layout_no_wrap(text, style.font.clone(), style.label);
            let middle = egui::pos2(self.g.x((left + right) / 2.0), self.g.strip.center().y);
            let boxed = Rect::from_center_size(middle, galley.size());
            painter.rect_filled(boxed, 0.0, style.fill);
            painter.galley(boxed.min, galley, style.label);
        }
    }

    /// Reports an item to AccessKit with its value in the unit.
    fn report(&self, ui: &Ui, item: Item, sensed: Sensed, before: &RulerValues, shown: &RulerValues) {
        let at = item.position(shown, self.g.width);
        let range = 0.0..=f64::from(self.g.width);
        let r = sensed.response().clone();
        sensed.value(self.units.format_value(f64::from(at), 0.0)).numeric(f64::from(at), range, f64::from(self.step_or_default())).finish(ui);
        if !r.dragged() {
            r.on_hover_text(self.name(item, before));
        }
    }

    /// The tab selector in the corner square left of the strip: a click picks the next kind, a
    /// right click the previous one.
    fn selector(&self, ui: &Ui, kind: &mut TabAlign) {
        let strip = self.g.strip;
        let corner = Rect::from_min_max(egui::pos2(strip.min.x - strip.height(), strip.min.y), egui::pos2(strip.min.x, strip.max.y));
        let label = fmt(&self.labels.tab_selector, &[("kind", self.tab_name(*kind))]);
        let sensed = Interactive::with_id(self.id.with("selector"), Role::Button, label.as_str())
            .sense(Sense::click())
            .state_layer(false)
            .interact_at(ui, corner);
        let r = sensed.response().clone();
        if r.clicked() {
            *kind = kind.next(false);
        } else if r.secondary_clicked() {
            *kind = kind.next(true);
        }
        let painter = ui.painter_at(corner);
        painter.rect_filled(corner, 0.0, self.style.fill);
        let boxed = Rect::from_center_size(corner.center(), egui::Vec2::splat(self.shapes.selector));
        painter.rect(boxed, 0.0, self.style.fill, self.style.tick, egui::StrokeKind::Inside);
        self.shapes.glyph(
            &painter,
            boxed.center().x,
            boxed.max.y - (boxed.height() - self.shapes.selector * 0.5) * 0.5,
            *kind,
            false,
            self.style.marker,
        );
        sensed.value(self.tab_name(*kind)).finish(ui);
        r.on_hover_text(label);
    }
}

fn item_key(item: Item) -> &'static str {
    match item {
        Item::FirstLine => "first",
        Item::Hanging => "hanging",
        Item::LeftBox => "left",
        Item::Right => "right",
        Item::Stop(_) => "stop",
    }
}

/// The item after (or before) `picked` in `items`, staying at the ends.
fn pick(items: &[Item], picked: Option<Item>, forward: bool) -> Option<Item> {
    let at = picked.and_then(|picked| items.iter().position(|item| *item == picked));
    let to = match (at, forward) {
        (None, true) => 0,
        (None, false) => items.len().saturating_sub(1),
        (Some(at), true) => (at + 1).min(items.len().saturating_sub(1)),
        (Some(at), false) => at.saturating_sub(1),
    };
    items.get(to).copied()
}
