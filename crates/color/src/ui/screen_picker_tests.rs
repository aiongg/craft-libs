//! Behaviour tests of `screen_picker`: a fake capture in place of the screen.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use craft_platform::screen::{self, Capture, ScreenImage};
use egui::{Context, Id, Key, PointerButton, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::screen_picker::{CaptureResult, ScreenColorPicker, Source, frame, lock, runtime};

struct Fake(Mutex<Receiver<CaptureResult>>);

impl Source for Fake {
    fn poll(&self) -> Option<CaptureResult> {
        self.0.lock().unwrap().try_recv().ok()
    }
}

/// Two screen pixels side by side: red, then azure.
fn capture() -> CaptureResult {
    Ok(Capture::Images(vec![ScreenImage {
        origin: [0, 0],
        size: [2, 1],
        scale: 1.0,
        rgba: vec![255, 0, 0, 255, 0, 128, 255, 255].into(),
        icc_profile: None,
        name: "Fake".into(),
    }]))
}

const LEFT: egui::Pos2 = egui::pos2(40.0, 50.0);

/// A harness whose single pass is the capture window's frame over the running pick's first
/// snapshot (a 200 x 100 window).
fn window() -> Harness<'static> {
    Harness::builder().with_size(vec2(200.0, 100.0)).build_ui(|ui| {
        let shared = runtime(ui.ctx());
        let snapshot = lock(&shared).images.first().cloned();
        if let Some(snapshot) = snapshot {
            frame(ui, &shared, &snapshot);
        }
    })
}

/// Starts a pick for `owner` on a fake capture that has already finished, and polls it in.
fn started(ctx: &Context, owner: Id, target: u64) -> Sender<CaptureResult> {
    started_with(ctx, owner, target, capture())
}

fn started_with(ctx: &Context, owner: Id, target: u64, result: CaptureResult) -> Sender<CaptureResult> {
    let (tx, rx) = channel();
    tx.send(result).unwrap();
    ScreenColorPicker::open(ctx, owner);
    let rx = std::cell::Cell::new(Some(rx));
    ScreenColorPicker::tick_with(ctx, target, &|| Box::new(Fake(Mutex::new(rx.take().unwrap_or_else(|| channel().1)))));
    tx
}

fn click(h: &mut Harness<'_>, at: egui::Pos2) {
    h.hover_at(at);
    h.run();
    h.drag_at(at);
    h.run();
    h.drop_at(at);
    h.run();
}

/// A pick's result goes only to the widget that asked.
#[test]
fn the_result_goes_to_its_owner_only() {
    let mut h = window();
    let ctx = h.ctx.clone();
    let owner = Id::unique("asker");
    let _tx = started(&ctx, owner, 1);
    assert!(ScreenColorPicker::showing(&ctx) && ScreenColorPicker::busy(&ctx));
    click(&mut h, LEFT);
    ScreenColorPicker::tick_with(&ctx, 1, &|| unreachable!());
    assert!(!ScreenColorPicker::busy(&ctx) && !ScreenColorPicker::showing(&ctx));
    assert_eq!(ScreenColorPicker::take(&ctx, Id::unique("other")), None);
    assert_eq!(ScreenColorPicker::take(&ctx, owner).map(|c| c.to_hex()), Some("#ff0000".into()));
    assert_eq!(ScreenColorPicker::take(&ctx, owner), None, "taken once");
}

/// Esc, a right-click and a changed editing target cancel with no colour.
#[test]
fn every_way_out_cancels() {
    let mut h = window();
    let ctx = h.ctx.clone();
    let owner = Id::unique("asker");
    let ends = |ctx: &Context| {
        ScreenColorPicker::tick_with(ctx, 1, &|| unreachable!());
        assert!(!ScreenColorPicker::busy(ctx) && !ScreenColorPicker::showing(ctx));
        assert_eq!(ScreenColorPicker::take(ctx, Id::unique("asker")), None);
    };

    let _tx = started(&ctx, owner, 1);
    h.run();
    h.key_press(Key::Escape);
    h.run();
    ends(&ctx);

    let _tx = started(&ctx, owner, 1);
    h.hover_at(LEFT);
    h.run();
    let secondary = |pressed| egui::Event::PointerButton { pos: LEFT, button: PointerButton::Secondary, pressed, modifiers: Default::default() };
    h.event(secondary(true));
    h.run();
    h.event(secondary(false));
    h.run();
    ends(&ctx);

    let _tx = started(&ctx, owner, 1);
    ScreenColorPicker::tick_with(&ctx, 2, &|| unreachable!());
    assert!(!ScreenColorPicker::busy(&ctx) && !ScreenColorPicker::showing(&ctx));
    assert_eq!(ScreenColorPicker::take(&ctx, owner), None);
}

/// The button is disabled while a pick runs.
#[test]
fn the_button_is_disabled_while_busy() {
    let owner = Id::unique("asker");
    let mut h = Harness::new_ui(move |ui| {
        ScreenColorPicker::button(ui, owner);
    });
    h.run();
    assert!(!h.get_by_label("Pick screen color").accesskit_node().is_disabled());
    ScreenColorPicker::open(&h.ctx, owner);
    h.run();
    assert!(h.get_by_label("Pick screen color").accesskit_node().is_disabled());
}

/// A colour from the compositor's own picker (Wayland) answers at once, with no capture windows.
#[test]
fn a_picked_color_answers_without_capture_windows() {
    let h = window();
    let ctx = h.ctx.clone();
    let owner = Id::unique("asker");
    let _tx = started_with(&ctx, owner, 1, Ok(Capture::Color([1.0, 0.0, 0.0])));
    assert!(!ScreenColorPicker::busy(&ctx) && !ScreenColorPicker::showing(&ctx));
    assert_eq!(ScreenColorPicker::take(&ctx, owner).map(|c| c.to_hex()), Some("#ff0000".into()));
}

/// A failed capture reaches the status bar through the picker's labels, with the platform's text
/// as the reason only.
#[test]
fn failures_are_worded_by_the_labels() {
    let h = window();
    let ctx = h.ctx.clone();
    let owner = Id::unique("asker");
    let _tx = started_with(&ctx, owner, 1, Err(screen::ScreenError::Failed("no display".into())));
    assert_eq!(ScreenColorPicker::take_notice(&ctx).as_deref(), Some("Screen capture failed: no display"));
    let bad = ScreenImage {
        size: [3, 3],
        ..match capture() {
            Ok(Capture::Images(mut images)) => images.remove(0),
            _ => unreachable!(),
        }
    };
    let _tx = started_with(&ctx, owner, 1, Ok(Capture::Images(vec![bad])));
    assert_eq!(ScreenColorPicker::take_notice(&ctx).as_deref(), Some("The screen capture is not valid."));
    let _tx = started_with(&ctx, owner, 1, Ok(Capture::Images(Vec::new())));
    assert_eq!(ScreenColorPicker::take_notice(&ctx).as_deref(), Some("No screen was captured."));
}

/// A pass of a lone button that tracks a press-and-drag from it; the pixel sampled and the
/// screenshot requests the pass made.
fn hold_pass(ctx: &Context, owner: Id, events: Vec<egui::Event>) -> (egui::Rect, Option<craft_theme::Srgba>, Vec<egui::ScreenshotCallback>) {
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(200.0, 100.0))), events, ..Default::default() };
    let (mut rect, mut sampled) = (egui::Rect::NOTHING, None);
    let output = ctx.run_ui(input, |ui| {
        let button = ui.button("Pick");
        rect = button.rect;
        sampled = ScreenColorPicker::hold_drag(ui, &button, owner);
    });
    let shots = output
        .viewport_output
        .values()
        .flat_map(|v| v.commands.iter())
        .filter_map(|c| match c {
            egui::ViewportCommand::Screenshot(shot) => Some(shot.clone()),
            _ => None,
        })
        .collect();
    output.drop_without_applying_deltas();
    (rect, sampled, shots)
}

/// Photoshop's gesture inside the app: a press on the button and a drag samples the pixel of the
/// app's last frame under the pointer each pass; a release outside the app's windows starts the
/// capture view instead.
#[test]
fn a_press_and_drag_from_the_button_samples_the_apps_frame() {
    use egui::{Event, Modifiers, Pos2};
    let ctx = Context::default();
    let owner = Id::unique("asker");
    let (rect, ..) = hold_pass(&ctx, owner, Vec::new());
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let from = rect.center();
    hold_pass(&ctx, owner, vec![Event::PointerMoved(from)]);
    hold_pass(&ctx, owner, vec![button(from, true)]);
    let inside = Pos2::new(120.0, 60.0);
    let (_, sampled, shots) = hold_pass(&ctx, owner, vec![Event::PointerMoved(inside)]);
    assert_eq!(sampled, None, "no frame has arrived yet");
    assert!(!shots.is_empty(), "a dragged press asks for the app's frame");
    for shot in shots {
        shot.complete(std::sync::Arc::new(egui::ColorImage::filled([200, 100], egui::Color32::from_rgb(255, 0, 0))));
    }
    let (_, sampled, _) = hold_pass(&ctx, owner, vec![Event::PointerMoved(inside + vec2(1.0, 0.0))]);
    assert_eq!(sampled.map(|c| c.to_hex()), Some("#ff0000".into()), "the held drag samples live");
    let (_, sampled, _) = hold_pass(&ctx, owner, vec![button(inside, false)]);
    assert_eq!(sampled.map(|c| c.to_hex()), Some("#ff0000".into()), "the release inside samples");
    assert!(!ScreenColorPicker::busy(&ctx));

    // A second drag released outside the app's window opens the capture view.
    hold_pass(&ctx, owner, vec![Event::PointerMoved(from)]);
    hold_pass(&ctx, owner, vec![button(from, true)]);
    hold_pass(&ctx, owner, vec![Event::PointerMoved(inside)]);
    let outside = Pos2::new(900.0, 900.0);
    hold_pass(&ctx, owner, vec![Event::PointerMoved(outside)]);
    hold_pass(&ctx, owner, vec![button(outside, false)]);
    assert!(ScreenColorPicker::busy(&ctx), "the release outside asked for a pick");
}
