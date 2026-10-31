//! Behaviour tests of `screenshot`.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use egui::{ColorImage, Context, ScreenshotCallback};

use super::*;

fn geometry(width: f32, height: f32) -> CaptureGeometry {
    CaptureGeometry { size_points: [width, height], pixels_per_point: 1.0, zoom: 1.0, max_texture_side: 16384, minimized: false, document: None }
}

fn admit(captures: &mut Captures, token: u64, now: Instant) -> Result<Receiver<ControlResponse>, String> {
    let (tx, rx) = channel();
    captures.admit(token, None, tx, None, 0.0, now).map(|()| rx).map_err(|(_, e)| e)
}

fn image(geometry: &CaptureGeometry) -> Arc<ColorImage> {
    let [w, h] = geometry.expected_pixels().unwrap();
    Arc::new(ColorImage::filled([w, h], egui::Color32::WHITE))
}

/// Settle and issue the one request; returns its callback.
fn issue_one(captures: &mut Captures, ctx: &Context, now: Instant) -> ScreenshotCallback {
    for _ in 0..SETTLE_PASSES {
        captures.note_pass();
    }
    let mut sent = Vec::new();
    let finished = captures.issue(ctx, now, None, |cb| sent.push(cb));
    assert!(finished.is_empty());
    sent.pop().expect("a capture command was sent")
}

// More than MAX_PENDING or over the byte budgets is refused with the reply sender returned.
#[test]
fn admission_budget_refuses_excess() {
    let now = Instant::now();
    let mut captures = Captures::default();
    captures.note_geometry(geometry(100.0, 100.0));
    let _keep: Vec<_> = (0..MAX_PENDING as u64).map(|t| admit(&mut captures, t, now).unwrap()).collect();
    assert_eq!(admit(&mut captures, 0, now).unwrap_err(), "screenshot token 0 is already pending");
    assert_eq!(admit(&mut captures, 99, now).unwrap_err(), format!("too many screenshots pending (at most {MAX_PENDING})"));
    assert_eq!(captures.pending(), MAX_PENDING);

    let mut captures = Captures::default();
    captures.note_geometry(geometry(12_000.0, 12_000.0));
    assert!(admit(&mut captures, 1, now).unwrap_err().contains("per-image limit"));
    // 5000 x 5000 x 4 bytes is 100 MB: two fit in 256 MiB, three do not.
    captures.note_geometry(geometry(5000.0, 5000.0));
    let _a = admit(&mut captures, 1, now).unwrap();
    let _b = admit(&mut captures, 2, now).unwrap();
    assert!(admit(&mut captures, 3, now).unwrap_err().starts_with("screenshot budget exhausted: 300000000 bytes"));
    assert_eq!(captures.reserved_bytes(), 200_000_000);
}

// A geometry change between issue and collection retries up to MAX_RETRIES, then fails.
#[test]
fn geometry_change_in_flight_retries() {
    let ctx = Context::default();
    let now = Instant::now();
    let mut captures = Captures::default();
    let mut current = geometry(100.0, 80.0);
    captures.note_geometry(current);
    let rx = admit(&mut captures, 7, now).unwrap();

    for attempt in 0..=MAX_RETRIES {
        let callback = issue_one(&mut captures, &ctx, now);
        current = geometry(100.0 + f32::from(attempt) + 1.0, 80.0);
        captures.note_geometry(current);
        callback.complete(image(&geometry(100.0 + f32::from(attempt), 80.0)));
        let finished = captures.drain(now, captures.last_geometry(), None);
        if attempt < MAX_RETRIES {
            assert!(finished.is_empty(), "attempt {attempt} retries");
            assert_eq!(captures.pending(), 1);
        } else {
            assert_eq!(finished.len(), 1);
            assert_eq!(finished[0].result.as_ref().unwrap_err(), "screenshot 7 failed: the layout changed while it was captured");
        }
    }
    assert_eq!(captures.pending(), 0);
    assert!(rx.try_recv().is_err(), "the caller replies from the Finished, not the queue");
}

// A request before the first geometry waits and is issued once geometry is known.
#[test]
fn early_request_waits_for_geometry() {
    let ctx = Context::default();
    let now = Instant::now();
    let mut captures = Captures::default();
    let _rx = admit(&mut captures, 1, now).unwrap();
    assert_eq!((captures.pending(), captures.any_unissued()), (1, true));

    let g = geometry(64.0, 48.0);
    captures.note_geometry(g);
    assert!(captures.drain(now, Some(g), None).is_empty());
    let callback = issue_one(&mut captures, &ctx, now);
    assert!(!captures.any_unissued());
    callback.complete(image(&g));
    let finished = captures.drain(now, Some(g), None);
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].result.as_ref().unwrap().size, [64, 48]);
}
