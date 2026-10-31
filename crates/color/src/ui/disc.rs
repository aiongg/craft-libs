//! [`ColorDisc`]: a hue and saturation disc with one draggable marker per colour of a set, as
//! VectorCraft's harmony wheel.

use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba};
use egui::accesskit::{Action, Role};
use egui::{AsIdSalt, Color32, Id, IdSalt, InteractOptions, Key, Mesh, Modifiers, Pos2, Rect, Sense, Stroke, Ui, UiBuilder, vec2};

use super::picker::installed;
use super::wheel::{ColorWheelStyle, finite, hue_at, hue_direction, vertex};
use super::{ColorWheel, faded_stroke};
use craft_ui::controls::slider::{KEY_FILTER, tap};
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse, Gesture};
use craft_ui::primitives::{FocusRing, Interactive};
use craft_ui::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// Segments round the disc and rings out from its centre (VectorCraft's 72 and 8).
const SEGMENTS: u32 = 72;
const RINGS: u32 = 8;

/// A marker's place on the disc: what a drag or a key changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscMove {
    pub marker: usize,
    /// Degrees, 0 to 360, red at 0 and growing counter-clockwise.
    pub hue: f32,
    /// 0 (the centre) to 1 (the rim).
    pub saturation: f32,
}

/// The disc's sizes, colours and the strokes it shares with [`ColorWheel`], from
/// [`ColorDisc::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorDiscStyle {
    /// Default side of the area: `color_wheel.side`.
    pub side: f32,
    /// Least side of the area: `color_wheel.side_min`.
    pub side_min: f32,
    /// Distance from the area's edge to the disc: `color_wheel.disc_inset`.
    pub inset: f32,
    /// A marker's radius, and a selected one's.
    pub marker: f32,
    pub marker_selected: f32,
    /// A press within this of a marker grabs it.
    pub grab: f32,
    /// Least brightness the disc draws, as a proportion.
    pub floor: f32,
    /// The line from the centre to a marker: `color_wheel.spoke`.
    pub spoke: Stroke,
    /// The wheel's edge and marker strokes.
    pub wheel: ColorWheelStyle,
}

/// Hue and saturation disc with a marker per colour; the host owns the colours and the rule that
/// moves them together:
///
/// ```
/// use craft_theme::Srgba;
/// use craft_color::ui::ColorDisc;
/// # egui::__run_test_ui(|ui| { let colors = [Srgba::rgb(0.9, 0.2, 0.2), Srgba::rgb(0.2, 0.7, 0.4)];
/// let disc = ColorDisc::new("disc", "Harmony", &colors).selected(Some(0)).show(ui);
/// if let Some(index) = disc.pressed { /* select `index` */ }
/// if let Some(v) = disc.edit.event.as_ref().and_then(|e| e.value()) { /* move colour `v.marker` */ }
/// # });
/// ```
///
/// - Pointer (primary button): a press grabs the nearest marker within `color_wheel.disc_grab`
///   ([`pressed`](ColorDiscResponse::pressed) reports it, for the host to select it); a press
///   elsewhere grabs nothing. A drag moves the grabbed marker's hue (the pointer's angle, whole
///   degrees) and saturation (its distance over the radius, whole percents, at most 1), one
///   gesture ([`craft_ui::edit`]); the marker's brightness stays, the host's slider sets it. A press
///   and release in one frame is a [`Set`](EditEvent::Set).
/// - Keys: one Tab stop, the selected marker (the first without a selection). Left and Right
///   change the hue by 1 degree, Up and Down the saturation by 1 percent; Shift steps ten; each
///   key is a `Set`. During a drag keys change nothing.
/// - Esc during a drag cancels it.
/// - AccessKit: a `Group` named `label` holding a `Slider` per marker, named by
///   [`names`](ColorDisc::names) or "Color N", with the value text "Hue 120 degrees, saturation
///   80 %". Focus on any marker is reported as `pressed`.
/// - Right to left nothing mirrors: red stays at 3 o'clock.
#[derive(Clone, Debug)]
#[must_use = "show the disc with `show`"]
pub struct ColorDisc {
    id_salt: IdSalt,
    label: String,
    markers: Vec<Srgba>,
    selected: Option<usize>,
    brightness: f32,
    side: Option<f32>,
    policy: EditPolicy,
    enabled: bool,
    marker_names: Vec<String>,
    words: Option<[String; 4]>,
}

/// What [`ColorDisc::show`] reports.
#[derive(Clone, Debug)]
pub struct ColorDiscResponse {
    /// The marker a press grabbed, or a screen reader focused: the host selects it.
    pub pressed: Option<usize>,
    /// This frame's edit event, carrying the moved marker; the response is the group's.
    pub edit: EditResponse<DiscMove>,
    /// The area the disc was given.
    pub area: Rect,
    /// The ids of the markers' focus stops, in order.
    pub markers: Vec<Id>,
}

impl ColorDisc {
    /// The English words the disc speaks, the keys a host translates with [`text`](ColorDisc::text):
    /// the marker's default name (a number follows it), the two components and the hue's unit.
    pub const TEXTS: [&'static str; 4] = ["Color", "Hue", "Saturation", "degrees"];

    /// A disc named `label` (translated by the host) with a marker at each of `markers`;
    /// `id_salt` keeps its gesture and focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, markers: &[Srgba]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            markers: markers.to_vec(),
            selected: None,
            brightness: 1.0,
            side: None,
            policy: EditPolicy::default(),
            enabled: true,
            marker_names: Vec::new(),
            words: None,
        }
    }

    /// The marker drawn larger and reached by Tab; the host owns it.
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// The brightness the disc is drawn at, 0 to 1; never below `color_wheel.disc_floor`. 1 without it.
    pub fn brightness(mut self, brightness: f32) -> Self {
        self.brightness = brightness;
        self
    }

    /// The area's side; `color_wheel.side` without it. Never below `color_wheel.side_min`.
    pub fn side(mut self, side: f32) -> Self {
        self.side = Some(side);
        self
    }

    /// How the gesture reports ([`EditPolicy`]).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The markers' accessible names, in order; a marker without one is "Color N".
    pub fn names(mut self, names: &[String]) -> Self {
        self.marker_names = names.to_vec();
        self
    }

    /// Translates the disc's words, given as the English [`TEXTS`](ColorDisc::TEXTS).
    pub fn text(mut self, translate: impl Fn(&str) -> String) -> Self {
        self.words = Some(Self::TEXTS.map(translate));
        self
    }

    pub fn style(tokens: &UiTokens) -> ColorDiscStyle {
        let t = &tokens.component.color_wheel;
        ColorDiscStyle {
            side: t.side,
            side_min: t.side_min,
            inset: t.disc_inset,
            marker: t.disc_marker,
            marker_selected: t.disc_marker_selected,
            grab: t.disc_grab,
            floor: t.disc_floor,
            spoke: Stroke::new(1.0, t.spoke.to_egui()),
            wheel: ColorWheel::style(tokens),
        }
    }

    /// The disc's centre and radius in `area`.
    pub fn geometry(style: &ColorDiscStyle, area: Rect) -> (Pos2, f32) {
        (area.center(), (area.width().min(area.height()) / 2.0 - style.inset).max(1.0))
    }

    /// Where a marker of hue `hue` degrees and saturation `saturation` (0 to 1) lies.
    pub fn marker_pos(center: Pos2, radius: f32, hue: f32, saturation: f32) -> Pos2 {
        center + hue_direction(hue) * (radius * saturation.clamp(0.0, 1.0))
    }

    /// Shows the disc and returns this frame's edit event.
    pub fn show(self, ui: &mut Ui) -> ColorDiscResponse {
        let style = ColorDisc::style(&Tokens::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(self.label.clone());
        let inner = ui.scope_builder(builder, |ui| self.show_inside(ui, &style, id));
        let (event, area, pressed, markers) = inner.inner;
        ColorDiscResponse { pressed, edit: EditResponse::new(inner.response, event), area, markers }
    }

    fn show_inside(self, ui: &mut Ui, style: &ColorDiscStyle, id: Id) -> (Option<EditEvent<DiscMove>>, Rect, Option<usize>, Vec<Id>) {
        let side = self.side.filter(|s| s.is_finite()).unwrap_or(style.side).max(style.side_min);
        let width = if ui.available_width().is_finite() { ui.available_width() } else { side };
        let (_, area) = ui.allocate_space(vec2(width, side));
        let (center, radius) = ColorDisc::geometry(style, area);
        let hsb: Vec<[f32; 3]> = self.markers.iter().map(|c| HsbModel.from_srgb(*c).values.map(finite)).map(|v| [v[0], v[1], v[2]]).collect();
        let at = |i: usize| hsb.get(i).map(|v| ColorDisc::marker_pos(center, radius, v[0], v[1] / 100.0));
        let marker_value =
            |i: usize| DiscMove { marker: i, hue: hsb.get(i).map_or(0.0, |v| v[0]), saturation: hsb.get(i).map_or(0.0, |v| v[1] / 100.0) };
        let ids: Vec<Id> = (0..self.markers.len()).map(|i| Interactive::id_in(ui, ("marker", i))).collect();

        let pointer_id = id.with("pointer");
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        // Only the disc and the markers' grab slack take presses; the rest of the row is the host's.
        let reach = Rect::from_center_size(center, vec2(2.0 * (radius + style.grab), 2.0 * (radius + style.grab))).intersect(area);
        let response = ui.interact_opt(reach, pointer_id, Sense::DRAG, options);
        let enabled = response.enabled();

        let tapped = tap(ui, &response);
        let origin = response.drag_started_by(egui::PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let grabbed = origin.or(tapped).and_then(|p| {
            let near = (0..hsb.len()).filter_map(|i| at(i).map(|m| (i, m.distance(p)))).filter(|(_, d)| *d <= style.grab);
            near.min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
        });
        // The marker a screen reader asked to focus.
        let requested = if enabled { ids.iter().position(|&m| ui.input(|i| i.has_accesskit_action_request(m, Action::Focus))) } else { None };
        let resting = self.selected.filter(|&s| s < self.markers.len()).unwrap_or(0);
        let stop = grabbed.or(requested).unwrap_or(resting);
        // The marker a running drag moves keeps its place as the host's value.
        let reported = Gesture::<DiscMove>::reported(ui, pointer_id).map(|(_, v)| v);
        let held = grabbed.or(reported.map(|v| v.marker)).unwrap_or(stop);
        // The host's colour need not hold what the drag reported: black has no hue, and a colour
        // near the centre loses it to the host's precision. While the colour the reported value
        // makes is the host's own, to its 8-bit channels, the reported value is the host's value;
        // any other colour is an outside change, which cancels the drag.
        let host = match (reported, hsb.get(held)) {
            (Some(r), Some(v)) if r.marker == held => {
                let echo = HsbModel.to_srgb(&ModelValues::new("hsb", &[r.hue, r.saturation * 100.0, v[2]]).values);
                if echo.same_rgb8(self.markers[held]) { r } else { marker_value(held) }
            }
            _ => marker_value(held),
        };

        let mut gesture = Gesture::load(ui, pointer_id, &host, self.policy).focus_owners(&ids).event_filter(KEY_FILTER).enabled(enabled);
        let modifiers = || ui.input(|i| i.modifiers);
        let value_at = |base: &DiscMove, pos: Pos2| {
            let offset = pos - center;
            let mut out = *base;
            if offset.length().is_finite() {
                out.saturation = ((offset.length() / radius).clamp(0.0, 1.0) * 100.0).round() / 100.0;
                if offset.length() >= 0.5
                    && let Some(hue) = hue_at(center, pos)
                {
                    out.hue = hue;
                }
            }
            out
        };
        if origin.is_some() && grabbed.is_some() {
            gesture.begin();
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(start) = gesture.start().copied()
            && let Some(pos) = response.interact_pointer_pos()
        {
            gesture.update(value_at(&start, pos), modifiers());
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if let Some(pos) = tapped.filter(|_| grabbed.is_some()) {
            gesture.set(value_at(gesture.shown(), pos));
        }

        let focused = ids.iter().position(|&m| ui.memory(|mem| mem.has_focus(m)));
        if enabled && focused.is_some() && !response.dragged() {
            let step = |plus: Key, minus: Key| {
                ui.input_mut(|i| {
                    let mut count = |m, k| i.count_and_consume_key(m, k) as f32;
                    // Shift first: `Modifiers::NONE` also matches a press with Shift held.
                    10.0 * (count(Modifiers::SHIFT, plus) - count(Modifiers::SHIFT, minus)) + count(Modifiers::NONE, plus)
                        - count(Modifiers::NONE, minus)
                })
            };
            let (hue, saturation) = (step(Key::ArrowRight, Key::ArrowLeft), step(Key::ArrowUp, Key::ArrowDown));
            if hue != 0.0 || saturation != 0.0 {
                let mut out = *gesture.shown();
                out.hue = (out.hue + hue).round().rem_euclid(360.0);
                out.saturation = ((out.saturation * 100.0 + saturation).round().clamp(0.0, 100.0)) / 100.0;
                gesture.set(out);
            }
        }

        // The markers, for the keyboard and a screen reader; only the stop takes the focus.
        let shown = *gesture.shown();
        let words = self.words.clone().unwrap_or_else(|| installed(ui, Self::TEXTS));
        let mut parts = Vec::with_capacity(ids.len());
        for i in 0..ids.len() {
            let live = if i == shown.marker { shown } else { marker_value(i) };
            let name = self.marker_names.get(i).cloned().unwrap_or_else(|| format!("{} {}", words[0], i + 1));
            let value = format!("{} {:.0} {}, {} {:.0} %", words[1], live.hue.rem_euclid(360.0), words[3], words[2], live.saturation * 100.0);
            let big = Some(i) == self.selected;
            let r = if big { style.marker_selected } else { style.marker };
            let rect = Rect::from_center_size(at_value(center, radius, &live), vec2(2.0 * r, 2.0 * r));
            let sense = if i == stop { Sense::FOCUSABLE } else { Sense::empty() };
            let part = Interactive::new(("marker", i), Role::Slider, name).sense(sense).enabled(enabled).state_layer(false).focus_ring(false);
            let part = part.interact_at(ui, rect).value(value).finish(ui);
            if i != stop && enabled {
                ui.ctx().accesskit_node_builder(part.id, |node| node.add_action(Action::Focus));
            }
            parts.push((rect, part));
        }
        if enabled && (grabbed.is_some() || requested.is_some() || focused.is_some_and(|f| f != stop)) && !ui.memory(|m| m.has_focus(ids[stop])) {
            ui.memory_mut(|m| m.request_focus(ids[stop]));
        }

        if ui.is_rect_visible(area) {
            let brightness = if self.brightness.is_finite() { self.brightness } else { 1.0 };
            let painter = ui.painter();
            painter.add(disc_mesh(center, radius, brightness.clamp(0.0, 1.0).max(style.floor)));
            let edge = faded_stroke(style.wheel.edge, enabled);
            painter.circle_stroke(center, radius + edge.width / 2.0, edge);
            let spoke = faded_stroke(style.spoke, enabled);
            let places: Vec<Pos2> =
                (0..self.markers.len()).map(|i| at_value(center, radius, &if i == shown.marker { shown } else { marker_value(i) })).collect();
            for p in &places {
                painter.line_segment([center, *p], spoke);
            }
            let ring = style.wheel.marker_outer_radius - style.wheel.marker_inner_radius;
            let order = (0..places.len()).filter(|i| Some(*i) != self.selected).chain(self.selected.filter(|s| *s < places.len()));
            for i in order {
                let r = if Some(i) == self.selected { style.marker_selected } else { style.marker };
                let fill: Color32 = self.markers[i].into();
                painter.circle_filled(places[i], r, fill);
                painter.circle_stroke(places[i], r, faded_stroke(style.wheel.marker_inner, enabled));
                painter.circle_stroke(places[i], r + ring, faded_stroke(style.wheel.marker_outer, enabled));
            }
            for (i, (_, part)) in parts.iter().enumerate() {
                let r = if Some(i) == self.selected { style.marker_selected } else { style.marker };
                FocusRing::paint_circle(ui, places[i], r + ring + style.wheel.marker_outer.width / 2.0, None, part.has_focus());
            }
        }
        (gesture.finish(), area, grabbed.or(requested), ids)
    }
}

fn at_value(center: Pos2, radius: f32, v: &DiscMove) -> Pos2 {
    ColorDisc::marker_pos(center, radius, v.hue, v.saturation)
}

/// The disc at `brightness`: a centre vertex and rings out to the rim, hue around.
fn disc_mesh(center: Pos2, radius: f32, brightness: f32) -> Mesh {
    let mut mesh = Mesh::default();
    let b = brightness * 100.0;
    mesh.colored_vertex(center, vertex(0.0, 0.0, b));
    for ring in 1..=RINGS {
        let s = ring as f32 / RINGS as f32;
        for k in 0..SEGMENTS {
            let hue = k as f32 * 360.0 / SEGMENTS as f32;
            mesh.colored_vertex(ColorDisc::marker_pos(center, radius, hue, s), vertex(hue, s * 100.0, b));
        }
    }
    let at = |ring: u32, k: u32| if ring == 0 { 0 } else { 1 + (ring - 1) * SEGMENTS + k % SEGMENTS };
    for ring in 0..RINGS {
        for k in 0..SEGMENTS {
            if ring == 0 {
                mesh.add_triangle(0, at(1, k), at(1, k + 1));
            } else {
                mesh.add_triangle(at(ring, k), at(ring + 1, k), at(ring + 1, k + 1));
                mesh.add_triangle(at(ring, k), at(ring + 1, k + 1), at(ring, k + 1));
            }
        }
    }
    mesh
}
