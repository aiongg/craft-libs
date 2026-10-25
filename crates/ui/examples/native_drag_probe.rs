//! A macOS probe for moving a window by the OS: a press on the bar of the "Palette" window
//! sends `ViewportCommand::StartDrag` (winit's `drag_window`, AppKit's
//! `performWindowDragWithEvent` with `NSApp.currentEvent`). Run with
//! `cargo run -p craft-ui --example native_drag_probe`.
//!
//! Every frame of the palette logs a line to stderr: the time and the gap since
//! the previous frame, egui's input events, whether egui and AppKit hold the primary button,
//! AppKit's current event type, the window's outer origin, AppKit's cursor and the cursor's
//! offset from the origin. Each drag ends with a `SUMMARY` line when AppKit reports the button
//! up.
//!
//! - `PROBE_START=drag` sends StartDrag once egui's drag starts (after a few points of motion)
//!   instead of on the press.
//! - `PROBE_REPAINT=1` requests a repaint every frame, so frames run with no input too.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use egui::{Color32, Event, Key, Pos2, Rect, Sense, ViewportBuilder, ViewportCommand, ViewportId, WindowLevel, vec2};

const BAR_HEIGHT: f32 = 28.0;

/// What AppKit says this frame.
struct AppKit {
    /// `NSEvent.mouseLocation`, flipped to egui's desktop points against the primary screen.
    cursor: Option<Pos2>,
    /// `NSEvent.pressedMouseButtons`: bit 0 is the primary button.
    buttons: usize,
    /// `NSApp.currentEvent`'s `NSEventType` (1 left down, 2 left up, 6 left dragged, 5 moved).
    event: Option<usize>,
}

#[cfg(target_os = "macos")]
fn appkit() -> AppKit {
    use objc2_app_kit::{NSApplication, NSEvent, NSScreen};
    let Some(mtm) = objc2::MainThreadMarker::new() else { return AppKit { cursor: None, buttons: 0, event: None } };
    let height = NSScreen::screens(mtm).firstObject().map(|s| s.frame().size.height);
    let at = NSEvent::mouseLocation();
    let event = NSApplication::sharedApplication(mtm).currentEvent().map(|e| e.r#type().0);
    AppKit { cursor: height.map(|h| egui::pos2(at.x as f32, (h - at.y) as f32)), buttons: NSEvent::pressedMouseButtons(), event }
}

#[cfg(not(target_os = "macos"))]
fn appkit() -> AppKit {
    AppKit { cursor: None, buttons: 0, event: None }
}

/// Counts for one drag, from StartDrag to AppKit's button up.
struct Drag {
    sent: Instant,
    frames: u32,
    /// Frames with no egui input event: woken by something else (a `windowDidMove`, or a
    /// requested repaint).
    empty_frames: u32,
    pointer_moves: u32,
    origin_changes: u32,
    last_origin: Option<Pos2>,
    max_gap: f64,
    offsets: Vec<f32>,
    egui_release: bool,
    escape: bool,
}

struct Probe {
    start: Instant,
    last: Option<Instant>,
    on_drag: bool,
    repaint: bool,
    drag: Option<Drag>,
    /// Shared with the watchdog thread: a drag is running, and the time of the last frame.
    dragging: Arc<AtomicBool>,
    last_frame_ms: Arc<AtomicU64>,
}

impl Probe {
    fn line(&mut self, text: String) {
        eprintln!("{text}");
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        let gap = self.last.map_or(0.0, |last| (now - last).as_secs_f64() * 1000.0);
        self.last = Some(now);
        let t = (now - self.start).as_secs_f64() * 1000.0;
        self.last_frame_ms.store(t as u64, Ordering::Relaxed);
        let (events, egui_down, origin) = ctx.input(|i| (events(&i.raw.events), i.pointer.primary_down(), i.viewport().outer_rect.map(|r| r.min)));
        let kit = appkit();
        let offset = kit.cursor.zip(origin).map(|(c, o)| c - o);

        let full = ui.max_rect();
        let bar = Rect::from_min_size(full.min, vec2(full.width(), BAR_HEIGHT));
        ui.painter().rect_filled(full, 0.0, Color32::from_gray(40));
        ui.painter().rect_filled(bar, 0.0, Color32::from_rgb(60, 90, 140));
        ui.painter().text(bar.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, "Drag me", egui::FontId::proportional(13.0), Color32::WHITE);
        let response = ui.interact(bar, egui::Id::new("probe-bar"), Sense::click_and_drag());
        let start =
            if self.on_drag { response.drag_started() } else { response.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_pressed()) };

        self.line(format!(
            "{t:9.1} gap {gap:6.1} [{events}] egui_down {egui_down} appkit_buttons {} current_event {:?} origin {} cursor {} offset {}",
            kit.buttons,
            kit.event,
            fmt(origin),
            fmt(kit.cursor),
            offset.map_or("-".into(), |o| format!("({:.1}, {:.1})", o.x, o.y)),
        ));

        if let Some(drag) = &mut self.drag {
            drag.frames += 1;
            drag.max_gap = drag.max_gap.max(gap);
            if events.is_empty() {
                drag.empty_frames += 1;
            }
            drag.pointer_moves += events.matches("move").count() as u32;
            drag.egui_release |= events.contains("up");
            drag.escape |= events.contains("esc");
            if origin != drag.last_origin {
                drag.origin_changes += 1;
                drag.last_origin = origin;
            }
            if let Some(o) = offset {
                drag.offsets.push(o.length());
            }
            if kit.buttons & 1 == 0 {
                self.summary(egui_down);
            }
        }
        if start {
            self.line(format!("{t:9.1} START_DRAG sent (current_event {:?})", kit.event));
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            self.drag = Some(Drag {
                sent: now,
                frames: 0,
                empty_frames: 0,
                pointer_moves: 0,
                origin_changes: 0,
                last_origin: origin,
                max_gap: 0.0,
                offsets: Vec::new(),
                egui_release: false,
                escape: false,
            });
            self.dragging.store(true, Ordering::Relaxed);
        }
        if self.repaint {
            ctx.request_repaint();
        }
    }

    fn summary(&mut self, egui_down: bool) {
        let Some(drag) = self.drag.take() else { return };
        self.dragging.store(false, Ordering::Relaxed);
        // Frame-to-frame change of the cursor-to-window offset: 0 when the window follows the
        // cursor exactly.
        let steps: Vec<f32> = drag.offsets.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let mean = if steps.is_empty() { 0.0 } else { steps.iter().sum::<f32>() / steps.len() as f32 };
        let jumps = steps.iter().filter(|s| **s >= 5.0).count();
        self.line(format!(
            "SUMMARY {:.0} ms: frames {} (no input {}), egui pointer moves {}, origin changes {}, max frame gap {:.0} ms, offset change mean {mean:.1} pt, >=5 pt {jumps}/{}, egui saw the release {}, egui still holds the button {egui_down}, escape seen {}",
            drag.sent.elapsed().as_secs_f64() * 1000.0,
            drag.frames,
            drag.empty_frames,
            drag.pointer_moves,
            drag.origin_changes,
            drag.max_gap,
            steps.len(),
            drag.egui_release,
            drag.escape,
        ));
    }
}

/// egui's input events in short: `move`, `down`, `up`, `esc`, `focus:<bool>`, `gone`; others
/// are left out.
fn events(events: &[Event]) -> String {
    let mut out = Vec::new();
    for event in events {
        match event {
            Event::PointerMoved(_) => out.push("move".to_owned()),
            Event::PointerButton { pressed, .. } => out.push(if *pressed { "down" } else { "up" }.to_owned()),
            Event::Key { key: Key::Escape, pressed: true, .. } => out.push("esc".to_owned()),
            Event::WindowFocused(focused) => out.push(format!("focus:{focused}")),
            Event::PointerGone => out.push("gone".to_owned()),
            _ => {}
        }
    }
    out.join(" ")
}

fn fmt(at: Option<Pos2>) -> String {
    at.map_or("-".into(), |p| format!("({:.1}, {:.1})", p.x, p.y))
}

impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.label("Drag the Palette window by its blue bar. The log goes to stderr.");
        let builder = ViewportBuilder::default()
            .with_title("Palette")
            .with_position([300.0, 200.0])
            .with_inner_size([260.0, 180.0])
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_taskbar(false);
        // The frame craft-ui gives a native float on macOS.
        #[cfg(target_os = "macos")]
        let builder = builder.with_titlebar_shown(false).with_title_shown(false).with_fullsize_content_view(true).with_titlebar_buttons_shown(false);
        ui.ctx().show_viewport_immediate(ViewportId::from_hash_of("probe-palette"), builder, |ui, _| self.palette(ui));
    }
}

fn main() -> eframe::Result {
    let on_drag = std::env::var("PROBE_START").is_ok_and(|v| v == "drag");
    let repaint = std::env::var("PROBE_REPAINT").is_ok_and(|v| v == "1");
    eprintln!("PROBE_START={} PROBE_REPAINT={}", if on_drag { "drag" } else { "press" }, u8::from(repaint));
    log::set_logger(&LOGGER).ok();
    log::set_max_level(log::LevelFilter::Warn);

    // While a drag runs, report every 250 ms without a frame: the main thread is busy, or
    // nothing woke it.
    let dragging = Arc::new(AtomicBool::new(false));
    let last_frame_ms = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    {
        let (dragging, last_frame_ms) = (dragging.clone(), last_frame_ms.clone());
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let now = start.elapsed().as_millis() as u64;
                let last = last_frame_ms.load(Ordering::Relaxed);
                if dragging.load(Ordering::Relaxed) && now.saturating_sub(last) >= 250 {
                    eprintln!("{now:9} WATCHDOG no palette frame for {} ms", now - last);
                }
            }
        });
    }

    let probe = Probe { start, last: None, on_drag, repaint, drag: None, dragging, last_frame_ms };
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([520.0, 120.0]).with_position([40.0, 40.0]),
        ..Default::default()
    };
    eframe::run_native("Drag probe", options, Box::new(|_| Ok(Box::new(probe))))
}

/// Prints warnings, such as egui-winit's when `drag_window` fails.
struct Logger;

static LOGGER: Logger = Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("LOG {} {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}
