//! [`BlendIfBar`]: one Blend If slider (This Layer or Underlying Layer) with its four handles
//!.

use egui::accesskit::{Action, ActionData, Role};
use egui::{AsIdSalt, Color32, EventFilter, Id, IdSalt, Key, Modifiers, Pos2, Rect, Sense, Shape, Stroke, Ui, vec2};

use craft_ui::direction;
use craft_ui::edit::{EditPolicy, EditResponse, Gesture};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Tokens, TokensOf as _};

use super::labels::BlendIfBarLabels;
use super::style::BlendIfBarStyle;
use crate::layer_style::BlendIfRange;

/// How many vertical strips draw the ramp (PhotoCraft's resolution of the grey scale).
const STRIPS: usize = 32;

/// The face of the black handles: grey 20, an image tone (not a token).
// pc: crates/ui-egui/src/dialogs/layer_style.rs:1442.
pub(crate) const BLACK_FACE: Color32 = Color32::from_gray(20);
/// The face of the white handles: grey 240, an image tone (not a token).
// pc: crates/ui-egui/src/dialogs/layer_style.rs:1443.
pub(crate) const WHITE_FACE: Color32 = Color32::from_gray(240);

/// A Blend If slider: a black-to-white ramp (`blend_if.ramp` high, `blend_if.inset` in from each
/// side, the bar's width the available width clamped to `blend_if.width_min..=width_max`,
/// 120..=320; a 32-strip grey ramp with a `blend_if.ramp_border` outline) with a black and a white
/// handle under it, each split into two halves by Alt (the layer fades between them). A split
/// pair shows two half triangles; an unsplit pair one whole triangle (`blend_if.handle_width` ×
/// `blend_if.handle_height`, faces grey 20 and grey 240, edges `text.muted`). The
/// value readout ("0 / 20    255": a split side as "low / high") sits above the bar at the
/// trailing end. The ramp never mirrors in a right-to-left interface: it is a tone scale.
///
/// - Pointer: a press picks the nearest handle; between two coinciding halves, the one on the
///   pointer's side. A drag moves it by [`BlendIfRange::moved`]: alone with Alt held, else the
///   pair keeping its spread; the order black low ≤ black high ≤ white low ≤ white high is kept by
///   pushing the others. One gesture per drag (`Begin`, `Update`…, `Commit`; Esc cancels).
/// - Keys (Photoshop has none): the four handles are Tab stops in order. ← and →
///   move the focused handle's pair one level, ⇧ ten levels; ⌥←, ⌥→ move only the focused half.
///   Each press is one `Set`.
/// - AccessKit: each handle is a `Slider` 0..=255 named
///   [`BlendIfBarLabels::handle`] ("Black low, This Layer"), its value the level.
#[must_use = "show the bar with `show`"]
pub struct BlendIfBar {
    id_salt: IdSalt,
    layer: String,
    range: BlendIfRange,
    labels: Option<BlendIfBarLabels>,
    numbers: Option<LocaleNumbers>,
    policy: EditPolicy,
    enabled: bool,
}

impl BlendIfBar {
    /// A bar named `layer` ("This Layer", "Underlying Layer") over `range`.
    pub fn new(id_salt: impl AsIdSalt, layer: impl Into<String>, range: BlendIfRange) -> Self {
        BlendIfBar {
            id_salt: IdSalt::new(id_salt),
            layer: layer.into(),
            range,
            labels: None,
            numbers: None,
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// The handles' accessible names; the installed words by default.
    pub fn labels(mut self, labels: BlendIfBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// How the levels are written (readout and accessible values); the installed region's by
    /// default.
    pub fn numbers(mut self, numbers: &LocaleNumbers) -> Self {
        self.numbers = Some(numbers.clone());
        self
    }

    /// How a drag reports; realtime by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Turns editing off with `false`. The editor is also off while `ui` is disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the readout and the bar and senses the handles.
    pub fn show(self, ui: &mut Ui) -> EditResponse<BlendIfRange> {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:1333-1344 (readout above the bar),
        // :1368-1454 `blend_if_bar` (ramp, nearest handle, drag rules, triangles).
        // new: keys, Tab stops and AccessKit; look from `BlendIfBar::style`.
        let Self { id_salt, layer, range, labels, numbers, policy, enabled } = self;
        let tokens = Tokens::of(ui);
        let style = BlendIfBar::style(&tokens);
        let labels = labels.unwrap_or_else(|| BlendIfBarLabels::of(ui));
        let numbers = numbers.unwrap_or_else(|| (*LocaleNumbers::of(ui)).clone());
        let id = ui.make_persistent_id(id_salt);
        let enabled = enabled && ui.is_enabled();
        let mut gesture = Gesture::load(ui, id, &range, policy).enabled(enabled);

        let width = ui.available_width().clamp(style.width_min, style.width_max);
        // The readout's line is as high as its text; the text itself is painted once the drag of
        // this pass has moved the handles.
        let line = ui.painter().layout_no_wrap(readout(&range, &numbers, &labels), style.readout_font.clone(), style.readout_color).size().y;
        let (readout_rect, _) = ui.allocate_exact_size(vec2(width, line), Sense::hover());
        let (rect, _) = ui.allocate_exact_size(vec2(width, style.height), Sense::hover());
        let response = ui.interact(rect, id, Sense::CLICK | Sense::DRAG);
        let ramp = Rect::from_min_size(rect.min + vec2(style.inset, 0.0), vec2(width - 2.0 * style.inset, style.ramp));
        let handle_id = |i: usize| id.with(("handle", i));

        if enabled {
            let focused = (0..4).find(|&i| ui.memory(|m| m.has_focus(handle_id(i))));
            if let Some(h) = focused {
                keys(ui, &mut gesture, h, handle_id(h));
            }
            pointer(ui, &response, &mut gesture, ramp, id, &handle_id);
        }
        let value = *gesture.shown();

        paint_readout(ui, &style, readout_rect, readout(&value, &numbers, &labels));
        paint_ramp(ui, &style, ramp);
        paint_handles(ui, &style, &labels, &numbers, &layer, ramp, value, &handle_id, enabled);

        let event = gesture.finish();
        EditResponse::new(response, event)
    }
}

/// "0 / 20    255": a split side as "low / high", an unsplit one as its level, in the region's digits
/// and the words' layout.
fn readout(range: &BlendIfRange, numbers: &LocaleNumbers, labels: &BlendIfBarLabels) -> String {
    let level = |level: u8| numbers.format(f64::from(level), 0..=0);
    let side = |low: u8, high: u8| {
        if low == high { level(low) } else { craft_i18n::fmt(&labels.split_levels, &[("low", level(low).as_str()), ("high", level(high).as_str())]) }
    };
    let [a, b, c, d] = range.0;
    craft_i18n::fmt(&labels.readout, &[("black", side(a, b).as_str()), ("white", side(c, d).as_str())])
}

fn paint_readout(ui: &Ui, style: &BlendIfBarStyle, rect: Rect, text: String) {
    let align = if direction::is_rtl(ui) { egui::Align2::LEFT_TOP } else { egui::Align2::RIGHT_TOP };
    let at = if direction::is_rtl(ui) { rect.left_top() } else { rect.right_top() };
    ui.painter().text(at, align, text, style.readout_font.clone(), style.readout_color);
}

/// The black-to-white ramp as vertical strips with an outline. It never mirrors: it is a tone
/// scale.
fn paint_ramp(ui: &Ui, style: &BlendIfBarStyle, ramp: Rect) {
    for strip in 0..STRIPS {
        let (x0, x1) = (ramp.left() + ramp.width() * strip as f32 / STRIPS as f32, ramp.left() + ramp.width() * (strip + 1) as f32 / STRIPS as f32);
        let grey = (255.0 * (strip as f32 + 0.5) / STRIPS as f32) as u8;
        ui.painter().rect_filled(Rect::from_x_y_ranges(x0..=x1, ramp.y_range()), 0.0, Color32::from_gray(grey));
    }
    ui.painter().rect_stroke(ramp, 0.0, Stroke::new(craft_theme::Stroke::W1.width(1.0), style.ramp_border), egui::StrokeKind::Outside);
}

fn x_of(ramp: Rect, level: u8) -> f32 {
    ramp.left() + ramp.width() * f32::from(level) / 255.0
}

fn level_at(ramp: Rect, x: f32) -> u8 {
    (((x - ramp.left()) / ramp.width().max(1.0)) * 255.0).round().clamp(0.0, 255.0) as u8
}

/// The handle a press at `x` takes: the nearest; between two coinciding halves, the one on the
/// pointer's side.
fn nearest(ramp: Rect, range: BlendIfRange, x: f32) -> usize {
    let q = range.0;
    let near = (0..4).min_by(|a, b| (x_of(ramp, q[*a]) - x).abs().total_cmp(&(x_of(ramp, q[*b]) - x).abs())).unwrap_or(0);
    match near {
        0 | 1 if q[0] == q[1] => usize::from(x > x_of(ramp, q[0])),
        2 | 3 if q[2] == q[3] => 2 + usize::from(x > x_of(ramp, q[2])),
        n => n,
    }
}

/// Takes the presses of `key` with exactly `modifiers` held and returns how many there were.
fn take_exact(ui: &Ui, modifiers: Modifiers, key: Key) -> usize {
    ui.input_mut(|i| {
        let before = i.events.len();
        i.events.retain(
            |event| !matches!(event, egui::Event::Key { key: k, pressed: true, modifiers: m, .. } if *k == key && m.matches_exact(modifiers)),
        );
        before - i.events.len()
    })
}

/// ← and → move the focused handle's pair one level, ⇧ ten, ⌥ only the focused half; a screen
/// reader's increment, decrement and set-value do the same. Each press is one `Set`.
fn keys(ui: &Ui, gesture: &mut Gesture<BlendIfRange>, h: usize, id: Id) {
    ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, ..Default::default() }));
    let step = |gesture: &mut Gesture<BlendIfRange>, by: i32, split: bool| {
        let current = *gesture.shown();
        let to = (i32::from(current.0[h]) + by).clamp(0, 255) as u8;
        gesture.set(current.moved(h, to, split));
    };
    for (modifiers, by, split) in [(Modifiers::ALT, 1, true), (Modifiers::SHIFT, 10, false), (Modifiers::NONE, 1, false)] {
        for (key, sign) in [(Key::ArrowLeft, -1), (Key::ArrowRight, 1)] {
            for _ in 0..take_exact(ui, modifiers, key) {
                step(gesture, sign * by, split);
            }
        }
    }
    let (up, down) = ui.input(|i| (i.num_accesskit_action_requests(id, Action::Increment), i.num_accesskit_action_requests(id, Action::Decrement)));
    for _ in 0..up {
        step(gesture, 1, false);
    }
    for _ in 0..down {
        step(gesture, -1, false);
    }
    let set = ui.input(|i| {
        i.accesskit_action_requests(id, Action::SetValue)
            .filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                _ => None,
            })
            .last()
    });
    if let Some(v) = set {
        let current = *gesture.shown();
        gesture.set(current.moved(h, v.round().clamp(0.0, 255.0) as u8, false));
    }
}

/// A press picks the nearest handle and focuses it; a drag moves it by [`BlendIfRange::moved`],
/// alone while Alt is held.
fn pointer(ui: &Ui, response: &egui::Response, gesture: &mut Gesture<BlendIfRange>, ramp: Rect, id: Id, handle_id: &dyn Fn(usize) -> Id) {
    let grab = id.with("grab");
    let pos = response.interact_pointer_pos();
    if response.clicked()
        && let Some(Pos2 { x, .. }) = pos
    {
        let h = nearest(ramp, *gesture.shown(), x);
        ui.memory_mut(|m| m.request_focus(handle_id(h)));
    }
    if response.drag_started()
        && let Some(Pos2 { x, .. }) = pos
    {
        let h = nearest(ramp, *gesture.shown(), x);
        ui.data_mut(|d| d.insert_temp(grab, h));
        ui.memory_mut(|m| m.request_focus(handle_id(h)));
        gesture.begin();
    }
    if response.dragged()
        && let (Some(h), Some(pos)) = (ui.data(|d| d.get_temp::<usize>(grab)), pos)
    {
        let modifiers = ui.input(|i| i.modifiers);
        let moved = gesture.shown().moved(h, level_at(ramp, pos.x), modifiers.alt);
        gesture.update(moved, modifiers);
    }
    if response.drag_stopped() {
        gesture.commit();
    }
}

/// The four triangles under the ramp: a split pair shows two half triangles, an unsplit pair one
/// whole triangle. Each handle is a Tab stop and an AccessKit slider.
#[expect(clippy::too_many_arguments, reason = "one pass of drawing, named parts of one bar")]
fn paint_handles(
    ui: &mut Ui,
    style: &BlendIfBarStyle,
    labels: &BlendIfBarLabels,
    numbers: &LocaleNumbers,
    layer: &str,
    ramp: Rect,
    value: BlendIfRange,
    handle_id: &dyn Fn(usize) -> Id,
    enabled: bool,
) {
    let q = value.0;
    // Centred in the room under the ramp.
    let top = ramp.bottom() + (style.height - style.ramp - style.handle_height) / 2.0;
    let half = style.handle_width / 2.0;
    let edge = Stroke::new(craft_theme::Stroke::W1.width(1.0), style.handle_edge);
    let names = [&labels.black_low, &labels.black_high, &labels.white_low, &labels.white_high];
    for (pair, face) in [(0, BLACK_FACE), (2, WHITE_FACE)] {
        let whole = q[pair] == q[pair + 1];
        for (side, h) in [(-1.0_f32, pair), (1.0, pair + 1)] {
            let x = x_of(ramp, q[h]);
            // The half this handle owns: the whole triangle when the pair is not split.
            let (left, right) = if whole {
                (x - half, x + half)
            } else if side < 0.0 {
                (x - half, x)
            } else {
                (x, x + half)
            };
            let rect = Rect::from_x_y_ranges(left..=right, top..=top + style.handle_height);
            let name = craft_i18n::fmt(&labels.handle, &[("handle", names[h]), ("layer", layer)]);
            let sensed = Interactive::with_id(handle_id(h), Role::Slider, name)
                .sense(Sense::focusable_noninteractive())
                .enabled(enabled)
                .state_layer(false)
                .interact_at(ui, rect)
                .numeric(f64::from(q[h]), 0.0..=255.0, 1.0)
                .value(numbers.format(f64::from(q[h]), 0..=0));
            sensed.finish(ui);
            // Draw a whole triangle once, from the pair's first handle.
            if whole && side > 0.0 {
                continue;
            }
            let apex = egui::pos2(x, top);
            let points = vec![apex, egui::pos2(right, rect.bottom()), egui::pos2(left, rect.bottom())];
            ui.painter().add(Shape::convex_polygon(points, face, edge));
        }
    }
}
