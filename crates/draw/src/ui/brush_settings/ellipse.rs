//! Photoshop's tip angle and roundness widget ([`TipEllipse`]) and its geometry.

use craft_ui::controls::slider::{KEY_FILTER, pointer, tap};
use craft_ui::edit::{EditPolicy, EditResponse, Gesture};
use craft_ui::localized::{Labels as _, Units};
use craft_ui::primitives::{FocusRing, Interactive};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use egui::accesskit::{Action, ActionData, Role};
use egui::{AsIdSalt, CornerRadius, IdSalt, Key, Modifiers, PointerButton, Pos2, Sense, Shape, Stroke, Ui, Vec2, vec2};

use super::labels::BrushSettingsLabels;
use super::style::TipEllipseStyle;

/// A tip's angle and roundness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipAngle {
    /// The major axis's angle in whole degrees, counter-clockwise from the right, −180 to 180.
    pub degrees: f64,
    /// The minor axis over the major, 0.01 to 1.
    pub roundness: f64,
}

impl Default for TipAngle {
    fn default() -> Self {
        Self { degrees: 0.0, roundness: 1.0 }
    }
}

/// What a press on the widget grabbed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TipEllipseGrab {
    /// The arrow: a drag turns the angle.
    Angle,
    /// A roundness dot: a drag squashes the ellipse.
    Roundness,
}

/// The two roundness dots, relative to the widget's centre in points (screen y down), for an
/// ellipse of major radius `radius` (`brush.ellipse_radius`, 34): `±minor × radius ×
/// clamp(roundness, 0, 1)`, where the minor axis is the major turned 90° counter-clockwise,
/// `(−sin a, −cos a)`.
pub fn ellipse_handles(angle: TipAngle, radius: f32) -> [Vec2; 2] {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:163-168 `roundness_handles`
    let (sin, cos) = (angle.degrees as f32).to_radians().sin_cos();
    let minor = vec2(-sin, -cos) * radius * (angle.roundness as f32).clamp(0.0, 1.0);
    [minor, -minor]
}

/// What a press at `at` (relative to the centre) grabs: a roundness dot within `reach`
/// (`brush.ellipse_grab`, 7) of either, else the arrow.
pub fn ellipse_grab(at: Vec2, angle: TipAngle, radius: f32, reach: f32) -> TipEllipseGrab {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:171-173 `ellipse_grab` (`<= 7.0`)
    if ellipse_handles(angle, radius).iter().any(|h| (*h - at).length() <= reach) { TipEllipseGrab::Roundness } else { TipEllipseGrab::Angle }
}

/// The value after a drag to `at` (relative to the centre) holding `grab`. Angle: the direction
/// of `at` in whole degrees, counter-clockwise, `atan2(−y, x)` rounded; unchanged within 1 pt of
/// the centre. Roundness: `|at · minor| / radius` clamped to 0.01 to 1; the angle stays.
pub fn ellipse_drag(grab: TipEllipseGrab, at: Vec2, angle: TipAngle, radius: f32) -> TipAngle {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:177-187 `ellipse_drag`
    if !(at.x.is_finite() && at.y.is_finite()) {
        return angle;
    }
    match grab {
        TipEllipseGrab::Angle if at.length() > 1.0 => TipAngle { degrees: f64::from((-at.y).atan2(at.x).to_degrees().round()), ..angle },
        TipEllipseGrab::Angle => angle,
        TipEllipseGrab::Roundness => {
            let (sin, cos) = (angle.degrees as f32).to_radians().sin_cos();
            let along = (at.x * -sin + at.y * -cos).abs();
            TipAngle { roundness: f64::from(along / radius.max(f32::EPSILON)).clamp(0.01, 1.0), ..angle }
        }
    }
}

/// Photoshop's angle and roundness widget: a disc with cross lines, the tip's ellipse, an arrow
/// along the major axis and two dots on the minor axis. Drag the arrow (anywhere off the dots) to
/// turn the angle, a dot to set the roundness. Reports [`EditEvent<TipAngle>`](craft_ui::edit::EditEvent):
/// a drag is one gesture (`Begin`, `Update`, `Commit`; Esc cancels and the value goes back); a
/// click on the disc is a `Set`.
///
/// - **Look.** `brush.ellipse_size` (84) square; the disc radius `brush.ellipse_disc` (40) on
///   `brush.ellipse_disc_fill` with a 1 pt `brush.ellipse_disc_border` and `line.divider` cross lines; the ellipse
///   `brush.ellipse_radius` (34) filled `text.default` at 18 % with a `brush.ellipse_line` (1.5)
///   `text.default` outline (48 segments); an `accent` arrow of the same width to
///   `brush.ellipse_arrow_reach` (5) past the ellipse, its head `brush.ellipse_head` (7 long, 8
///   wide); the dots `brush.ellipse_dot` (4) in `overlay.handle_fill` with a 1 pt `accent` ring.
///   Focus ring around the disc.
/// - **Keys.** One Tab stop. Up and Right turn the angle 1° counter-clockwise, Down and Left 1°
///   clockwise (Shift: 15°), wrapping at ±180; Alt with the arrows changes the roundness by 1 %
///   (Up and Right rounder). Each press is a `Set`.
/// - **AccessKit.** A slider "Tip angle and roundness" with the value "{angle}, {roundness}"
///   (the angle in degrees, the roundness in percent, through the unit words); the arrow keys as
///   Increment and Decrement of the angle, and SetValue sets the angle in whole degrees within
///   ±180, keeping the roundness.
/// - **RTL.** Not mirrored: it shows document geometry.
#[must_use = "show the widget with `show`"]
pub struct TipEllipse {
    id_salt: IdSalt,
    value: TipAngle,
    labels: Option<BrushSettingsLabels>,
}

impl TipEllipse {
    /// The widget for `value`.
    pub fn new(id_salt: impl AsIdSalt, value: TipAngle) -> Self {
        Self { id_salt: IdSalt::new(id_salt), value, labels: None }
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The sizes and colors of the widget from `tokens`.
    pub fn style(tokens: &UiTokens) -> TipEllipseStyle {
        // pc: crates/ui-egui/src/panels/brush_sections.rs:192-242.
        let (b, c) = (&tokens.component.brush, &tokens.color);
        TipEllipseStyle {
            size: b.ellipse_size,
            disc: b.ellipse_disc,
            radius: b.ellipse_radius,
            dot: b.ellipse_dot,
            grab: b.ellipse_grab,
            line: b.ellipse_line,
            arrow_reach: b.ellipse_arrow_reach,
            head: Vec2::from(b.ellipse_head),
            disc_fill: b.ellipse_disc_fill.to_egui(),
            disc_border: Stroke::new(1.0, b.ellipse_disc_border.to_egui()),
            cross: Stroke::new(1.0, c.line.divider.to_egui()),
            ellipse: c.text.normal.to_egui(),
            accent: c.accent.solid.to_egui(),
            dot_fill: c.overlay.handle_fill.to_egui(),
        }
    }

    /// Shows the widget.
    pub fn show(self, ui: &mut Ui) -> EditResponse<TipAngle> {
        // pc: crates/ui-egui/src/panels/brush_sections.rs:190-246 `ellipse_widget` (84 square, disc 40, ellipse 34 at 18 %, arrow +5 with a head 7 long and 8 wide, dots 4)
        let labels = self.labels.unwrap_or_else(|| BrushSettingsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let enabled = ui.is_enabled();
        let sensed = Interactive::salted(self.id_salt, Role::Slider, labels.tip_angle.as_str())
            .sense(Sense::drag())
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact(ui, Vec2::splat(style.size));
        let response = sensed.response().clone();
        let center = response.rect.center();
        let id = response.id;
        let grab_key = id.with("grab");

        let mut gesture = Gesture::load(ui, id, &self.value, EditPolicy::default()).event_filter(KEY_FILTER).enabled(enabled);
        let drag_to = |gesture: &mut Gesture<TipAngle>, grab: TipEllipseGrab, pos: Pos2| {
            let to = ellipse_drag(grab, pos - center, *gesture.shown(), style.radius);
            gesture.update(to, ui.input(|i| i.modifiers));
        };
        let grab_of = |ui: &Ui| ui.data(|d| d.get_temp::<TipEllipseGrab>(grab_key)).unwrap_or(TipEllipseGrab::Angle);
        if response.drag_started_by(PointerButton::Primary) {
            response.request_focus();
            // The grab is decided where the button went down, not where the drag was first noticed.
            if let Some((_, origin)) = pointer(ui, &response) {
                let grab = ellipse_grab(origin - center, *gesture.shown(), style.radius, style.grab);
                ui.data_mut(|d| d.insert_temp(grab_key, grab));
            }
            gesture.begin();
        }
        if response.dragged_by(PointerButton::Primary)
            && gesture.is_active()
            && let Some(pos) = response.interact_pointer_pos()
        {
            drag_to(&mut gesture, grab_of(ui), pos);
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        if let Some(pos) = tap(ui, &response) {
            response.request_focus();
            let now = *gesture.shown();
            let grab = ellipse_grab(pos - center, now, style.radius, style.grab);
            gesture.set(ellipse_drag(grab, pos - center, now, style.radius));
        }
        if !response.dragged()
            && let Some(to) = keyed(ui, &response, *gesture.shown())
        {
            gesture.set(to);
        }
        if !response.dragged()
            && let Some(degrees) = set_value(ui, &response)
        {
            gesture.set(TipAngle { degrees: degrees.round().clamp(-180.0, 180.0), ..*gesture.shown() });
        }

        let shown = *gesture.shown();
        let units = Units::of(ui);
        let text = craft_i18n::fmt(
            &labels.tip_angle_value,
            &[("angle", &units.degrees(shown.degrees)), ("roundness", &units.percent((shown.roundness * 100.0).round()))],
        );
        let response = sensed.numeric(shown.degrees, -180.0..=180.0, 1.0).value(text).finish(ui);
        if ui.is_rect_visible(response.rect) {
            paint(ui, &style, center, shown, enabled);
        }
        let disc = egui::Rect::from_center_size(center, Vec2::splat(style.disc * 2.0));
        FocusRing::paint_radius(ui, disc, CornerRadius::same(style.disc.round() as u8), &response);
        let _ = response.clone().on_hover_text(labels.tip_angle_tip.as_str());
        EditResponse::new(response, gesture.finish())
    }
}

/// The last finite SetValue a screen reader sent to the enabled widget this frame.
fn set_value(ui: &Ui, response: &egui::Response) -> Option<f64> {
    if !response.enabled() {
        return None;
    }
    ui.input(|i| {
        i.accesskit_action_requests(response.id, Action::SetValue)
            .filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                _ => None,
            })
            .next_back()
    })
}

/// The value after this frame's keys and screen-reader steps on the focused widget, or `None`.
fn keyed(ui: &Ui, response: &egui::Response, value: TipAngle) -> Option<TipAngle> {
    if !response.enabled() {
        return None;
    }
    let focused = ui.memory(|m| m.has_focus(response.id));
    let (mut turn, mut squash) = (0.0_f64, 0.0_f64);
    if focused {
        ui.input_mut(|i| {
            let mut count = |modifiers, key| f64::from(i.count_and_consume_key(modifiers, key) as u32);
            for (key, sign) in [(Key::ArrowUp, 1.0), (Key::ArrowRight, 1.0), (Key::ArrowDown, -1.0), (Key::ArrowLeft, -1.0)] {
                // Alt first: it is the roundness. Shift before none: `Modifiers::NONE` matches a held Shift.
                squash += sign * count(Modifiers::ALT, key);
                turn += sign * (15.0 * count(Modifiers::SHIFT, key) + count(Modifiers::NONE, key));
            }
        });
    }
    ui.input(|i| {
        let requests = |action| i.num_accesskit_action_requests(response.id, action) as f64;
        turn += requests(Action::Increment) - requests(Action::Decrement);
    });
    if turn == 0.0 && squash == 0.0 {
        return None;
    }
    let mut degrees = value.degrees + turn;
    if degrees > 180.0 {
        degrees -= 360.0;
    } else if degrees < -180.0 {
        degrees += 360.0;
    }
    let roundness = (value.roundness + squash / 100.0).clamp(0.01, 1.0);
    Some(TipAngle { degrees: degrees.round(), roundness: (roundness * 100.0).round() / 100.0 })
}

fn paint(ui: &Ui, style: &TipEllipseStyle, center: Pos2, value: TipAngle, enabled: bool) {
    let fade = |c: egui::Color32| if enabled { c } else { c.gamma_multiply(0.5) };
    let painter = ui.painter();
    painter.circle_filled(center, style.disc, fade(style.disc_fill));
    painter.circle_stroke(center, style.disc, Stroke::new(style.disc_border.width, fade(style.disc_border.color)));
    let cross = Stroke::new(style.cross.width, fade(style.cross.color));
    painter.line_segment([center - vec2(style.disc, 0.0), center + vec2(style.disc, 0.0)], cross);
    painter.line_segment([center - vec2(0.0, style.disc), center + vec2(0.0, style.disc)], cross);
    let (sin, cos) = (value.degrees as f32).to_radians().sin_cos();
    let (major, minor) = (vec2(cos, -sin), vec2(-sin, -cos));
    let round = (value.roundness as f32).clamp(0.01, 1.0);
    let ring = fade(style.ellipse);
    let points: Vec<Pos2> = (0..48)
        .map(|i| {
            let t = i as f32 / 48.0 * std::f32::consts::TAU;
            center + major * t.cos() * style.radius + minor * t.sin() * style.radius * round
        })
        .collect();
    painter.add(Shape::convex_polygon(points, ring.gamma_multiply(0.18), Stroke::new(style.line, ring)));
    let accent = fade(style.accent);
    let tip = center + major * (style.radius + style.arrow_reach);
    painter.line_segment([center, tip], Stroke::new(style.line, accent));
    let back = tip - major * style.head.x;
    let half = style.head.y / 2.0;
    painter.add(Shape::convex_polygon(vec![tip, back + minor * half, back - minor * half], accent, Stroke::NONE));
    for handle in ellipse_handles(value, style.radius) {
        painter.circle_filled(center + handle, style.dot, fade(style.dot_fill));
        painter.circle_stroke(center + handle, style.dot, Stroke::new(1.0, accent));
    }
}
