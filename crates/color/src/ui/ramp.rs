//! [`ColorRamp`]: the bar under a colour panel's slider rows, with white and black boxes at its end.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use craft_theme::{Hsb, Srgba};
use egui::accesskit::{Action, Role};
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Key, Mesh, Modifiers, PointerButton, Pos2, Rect, Response, Sense, StrokeKind, Ui, pos2, vec2};

use super::picker::installed;
use super::{ColorArea, ColorPartsStyle, faded_stroke};
use craft_ui::controls::slider::{KEY_FILTER, PressTurn, press_turn, tap};
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse, Gesture};
use craft_ui::primitives::{FocusRing, Interactive};
use craft_ui::theme::{Tokens, TokensOf};

/// Which colours the ramp shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum RampKind {
    /// Hue across; white at the top through the full colour in the middle to black at the bottom.
    #[default]
    Spectrum,
    /// The [`Spectrum`](RampKind::Spectrum) in the host's CMYK gamut: each colour goes through
    /// [`gamut`](ColorRamp::gamut) (a CMYK round trip). Without a `gamut` it is the spectrum.
    CmykSpectrum,
    /// White at the left to black at the right.
    Grayscale,
    /// From one colour at the left to another at the right, in encoded sRGB
    /// ([`ends`](ColorRamp::ends)).
    Between,
}

/// A colour the ramp picked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RampPick {
    pub color: Srgba,
    /// Alt was held at the press (or at the click, key or box press): the host sets the colour it
    /// is not editing.
    pub other: bool,
}

/// What [`ColorRamp::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ColorRampResponse {
    /// The pick, through the edit lifecycle. The ramp has no value of the host's to compare with,
    /// so every pick is a change.
    pub edit: EditResponse<RampPick>,
    /// A click with Shift: the host moves to its next ramp. Nothing was picked.
    pub cycle: bool,
    /// The bar's response, for the host's `ContextMenu`.
    pub bar: Response,
    /// A secondary click that began and ended on the bar: where the host opens its menu, with
    /// `ContextMenu::open_at(Some(OpenAt::Pointer(pos)))` and `own_gestures(false)` on
    /// [`bar`](ColorRampResponse::bar). The bar senses drags only, so egui's own secondary click
    /// never fires on it.
    pub menu_at: Option<Pos2>,
}

/// The value the gesture compares picks against: no colour a pick can have.
const NO_PICK: RampPick = RampPick { color: Srgba::new(-1.0, -1.0, -1.0, -1.0), other: false };

/// The bar under the Color panel's slider rows (Photoshop's colour ramp, PhotoCraft's `ramp`): a
/// spectrum, a grey ramp or a ramp between two colours, with a white and a black box at its end.
///
/// ```
/// use craft_theme::Srgba;
/// use craft_color::ui::{ColorRamp, RampKind};
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.2, 0.5, 0.8);
/// let shown = ColorRamp::new("ramp", "Color ramp", RampKind::Spectrum).show(ui);
/// if let Some(pick) = shown.edit.event.as_ref().and_then(|e| e.value()) {
///     color = pick.color;
/// }
/// # });
/// ```
///
/// The bar is 48 by 8 cells, shaded between the vertices of a 49 by 9 grid; a [`web_safe`](ColorRamp::web_safe)
/// ramp is flat cells coloured at their centres. Hue is a function of x only, so a pick carries
/// hue and saturation; the host's slider rows show it through the kept rule. The colours are
/// computed when the kind, ends, web flag or [`gamut_revision`](ColorRamp::gamut_revision)
/// change, and cached.
///
/// - Pointer: a press picks the colour under the pointer, drawn in the press frame (the cell's
///   centre on a web-safe ramp); a drag updates, also outside the bar, where the pick stays at the
///   edge; the release commits. A press and release in one frame is a
///   [`Set`](craft_ui::edit::EditEvent::Set). A [`Between`](RampKind::Between) ramp keeps the ends
///   it had at the press for the whole gesture. Alt at the press sets
///   [`other`](RampPick::other) on every event. A press with Shift starts no gesture; releasing
///   it without moving sets [`cycle`](ColorRampResponse::cycle). A right-click on the bar is
///   reported as [`menu_at`](ColorRampResponse::menu_at) for the host's `ContextMenu`.
/// - Boxes: a click on the white or the black box (also by keyboard) is a
///   [`Set`](craft_ui::edit::EditEvent::Set) of that colour; they are Tab stops of their own.
/// - Keys, with the bar focused: it has a cursor cell, starting at the left end of the middle row,
///   shown as the picker's marker. Arrows move it one cell (Shift ten), Home and End to the row's
///   ends, Page Up and Page Down to the top and bottom rows; Left and Right do not mirror. Enter
///   or Space picks the cursor cell (a `Set`), Alt+Enter with `other`. A press moves the cursor to
///   its cell.
/// - AccessKit: a `Group` named `label` whose value is [`kind_name`](ColorRamp::kind_name); its
///   click picks the cursor cell. The boxes are `Button`s named by [`TEXTS`](ColorRamp::TEXTS).
/// - Nothing mirrors right to left; the boxes stay at the right.
#[must_use = "show the ramp with `show`"]
pub struct ColorRamp<'a> {
    id_salt: IdSalt,
    label: String,
    kind: RampKind,
    kind_name: Option<String>,
    ends: [Srgba; 2],
    gamut: Option<&'a dyn Fn(Srgba) -> Srgba>,
    gamut_revision: u64,
    web_safe: bool,
    boxes: bool,
    height: Option<f32>,
    policy: EditPolicy,
    enabled: bool,
    press_waits: bool,
    texts: Option<[String; 2]>,
}

impl<'a> ColorRamp<'a> {
    /// The words the boxes' accessible names ask [`texts`](ColorRamp::texts) for.
    pub const TEXTS: [&'static str; 2] = ["White", "Black"];

    /// A ramp named `label` (translated by the host: "Color ramp"); `id_salt` keeps its gesture,
    /// focus and cursor when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, kind: RampKind) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            kind,
            kind_name: None,
            ends: [Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0)],
            gamut: None,
            gamut_revision: 0,
            web_safe: false,
            boxes: true,
            height: None,
            policy: EditPolicy::default(),
            enabled: true,
            press_waits: false,
            texts: None,
        }
    }

    /// The colours at the left and the right of a [`Between`](RampKind::Between) ramp.
    pub fn ends(mut self, from: Srgba, to: Srgba) -> Self {
        self.ends = [from, to];
        self
    }

    /// Maps each colour of a [`Spectrum`](RampKind::Spectrum) or [`CmykSpectrum`](RampKind::CmykSpectrum) to the host's gamut (a CMYK round
    /// trip). It runs for the cached vertices only, and again for a pick.
    pub fn gamut(mut self, gamut: &'a dyn Fn(Srgba) -> Srgba) -> Self {
        self.gamut = Some(gamut);
        self
    }

    /// The host's counter for the working space `gamut` maps into; a change recomputes the colours.
    pub fn gamut_revision(mut self, revision: u64) -> Self {
        self.gamut_revision = revision;
        self
    }

    /// Whether every channel is rounded to the nearest of 0, 0.2, ... 1 and the ramp is flat
    /// cells.
    pub fn web_safe(mut self, web_safe: bool) -> Self {
        self.web_safe = web_safe;
        self
    }

    /// Whether the white and black boxes follow the bar; on by default.
    pub fn boxes(mut self, boxes: bool) -> Self {
        self.boxes = boxes;
        self
    }

    /// The ramp's height, not below `color_ramp.height_min`; `color_ramp.height` without it.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
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

    /// Whether a press waits for another control's typing run to commit and then edits the colour
    /// that run left: on inside a panel with number fields, off standalone.
    pub fn press_waits(mut self, waits: bool) -> Self {
        self.press_waits = waits;
        self
    }

    /// The ramp's kind as the host names it ("RGB Spectrum"), the group's accessible value.
    pub fn kind_name(mut self, name: impl Into<String>) -> Self {
        self.kind_name = Some(name.into());
        self
    }

    /// The boxes' accessible names, translated by the host, as [`TEXTS`](ColorRamp::TEXTS) lists
    /// them: the white box's, then the black one's.
    pub fn texts(mut self, white: impl Into<String>, black: impl Into<String>) -> Self {
        self.texts = Some([white.into(), black.into()]);
        self
    }

    /// The colour at (`x`, `y`) as fractions of the bar, `y` down. A non-finite or outside fraction
    /// is clamped into the bar. Spectrum: hue `x` (below 360), white at the top through the full
    /// colour at the middle to black at the bottom, through `gamut` when given; grayscale: white
    /// to black; between: `ends` from the left to the right. `web_safe` rounds each channel.
    pub fn color_at(kind: RampKind, web_safe: bool, gamut: Option<&dyn Fn(Srgba) -> Srgba>, ends: [Srgba; 2], x: f32, y: f32) -> Srgba {
        let unit = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        let (x, y) = (unit(x), unit(y));
        let color = match kind {
            RampKind::Spectrum | RampKind::CmykSpectrum => {
                let (s, b) = if y < 0.5 { (y * 2.0, 1.0) } else { (1.0, 1.0 - (y - 0.5) * 2.0) };
                let color = Srgba::from_hsb(Hsb { h: x.min(0.9999) * 360.0, s, b });
                gamut.map_or(color, |g| g(color))
            }
            RampKind::Grayscale => Srgba::new(1.0 - x, 1.0 - x, 1.0 - x, 1.0),
            RampKind::Between => {
                let [a, b] = ends;
                let mix = |from: f32, to: f32| from + (to - from) * x;
                Srgba::new(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b), mix(a.a, b.a))
            }
        };
        let web = |v: f32| if web_safe { (unit(v) * 5.0).round() / 5.0 } else { unit(v) };
        Srgba::new(web(color.r), web(color.g), web(color.b), unit(color.a))
    }

    /// Shows the ramp and returns this frame's pick. The bar's response covers the bar.
    pub fn show(self, ui: &mut Ui) -> ColorRampResponse {
        let tokens = Tokens::of(ui);
        let t = tokens.component.color_ramp;
        let style = ColorArea::style(&tokens);
        let (nx, ny) = (t.cells_x.max(1) as usize, t.cells_y.max(1) as usize);
        let height = self.height.filter(|h| h.is_finite()).unwrap_or(t.height).max(t.height_min);
        let (all, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let boxes_width = if self.boxes { t.boxes.min(all.width() / 2.0) } else { 0.0 };
        let gap = if self.boxes { t.boxes_gap } else { 0.0 };
        let bar = Rect::from_min_max(all.min, pos2((all.right() - boxes_width - gap).max(all.left()), all.bottom()));
        let boxes = Rect::from_min_max(pos2(all.right() - boxes_width, all.top()), all.max);

        let sensed = Interactive::salted(self.id_salt, Role::Group, self.label.as_str())
            .sense(Sense::drag())
            .enabled(self.enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, bar);
        let response = sensed.response().clone();
        let (id, enabled) = (response.id, response.enabled());
        let mods = ui.input(|i| i.modifiers);
        let menu_at = secondary_click(ui, &response, bar);

        // The host holds no pick, so the gesture compares against the one it last reported while it
        // runs: reading that as the host's echo keeps it from being cancelled as an outside change.
        let host_key = id.with("craft_ui::ramp_host");
        let host = ui.data(|d| d.get_temp::<RampPick>(host_key)).unwrap_or(NO_PICK);
        let mut gesture = Gesture::load(ui, id, &host, self.policy).event_filter(KEY_FILTER).enabled(enabled);
        let started = response.drag_started_by(PointerButton::Primary).then(|| response.interact_pointer_pos()).flatten();
        let tapped = tap(ui, &response);
        let turn = press_turn(ui, id, self.press_waits, gesture.id(), started.or(tapped));
        let waits = matches!(turn, PressTurn::Waits);
        let dragging = response.dragged_by(PointerButton::Primary);

        // The press's state: its modifiers and, for a Between ramp, the ends it began with.
        let press_key = id.with("craft_ui::ramp_press");
        let mut press =
            ui.data(|d| d.get_temp::<Press>(press_key)).unwrap_or(Press { shift: false, alt: false, moved: false, origin: None, ends: self.ends });
        let press_mods = press_modifiers(ui, id, mods, waits, started.is_some() || tapped.is_some(), matches!(turn, PressTurn::Held(_)));
        let new_press = !waits && (started.is_some() || tapped.is_some() || matches!(turn, PressTurn::Held(_)));
        if new_press {
            let origin = match turn {
                PressTurn::Held(at) => Some(at),
                _ => started.or(tapped),
            };
            press = Press { shift: press_mods.shift, alt: press_mods.alt, moved: false, origin, ends: self.ends };
        }
        let ends = if dragging && !new_press { press.ends } else { self.ends };
        let cell_at = |pos: Pos2| cell_of(bar, nx, ny, pos);
        let pick_at = |pos: Pos2, other: bool| {
            let (fx, fy) = fractions(bar, pos);
            let (fx, fy) = if self.web_safe { cell_centre(fx, fy, nx, ny) } else { (fx, fy) };
            RampPick { color: ColorRamp::color_at(self.kind, self.web_safe, self.gamut, ends, fx, fy), other }
        };
        let cursor_key = id.with("craft_ui::ramp_cursor");
        let mut cursor = ui.data(|d| d.get_temp::<(usize, usize)>(cursor_key)).unwrap_or((0, ny / 2));
        cursor = (cursor.0.min(nx - 1), cursor.1.min(ny - 1));

        let mut cycle = false;
        if let PressTurn::Held(held) = turn {
            cursor = cell_at(held);
            if press.shift {
                cycle = !dragging;
            } else {
                gesture.begin_with_value();
                gesture.update(pick_at(held, press.alt), press_mods);
                if !dragging {
                    gesture.commit();
                }
            }
        }
        if started.is_some() {
            response.request_focus();
            if !waits {
                cursor = started.map_or(cursor, cell_at);
                if !press.shift {
                    gesture.begin_with_value();
                }
            }
        }
        if dragging
            && !new_press
            && let Some(pos) = response.interact_pointer_pos()
        {
            if press.origin.is_some_and(|o| o.distance(pos) > MOVED) {
                press.moved = true;
            }
            if !press.shift {
                cursor = cell_at(pos);
            }
        }
        if dragging
            && !press.shift
            && !matches!(turn, PressTurn::Held(_))
            && let Some(pos) = response.interact_pointer_pos()
        {
            gesture.update(pick_at(pos, press.alt), mods);
        }
        if response.drag_stopped() {
            gesture.commit();
            cycle |= press.shift && !press.moved;
        }
        if let Some(pos) = tapped.filter(|_| !waits) {
            response.request_focus();
            cursor = cell_at(pos);
            if mods.shift {
                cycle = true;
            } else {
                gesture.set(pick_at(pos, mods.alt));
            }
        }
        ui.data_mut(|d| d.insert_temp(press_key, press));

        // Keys.
        if enabled && response.has_focus() && !dragging {
            let step = key_move(ui, nx, ny, cursor);
            cursor = step.cursor;
            if let Some(other) = step.pick {
                let (cx, cy) = cell_centre_of(cursor, nx, ny);
                gesture.set(RampPick { color: ColorRamp::color_at(self.kind, self.web_safe, self.gamut, ends, cx, cy), other });
            }
        }
        if enabled && ui.input(|i| i.num_accesskit_action_requests(id, Action::Click)) > 0 {
            response.request_focus();
            let (cx, cy) = cell_centre_of(cursor, nx, ny);
            gesture.set(RampPick { color: ColorRamp::color_at(self.kind, self.web_safe, self.gamut, ends, cx, cy), other: false });
        }
        ui.data_mut(|d| d.insert_temp(cursor_key, cursor));

        let box_focus = self.show_boxes(ui, id, boxes, &style, &mut gesture);

        // The bar.
        let sensed = match &self.kind_name {
            Some(name) => sensed.value(name.clone()),
            None => sensed,
        };
        let response = sensed.finish(ui);
        if enabled {
            ui.ctx().accesskit_node_builder(id, |node| node.add_action(Action::Click));
        }
        let focused = response.has_focus();
        if ui.is_rect_visible(all) {
            let colors = self.colors(ui, id, nx, ny, ends);
            ui.painter().add(ramp_mesh(bar, &colors, nx, ny, self.web_safe));
            ui.painter().rect_stroke(bar, CornerRadius::ZERO, faded_stroke(style.edge, enabled), StrokeKind::Outside);
            if focused && FocusRing::visible(ui.ctx()) {
                let at = pos2(
                    bar.left() + bar.width() * (cursor.0 as f32 + 0.5) / nx as f32,
                    bar.top() + bar.height() * (cursor.1 as f32 + 0.5) / ny as f32,
                );
                ui.painter().circle_stroke(at, style.marker_outer_radius, faded_stroke(style.marker_outer, enabled));
                ui.painter().circle_stroke(at, style.marker_inner_radius, faded_stroke(style.marker_inner, enabled));
            }
        }
        FocusRing::paint_if(ui, all.expand(style.edge.width), CornerRadius::ZERO, focused || box_focus);
        let event = gesture.finish();
        remember_host(ui, host_key, &event);
        ColorRampResponse { edit: EditResponse::new(response.clone(), event), cycle, bar: response, menu_at }
    }

    /// The white and black boxes, each a [`Set`](EditEvent::Set) of its colour through `gesture`;
    /// whether one has the keyboard focus.
    fn show_boxes(&self, ui: &Ui, id: egui::Id, boxes: Rect, style: &ColorPartsStyle, gesture: &mut Gesture<RampPick>) -> bool {
        let enabled = self.enabled;
        let half = boxes.center().y;
        let white_rect = Rect::from_min_max(boxes.min, pos2(boxes.right(), half));
        let black_rect = Rect::from_min_max(pos2(boxes.left(), half), boxes.max);
        let mut box_focus = false;
        let texts = self.texts.clone().unwrap_or_else(|| installed(ui, Self::TEXTS));
        if self.boxes {
            for (index, (rect, color)) in [(white_rect, Srgba::rgb(1.0, 1.0, 1.0)), (black_rect, Srgba::rgb(0.0, 0.0, 0.0))].into_iter().enumerate() {
                let name = if index == 0 { "white" } else { "black" };
                let sensed = Interactive::with_id(Interactive::id_in(ui, (id, name)), Role::Button, texts[index].as_str())
                    .sense(Sense::click())
                    .enabled(self.enabled)
                    .state_layer(false)
                    .focus_ring(false)
                    .interact_at(ui, rect);
                let clicked = sensed.response().clicked();
                let response = sensed.finish(ui);
                box_focus |= response.has_focus();
                if clicked {
                    gesture.set(RampPick { color, other: ui.input(|i| i.modifiers.alt) });
                }
                ui.painter().rect_filled(rect, CornerRadius::ZERO, Color32::from(color));
            }
            ui.painter().rect_stroke(boxes, CornerRadius::ZERO, faded_stroke(style.edge, enabled), StrokeKind::Outside);
        }
        box_focus
    }

    /// The colours of the mesh for `ends` (the ones a pick uses, so a dragged Between ramp stays
    /// as the press drew it), from the cache while nothing they depend on has changed: 49 by 9
    /// vertices, or 48 by 8 cell centres for a web-safe ramp.
    fn colors(&self, ui: &Ui, id: egui::Id, nx: usize, ny: usize, ends: [Srgba; 2]) -> Arc<Vec<Color32>> {
        let mut hasher = DefaultHasher::new();
        (self.kind, self.web_safe, nx, ny).hash(&mut hasher);
        if self.kind == RampKind::Between {
            ends.iter().for_each(|c| [c.r, c.g, c.b, c.a].map(f32::to_bits).hash(&mut hasher));
        }
        if matches!(self.kind, RampKind::Spectrum | RampKind::CmykSpectrum) {
            (self.gamut.is_some(), self.gamut_revision).hash(&mut hasher);
        }
        let signature = hasher.finish();
        let key = id.with("craft_ui::ramp_cache");
        if let Some((cached, colors)) = ui.data(|d| d.get_temp::<(u64, Arc<Vec<Color32>>)>(key))
            && cached == signature
        {
            return colors;
        }
        let (cols, rows, half) = if self.web_safe { (nx, ny, 0.5) } else { (nx + 1, ny + 1, 0.0) };
        let mut colors = Vec::with_capacity(cols * rows);
        for row in 0..rows {
            for col in 0..cols {
                let (x, y) = ((col as f32 + half) / nx as f32, (row as f32 + half) / ny as f32);
                colors.push(Color32::from(ColorRamp::color_at(self.kind, self.web_safe, self.gamut, ends, x, y)));
            }
        }
        let colors = Arc::new(colors);
        ui.data_mut(|d| d.insert_temp(key, (signature, colors.clone())));
        colors
    }
}

/// Points the pointer may move between a Shift press and its release for it to be a click.
const MOVED: f32 = 4.0;

/// What a press began with, kept for its gesture.
#[derive(Clone, Copy, Debug)]
struct Press {
    shift: bool,
    alt: bool,
    /// The pointer left the press position by more than a click's slack.
    moved: bool,
    origin: Option<Pos2>,
    ends: [Srgba; 2],
}

/// Keeps the pick the gesture last reported as the value it compares against while it runs, and
/// forgets it when the gesture ends.
fn remember_host(ui: &Ui, key: egui::Id, event: &Option<EditEvent<RampPick>>) {
    match event {
        Some(EditEvent::Update { value, .. } | EditEvent::Begin { value: Some(value), .. }) => ui.data_mut(|d| {
            d.insert_temp(key, *value);
        }),
        Some(EditEvent::Begin { value: None, .. }) | None => {}
        Some(_) => ui.data_mut(|d| {
            d.insert_temp(key, NO_PICK);
        }),
    }
}

/// The modifiers of the press this pass runs. A press that waits keeps the ones it arrived with:
/// the pass that runs it may see others.
fn press_modifiers(ui: &Ui, id: egui::Id, mods: Modifiers, waits: bool, arrived: bool, held: bool) -> Modifiers {
    let key = id.with("craft_ui::ramp_waiting_mods");
    if waits && arrived {
        ui.data_mut(|d| d.insert_temp(key, mods));
    }
    let kept = if held { ui.data_mut(|d| d.remove_temp::<Modifiers>(key)) } else { None };
    kept.unwrap_or(mods)
}

/// The position of a secondary click this frame whose press and release were both on `bar` and
/// not under another layer. Read from the raw events because the bar senses drags only.
fn secondary_click(ui: &Ui, response: &Response, bar: Rect) -> Option<Pos2> {
    if !response.enabled() {
        return None;
    }
    let key = response.id.with("craft_ui::ramp_secondary");
    let mut origin = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let on_bar = |pos: Pos2| bar.contains(pos) && ui.ctx().layer_id_at(pos) == Some(response.layer_id);
    let mut click = None;
    let presses: Vec<(Pos2, bool)> = ui.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::PointerButton { pos, button: PointerButton::Secondary, pressed, .. } => Some((*pos, *pressed)),
                _ => None,
            })
            .collect()
    });
    for (pos, pressed) in presses {
        if pressed {
            origin = on_bar(pos);
        } else {
            if origin && on_bar(pos) {
                click = Some(pos);
            }
            origin = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(key, origin));
    click
}

/// `pos` as fractions of `bar`, kept inside it.
fn fractions(bar: Rect, pos: Pos2) -> (f32, f32) {
    let along = |p: f32, from: f32, length: f32| if length > 0.0 && p.is_finite() { ((p - from) / length).clamp(0.0, 1.0) } else { 0.0 };
    (along(pos.x, bar.left(), bar.width()), along(pos.y, bar.top(), bar.height()))
}

/// The cell of `bar`'s `nx` by `ny` grid under `pos`, kept inside it.
fn cell_of(bar: Rect, nx: usize, ny: usize, pos: Pos2) -> (usize, usize) {
    let (fx, fy) = fractions(bar, pos);
    (((fx * nx as f32) as usize).min(nx - 1), ((fy * ny as f32) as usize).min(ny - 1))
}

/// The centre of the cell holding the fractions (`fx`, `fy`).
fn cell_centre(fx: f32, fy: f32, nx: usize, ny: usize) -> (f32, f32) {
    let centre = |f: f32, n: usize| ((f * n as f32).floor().clamp(0.0, (n - 1) as f32) + 0.5) / n as f32;
    (centre(fx, nx), centre(fy, ny))
}

/// The centre of `cell` as fractions of the bar.
fn cell_centre_of(cell: (usize, usize), nx: usize, ny: usize) -> (f32, f32) {
    ((cell.0 as f32 + 0.5) / nx as f32, (cell.1 as f32 + 0.5) / ny as f32)
}

/// What this frame's keys did to the cursor.
struct KeyMove {
    cursor: (usize, usize),
    /// Enter or Space picked the cursor cell: with `other` for Alt+Enter.
    pick: Option<bool>,
}

/// The focused bar's keys: arrows (Shift ten), Home and End, Page Up and Page Down, Enter and
/// Space. Left and Right are not mirrored.
fn key_move(ui: &Ui, nx: usize, ny: usize, cursor: (usize, usize)) -> KeyMove {
    let (mut dx, mut dy, mut col, mut row, mut pick) = (0_i64, 0_i64, None, None, None);
    ui.input_mut(|i| {
        let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as i64;
        // Shift first: `Modifiers::NONE` also matches a press with Shift held.
        let mut steps = |plus, minus| {
            10 * (count(Modifiers::SHIFT, plus) - count(Modifiers::SHIFT, minus)) + count(Modifiers::NONE, plus) - count(Modifiers::NONE, minus)
        };
        dx = steps(Key::ArrowRight, Key::ArrowLeft);
        dy = steps(Key::ArrowDown, Key::ArrowUp);
        if count(Modifiers::NONE, Key::Home) > 0 {
            col = Some(0);
        }
        if count(Modifiers::NONE, Key::End) > 0 {
            col = Some(nx - 1);
        }
        if count(Modifiers::NONE, Key::PageUp) > 0 {
            row = Some(0);
        }
        if count(Modifiers::NONE, Key::PageDown) > 0 {
            row = Some(ny - 1);
        }
        if count(Modifiers::ALT, Key::Enter) > 0 {
            pick = Some(true);
        }
        if count(Modifiers::NONE, Key::Enter) + count(Modifiers::NONE, Key::Space) > 0 {
            pick = Some(false);
        }
    });
    let moved = |at: usize, delta: i64, to: Option<usize>, n: usize| to.unwrap_or_else(|| (at as i64 + delta).clamp(0, n as i64 - 1) as usize);
    KeyMove { cursor: (moved(cursor.0, dx, col, nx), moved(cursor.1, dy, row, ny)), pick }
}

/// The ramp mesh over `bar`: shaded between the vertices, or flat cells when web safe.
fn ramp_mesh(bar: Rect, colors: &[Color32], nx: usize, ny: usize, flat: bool) -> Mesh {
    let mut mesh = Mesh::default();
    let at = |i: usize, j: usize| pos2(bar.left() + bar.width() * i as f32 / nx as f32, bar.top() + bar.height() * j as f32 / ny as f32);
    if flat {
        for j in 0..ny {
            for i in 0..nx {
                let color = colors.get(j * nx + i).copied().unwrap_or(Color32::BLACK);
                mesh.add_colored_rect(Rect::from_min_max(at(i, j), at(i + 1, j + 1)), color);
            }
        }
        return mesh;
    }
    for j in 0..=ny {
        for i in 0..=nx {
            mesh.colored_vertex(at(i, j), colors.get(j * (nx + 1) + i).copied().unwrap_or(Color32::BLACK));
        }
    }
    let row = (nx + 1) as u32;
    for j in 0..ny as u32 {
        for i in 0..nx as u32 {
            let a = j * row + i;
            mesh.add_triangle(a, a + 1, a + row + 1);
            mesh.add_triangle(a, a + row + 1, a + row);
        }
    }
    mesh
}
