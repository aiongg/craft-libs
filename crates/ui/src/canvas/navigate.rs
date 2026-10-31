//! Navigation over a canvas, as in Photoshop: the wheel, trackpad pinch and twist, the page keys,
//! a Hand drag's flick, autoscroll during a tool drag, and Rotate View with its compass. The
//! user's choices are [`NavigationPrefs`] (Photoshop's Preferences › General, Tools and Enhanced
//! Controls rows).
//!
//! Each piece reads one pass's input and changes a [`CanvasView`] the app copies back into its
//! own view; zooms go through [`zoom`](super::zoom) and clamp to [`ZoomLadder`](super::ZoomLadder).
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasView, navigate::{NavigationPrefs, WheelNavigation}, kurbo::Size};
//! # fn demo(ui: &egui::Ui, canvas: &egui::Response, view: &mut CanvasView, prefs: &NavigationPrefs) {
//! if let Some(action) = WheelNavigation::read(ui.ctx(), canvas, prefs) {
//!     let pointer = canvas.hover_pos().unwrap_or(view.rect.center());
//!     WheelNavigation::apply(view, action, pointer, Size::new(2000.0, 1500.0), prefs, ui.ctx().pixels_per_point());
//! }
//! # }
//! ```

use egui::accesskit::Role;
use egui::{
    Align2, Context, Event, Id, InputState, Key, Modifiers, MouseWheelUnit, Painter, Pos2, RawInput, Rect as ScreenRect, Response, Sense, Stroke,
    TouchPhase, Ui, Vec2 as ScreenVec, WidgetInfo,
};
use kurbo::{Point, Rect, Size};

use super::cursor::ToolCursor;
use super::scrollbars::CanvasScrollbars;
use super::tool_host::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPointer};
use super::view::CanvasView;
use super::zoom::{self, ZoomAnimation, ZoomLadder};
use crate::localized::Labels;
use crate::theme::{ToEgui, UiTokens};
use craft_theme::TypeRole;

/// The navigation preferences, with Photoshop's defaults. The app stores them (craft-settings)
/// and shows them in its Preferences; every navigation piece reads them from here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationPrefs {
    /// General › Zoom with Scroll Wheel: the wheel zooms and Alt + wheel scrolls. Off.
    pub zoom_with_wheel: bool,
    /// General › Animated Zoom: wheel and command zooms ease, and a held Zoom tool press zooms
    /// continuously. On.
    pub animated_zoom: bool,
    /// General › Zoom Resizes Windows (also the Zoom tool's Resize Windows to Fit). Off.
    pub zoom_resizes_windows: bool,
    /// General › Zoom Clicked Point to Center: a Zoom tool click centres the clicked point. Off.
    pub zoom_clicked_point_to_center: bool,
    /// Tools › Enable Flick Panning: a Hand drag released while moving glides on. On.
    pub flick_panning: bool,
    /// Tools › Overscroll: the view may scroll the document past the canvas edge. Off.
    pub overscroll: bool,
    /// The Zoom tool's Scrubby Zoom option: a Zoom tool drag zooms continuously instead of
    /// drawing a rectangle. On.
    pub scrubby_zoom: bool,
    /// Enhanced Controls › Zoom with Trackpad Pinch. On.
    pub pinch_zooms: bool,
    /// Enhanced Controls › Rotate View with Trackpad. Off.
    pub trackpad_rotate: bool,
    /// Enhanced Controls › Enable Gestures: a touch screen's two-finger pinch and twist. On.
    pub touch_gestures: bool,
}

impl Default for NavigationPrefs {
    // pc: crates/engine/src/prefs.rs:383 `enable_flick_panning`, :392 `overscroll`, :922 Enhanced Controls defaults.
    fn default() -> Self {
        Self {
            zoom_with_wheel: false,
            animated_zoom: true,
            zoom_resizes_windows: false,
            zoom_clicked_point_to_center: false,
            flick_panning: true,
            overscroll: false,
            scrubby_zoom: true,
            pinch_zooms: true,
            trackpad_rotate: false,
            touch_gestures: true,
        }
    }
}

/// What the wheel or a pinch does to the view this pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelAction {
    /// Multiply the zoom by this factor about the pointer (×1.1 a notch).
    Zoom(f64),
    /// Step this many ladder rungs (positive in, negative out).
    SnapZoom(i32),
    /// Move the image by this many screen points (egui's content direction).
    Pan(ScreenVec),
}

/// One pass of wheel input over the canvas, read from egui's input by [`WheelNavigation::read`]
/// or built by a test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInput {
    /// Smoothed scroll delta (points, content direction): what panning uses, and what an eased
    /// wheel zoom follows.
    pub scroll: ScreenVec,
    /// This pass's wheel events unsmoothed: what an instant zoom and the snap zoom use.
    pub raw: ScreenVec,
    /// egui's pinch factor (1: none).
    pub zoom_delta: f32,
    /// Alt was held for the wheel gesture in progress (kept until the next wheel event, so a
    /// release during the smoothing tail does not turn it into a pan).
    pub alt: bool,
    /// Shift was held for the gesture.
    pub shift: bool,
    /// Points per wheel notch (egui's `line_scroll_speed`).
    pub notch: f32,
}

/// Mouse wheel and trackpad navigation, as in Photoshop:
///
/// - The wheel scrolls up and down; Shift or Cmd/Ctrl + wheel scrolls sideways.
/// - Alt + wheel zooms about the pointer, ×1.1 a notch; Shift + Alt + wheel steps the ladder.
/// - Zoom with Scroll Wheel swaps them: the wheel zooms, Alt + wheel scrolls, Shift + wheel steps
///   the ladder.
/// - A trackpad pinch zooms about the pointer (Zoom with Trackpad Pinch); a touch screen's pinch
///   needs Enable Gestures too.
/// - Animated Zoom eases a notch's ×1.1 over several passes (the factors multiply to ×1.1);
///   off, a notch is one instant step.
pub struct WheelNavigation;

impl WheelNavigation {
    /// The zoom factor of one wheel notch.
    pub const NOTCH: f64 = 1.1;

    /// The view change for one pass of `input` under `prefs`, if any.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:85 `classify`.
    pub fn classify(input: WheelInput, prefs: &NavigationPrefs) -> Option<WheelAction> {
        let notch = if input.notch.is_finite() && input.notch > 0.0 { input.notch } else { 40.0 };
        if prefs.pinch_zooms && input.zoom_delta.is_finite() && input.zoom_delta > 0.0 && input.zoom_delta != 1.0 {
            return Some(WheelAction::Zoom(f64::from(input.zoom_delta)));
        }
        // One notch (`notch` points) is one x1.1 step.
        let step = |dy: f32| {
            let f = Self::NOTCH.powf(f64::from(dy / notch));
            (f.is_finite() && f > 0.0 && f != 1.0).then_some(WheelAction::Zoom(f))
        };
        let snap = |dy: f32| {
            let n = dy / notch;
            if dy == 0.0 || !n.is_finite() {
                return None;
            }
            let steps = n.clamp(-10.0, 10.0).round() as i32;
            (steps != 0).then_some(WheelAction::SnapZoom(steps))
        };
        let scroll_finite = input.scroll.x.is_finite() && input.scroll.y.is_finite();
        match (input.alt, prefs.zoom_with_wheel) {
            // Shift + Alt + wheel: snap to the ladder instead of x1.1.
            (true, false) if input.shift => snap(input.raw.y + input.raw.x),
            // Alt + wheel with Animated Zoom: follow the smoothed delta across passes.
            (true, false) if prefs.animated_zoom && scroll_finite => step(input.scroll.y + input.scroll.x),
            // Without it: one instant step from the wheel events, ignoring the smoothed tail.
            (true, false) if !prefs.animated_zoom => step(input.raw.y + input.raw.x),
            // Shift + wheel with Zoom with Scroll Wheel: some systems deliver it sideways (x, y = 0).
            (false, true) if input.shift => snap(input.raw.y + input.raw.x),
            // Cmd/Ctrl folds the wheel into x: those still scroll sideways.
            (false, true) if prefs.animated_zoom && scroll_finite && input.scroll.y != 0.0 => step(input.scroll.y),
            (false, true) if !prefs.animated_zoom && (input.raw.y != 0.0 || input.scroll.y != 0.0) => step(input.raw.y),
            _ if !scroll_finite || input.scroll == ScreenVec::ZERO => None,
            _ => Some(WheelAction::Pan(input.scroll)),
        }
    }

    /// This pass's wheel input over `canvas` (hovered, no popup above it) classified under
    /// `prefs`, or `None`. Consumes the scroll so a parent scroll area does not also move.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:338 `read`, :350 `read_with`, :502 `wheel_events`,
    // :528 `pinch_factor`, :245 `ease_zoom_delta`.
    pub fn read(ctx: &Context, canvas: &Response, prefs: &NavigationPrefs) -> Option<WheelAction> {
        let owner = canvas.id;
        let notch = ctx.options(|o| o.input_options.line_scroll_speed);
        let (scroll, zoom_delta, latest_alt, latest_shift, wheel, page) = ctx.input(|i| {
            let wheel = wheel_events(i);
            let latest_alt = wheel.last().map(|(_, _, m, _)| m.alt && !m.command && !m.ctrl);
            let latest_shift = wheel.last().map(|(_, _, m, _)| m.shift && !m.command && !m.ctrl);
            (i.smooth_scroll_delta, pinch_factor(i, prefs.touch_gestures), latest_alt, latest_shift, wheel, i.content_rect().height())
        });
        let wheel = mark_unsmoothed(ctx, wheel);
        // Alt and Shift follow the latest wheel event and stay until the next one.
        let alt = remembered(ctx, alt_id(), latest_alt);
        let shift = remembered(ctx, shift_id(), latest_shift);
        if !canvas.hovered() {
            return None;
        }
        // The events of the gesture in progress: Shift + Alt, Alt, Shift or plain.
        let (raw, instant) = wheel
            .iter()
            .filter(|(_, _, m, _)| {
                let plain = !m.command && !m.ctrl;
                if alt && shift {
                    m.alt && m.shift && plain
                } else if alt {
                    m.alt && !m.shift && plain
                } else if shift {
                    m.shift && !m.alt && plain
                } else {
                    !m.alt && !m.shift && plain
                }
            })
            .fold((ScreenVec::ZERO, ScreenVec::ZERO), |(sum, instant), (unit, delta, _, unsmoothed)| {
                let d = *delta
                    * match unit {
                        MouseWheelUnit::Point => 1.0,
                        MouseWheelUnit::Line => notch,
                        MouseWheelUnit::Page => page,
                    };
                (sum + d, if *unsmoothed { instant + d } else { instant })
            });
        let snap_mode = shift && (alt != prefs.zoom_with_wheel);
        let has_pinch = prefs.pinch_zooms && zoom_delta.is_finite() && zoom_delta > 0.0 && zoom_delta != 1.0;
        let action = if snap_mode && !has_pinch {
            // Shift snap zoom takes over: an unfinished eased notch is dropped.
            ctx.data_mut(|d| d.remove::<ZoomEase>(zoom_ease_id(owner)));
            snap_steps(ctx, owner, raw.y + raw.x, notch).map(WheelAction::SnapZoom)
        } else {
            if !snap_mode {
                ctx.data_mut(|d| {
                    d.remove::<f32>(snap_residual_id(owner));
                    d.remove::<f64>(snap_time_id(owner));
                });
            }
            // Zoom-mode input eases from this mode's own wheel events: egui's smoothed delta has no
            // notion of modifiers and would carry the tail of a pan notch into the zoom.
            let wheel_zooms = !shift && (alt != prefs.zoom_with_wheel);
            let scroll = if prefs.animated_zoom && wheel_zooms {
                let along = |v: ScreenVec| if alt { v.y + v.x } else { v.y };
                let eased = ease_zoom_delta(ctx, owner, along(raw - instant)) + along(instant);
                // The sideways part of the plain wheel is not a zoom event: it pans.
                if alt { ScreenVec::new(0.0, eased) } else { ScreenVec::new(scroll.x, eased) }
            } else {
                ctx.data_mut(|d| d.remove::<ZoomEase>(zoom_ease_id(owner)));
                scroll
            };
            Self::classify(WheelInput { scroll, raw, zoom_delta, alt, shift, notch }, prefs)
        };
        if action.is_some() {
            ctx.input_mut(|i| i.smooth_scroll_delta = ScreenVec::ZERO);
        }
        action
    }

    /// Apply `action` to `view` with the pointer at `pointer`: a factor zoom anchors at
    /// [`zoom::wheel_anchor`](super::zoom::wheel_anchor), a ladder step at the pointer, and both clamp to the ladder's range; a
    /// pan moves the view (within the scroll limits unless `prefs.overscroll`). At a zoom limit
    /// the view stays put, as in Photoshop: a notch past 12800 % neither zooms nor slides the
    /// image toward the pointer. Returns whether the view changed.
    // pc: crates/ui-egui/src/canvas.rs:588 (`Zoom`: clamp, about `wheel_anchor`), :597 (`SnapZoom`: n ladder steps).
    pub fn apply(view: &mut CanvasView, action: WheelAction, pointer: Pos2, size: Size, prefs: &NavigationPrefs, pixels_per_point: f32) -> bool {
        let before = *view;
        match action {
            WheelAction::Zoom(factor) => {
                let current = zoom::device_zoom(view, pixels_per_point);
                let next = ZoomLadder::clamp(current * factor, size);
                // At a limit the view stays put: no zoom, no slide toward the pointer.
                if factor.is_finite() && factor > 0.0 && (next / current - 1.0).abs() > 1e-12 {
                    let at = zoom::wheel_anchor(view, pointer, size);
                    zoom::zoom_about(view, at, next, false, pixels_per_point);
                }
            }
            WheelAction::SnapZoom(steps) => {
                let dir = steps.signum();
                let current = zoom::device_zoom(view, pixels_per_point);
                let next = (0..steps.unsigned_abs()).fold(current, |z, _| ZoomLadder::step(z, dir, size));
                if dir != 0 && (next / current - 1.0).abs() > 1e-12 {
                    zoom::zoom_about(view, pointer, next, false, pixels_per_point);
                }
            }
            WheelAction::Pan(delta) => {
                pan(view, delta);
                if !prefs.overscroll {
                    clamp_to_document(view, size);
                }
            }
        }
        *view != before
    }

    /// Drop the unfinished eased notch of `canvas`. Call it when a [`WheelAction::Zoom`] was
    /// stopped by a zoom limit ([`apply`](Self::apply) left the zoom short of the factor), so the
    /// rest of the notch does not cancel the first notch back out of the limit.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:`settle`.
    pub fn settle(ctx: &Context, canvas: &Response) {
        let id = zoom_ease_id(canvas.id);
        ctx.data_mut(|d| {
            if let Some(e) = d.get_temp::<ZoomEase>(id) {
                d.insert_temp(id, ZoomEase { residual: 0.0, ..e });
            }
        });
    }

    /// Set egui's wheel modifiers once at startup: Cmd/Ctrl + wheel scrolls sideways like Shift +
    /// wheel instead of zooming, everywhere in the app, so only a real pinch reaches
    /// `zoom_delta`.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:128 `configure`.
    pub fn configure(ctx: &Context) {
        ctx.options_mut(|o| {
            o.input_options.zoom_modifier = Modifiers::NONE;
            o.input_options.horizontal_scroll_modifier = Modifiers::SHIFT | Modifiers::COMMAND;
        });
    }

    /// Turn Windows' (and some Wayland systems') Ctrl + wheel fractions of a notch back into a
    /// pinch before egui sees them, so Ctrl + a notched wheel scrolls sideways and a precision
    /// touchpad pinch still zooms. Call from the app's `raw_input_hook`.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:170 `fold_legacy_pinch`.
    pub fn fold_legacy_pinch(ctx: &Context, raw: &mut RawInput) {
        // Native gesture events carry their own magnification; folding a Ctrl-wheel event the backend
        // sent beside them would apply the pinch twice. They also end any legacy continuation.
        if raw.events.iter().any(|event| matches!(event, Event::Zoom(_))) {
            ctx.data_mut(|d| d.remove::<f64>(legacy_pinch_id()));
            return;
        }
        let (notch, speed) = ctx.options(|o| (o.input_options.line_scroll_speed, o.input_options.scroll_zoom_speed));
        let now = raw.time.unwrap_or_else(|| ctx.input(|i| i.time));
        let last = ctx.data(|d| d.get_temp::<f64>(legacy_pinch_id()));
        let latest = legacy_pinch_to_zoom(&mut raw.events, now, last, notch, speed);
        if latest != last
            && let Some(t) = latest
        {
            ctx.data_mut(|d| d.insert_temp(legacy_pinch_id(), t));
        }
    }
}

/// What a page key does to the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageKey {
    /// Scroll by this many screen points (positive: show what is below or to the right).
    Scroll(ScreenVec),
    /// Show the document's top-left corner (Home) or bottom-right corner (End).
    Corner { end: bool },
}

impl PageKey {
    /// Points a Shift + page key scrolls.
    pub const SMALL_STEP: f32 = 10.0;

    /// Photoshop's page keys: Page Up and Page Down a canvas height up or down, Cmd/Ctrl turns
    /// them sideways (Page Up left, Page Down right), Shift makes the step
    /// [`SMALL_STEP`](Self::SMALL_STEP); Home and End show the corners. `None` for other keys,
    /// with Alt, or Shift on Home and End. `canvas` is the canvas size in points.
    // pc: crates/ui-egui/src/input/page_keys.rs:24 `page_key`.
    pub fn of(key: Key, modifiers: Modifiers, canvas: ScreenVec) -> Option<PageKey> {
        if modifiers.alt {
            return None;
        }
        let sign = match key {
            Key::PageUp => -1.0,
            Key::PageDown => 1.0,
            Key::Home | Key::End if !modifiers.shift && !modifiers.command => return Some(PageKey::Corner { end: key == Key::End }),
            _ => return None,
        };
        let sideways = modifiers.command;
        let step = match (modifiers.shift, sideways) {
            (true, _) => Self::SMALL_STEP,
            (false, true) => canvas.x,
            (false, false) => canvas.y,
        };
        Some(PageKey::Scroll(if sideways { ScreenVec::new(sign * step, 0.0) } else { ScreenVec::new(0.0, sign * step) }))
    }

    /// Consume this pass's page keys while the canvas has the keyboard (no text field focused,
    /// no binding of the app's took them) and apply them to `view` within the scroll limits.
    /// Returns whether one ran.
    // pc: crates/ui-egui/src/input/page_keys.rs:44 `handle`.
    pub fn handle(ctx: &Context, view: &mut CanvasView, size: Size, overscroll: bool) -> bool {
        if ctx.egui_wants_keyboard_input() {
            return false;
        }
        let canvas = view.rect.size();
        let presses: Vec<PageKey> = ctx.input_mut(|input| {
            let mut out = Vec::new();
            input.events.retain(|e| match e {
                Event::Key { key, pressed: true, modifiers, .. } => match PageKey::of(*key, *modifiers, canvas) {
                    Some(p) => {
                        out.push(p);
                        false
                    }
                    None => true,
                },
                _ => true,
            });
            out
        });
        if presses.is_empty() {
            return false;
        }
        for key in presses {
            key.apply(view, size, overscroll);
        }
        ctx.request_repaint();
        true
    }

    /// Apply this key to `view` for a document of `size`: a scroll within the limits, or the
    /// corner at the canvas's matching corner. The zoom stays.
    pub fn apply(self, view: &mut CanvasView, size: Size, overscroll: bool) -> bool {
        let before = *view;
        match self {
            PageKey::Scroll(delta) => {
                let d = view.unmap_vec(delta);
                if d.x.is_finite() && d.y.is_finite() {
                    view.center += d;
                }
            }
            PageKey::Corner { end } => {
                // The document corner goes to the canvas corner the document runs on from: the top-left
                // one in an upright view, another when it is turned or mirrored.
                let (corner, inward) = if end {
                    (Point::new(size.width, size.height), kurbo::Vec2::new(-1.0, -1.0))
                } else {
                    (Point::ORIGIN, kurbo::Vec2::new(1.0, 1.0))
                };
                let dir = view.map_vec(inward);
                let canvas = view.rect.size();
                let at = ScreenVec::new(-dir.x.signum() * canvas.x / 2.0, -dir.y.signum() * canvas.y / 2.0);
                let center = corner - view.unmap_vec(at);
                if center.x.is_finite() && center.y.is_finite() {
                    view.center = center;
                }
            }
        }
        if !overscroll {
            clamp_to_document(view, size);
        }
        *view != before
    }
}

/// Move `view` by `delta` screen points (a Hand drag, a scroll): the document follows the
/// pointer, in any rotation or mirror.
pub fn pan(view: &mut CanvasView, delta: ScreenVec) {
    let d = view.unmap_vec(delta);
    if d.x.is_finite() && d.y.is_finite() {
        view.center -= d;
    }
}

/// Flick panning: releasing a Hand drag (the selected Hand, the Space Hand or a middle-button
/// drag) while the pointer still moves keeps the view gliding that way, slowing to a stop. A
/// click, a new drag or a key stops it. Kept by the canvas in egui memory per view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlickPan {
    /// Recent drag steps: (time, pass interval, delta).
    samples: Vec<(f64, f32, ScreenVec)>,
    /// A Hand drag was in progress last pass.
    dragging: bool,
    /// The glide's speed in points per second, while gliding.
    velocity: Option<ScreenVec>,
    /// Time of the previous pass.
    last: Option<f64>,
}

impl FlickPan {
    /// Pointer movement this recent (seconds) sets the release velocity.
    const WINDOW: f64 = 0.08;
    /// Time constant of the exponential friction (seconds).
    const TAU: f32 = 0.18;
    /// A release slower than this (points per second) does not glide.
    const MIN_START: f32 = 150.0;
    /// The glide stops below this speed.
    const MIN_SPEED: f32 = 15.0;
    /// Fastest glide; a glitchy delta cannot fling the view away.
    const MAX_SPEED: f32 = 6000.0;
    /// Longest pass step the glide integrates.
    const MAX_DT: f32 = 0.1;

    /// Seconds since the previous pass, within `0..=MAX_DT`; records `now`.
    fn tick(&mut self, now: f64) -> f32 {
        let dt = self.last.map_or(0.0, |l| (now - l) as f32);
        self.last = Some(now);
        if dt.is_finite() { dt.clamp(0.0, Self::MAX_DT) } else { 0.0 }
    }

    fn track(&mut self, now: f64, dt: f32, delta: ScreenVec) {
        self.velocity = None;
        self.dragging = true;
        if delta.is_finite() && dt.is_finite() {
            self.samples.push((now, dt, delta));
        }
        self.samples.retain(|s| now - s.0 <= Self::WINDOW);
    }

    /// Pointer velocity over the last `WINDOW` seconds before `now`.
    fn release_velocity(&self, now: f64) -> ScreenVec {
        let (mut d, mut t) = (ScreenVec::ZERO, 0.0f32);
        for &(at, dt, delta) in &self.samples {
            if now - at <= Self::WINDOW {
                d += delta;
                t += dt;
            }
        }
        if t <= 1e-4 {
            return ScreenVec::ZERO;
        }
        let v = d / t;
        if !v.is_finite() {
            return ScreenVec::ZERO;
        }
        let speed = v.length();
        if speed > Self::MAX_SPEED { v * (Self::MAX_SPEED / speed) } else { v }
    }

    fn release(&mut self, now: f64, enabled: bool) {
        self.dragging = false;
        let v = self.release_velocity(now);
        self.samples.clear();
        self.velocity = (enabled && v.length() >= Self::MIN_START).then_some(v);
    }

    /// The distance the glide covers in `dt` seconds (the exact integral of the decaying speed).
    fn advance(&mut self, dt: f32) -> Option<ScreenVec> {
        let v = self.velocity?;
        let dt = if dt.is_finite() { dt.clamp(0.0, Self::MAX_DT) } else { 0.0 };
        let decay = (-dt / Self::TAU).exp();
        let moved = v * (Self::TAU * (1.0 - decay));
        let next = v * decay;
        self.velocity = (next.is_finite() && next.length() >= Self::MIN_SPEED).then_some(next);
        moved.is_finite().then_some(moved)
    }

    fn clear(&mut self) {
        self.velocity = None;
        self.samples.clear();
    }

    /// Feed one pass of a Hand drag (`dragging`, its `delta` in points) at `time` with the
    /// preference `enabled`; returns the glide to apply to the view this pass after the
    /// release, and requests a repaint while it glides. State under `id`.
    // pc: crates/ui-egui/src/prefs/flick_pan.rs `frame`, `Flick::{track, release, step}`.
    pub fn frame(ctx: &Context, id: Id, dragging: bool, delta: ScreenVec, enabled: bool) -> Option<ScreenVec> {
        let (now, interrupt) = ctx.input(|i| {
            let key = i.events.iter().any(|e| matches!(e, Event::Key { pressed: true, .. }));
            (i.time, i.pointer.any_pressed() || key)
        });
        let mut flick = ctx.data(|d| d.get_temp::<FlickPan>(id)).unwrap_or_default();
        let dt = flick.tick(now);
        let moved = if dragging {
            flick.track(now, dt, delta);
            None
        } else {
            if flick.dragging {
                flick.release(now, enabled);
            }
            if interrupt || !enabled {
                flick.clear();
            }
            flick.advance(dt)
        };
        // A moving drag asks for one more pass, so its release is seen even if nothing else repaints.
        if flick.velocity.is_some() || (dragging && delta != ScreenVec::ZERO) {
            ctx.request_repaint();
        }
        ctx.data_mut(|d| d.insert_temp(id, flick));
        moved
    }

    /// Stop a glide (the view hit a limit, a click).
    // pc: crates/ui-egui/src/prefs/flick_pan.rs:99 `stop`.
    pub fn stop(ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.get_temp_mut_or_default::<FlickPan>(id).clear());
    }
}

/// Touchpad momentum where the platform leaves it to the app (Wayland, which reports a lift as
/// the end of the scroll): a two-finger swipe keeps gliding after the fingers lift. Off on
/// macOS, which sends its own momentum, and on the web.
#[derive(Clone, Debug, PartialEq)]
pub struct TouchpadGlide {
    enabled: bool,
    samples: Vec<(f64, ScreenVec)>,
    /// The glide's speed in points per second (positive moves the content down), or zero.
    velocity: ScreenVec,
    /// When the glide last moved.
    at: f64,
    /// Passes in a row the glide moved the view not at all: it is against an edge.
    stalled: u8,
}

impl TouchpadGlide {
    /// Seconds of the gesture the lift speed is measured over.
    const WINDOW: f64 = 0.1;
    /// A lift this long after the last motion is a finger that stopped first: no glide.
    const STILL: f64 = 0.05;
    /// The glide slows by `e` every this many seconds.
    const TAU: f32 = 0.35;
    /// Below this speed (points per second) a lift does not glide.
    const START_SPEED: f32 = 150.0;
    /// A glide stops below this speed.
    const STOP_SPEED: f32 = 10.0;
    /// Fastest glide, so a lift measured over a hiccup cannot throw the document to its end.
    const MAX_SPEED: f32 = 8000.0;
    /// A pass this long after the last one (the view was hidden, the app stalled) ends a glide.
    const STALE: f64 = 0.1;

    fn halt(&mut self) {
        self.velocity = ScreenVec::ZERO;
        self.stalled = 0;
    }

    fn push(&mut self, now: f64, delta: ScreenVec) {
        if !delta.is_finite() {
            return;
        }
        match self.samples.last_mut() {
            // Several events in one pass are one sample.
            Some((at, d)) if *at == now => *d += delta,
            _ => self.samples.push((now, delta)),
        }
        self.samples.retain(|(at, _)| now - at <= Self::WINDOW);
    }

    /// The finger's speed as it lifted at `now`, or zero when it should not glide.
    fn lift_velocity(&self, now: f64) -> ScreenVec {
        let recent: Vec<&(f64, ScreenVec)> = self.samples.iter().filter(|(at, _)| now - at <= Self::WINDOW).collect();
        let (Some(first), Some(last)) = (recent.first(), recent.last()) else { return ScreenVec::ZERO };
        let span = (last.0 - first.0) as f32;
        if recent.len() < 2 || span <= 0.0 || now - last.0 > Self::STILL {
            return ScreenVec::ZERO;
        }
        // Each sample is the motion since the pass before it, so the first one's motion happened
        // before the span starts.
        let moved = recent.iter().skip(1).fold(ScreenVec::ZERO, |sum, (_, d)| sum + *d);
        let v = moved / span;
        if !v.is_finite() || v.length() < Self::START_SPEED {
            return ScreenVec::ZERO;
        }
        if v.length() > Self::MAX_SPEED { v * (Self::MAX_SPEED / v.length()) } else { v }
    }

    /// A glide that runs where the platform needs it.
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:43 `Default`.
    pub fn new() -> Self {
        Self { enabled: !cfg!(any(target_os = "macos", target_arch = "wasm32")), samples: Vec::new(), velocity: ScreenVec::ZERO, at: 0.0, stalled: 0 }
    }

    /// Read one pass's wheel events at `now`; any wheel input stops a glide, a touchpad
    /// gesture's end may start one. `glide` is false where the view must not glide this pass
    /// (a zoom, a dialog).
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:65 `read`.
    pub fn read(&mut self, events: &[Event], now: f64, glide: bool) {
        for event in events {
            let Event::MouseWheel { unit, delta, phase, modifiers, .. } = event else { continue };
            self.halt();
            // Zooming, a mouse wheel and sideways Shift scrolling never glide.
            if *unit != MouseWheelUnit::Point || modifiers.command || modifiers.ctrl || modifiers.shift {
                self.samples.clear();
                continue;
            }
            match phase {
                TouchPhase::Start => {
                    self.samples.clear();
                    self.push(now, *delta);
                }
                TouchPhase::Move => self.push(now, *delta),
                TouchPhase::End => {
                    if glide && self.enabled {
                        self.velocity = self.lift_velocity(now);
                        self.at = now;
                    }
                    self.samples.clear();
                }
                TouchPhase::Cancel => self.samples.clear(),
            }
        }
    }

    /// The scroll this pass at `now`, in points; zero once stopped.
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:124 `step`.
    pub fn step(&mut self, now: f64) -> ScreenVec {
        if self.velocity == ScreenVec::ZERO {
            return ScreenVec::ZERO;
        }
        let dt = now - self.at;
        if !(0.0..=Self::STALE).contains(&dt) {
            self.halt();
            return ScreenVec::ZERO;
        }
        self.at = now;
        let decay = (-(dt as f32) / Self::TAU).exp();
        let moved = self.velocity * Self::TAU * (1.0 - decay);
        self.velocity *= decay;
        if self.velocity.length() < Self::STOP_SPEED {
            self.halt();
        }
        moved
    }

    /// Whether the view moved last pass; two passes without moving (against an edge) stop the
    /// glide.
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:145 `moved`.
    pub fn moved(&mut self, moved: bool) {
        if moved {
            self.stalled = 0;
        } else {
            self.stalled = self.stalled.saturating_add(1);
            if self.stalled >= 2 {
                self.halt();
            }
        }
    }
}

impl Default for TouchpadGlide {
    fn default() -> Self {
        Self::new()
    }
}

/// Autoscroll while a tool drag runs past the canvas edge (marquee, lasso, brush, crop): the view
/// scrolls toward the pointer, faster the further past the edge, and stops at the scroll limits
/// (an image that fits is never scrolled into empty pasteboard). The tool host runs it for tools
/// whose [`autoscrolls`](super::CanvasTool::autoscrolls) is true.
pub struct Autoscroll;

impl Autoscroll {
    /// Slowest and fastest scroll, points per second.
    pub const MIN_SPEED: f32 = 120.0;
    pub const MAX_SPEED: f32 = 2400.0;
    /// Extra speed per point the pointer is past the edge.
    pub const GAIN: f32 = 12.0;
    /// A long frame scrolls at most this many seconds' worth.
    pub const MAX_DT: f32 = 0.05;

    /// The scroll this pass, in screen points, for a drag with the pointer at `pointer` over a
    /// canvas `rect`: zero inside it, each axis toward the pointer.
    // pc: crates/ui-egui/src/canvas/canvas_autoscroll.rs:21 `edge_step`.
    pub fn edge_step(pointer: Pos2, rect: ScreenRect, dt: f32) -> ScreenVec {
        if !pointer.x.is_finite() || !pointer.y.is_finite() || !dt.is_finite() || dt <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
            return ScreenVec::ZERO;
        }
        let past = |p: f32, lo: f32, hi: f32| {
            if p < lo {
                p - lo
            } else if p > hi {
                p - hi
            } else {
                0.0
            }
        };
        let dt = dt.min(Self::MAX_DT);
        let axis = |d: f32| if d == 0.0 { 0.0 } else { d.signum() * (Self::MIN_SPEED + Self::GAIN * d.abs()).min(Self::MAX_SPEED) * dt };
        ScreenVec::new(axis(past(pointer.x, rect.left(), rect.right())), axis(past(pointer.y, rect.top(), rect.bottom())))
    }

    /// Move `view` by `step` screen points, never past the image's on-screen bounds in the
    /// direction of travel (a view already past a limit is not pulled back). Returns the move
    /// in document units, or `None` when the view did not move.
    // pc: crates/ui-egui/src/canvas/canvas_autoscroll.rs:42 `scroll`.
    pub fn scroll(view: &mut CanvasView, step: ScreenVec, size: Size) -> Option<kurbo::Vec2> {
        if !step.is_finite() || step == ScreenVec::ZERO || !view.zoom.is_finite() || view.zoom <= 0.0 {
            return None;
        }
        let image = document_bounds(view, size)?;
        let canvas = view.rect;
        // Screen axes are compared before unrotating: a document axis that fits before rotation may be
        // clipped afterwards, and a vertical screen drag stays vertical at any angle.
        let axis = |step: f32, lo: f32, hi: f32, v0: f32, v1: f32| {
            if hi - lo <= v1 - v0 {
                0.0
            } else if step > 0.0 {
                step.min((hi - v1).max(0.0))
            } else {
                step.max((lo - v0).min(0.0))
            }
        };
        let step = ScreenVec::new(
            axis(step.x, image.left(), image.right(), canvas.left(), canvas.right()),
            axis(step.y, image.top(), image.bottom(), canvas.top(), canvas.bottom()),
        );
        let d = view.unmap_vec(step);
        let moved = d.x.is_finite() && d.y.is_finite() && (d.x.abs() > 1e-3 || d.y.abs() > 1e-3);
        if moved {
            view.center += d;
        }
        moved.then_some(d)
    }
}

/// The Rotate View tool's drag: the image turns about the view centre following the pointer
/// (clockwise increases the angle; mirrored in a flipped view so it stays under the pointer);
/// Shift snaps to [`SNAP_DEG`](Self::SNAP_DEG). Esc, a double-click, Reset View or the compass
/// put the angle back to 0. The view's rotation is not document history.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotateView {
    pivot: Pos2,
    start_pointer: f64,
    start_rotation: f64,
}

impl RotateView {
    /// Degrees Shift snaps to.
    pub const SNAP_DEG: f64 = 15.0;
    /// The compass's side and its margin from the canvas's top-right corner, in points.
    pub const COMPASS: f32 = 36.0;
    pub const COMPASS_MARGIN: f32 = 10.0;
    /// The compass shows while the view is turned by more than this many degrees.
    const COMPASS_VISIBLE_DEG: f64 = 0.05;

    /// The change of the view's rotation for a pointer or trackpad turn of `delta` screen degrees
    /// (clockwise positive). A mirrored view turns the other way on screen for the same angle,
    /// so the sign flips to keep the image under the pointer.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:`screen_delta`.
    pub fn screen_delta(delta: f64, flip_x: bool) -> f64 {
        let d = Self::wrap_deg(delta);
        if flip_x { Self::wrap_deg(-d) } else { d }
    }

    /// A drag pressed at `screen` on `view`.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:186 `drag` (b.started).
    pub fn begin(view: &CanvasView, screen: Pos2) -> Self {
        let pivot = view.rect.center();
        Self { pivot, start_pointer: pointer_deg(screen, pivot), start_rotation: view.rotation }
    }

    /// The rotation for the pointer at `pointer` (Shift: `snap`), in `(-180, 180]`.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:186 `drag`.
    pub fn drag(&self, pointer: Pos2, snap: bool, flip_x: bool) -> f64 {
        let delta = Self::screen_delta(pointer_deg(pointer, self.pivot) - self.start_pointer, flip_x);
        let mut angle = self.start_rotation + delta;
        if !angle.is_finite() {
            angle = self.start_rotation;
        }
        if snap {
            angle = Self::snap_deg(angle, Self::SNAP_DEG);
        }
        Self::wrap_deg(angle)
    }

    /// `a` degrees wrapped into `(-180, 180]`; not a finite number gives 0.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:42 `wrap_deg`.
    pub fn wrap_deg(a: f64) -> f64 {
        if !a.is_finite() {
            return 0.0;
        }
        let mut x = a % 360.0;
        if x > 180.0 {
            x -= 360.0;
        } else if x <= -180.0 {
            x += 360.0;
        }
        x
    }

    /// The nearest multiple of `step` degrees, wrapped; a step that is not positive is ignored.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:56 `snap_deg`.
    pub fn snap_deg(a: f64, step: f64) -> f64 {
        if !(step.is_finite() && step > 0.0) {
            return Self::wrap_deg(a);
        }
        Self::wrap_deg((a / step).round() * step)
    }

    /// This pass's trackpad twist in degrees, when Rotate View with Trackpad is on: the
    /// trackpad's own rotate events always, a touch screen's two-finger twist only with Enable
    /// Gestures.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:142 `trackpad_delta`, :124 `apply_trackpad`.
    pub fn trackpad_delta(ctx: &Context, prefs: &NavigationPrefs) -> Option<f64> {
        if !prefs.trackpad_rotate {
            return None;
        }
        let rad = ctx.input(|i| {
            let from_events: f32 = i
                .events
                .iter()
                .filter_map(|e| match e {
                    Event::Rotate(r) if r.is_finite() => Some(*r),
                    _ => None,
                })
                .sum();
            if from_events != 0.0 {
                from_events
            } else if prefs.touch_gestures {
                i.rotation_delta()
            } else {
                0.0
            }
        });
        let deg = f64::from(rad).to_degrees();
        (rad.is_finite() && rad != 0.0 && deg.is_finite()).then_some(deg)
    }

    /// The compass's rect in the canvas's top-right corner.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:217 `compass_rect`.
    pub fn compass_rect(canvas: ScreenRect) -> ScreenRect {
        ScreenRect::from_min_size(
            Pos2::new(canvas.right() - Self::COMPASS_MARGIN - Self::COMPASS, canvas.top() + Self::COMPASS_MARGIN),
            ScreenVec::splat(Self::COMPASS),
        )
    }

    /// Draw the compass (north is the document's up) while the view is turned (more than 0.05°)
    /// and sense it: a click resets the angle. Returns whether it reset, and whether the pointer
    /// is over it (no rotate drag starts there). An AccessKit button named
    /// [`NavigationLabels::reset_rotation`] from the labels installed in the context.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:230 `overlay`, :224 `compass_visible`.
    pub fn compass(ui: &mut Ui, view: &CanvasView, tokens: &UiTokens) -> CompassResponse {
        if !(view.rotation.is_finite() && view.rotation.abs() > Self::COMPASS_VISIBLE_DEG) {
            return CompassResponse::default();
        }
        let rect = Self::compass_rect(view.rect);
        let labels = NavigationLabels::of(ui);
        let response = ui.interact(rect, ui.scope_id().with("rotate-view-compass"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), &labels.reset_rotation));
        let response = response.on_hover_text(&labels.reset_rotation);
        let colors = &tokens.color;
        let painter = ui.painter();
        let (center, radius) = (rect.center(), rect.width() / 2.0);
        let face = if response.hovered() { colors.surface.raised } else { colors.surface.chrome };
        painter.circle_filled(center, radius, face.to_egui());
        painter.circle_stroke(center, radius - 0.5, Stroke::new(1.0, colors.line.border.to_egui()));
        if response.has_focus() {
            painter.circle_stroke(center, radius, Stroke::new(2.0, colors.line.focus.to_egui()));
        }
        // Document up as the view draws it.
        let up = view.map_vec(kurbo::Vec2::new(0.0, -1.0));
        let up = if up.length() > 0.0 { up.normalized() } else { ScreenVec::new(0.0, -1.0) };
        let tip = center + up * (rect.width() * 0.32);
        let accent = colors.accent.solid.to_egui();
        painter.line_segment([center, tip], Stroke::new(2.0, accent));
        painter.circle_filled(tip, 2.5, accent);
        painter.text(
            center + ScreenVec::new(0.0, rect.width() * 0.18),
            Align2::CENTER_CENTER,
            "N",
            tokens.font(TypeRole::Caption),
            colors.text.faint.to_egui(),
        );
        CompassResponse { reset: response.clicked(), hovered: response.hovered() }
    }
}

/// What the Rotate View compass did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompassResponse {
    /// It was clicked: the app sets the rotation to 0.
    pub reset: bool,
    /// The pointer is over it.
    pub hovered: bool,
}

crate::words! {
    /// The words of the navigation controls on the canvas.
    #[non_exhaustive]
    pub struct NavigationLabels {
        /// The Rotate View compass's AccessKit name and tooltip.
        reset_rotation: "navigate.resetRotation" = "Reset Rotation";
    }
}

/// The Hand tool: a drag pans the view with the document under the pointer (the closed hand
/// while dragging, the open hand otherwise); a release while still moving glides on with
/// [`FlickPan`] when Flick Panning is on; Esc during the drag puts the view back. It does not
/// autoscroll. The tool the Space key holds ([`MomentaryKeys`](super::MomentaryKeys)).
#[derive(Clone, Debug, Default)]
pub struct HandTool {
    /// The view and pointer when the drag began.
    start: Option<(CanvasView, Pos2)>,
    /// The pointer at the previous drag pass: a drag step is the flick's input.
    last: Pos2,
    /// The user's navigation preferences (Flick Panning, Overscroll), set by the app each pass.
    pub prefs: NavigationPrefs,
    /// The document's size in document units: with Overscroll off a glide stops at the scroll
    /// limits of this document. Empty: no limit.
    pub size: Size,
}

impl HandTool {
    /// A Hand tool for a document of `size`.
    pub fn new(size: Size, prefs: NavigationPrefs) -> Self {
        Self { size, prefs, ..Self::default() }
    }

    /// Move `view` by the glide step `glide`; a glide that reaches a scroll limit stops.
    fn glide(&self, cx: &ToolContext<'_>, view: &mut CanvasView, glide: ScreenVec) {
        pan(view, glide);
        if !self.prefs.overscroll && clamp_to_document(view, self.size) {
            FlickPan::stop(&cx.canvas.ctx, flick_id(cx));
        }
    }
}

/// The glide state of the canvas the tool works on.
fn flick_id(cx: &ToolContext<'_>) -> Id {
    cx.canvas.id.with("flick-pan")
}

impl CanvasTool for HandTool {
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs (Hand open/closed).
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::Hand { closed: self.start.is_some() }
    }

    // pc: crates/ui-egui/src/canvas.rs:1930 (hand_pan, flick_pan::frame).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let ctx = &cx.canvas.ctx;
        FlickPan::stop(ctx, flick_id(cx));
        ZoomAnimation::stop(ctx, cx.canvas.id);
        self.start = Some((*cx.view, pointer.screen));
        self.last = pointer.screen;
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some((view, from)) = self.start else { return };
        let delta = pointer.screen - self.last;
        self.last = pointer.screen;
        let mut next = view;
        pan(&mut next, pointer.screen - from);
        cx.set_view(next);
        FlickPan::frame(&cx.canvas.ctx, flick_id(cx), true, delta, self.prefs.flick_panning);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some((view, from)) = self.start.take() else { return };
        let mut next = view;
        pan(&mut next, pointer.screen - from);
        // Letting go while the pointer still moves starts the glide, whose first step is this pass's.
        if let Some(glide) = FlickPan::frame(&cx.canvas.ctx, flick_id(cx), false, ScreenVec::ZERO, self.prefs.flick_panning) {
            self.glide(cx, &mut next, glide);
        }
        cx.set_view(next);
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        if let Some((view, _)) = self.start.take() {
            cx.set_view(view);
        }
        FlickPan::stop(&cx.canvas.ctx, flick_id(cx));
    }

    /// A released drag keeps gliding while the pointer is over the canvas.
    fn hover(&mut self, _pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        if self.start.is_some() {
            return;
        }
        if let Some(glide) = FlickPan::frame(&cx.canvas.ctx, flick_id(cx), false, ScreenVec::ZERO, self.prefs.flick_panning) {
            let mut view = *cx.view;
            self.glide(cx, &mut view, glide);
            cx.set_view(view);
        }
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// The Zoom tool: a click steps the ladder about the clicked point (Alt: out;
/// [`zoom_clicked_point_to_center`](NavigationPrefs::zoom_clicked_point_to_center) centres it),
/// a press held still zooms continuously ([`HoldZoom`](super::HoldZoom), Animated Zoom only), a
/// drag scrubs or draws a zoom rectangle ([`ZoomDrag`](super::ZoomDrag), Scrubby Zoom). The cursor
/// is the magnifier, with a minus while Alt is held or `out` is set (Zoom Out held by
/// Alt+Space). At a zoom limit the view stays put.
#[derive(Clone, Debug)]
pub struct ZoomTool {
    /// Zoom out instead of in (Alt+Space held); Alt flips it.
    pub out: bool,
    /// The document's size in document units (the ladder's lower limit).
    pub size: Size,
    pub prefs: NavigationPrefs,
    hold: Option<super::HoldZoom>,
    drag: Option<super::ZoomDrag>,
    /// The view, time and screen point of the press in progress.
    press: Option<(CanvasView, f64, Pos2)>,
}

impl ZoomTool {
    pub fn new(size: Size, prefs: NavigationPrefs) -> Self {
        Self { out: false, size, prefs, hold: None, drag: None, press: None }
    }

    /// One pass of a press in progress: the hold zooms once past its delay (and then the drag
    /// has nothing to do), else the drag scrubs or moves the rectangle's corner.
    fn advance(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let (Some(drag), Some((_, pressed, _))) = (self.drag.as_mut(), self.press) else { return };
        let ppp = cx.canvas.ctx.pixels_per_point();
        let out = self.out != pointer.modifiers.alt;
        let mut view = *cx.view;
        let mut waiting = false;
        let held = match self.hold.as_mut() {
            Some(hold) => {
                let held = hold.advance(&mut view, pointer.screen, pointer.time, out, self.size, ppp);
                // The press asks for passes while it waits out the delay and while it zooms.
                waiting = held || (!hold.zooming() && pointer.time < pressed + super::HoldZoom::DELAY + 0.05);
                held
            }
            None => false,
        };
        if !held {
            drag.drag(&mut view, pointer.screen, self.size, ppp);
        }
        if waiting {
            cx.request_repaint();
        }
        if view != *cx.view {
            cx.set_view(view);
        }
    }
}

/// The view after a Zoom tool click at `screen`: one ladder rung in (or `out`) about the point,
/// or centred on it with Zoom Clicked Point to Center. `None` at a zoom limit.
fn click_step(view: &CanvasView, screen: Pos2, out: bool, size: Size, prefs: &NavigationPrefs, pixels_per_point: f32) -> Option<CanvasView> {
    let current = zoom::device_zoom(view, pixels_per_point);
    let next = ZoomLadder::step(current, if out { -1 } else { 1 }, size);
    if !next.is_finite() || (next / current - 1.0).abs() <= 1e-9 {
        return None;
    }
    let mut stepped = *view;
    zoom::zoom_about(&mut stepped, screen, next, prefs.zoom_clicked_point_to_center, pixels_per_point);
    Some(stepped)
}

impl CanvasTool for ZoomTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::Zoom { out: self.out != cx.modifiers.alt }
    }

    // pc: crates/ui-egui/src/canvas.rs:697 (click step), :1138; zoom_hold.rs:65 `update`; tools/zoom_tool.rs:55 `drag`.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let ctx = &cx.canvas.ctx;
        let ppp = ctx.pixels_per_point();
        ZoomAnimation::stop(ctx, cx.canvas.id);
        self.press = Some((*cx.view, pointer.time, pointer.screen));
        self.hold = self.prefs.animated_zoom.then(|| super::HoldZoom::press(pointer.screen, cx.view.to_doc(pointer.screen), pointer.time));
        self.drag = Some(super::ZoomDrag::begin(cx.view, pointer.screen, self.prefs.scrubby_zoom, ppp));
        if self.hold.is_some() {
            cx.request_repaint();
        }
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.advance(pointer, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let (Some((start, _, anchor)), Some(mut drag)) = (self.press.take(), self.drag.take()) else { return };
        let hold = self.hold.take();
        let ppp = cx.canvas.ctx.pixels_per_point();
        let out = self.out != pointer.modifiers.alt;
        // A hold that zoomed has already changed the view; the release is no click.
        if hold.is_some_and(|h| h.zooming()) {
            return;
        }
        let mut view = *cx.view;
        drag.drag(&mut view, pointer.screen, self.size, ppp);
        let click = match drag.rect() {
            Some(rect) => rect.width() < super::ZoomDrag::CLICK || rect.height() < super::ZoomDrag::CLICK,
            None => pointer.screen.distance(anchor) <= super::HoldZoom::SLOP,
        };
        if click {
            // A click steps from the view the press found, not one a few points of scrub have moved.
            if let Some(next) = click_step(&start, anchor, out, self.size, &self.prefs, ppp) {
                if self.prefs.animated_zoom {
                    let doc = (!self.prefs.zoom_clicked_point_to_center).then(|| (start.to_doc(anchor), anchor));
                    ZoomAnimation::begin(&cx.canvas.ctx, cx.canvas.id, &start, doc);
                }
                cx.set_view(next);
            } else {
                cx.set_view(start);
            }
            return;
        }
        drag.release(&mut view, out, self.size, ppp);
        if view != *cx.view {
            cx.set_view(view);
        }
    }

    /// The hold advances on hovers too, so a press held perfectly still keeps zooming.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.advance(pointer, cx);
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        self.hold = None;
        self.drag = None;
        if let Some((view, _, _)) = self.press.take() {
            cx.set_view(view);
        }
    }

    /// The zoom rectangle while drawn.
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:105 `draw`.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        if let Some(drag) = &self.drag {
            drag.paint(painter, cx.tokens);
        }
    }

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// The Rotate View tool: a drag turns the view about the canvas centre ([`RotateView`]); Esc or a
/// double-click resets the angle to 0. The cursor is the rotate hand.
#[derive(Clone, Debug, Default)]
pub struct RotateViewTool {
    drag: Option<(RotateView, CanvasView)>,
}

impl CanvasTool for RotateViewTool {
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:268 `cursor`.
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::Hand { closed: self.drag.is_some() }
    }

    // pc: crates/ui-egui/src/canvas/rotate_view.rs:186 `drag`.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        ZoomAnimation::stop(&cx.canvas.ctx, cx.canvas.id);
        self.drag = Some((RotateView::begin(cx.view, pointer.screen), *cx.view));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some((turn, start)) = &self.drag else { return };
        let mut view = *start;
        view.rotation = turn.drag(pointer.screen, pointer.modifiers.shift, start.flip_x);
        cx.set_view(view);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some((turn, start)) = self.drag.take() else { return };
        let mut view = start;
        view.rotation = if cx.canvas.double_clicked() || cx.canvas.triple_clicked() {
            0.0
        } else {
            turn.drag(pointer.screen, pointer.modifiers.shift, start.flip_x)
        };
        cx.set_view(view);
    }

    /// Esc puts the angle back to 0 (not during a drag: that cancels it).
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        if !key.pressed || key.key != Key::Escape || self.drag.is_some() || cx.view.rotation == 0.0 {
            return false;
        }
        let mut view = *cx.view;
        view.rotation = 0.0;
        cx.set_view(view);
        true
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        if let Some((_, view)) = self.drag.take() {
            cx.set_view(view);
        }
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// Pointer angle in degrees about `pivot` (y down, so clockwise is positive); 0 at the pivot.
fn pointer_deg(p: Pos2, pivot: Pos2) -> f64 {
    let d = p - pivot;
    if !(d.x.is_finite() && d.y.is_finite()) || d.length_sq() < 1e-8 {
        return 0.0;
    }
    f64::from(d.y).atan2(f64::from(d.x)).to_degrees()
}

/// The document's bounds on screen as `view` draws it (the bounding box of the turned quad).
fn document_bounds(view: &CanvasView, size: Size) -> Option<ScreenRect> {
    if !(size.width > 0.0 && size.height > 0.0) {
        return None;
    }
    let quad = view.quad(Rect::new(0.0, 0.0, size.width, size.height));
    let bounds = quad.iter().skip(1).fold(ScreenRect::from_min_max(quad[0], quad[0]), |r, &p| r.union(ScreenRect::from_min_max(p, p)));
    (bounds.is_finite() && view.rect.is_finite()).then_some(bounds)
}

/// Overscroll off: keep the view inside the document's scroll limits. An empty `size` has none.
fn clamp_to_document(view: &mut CanvasView, size: Size) -> bool {
    CanvasScrollbars::clamp(view, Rect::new(0.0, 0.0, size.width, size.height))
}

/// A Ctrl + wheel event this soon (seconds) after a pinch step still belongs to the pinch, even
/// when it happens to be a whole notch.
const PINCH_GAP: f64 = 0.25;

/// Turn Windows and Wayland touchpad pinches, which arrive as Ctrl + wheel in fractions of a
/// notch or in points, into `Event::Zoom` at egui's Ctrl-scroll zoom speed. Whole notches (a
/// mouse wheel) stay wheel events unless they come within [`PINCH_GAP`] of a pinch step. `last`
/// is the time of the previous pinch step; returns the time of the latest one.
fn legacy_pinch_to_zoom(events: &mut [Event], now: f64, last: Option<f64>, notch: f32, speed: f32) -> Option<f64> {
    let ctrl_wheel = |e: &Event| match e {
        Event::MouseWheel { unit: MouseWheelUnit::Line, delta, modifiers, .. } if modifiers.ctrl => Some((*delta, false)),
        // Wayland precision touchpads can report points: already screen points, not line notches.
        Event::MouseWheel { unit: MouseWheelUnit::Point, delta, modifiers, .. } if modifiers.ctrl => Some((*delta / notch.max(1.0), true)),
        _ => None,
    };
    let fraction = |d: ScreenVec| d.x.fract() != 0.0 || d.y.fract() != 0.0;
    let pinching = last.is_some_and(|t| (0.0..PINCH_GAP).contains(&(now - t)))
        || events.iter().filter_map(ctrl_wheel).any(|(d, high_res)| high_res || fraction(d));
    if !pinching {
        return last;
    }
    let mut latest = last;
    for e in events.iter_mut() {
        if let Some((d, _)) = ctrl_wheel(e) {
            let f = (speed * notch * (d.x + d.y)).exp();
            *e = Event::Zoom(if f.is_finite() && f > 0.0 { f } else { 1.0 });
            latest = Some(now);
        }
    }
    latest
}

fn legacy_pinch_id() -> Id {
    Id::unique("craft-wheel-legacy-pinch")
}

fn alt_id() -> Id {
    Id::unique("craft-wheel-alt")
}

fn shift_id() -> Id {
    Id::unique("craft-wheel-shift")
}

/// The modifier state of the gesture in progress: the latest wheel event's, else the last one seen.
fn remembered(ctx: &Context, id: Id, latest: Option<bool>) -> bool {
    match latest {
        Some(state) => {
            ctx.data_mut(|d| d.insert_temp(id, state));
            state
        }
        None => ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false),
    }
}

fn snap_residual_id(owner: Id) -> Id {
    Id::unique("craft-wheel-snap-residual").with(owner)
}

fn snap_time_id(owner: Id) -> Id {
    Id::unique("craft-wheel-snap-time").with(owner)
}

fn snap_frame_id(owner: Id) -> Id {
    Id::unique("craft-wheel-snap-pass").with(owner)
}

fn snap_owner_id() -> Id {
    Id::unique("craft-wheel-snap-owner")
}

/// Seconds of wheel silence after which a partial snap notch is forgotten.
const SNAP_IDLE_TIMEOUT: f64 = 0.5;

/// The ladder steps a snap-zoom wheel of `dy` points adds up to, carrying the part of a notch
/// that is left over to the next pass. A second call in one pass returns the first one's result.
fn snap_steps(ctx: &Context, owner: Id, dy: f32, notch: f32) -> Option<i32> {
    if dy == 0.0 {
        return None;
    }
    let pass = ctx.cumulative_pass_nr();
    if let Some((cached_pass, steps)) = ctx.data(|d| d.get_temp::<(u64, i32)>(snap_frame_id(owner)))
        && cached_pass == pass
    {
        return (steps != 0).then_some(steps);
    }
    // When another canvas receives the wheel, the old one's partial notch is dropped.
    let previous = ctx.data(|d| d.get_temp::<Id>(snap_owner_id()));
    if let Some(old) = previous.filter(|&p| p != owner) {
        ctx.data_mut(|d| {
            d.remove::<f32>(snap_residual_id(old));
            d.remove::<f64>(snap_time_id(old));
        });
    }
    ctx.data_mut(|d| d.insert_temp(snap_owner_id(), owner));
    let now = ctx.input(|i| i.time);
    let last = ctx.data(|d| d.get_temp::<f64>(snap_time_id(owner)));
    let carried = if last.is_some_and(|t| now - t < SNAP_IDLE_TIMEOUT && (now - t).is_finite()) {
        ctx.data(|d| d.get_temp::<f32>(snap_residual_id(owner))).unwrap_or(0.0)
    } else {
        0.0
    };
    let notch = if notch.is_finite() && notch > 0.0 { notch } else { 40.0 };
    let total = carried + dy;
    let whole = if total.is_finite() { (total / notch).trunc() } else { 0.0 };
    let remainder = total - whole * notch;
    let steps = whole.clamp(-10.0, 10.0) as i32;
    ctx.data_mut(|d| {
        d.insert_temp(snap_residual_id(owner), if remainder.is_finite() { remainder } else { 0.0 });
        d.insert_temp(snap_time_id(owner), now);
        d.insert_temp(snap_frame_id(owner), (pass, steps));
    });
    (steps != 0).then_some(steps)
}

/// egui's own wheel smoothing (`InputState::begin_pass`): 90 % of a notch is released in 0.1 s.
/// The eased zoom uses the same curve, so it paces like the eased pan.
const EASE_REACH: f32 = 0.90;
const EASE_REACH_SECS: f32 = 0.1;
/// An unfinished eased notch is dropped once its owner was not driven for more than this many
/// passes (the pointer left, or another gesture took over) and ...
const EASE_MAX_GAP_PASSES: u64 = 3;
/// ... at least this many seconds have passed. A single slow pass is no gap.
const EASE_STALE_SECS: f64 = 0.5;

/// The eased wheel zoom of one canvas: what is left to release, and the last release.
#[derive(Clone, Copy)]
struct ZoomEase {
    residual: f32,
    time: f64,
    pass: u64,
    released: f32,
}

fn zoom_ease_id(owner: Id) -> Id {
    Id::unique(("craft-wheel-zoom-ease", owner))
}

/// Ease `delta` (points of this canvas's zoom-mode wheel events) over the next passes and return
/// what is released this pass. Same curve as egui's wheel smoothing, but fed by the zoom notches
/// only, so the tail of a pan notch never leaks in. What is released adds up to what was fed. A
/// second call in the same pass returns the first one's result.
fn ease_zoom_delta(ctx: &Context, owner: Id, delta: f32) -> f32 {
    let id = zoom_ease_id(owner);
    let pass = ctx.cumulative_pass_nr();
    let (now, dt) = ctx.input(|i| (i.time, i.stable_dt.min(0.1)));
    let previous = ctx.data(|d| d.get_temp::<ZoomEase>(id));
    if let Some(e) = previous.filter(|e| e.pass == pass) {
        return e.released;
    }
    let fresh = |e: &ZoomEase| {
        let gap = pass.saturating_sub(e.pass);
        e.residual.is_finite() && (gap <= 1 || (gap <= EASE_MAX_GAP_PASSES && now - e.time < EASE_STALE_SECS))
    };
    let carried = previous.filter(fresh).map_or(0.0, |e| e.residual);
    let mut residual = carried + if delta.is_finite() { delta } else { 0.0 };
    let t = egui::emath::exponential_smooth_factor(EASE_REACH, EASE_REACH_SECS, dt);
    let released = if residual.abs() < 1.0 { residual } else { t * residual };
    residual -= released;
    ctx.data_mut(|d| d.insert_temp(id, ZoomEase { residual, time: now, pass, released }));
    // Keep passes coming until the notch is released, or a reactive UI would stop below x1.1.
    if residual != 0.0 {
        ctx.request_repaint();
    }
    released
}

/// Whether a trackpad touch is in progress (egui's `InTouch`) at the start and at the end of a
/// pass; the start is kept, so every read in one pass sees the same states.
#[derive(Clone, Copy)]
struct Touch {
    pass: u64,
    at_start: bool,
    at_end: bool,
}

type WheelEvent = (MouseWheelUnit, ScreenVec, Modifiers, TouchPhase);

/// This pass's wheel events with, for each, whether egui leaves it unsmoothed: during a trackpad
/// touch, or a point delta under 8 points. Those are already smooth, and easing them again would
/// only add latency; the rest (notches, the big pixel steps browsers report) is what Animated
/// Zoom eases.
fn mark_unsmoothed(ctx: &Context, events: Vec<WheelEvent>) -> Vec<(MouseWheelUnit, ScreenVec, Modifiers, bool)> {
    let pass = ctx.cumulative_pass_nr();
    let (horizontal, vertical) = ctx.options(|o| (o.input_options.horizontal_scroll_modifier, o.input_options.vertical_scroll_modifier));
    let id = Id::unique(("craft-wheel-touch", ctx.viewport_id()));
    // Not read in the previous pass (no canvas then): a touch could have ended unseen.
    let mut touch = match ctx.data(|d| d.get_temp::<Touch>(id)) {
        Some(t) if t.pass == pass => t.at_start,
        Some(t) if t.pass.saturating_add(1) == pass => t.at_end,
        _ => false,
    };
    let at_start = touch;
    let mut wheel = Vec::with_capacity(events.len());
    for (unit, delta, modifiers, phase) in events {
        match phase {
            TouchPhase::Start => touch = true,
            TouchPhase::End | TouchPhase::Cancel => touch = false,
            TouchPhase::Move => {}
        }
        // egui folds a scroll onto one axis by its modifiers before it measures the step.
        let step = match (modifiers.matches_any(horizontal), modifiers.matches_any(vertical)) {
            (true, false) | (false, true) => (delta.x + delta.y).abs(),
            _ => delta.length(),
        };
        wheel.push((unit, delta, modifiers, touch || (unit == MouseWheelUnit::Point && step < 8.0)));
    }
    ctx.data_mut(|d| d.insert_temp(id, Touch { pass, at_start, at_end: touch }));
    wheel
}

fn is_alt_key(key: Key) -> bool {
    matches!(key, Key::AltLeft | Key::AltRight)
}

/// This pass's wheel events, in order, with Alt added to their modifiers while an Alt key is down.
/// The modifiers a wheel event carries are the windowing layer's, which on Windows can miss a
/// held Alt, while the Alt key's own press and release still arrive. Alt is down from its press
/// until its release, starting from whether it was down before this pass's first Alt key event.
fn wheel_events(i: &InputState) -> Vec<WheelEvent> {
    let first_alt_key = i.events.iter().find_map(|e| match e {
        // A repeat means the key was already down; a release means it was down until then.
        Event::Key { key, pressed, repeat, .. } if is_alt_key(*key) => Some(!*pressed || *repeat),
        _ => None,
    });
    let mut alt_key = first_alt_key.unwrap_or_else(|| i.key_down(Key::AltLeft) || i.key_down(Key::AltRight));
    let mut wheel = Vec::new();
    for e in &i.events {
        match e {
            Event::Key { key, pressed, .. } if is_alt_key(*key) => alt_key = *pressed,
            Event::MouseWheel { unit, delta, phase, modifiers, .. } => {
                wheel.push((*unit, *delta, Modifiers { alt: modifiers.alt || alt_key, ..*modifiers }, *phase));
            }
            _ => {}
        }
    }
    wheel
}

/// This pass's pinch zoom factor (1: none). While two fingers are down on a touch screen, egui's
/// `zoom_delta` reports their pinch in place of the trackpad's `Event::Zoom`. With Enable
/// Gestures off the fingers are ignored and only the trackpad's own events count.
fn pinch_factor(i: &InputState, touch_gestures: bool) -> f32 {
    if touch_gestures || i.multi_touch().is_none() {
        return i.zoom_delta();
    }
    i.events
        .iter()
        .filter_map(|e| match e {
            Event::Zoom(f) if f.is_finite() && *f > 0.0 => Some(*f),
            _ => None,
        })
        .product()
}
