//! Keyframe lanes, the keyframe navigator and the animation toggle (N1.04).

use craft_ui::behavior::{ClaimedKey, KeyClaim, ListSelection, RowInfo, Selection, SelectionChange, SelectionMode, SelectionPick, Step};
use craft_ui::controls::IconButton;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse, Gesture};
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Corner, FocusRing, Interactive};
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _, scale};
use egui::accesskit::Role;
use egui::{
    AsIdSalt, Color32, EventFilter, Id, IdSalt, Key, Modifiers, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, WidgetInfo,
    pos2, vec2,
};

use super::style::TimelineStyle;
use crate::rates::FrameRate;
use crate::time::{TICKS_PER_SECOND, Tick, TimeRange, TimeView};
use crate::timecode::{TimeFormat, format_time};

craft_ui::words! {
    /// The keyframe controls' tooltips and names, English by default (FilmCraft's).
    #[non_exhaustive]
    pub struct KeyframeLabels {
        previous: "keyframes.previous" = "Go to previous keyframe";
        next: "keyframes.next" = "Go to next keyframe";
        add_remove: "keyframes.addRemove" = "Add/Remove Keyframe";
        toggle_animation: "keyframes.toggleAnimation" = "Toggle animation";
        lane: "keyframes.lane" = "Keyframes";
    }
}

/// A keyframe's id, the app's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyframeId(pub u64);

/// How the segment leaving a keyframe interpolates (Premiere's temporal set; After Effects'
/// Linear, Bezier and Hold are a subset). Also picks the glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyframeInterpolation {
    #[default]
    Linear,
    Hold,
    Bezier,
    AutoBezier,
    ContinuousBezier,
    EaseIn,
    EaseOut,
}

/// A keyframe on a lane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyframeMark {
    pub id: KeyframeId,
    pub time: Tick,
    pub interpolation: KeyframeInterpolation,
}

/// Draws a keyframe glyph centred at `center` (Premiere's glyphs): a `style.keyframe_hold`
/// square for Hold, the diamond filling the `style.keyframe_glyph` box for Linear, a circle of
/// radius `style.keyframe_bezier` for the Bezier kinds ([`TimelineStyle`](super::TimelineStyle)).
// filmcraft: crates/ui-egui/src/panels/effect_controls_selection.rs:256 `paint`.
pub fn paint_keyframe(painter: &Painter, center: Pos2, interpolation: KeyframeInterpolation, color: Color32, style: &super::TimelineStyle) {
    match interpolation {
        KeyframeInterpolation::Hold => {
            painter.rect_filled(Rect::from_center_size(center, vec2(style.keyframe_hold, style.keyframe_hold)), 0.0, color);
        }
        KeyframeInterpolation::Linear => {
            let half = style.keyframe_glyph / 2.0;
            let points = vec![center + vec2(0.0, -half), center + vec2(half, 0.0), center + vec2(0.0, half), center + vec2(-half, 0.0)];
            painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
        }
        _ => {
            painter.circle_filled(center, style.keyframe_bezier, color);
        }
    }
}

/// The keyframes of one property row on its lane.
///
/// - The selection is the app's [`Selection`] of keys in time order; clicks and keys change it
///   with the kernel's [`ListSelection`](craft_ui::behavior::ListSelection) rules over that
///   order, with Photoshop's and Premiere's picks: a click replaces, Shift+click and
///   Cmd/Ctrl+click toggle the key; a drag on empty lane draws a marquee (Shift adds to the
///   selection). The lane applies the change to the `&mut Selection` in the pass and reports it
///   as a [`SelectionChange`].
/// - Dragging a selected key moves every selected key by the same delta, snapped to frames of
///   `rate` and kept inside `bounds` (the clip); with Shift it snaps to the playhead and the
///   other keys within `style.keyframe_snap`. Reported as an `EditEvent<Tick>` (the delta)
///   gesture; Esc cancels. The lane draws the selected keys moved by the gesture's delta, so the
///   app applies the delta at the commit and keeps passing the keys as they were.
/// - A double-click and a secondary click on a key are reported (After Effects opens the
///   keyframe's dialog and menu, `timeline.rs:4083`, `:4101`).
/// - Keyboard: one Tab stop when it has keys; Left and Right move the selection to the previous
///   and next key, Shift+Left/Right nudge the selected keys one frame (a `Set` on
///   [`KeyframeLaneResponse::moved`]), Delete and Backspace report [`KeyframeLaneResponse::delete`] (claimed
///   through [`KeyClaim`](craft_ui::behavior::KeyClaim) so the app's Delete shortcut leaves it).
///   AccessKit: a `ListBox` named by [`KeyframeLabels::lane`], one `ListBoxOption` per key named
///   by its time as timecode of the lane's rate, `selected` set.
pub struct KeyframeLane<'a> {
    id_salt: IdSalt,
    keys: &'a [KeyframeMark],
    selection: &'a mut Selection<KeyframeId>,
    view: &'a TimeView,
    rate: FrameRate,
    playhead: Tick,
    bounds: Option<TimeRange>,
    labels: Option<KeyframeLabels>,
}

impl<'a> KeyframeLane<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        keys: &'a [KeyframeMark],
        selection: &'a mut Selection<KeyframeId>,
        view: &'a TimeView,
        rate: FrameRate,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), keys, selection, view, rate, playhead: Tick::ZERO, bounds: None, labels: None }
    }

    pub fn playhead(mut self, t: Tick) -> Self {
        self.playhead = t;
        self
    }

    /// Keys stay inside this range while dragged (the clip).
    pub fn bounds(mut self, range: TimeRange) -> Self {
        self.bounds = Some(range);
        self
    }

    pub fn labels(mut self, labels: KeyframeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/effect_controls_selection.rs:81 `update` (snapping),
    // `:110` `Interaction::begin`; effectcraft: crates/ui-egui/src/panels/timeline.rs:4059
    // `key_select`, `:4191` `key_drag_begin`, `:4246` `key_hit`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> KeyframeLaneResponse {
        let Self { id_salt, keys, selection, view, rate, playhead, bounds, labels } = self;
        let labels = labels.unwrap_or_else(|| KeyframeLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let sense = if keys.is_empty() { Sense::CLICK | Sense::DRAG } else { Sense::click_and_drag() };
        let response = ui.interact(rect, id, sense);
        response.widget_info(|| WidgetInfo::labeled(Role::ListBox, enabled, labels.lane.as_str()));

        let mut sorted: Vec<&KeyframeMark> = keys.iter().collect();
        sorted.sort_by_key(|key| key.time);
        let order: Vec<RowInfo<KeyframeId>> = sorted.iter().map(|key| RowInfo::leaf(key.id)).collect();
        let moves = Moves {
            keys,
            rate,
            playhead,
            bounds,
            points_per_tick: view.points_per_second / TICKS_PER_SECOND as f64,
            snap: f64::from(style.keyframe_snap),
        };
        let center_y = rect.center().y;
        let half = style.keyframe_glyph / 2.0;
        let center_of = |time: Tick| pos2(view.x_of(time, rect.left()), center_y);
        let hit = |pos: Pos2| {
            sorted
                .iter()
                .filter(|key| {
                    let center = center_of(key.time);
                    (pos.x - center.x).abs() <= half && (pos.y - center.y).abs() <= half
                })
                .min_by(|a, b| (center_of(a.time).x - pos.x).abs().total_cmp(&(center_of(b.time).x - pos.x).abs()))
                .map(|key| key.id)
        };

        let drag_key = id.with("drag");
        let marquee_key = id.with("marquee");
        let reported = Gesture::<Tick>::reported(ui, id).map_or(Tick::ZERO, |(_, delta)| delta);
        let mut gesture = Gesture::load(ui, id, &reported, EditPolicy::default()).enabled(enabled);
        if !gesture.is_active() {
            ui.data_mut(|d| d.remove::<Drag>(drag_key));
        }
        let modifiers = ui.input(|i| i.modifiers);
        let mut out = Interaction::default();

        if enabled {
            if response.drag_started_by(PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            {
                response.request_focus();
                let additive = modifiers.shift || modifiers.command;
                match hit(origin).and_then(|key| keys.iter().find(|k| k.id == key)) {
                    Some(grabbed) => {
                        let mut group = selection.items.clone();
                        if !selection.contains(&grabbed.id) {
                            let mut next = if additive { Selection { items: group, ..Selection::new() } } else { Selection::new() };
                            next.items.push(grabbed.id);
                            next.lead = Some(grabbed.id);
                            next.anchor = Some(grabbed.id);
                            group = next.items.clone();
                            let pick = if additive { SelectionPick::Toggle } else { SelectionPick::Replace };
                            out.change = Some(SelectionChange { pick, row: Some(grabbed.id), selection: next });
                        }
                        ui.data_mut(|d| d.insert_temp(drag_key, Drag { group, press_x: origin.x, anchor: grabbed.time }));
                        gesture.begin();
                    }
                    None => {
                        let base = if additive { selection.clone() } else { Selection::new() };
                        ui.data_mut(|d| d.insert_temp(marquee_key, Marquee { origin, base }));
                    }
                }
            }
            if let Some(drag) = ui.data(|d| d.get_temp::<Drag>(drag_key))
                && response.dragged_by(PointerButton::Primary)
                && gesture.is_active()
                && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            {
                gesture.update(moves.delta(&drag, pos.x, modifiers.shift, view), modifiers);
            }
            if response.drag_stopped() {
                gesture.commit();
                ui.data_mut(|d| d.remove::<Marquee>(marquee_key));
            }
            if let Some(marquee) = ui.data(|d| d.get_temp::<Marquee>(marquee_key)) {
                let cancelled = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                if cancelled {
                    ui.data_mut(|d| d.remove::<Marquee>(marquee_key));
                }
                let pointer = ui.input(|i| i.pointer.interact_pos());
                let next = if cancelled {
                    Some(marquee.base.clone())
                } else {
                    pointer.map(|pos| marquee.extended(Rect::from_two_pos(marquee.origin, pos).intersect(rect), &sorted, center_of))
                };
                if let Some(next) = next.filter(|next| *next != *selection) {
                    out.change = Some(SelectionChange {
                        pick: if next.is_empty() { SelectionPick::Clear } else { SelectionPick::Replace },
                        row: next.lead,
                        selection: next,
                    });
                }
            }
            out.pointer(&response, &hit, &order, selection, modifiers);
            if response.has_focus() && !keys.is_empty() {
                out.keyboard(ui, id, &order, selection, &moves, &mut gesture);
            }
        }

        if let Some(change) = &out.change {
            *selection = change.selection.clone();
        }
        let shown = *gesture.shown();
        let group = ui.data(|d| d.get_temp::<Drag>(drag_key)).map(|drag| drag.group).unwrap_or_default();
        let painter = ui.painter_at(rect);
        let colors = &tokens.color;
        for key in &sorted {
            let on = selection.contains(&key.id);
            let center = center_of(if on && group.contains(&key.id) { key.time + shown } else { key.time });
            if !rect.expand(half).contains(center) {
                continue;
            }
            let color = match (enabled, on) {
                (false, _) => colors.text.disabled,
                (true, true) => colors.accent.text,
                (true, false) => colors.text.muted,
            };
            paint_keyframe(&painter, center, key.interpolation, color.to_egui(), &style);
            let name = format_time(key.time, &TimeFormat::timecode(rate));
            let box_rect = Rect::from_center_size(center, vec2(style.keyframe_glyph, style.keyframe_glyph));
            Interactive::with_id(id.with(("key", key.id.0)), Role::ListBoxOption, name)
                .sense(Sense::hover())
                .selected(on)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, box_rect)
                .finish(ui);
        }
        if let Some(marquee) = ui.data(|d| d.get_temp::<Marquee>(marquee_key))
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            let area = Rect::from_two_pos(marquee.origin, pos).intersect(rect);
            painter.rect_filled(area, 0.0, colors.accent.soft.to_egui());
            painter.rect_stroke(area, 0.0, scale::Stroke::Hairline.egui(ui.ctx(), colors.accent.border.to_egui()), egui::StrokeKind::Inside);
        }
        let radius = Corner::Control.radius(&tokens);
        let lead = selection.lead.and_then(|lead| sorted.iter().find(|key| key.id == lead));
        let ring = lead.map_or(rect, |lead| Rect::from_center_size(center_of(lead.time), vec2(style.keyframe_glyph, style.keyframe_glyph)));
        FocusRing::paint_radius(ui, ring, radius, &response);

        KeyframeLaneResponse {
            response,
            selection: out.change,
            moved: gesture.finish(),
            double_clicked: out.double_clicked,
            context_menu: out.context_menu,
            delete: out.delete,
        }
    }
}

/// The keys that delete the selected keyframes: Delete, and Backspace where the Delete key sends
/// it (Premiere on a Mac keyboard).
const DELETE: [ClaimedKey; 2] = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];

/// What a pass of input did, besides the key move.
#[derive(Default)]
struct Interaction {
    change: Option<SelectionChange<KeyframeId>>,
    double_clicked: Option<KeyframeId>,
    context_menu: Option<KeyframeId>,
    delete: bool,
}

impl Interaction {
    /// Clicks, double clicks and secondary clicks on the lane.
    fn pointer(
        &mut self,
        response: &Response,
        hit: &impl Fn(Pos2) -> Option<KeyframeId>,
        order: &[RowInfo<KeyframeId>],
        selection: &Selection<KeyframeId>,
        modifiers: Modifiers,
    ) {
        let rules = ListSelection::new(SelectionMode::Multiple);
        let at = response.interact_pointer_pos().and_then(hit).and_then(|key| order.iter().position(|row| row.key == key));
        if response.clicked_by(PointerButton::Primary) {
            response.request_focus();
            // Shift toggles a key as Cmd/Ctrl does; a range has no meaning on a lane.
            self.change = match at {
                Some(at) => rules.click(selection, order, at, Modifiers { command: modifiers.command || modifiers.shift, ..Modifiers::NONE }),
                None if modifiers.shift || modifiers.command => None,
                None => rules.click_empty(selection),
            };
        }
        let key = at.map(|at| order[at].key);
        if response.double_clicked_by(PointerButton::Primary) {
            self.double_clicked = key;
        }
        if response.secondary_clicked() {
            self.context_menu = key;
        }
    }

    /// Left and Right select the previous and next key, Shift+Left and Right nudge the selected
    /// keys a frame, Delete asks to delete them.
    fn keyboard(
        &mut self,
        ui: &Ui,
        id: Id,
        order: &[RowInfo<KeyframeId>],
        selection: &Selection<KeyframeId>,
        moves: &Moves<'_>,
        gesture: &mut Gesture<Tick>,
    ) {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, ..EventFilter::default() }));
        KeyClaim::claim(ui.ctx(), id, &DELETE);
        let rules = ListSelection::new(SelectionMode::Multiple);
        let selected: Vec<KeyframeId> = moves.keys.iter().filter(|key| selection.contains(&key.id)).map(|key| key.id).collect();
        for (key, step, toward) in [(Key::ArrowLeft, Step::Previous, -1), (Key::ArrowRight, Step::Next, 1)] {
            if ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, key)) {
                let (low, high) = moves.limits(&selected);
                let delta = Tick(moves.rate.frame_duration().0 * toward);
                if !selected.is_empty() && (low..=high).contains(&delta.0) {
                    gesture.set(delta);
                }
            } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.change = rules.select(selection, order, step, 1, false);
            }
        }
        // Both keys are consumed even when both were pressed.
        let pressed = DELETE.map(|key| ui.input_mut(|i| key.consume(i))).contains(&true);
        self.delete = pressed && !selection.is_empty();
    }
}

/// The keys of a lane and how a drag moves them.
struct Moves<'a> {
    keys: &'a [KeyframeMark],
    rate: FrameRate,
    playhead: Tick,
    bounds: Option<TimeRange>,
    points_per_tick: f64,
    /// How near the playhead or another key a Shift-dragged key snaps, in points.
    snap: f64,
}

impl Moves<'_> {
    /// The range of deltas that keeps all of `group` inside the bounds.
    fn limits(&self, group: &[KeyframeId]) -> (i64, i64) {
        let (mut low, mut high) = (i64::MIN, i64::MAX);
        if let Some(bounds) = self.bounds {
            for key in self.keys.iter().filter(|key| group.contains(&key.id)) {
                low = low.max((bounds.start - key.time).0);
                high = high.min((bounds.end() - key.time).0);
            }
        }
        if low > high { (0, 0) } else { (low, high) }
    }

    /// The delta of a drag that has reached screen `x`: on the frame grid and inside the bounds;
    /// with `snapping` on the playhead or an unmoved key when one is near enough.
    fn delta(&self, drag: &Drag, x: f32, snapping: bool, view: &TimeView) -> Tick {
        let (low, high) = self.limits(&drag.group);
        let raw = Tick((f64::from(x - drag.press_x) * TICKS_PER_SECOND as f64 / view.points_per_second).round() as i64);
        let requested = self.rate.snap_nearest(drag.anchor + raw) - drag.anchor;
        let mut delta = Tick(requested.0.clamp(low, high));
        if snapping {
            let mut closest = self.snap;
            let targets: Vec<Tick> =
                std::iter::once(self.playhead).chain(self.keys.iter().filter(|k| !drag.group.contains(&k.id)).map(|k| k.time)).collect();
            for moved in self.keys.iter().filter(|k| drag.group.contains(&k.id)) {
                for target in &targets {
                    let snap = (*target - moved.time).0;
                    let distance = ((snap - requested.0) as f64 * self.points_per_tick).abs();
                    if distance <= closest && (low..=high).contains(&snap) {
                        closest = distance;
                        delta = Tick(snap);
                    }
                }
            }
        }
        delta
    }
}

/// A key drag under way: the keys it moves and the press it measures from.
#[derive(Clone)]
struct Drag {
    group: Vec<KeyframeId>,
    press_x: f32,
    /// The time of the key that was grabbed.
    anchor: Tick,
}

/// A marquee under way: where it began and the selection it adds to.
#[derive(Clone)]
struct Marquee {
    origin: Pos2,
    base: Selection<KeyframeId>,
}

impl Marquee {
    /// The base selection with the keys whose centres are inside `area`.
    fn extended(&self, area: Rect, keys: &[&KeyframeMark], center_of: impl Fn(Tick) -> Pos2) -> Selection<KeyframeId> {
        let mut next = self.base.clone();
        for key in keys.iter().filter(|key| area.contains(center_of(key.time))) {
            if !next.contains(&key.id) {
                next.items.push(key.id);
            }
            next.lead = Some(key.id);
            next.anchor = Some(key.id);
        }
        next
    }
}

/// What a keyframe lane did this pass.
pub struct KeyframeLaneResponse {
    pub response: Response,
    /// A click, marquee or key changed the selection.
    pub selection: Option<SelectionChange<KeyframeId>>,
    /// The selected keys' move, as a delta.
    pub moved: Option<EditEvent<Tick>>,
    pub double_clicked: Option<KeyframeId>,
    pub context_menu: Option<KeyframeId>,
    /// Delete or Backspace was pressed with keys selected.
    pub delete: bool,
}

/// Premiere's keyframe navigator at a property row: ◀ (previous key), ◆ (add a key at the
/// playhead, or remove the one there), ▶ (next key). The arrows are
/// `style.keyframe_arrow_width` × `style.keyframe_arrow_height`, `style.keyframe_nav_step`
/// either side of the diamond, dimmed without a key that way; the diamond is filled when a key is
/// at the playhead, else an outline. Three Tab stops (buttons named by [`KeyframeLabels`]),
/// Space or Enter presses.
pub struct KeyframeNavigator {
    id_salt: IdSalt,
    animated: bool,
    has_previous: bool,
    has_next: bool,
    at_key: bool,
    labels: Option<KeyframeLabels>,
}

impl KeyframeNavigator {
    /// `has_previous` / `has_next`: a key before / after the playhead; `at_key`: one at it.
    pub fn new(id_salt: impl AsIdSalt, has_previous: bool, has_next: bool, at_key: bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), animated: true, has_previous, has_next, at_key, labels: None }
    }

    /// Without animation only the diamond shows (it turns animation on).
    pub fn animated(mut self, on: bool) -> Self {
        self.animated = on;
        self
    }

    pub fn labels(mut self, labels: KeyframeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/effect_controls.rs:487 `keyframe_nav`.
    pub fn show(self, ui: &mut Ui) -> KeyframeNavigatorOutput {
        let Self { id_salt, animated, has_previous, has_next, at_key, labels } = self;
        let labels = labels.unwrap_or_else(|| KeyframeLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let colors = &tokens.color;
        let id = ui.make_persistent_id(id_salt);
        let width = if animated { 2.0 * style.keyframe_nav_step + style.keyframe_arrow_width } else { style.keyframe_glyph };
        let height = style.keyframe_arrow_height.max(style.keyframe_glyph);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let center = rect.center();
        let mut out = KeyframeNavigatorOutput::default();

        let arrow_size = vec2(style.keyframe_arrow_width, style.keyframe_arrow_height);
        let press = |ui: &mut Ui, name: &'static str, label: &str, area: Rect, active: bool| {
            let sensed = Interactive::with_id(id.with(name), Role::Button, label).sense(Sense::click()).interact_at(ui, area);
            let clicked = sensed.response().clicked();
            let response = sensed.finish(ui);
            (response, clicked && active)
        };
        if animated {
            for (name, label, direction, active) in [("previous", &labels.previous, -1.0_f32, has_previous), ("next", &labels.next, 1.0, has_next)] {
                let area = Rect::from_center_size(center + vec2(direction * style.keyframe_nav_step, 0.0), arrow_size);
                let (response, clicked) = press(ui, name, label, area, active);
                let c = area.center();
                let (reach, span) = (arrow_size.x / 4.0, arrow_size.y * 0.275);
                let points = vec![c + vec2(-reach * direction, -span), c + vec2(-reach * direction, span), c + vec2(reach * direction, 0.0)];
                let color = if active { colors.text.normal } else { colors.text.faint };
                ui.painter().add(Shape::convex_polygon(points, color.to_egui(), Stroke::NONE));
                FocusRing::paint(ui, area, Corner::Control, &response);
                if direction < 0.0 {
                    out.previous = clicked;
                } else {
                    out.next = clicked;
                }
            }
        }
        let area = Rect::from_center_size(center, vec2(style.keyframe_glyph, style.keyframe_glyph));
        let (response, clicked) = press(ui, "toggle", &labels.add_remove, area, true);
        out.toggle = clicked;
        if at_key {
            paint_keyframe(ui.painter(), center, KeyframeInterpolation::Linear, colors.accent.text.to_egui(), &style);
        } else {
            let half = style.keyframe_glyph * 0.31;
            let outline = vec![center + vec2(0.0, -half), center + vec2(half, 0.0), center + vec2(0.0, half), center + vec2(-half, 0.0)];
            let color = if response.hovered() { colors.text.normal } else { colors.text.muted };
            ui.painter().add(Shape::closed_line(outline, scale::Stroke::Hairline.egui(ui.ctx(), color.to_egui())));
        }
        FocusRing::paint(ui, area, Corner::Control, &response);
        out
    }
}

/// Which navigator button was pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct KeyframeNavigatorOutput {
    pub previous: bool,
    pub next: bool,
    /// The diamond: add a key at the playhead, or remove the one there.
    pub toggle: bool,
}

/// The stopwatch that turns a property's animation on and off (Premiere's Toggle animation,
/// After Effects' Time-Vary Stopwatch). A toggle button: one Tab stop, Space toggles; reports
/// the new state as a `Set`.
pub struct AnimationToggle<'a> {
    id_salt: IdSalt,
    on: &'a bool,
    labels: Option<KeyframeLabels>,
}

impl<'a> AnimationToggle<'a> {
    pub fn new(id_salt: impl AsIdSalt, on: &'a bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), on, labels: None }
    }

    pub fn labels(mut self, labels: KeyframeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // effectcraft: crates/ui-egui/src/panels/timeline.rs:3922 `prop_stopwatch`.
    pub fn show(self, ui: &mut Ui) -> craft_ui::edit::EditResponse<bool> {
        let Self { id_salt, on, labels } = self;
        let labels = labels.unwrap_or_else(|| KeyframeLabels::of(ui));
        let style = TimelineStyle::of(&Tokens::of(ui));
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let mut gesture = Gesture::load(ui, id, on, EditPolicy::default()).enabled(enabled);
        let button = IconButton::new(Icon::Clock, labels.toggle_animation.as_str())
            .with_id(id)
            .side(style.keyframe_glyph)
            .icon_size(style.keyframe_glyph)
            .selected(*gesture.shown());
        let response = ui.add(button);
        if response.clicked() {
            gesture.set(!*gesture.shown());
        }
        EditResponse::new(response, gesture.finish())
    }
}
