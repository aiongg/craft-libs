//! A strip of the app's own UI as the window's title bar: its empty part moves the window when
//! dragged and follows the system's title-bar double-click setting.
//!
//! This is for windows whose title bar the app draws over: on macOS a hidden title bar with a
//! full-size content view, elsewhere an undecorated window ([`Frame::App`](crate::Frame::App)).
//! Call [`empty_area`] with the strip's rect before drawing the strip's controls.
//!
//! On macOS the full-size content view makes AppKit hand every click in the title-bar band to the
//! app (that is how the strip's own controls work), and AppKit does not zoom the window. It does
//! move the window when a drag starts anywhere in the band, on the strip's controls and on any
//! in-window panel that lies over the band, unless the window is not movable. [`window_movable`]
//! says when the pointer is on one of those, and [`sync_movable`] sets the window's `movable` flag
//! from it before the press. Both are per viewport: each window of the app has its own flag.
//!
//! In full screen the strip is not a title bar: neither a drag nor a double-click acts on the
//! window.

use std::sync::{Once, OnceLock};

/// System Settings › Desktop & Dock › "Double-click a window's title bar to".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoubleClick {
    Zoom,
    Minimize,
    Nothing,
}

/// The value of the global default `AppleActionOnDoubleClick`, as `defaults read` prints it.
/// Anything other than "Minimize" or "None" zooms, the macOS default ("Maximize"; macOS 15 adds
/// "Fill", which zooming comes closest to).
pub fn parse(defaults_output: &str) -> DoubleClick {
    match defaults_output.trim() {
        "Minimize" => DoubleClick::Minimize,
        "None" => DoubleClick::Nothing,
        _ => DoubleClick::Zoom,
    }
}

static ACTION: OnceLock<DoubleClick> = OnceLock::new();
static READ: Once = Once::new();

/// Starts reading the system setting on a background thread, once per run (macOS only).
fn prefetch() {
    READ.call_once(|| {
        #[cfg(all(target_os = "macos", not(test)))]
        {
            // No thread, no setting: the strip zooms.
            let _ = std::thread::Builder::new().name("title-bar-setting".into()).spawn(|| {
                let action = match std::process::Command::new("/usr/bin/defaults").args(["read", "-g", "AppleActionOnDoubleClick"]).output() {
                    Ok(out) if out.status.success() => parse(&String::from_utf8_lossy(&out.stdout)),
                    // Absent (never changed) or unreadable: the default.
                    _ => DoubleClick::Zoom,
                };
                let _ = ACTION.set(action);
            });
        }
    });
}

/// The system setting; zoom until it has been read, and always zoom outside macOS.
pub fn action() -> DoubleClick {
    ACTION.get().copied().unwrap_or(DoubleClick::Zoom)
}

/// What a double-click on the empty title bar asks of the window.
pub fn double_click_command(action: DoubleClick, maximized: bool) -> Option<egui::ViewportCommand> {
    match action {
        DoubleClick::Zoom => Some(egui::ViewportCommand::Maximized(!maximized)),
        DoubleClick::Minimize => Some(egui::ViewportCommand::Minimized(true)),
        DoubleClick::Nothing => None,
    }
}

/// What [`empty_area`] saw on one frame.
#[derive(Debug)]
pub struct EmptyArea {
    /// The empty area's response, for an accessible label or a tooltip.
    pub response: egui::Response,
    /// A drag started on the empty area this frame and `ViewportCommand::StartDrag` was sent: the
    /// OS now moves the window, and the app's own pointer handling should let go.
    pub move_started: bool,
}

/// Makes the empty part of `rect` act as a title bar. Call it before drawing the strip's controls:
/// they sit on top and take their own clicks.
///
/// The area senses clicks only. A drag-sensing background would also receive drags that start
/// on a click-only control (egui gives clicks and drags to different widgets), so a drag is
/// detected here as a press on the empty area that has stopped being a click.
/// It takes no keyboard focus: Tab goes to the strip's controls.
pub fn empty_area(ui: &mut egui::Ui, rect: egui::Rect) -> EmptyArea {
    prefetch();
    let resp = ui.interact(rect, ui.scope_id().with("title_bar_empty"), egui::Sense::CLICK);
    let covered = ui.ctx().pointer_hover_pos().filter(|p| rect.contains(*p)).and_then(|p| ui.ctx().layer_id_at(p));
    let covered = covered.is_some_and(|top| covers(top, ui.layer_id(), resp.hovered()));
    let pass = ui.ctx().cumulative_pass_nr();
    // Read before `data_mut`, which holds the context's lock.
    let movable = movable_id(ui.ctx().viewport_id());
    ui.data_mut(|d| d.insert_temp(movable, (pass, !covered)));
    // egui forgets which widget a press landed on once it can no longer be a click, so the press
    // is remembered (by its start time) while it still can.
    let armed = resp.id.with("drag_press");
    if ui.input(|i| i.viewport().fullscreen).unwrap_or(false) {
        ui.data_mut(|d| d.remove::<Option<f64>>(armed));
        return EmptyArea { response: resp, move_started: false };
    }
    if resp.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized).unwrap_or(false);
        if let Some(cmd) = double_click_command(action(), maximized) {
            ui.ctx().send_viewport_cmd(cmd);
        }
    }
    let press = ui.input(|i| i.pointer.press_start_time());
    // Only the primary button moves the window.
    if resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
        ui.data_mut(|d| d.insert_temp(armed, press));
    }
    let mut move_started = false;
    if press.is_some() && ui.input(|i| i.pointer.is_decidedly_dragging()) && ui.data(|d| d.get_temp::<Option<f64>>(armed)) == Some(press) {
        // Once per press: AppKit's drag loop takes the mouse-up, so egui may still see the button
        // down afterwards.
        ui.data_mut(|d| d.remove::<Option<f64>>(armed));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        move_started = true;
    }
    EmptyArea { response: resp, move_started }
}

/// Whether `top`, the topmost layer under the pointer, keeps a drag there from moving the window.
///
/// In the strip's own layer a control covers the empty area when the empty area is not hovered.
/// A `Middle` layer is an in-window panel lying over the strip (a floating dock, an
/// `egui::Window`). A layer above those (a modal's backdrop, a tooltip) leaves the window movable,
/// as AppKit's own title bar would be.
fn covers(top: egui::LayerId, strip: egui::LayerId, empty_hovered: bool) -> bool {
    if top == strip { !empty_hovered } else { top.order == egui::Order::Middle }
}

fn movable_id(viewport: egui::ViewportId) -> egui::Id {
    egui::Id::unique(("title_bar_window_movable", viewport))
}

/// Whether AppKit may move the window when a drag starts in the title-bar band: false while the
/// pointer is on one of the strip's controls or on an in-window panel over the strip, so pressing
/// and dragging there does not move the window. True when neither this pass nor the one before
/// drew a title bar in the context's viewport.
pub fn window_movable(ctx: &egui::Context) -> bool {
    let id = movable_id(ctx.viewport_id());
    match ctx.data(|d| d.get_temp::<(u64, bool)>(id)) {
        Some((pass, movable)) if ctx.cumulative_pass_nr().saturating_sub(pass) <= 1 => movable,
        _ => true,
    }
}

/// Sets the `movable` flag of the context's window from this pass's [`window_movable`]. Call it
/// once per frame per window, after that window's UI has run. `applied` holds the value last set
/// for this window, so AppKit is only called when it changes.
///
/// A window is told apart by its viewport: the pointer is on the key window when this viewport
/// has focus, so an unfocused viewport is left as it is (AppKit activates the window on the first
/// click and the flag is set from the frame after). Only a window with a full-size content view
/// and visible window buttons is touched: the app draws over its title bar. Floating panels in
/// their own windows hide the buttons, and keep their own `movable`, which AppKit also needs to
/// resize them from their edges.
#[cfg(target_os = "macos")]
pub fn sync_movable(ctx: &egui::Context, applied: &mut Option<bool>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindowButton, NSWindowStyleMask};

    if ctx.input(|i| i.viewport().focused) != Some(true) {
        return;
    }
    let movable = window_movable(ctx);
    if *applied == Some(movable) {
        return;
    }
    // eframe runs the app on the main thread; anywhere else AppKit is out of reach.
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(window) = NSApplication::sharedApplication(mtm).keyWindow() else {
        return;
    };
    let buttons = window.standardWindowButton(NSWindowButton::CloseButton).is_some_and(|close| !close.isHidden());
    if window.styleMask().contains(NSWindowStyleMask::FullSizeContentView) && buttons {
        window.setMovable(movable);
        *applied = Some(movable);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, Order, PointerButton, Pos2, RawInput, Rect, ViewportCommand, ViewportId, pos2, vec2};

    #[test]
    fn parses_the_defaults_output() {
        assert_eq!(parse("Maximize\n"), DoubleClick::Zoom);
        assert_eq!(parse("Minimize\n"), DoubleClick::Minimize);
        assert_eq!(parse("None\n"), DoubleClick::Nothing);
        for junk in ["", "Fill", "minimize", "1", "The domain/default pair of (kCFPreferencesAnyApplication, X) does not exist"] {
            assert_eq!(parse(junk), DoubleClick::Zoom, "{junk:?}");
        }
    }

    #[test]
    fn tab_skips_the_empty_area() {
        let mut bar = Bar::new();
        bar.frame(vec![]);
        let tab = Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
        bar.frame(vec![tab]);
        bar.frame(vec![]);
        let focused = bar.ctx.memory(|m| m.focused()).and_then(|id| bar.ctx.read_response(id)).map(|r| r.rect);
        assert!(focused.is_some_and(|r| r.contains(BUTTON) && r.width() < 100.0), "the first Tab goes to the strip's button: {focused:?}");
    }

    #[test]
    fn zoom_toggles_and_minimize_minimizes() {
        use egui::ViewportCommand::{Maximized, Minimized};
        assert_eq!(double_click_command(DoubleClick::Zoom, false), Some(Maximized(true)));
        assert_eq!(double_click_command(DoubleClick::Zoom, true), Some(Maximized(false)));
        assert_eq!(double_click_command(DoubleClick::Minimize, false), Some(Minimized(true)));
        assert_eq!(double_click_command(DoubleClick::Nothing, true), None);
    }

    /// A 400×28 strip with a button at x 10..60, a `Middle` area at x 100..180 and a `Foreground`
    /// area at x 200..280, all over the strip.
    struct Bar {
        ctx: Context,
        time: f64,
        fullscreen: bool,
    }

    const BUTTON: Pos2 = pos2(30.0, 14.0);
    const EMPTY: Pos2 = pos2(350.0, 14.0);
    const FLOAT: Pos2 = pos2(140.0, 14.0);
    const OVERLAY: Pos2 = pos2(240.0, 14.0);

    impl Bar {
        fn new() -> Self {
            Self { ctx: Context::default(), time: 0.0, fullscreen: false }
        }

        /// Runs one frame with `events`; returns the viewport commands it sent.
        fn frame(&mut self, events: Vec<Event>) -> Vec<ViewportCommand> {
            self.time += 1.0 / 60.0;
            let mut input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
                time: Some(self.time),
                events,
                ..RawInput::default()
            };
            input.viewports.entry(ViewportId::ROOT).or_default().fullscreen = Some(self.fullscreen);
            let mut out = self.ctx.run_ui(input, |ui| {
                empty_area(ui, Rect::from_min_size(Pos2::ZERO, vec2(400.0, 28.0)));
                let _ = ui.put(Rect::from_min_max(pos2(10.0, 4.0), pos2(60.0, 24.0)), egui::Button::new("Menu"));
                for (id, order, x) in [("float", Order::Middle, 100.0), ("overlay", Order::Foreground, 200.0)] {
                    egui::Area::new(egui::Id::unique(id)).order(order).fixed_pos(pos2(x, 0.0)).show(ui.ctx(), |ui| {
                        ui.allocate_exact_size(vec2(80.0, 28.0), egui::Sense::hover());
                    });
                }
            });
            // No renderer here to take the font atlas.
            out.textures_delta.clear();
            out.viewport_output.get(&ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default()
        }

        /// Moves the pointer to `p` and lets layers and hover settle.
        fn hover(&mut self, p: Pos2) {
            for _ in 0..3 {
                self.frame(vec![Event::PointerMoved(p)]);
            }
        }

        /// Presses at `p`, drags 30 points right and releases; returns how many `StartDrag`s it sent.
        fn drag(&mut self, p: Pos2) -> usize {
            self.hover(p);
            let button = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
            let mut commands = self.frame(vec![button(p, true)]);
            for dx in [5.0, 10.0, 20.0, 30.0] {
                commands.extend(self.frame(vec![Event::PointerMoved(p + vec2(dx, 0.0))]));
            }
            commands.extend(self.frame(vec![button(p + vec2(30.0, 0.0), false)]));
            commands.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count()
        }
    }

    #[test]
    fn each_viewport_has_its_own_movable_flag() {
        let ctx = Context::default();
        let second = ViewportId::from_hash_of("second");
        // One frame of `viewport`: the strip and a button when `strip`, then the flag as that pass sees it.
        let run = |viewport: ViewportId, strip: bool| {
            let mut input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
                events: vec![Event::PointerMoved(BUTTON)],
                ..RawInput::default()
            };
            input.viewport_id = viewport;
            input.viewports.entry(viewport).or_default();
            let mut movable = None;
            let mut out = ctx.run_ui(input, |ui| {
                if strip {
                    empty_area(ui, Rect::from_min_size(Pos2::ZERO, vec2(400.0, 28.0)));
                    let _ = ui.put(Rect::from_min_max(pos2(10.0, 4.0), pos2(60.0, 24.0)), egui::Button::new("Menu"));
                }
                movable = Some(window_movable(ui.ctx()));
            });
            out.textures_delta.clear();
            movable
        };
        for _ in 0..3 {
            run(ViewportId::ROOT, true);
        }
        assert_eq!(run(ViewportId::ROOT, true), Some(false), "the pointer is on the first window's control");
        assert_eq!(run(second, false), Some(true), "the second window drew no strip, so its own flag is the default");
    }

    #[test]
    fn only_the_empty_strip_moves_the_window() {
        let mut bar = Bar::new();
        bar.hover(EMPTY);
        assert!(window_movable(&bar.ctx), "empty strip");
        bar.hover(BUTTON);
        assert!(!window_movable(&bar.ctx), "control on the strip");
        bar.hover(FLOAT);
        assert!(!window_movable(&bar.ctx), "Middle area over the strip");
        bar.hover(OVERLAY);
        assert!(window_movable(&bar.ctx), "Foreground area over the strip");

        assert_eq!(bar.drag(EMPTY), 1, "drag on the empty strip");
        assert_eq!(bar.drag(BUTTON), 0, "drag on a control");
        assert_eq!(bar.drag(FLOAT), 0, "drag on a Middle area");

        bar.fullscreen = true;
        assert_eq!(bar.drag(EMPTY), 0, "drag in full screen");
    }
}
