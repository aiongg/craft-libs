//! `Captures`: the admission budget, the geometry epoch and the reply encoding.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use craft_automation::control::ControlResponse;
use craft_automation::screenshot::{CaptureGeometry, Captures, MAX_PENDING, MAX_RETRIES, SETTLE_PASSES, screenshot_reply};
use craft_ui::runtime::windows::WindowHost;
use craft_ui::testing::trial;
use egui::{Color32, ColorImage, Context, ScreenshotCallback, ViewportCommand, ViewportId};

fn geometry(width: f32, height: f32) -> CaptureGeometry {
    CaptureGeometry { size_points: [width, height], pixels_per_point: 1.0, zoom: 1.0, max_texture_side: 16384, minimized: false, document: None }
}

fn admit(captures: &mut Captures, token: u64, now: Instant) -> Result<Receiver<ControlResponse>, String> {
    let (tx, rx) = channel();
    captures.admit(token, None, tx, None, 0.0, now).map(|()| rx).map_err(|(_, e)| e)
}

fn image(geometry: &CaptureGeometry) -> Arc<ColorImage> {
    let [w, h] = geometry.expected_pixels().expect("a capturable geometry");
    Arc::new(ColorImage::filled([w, h], Color32::WHITE))
}

/// Settles and issues the one request through the context's window host, and returns the callback of
/// the screenshot command the host sent.
fn issue_one(captures: &mut Captures, ctx: &Context, now: Instant) -> ScreenshotCallback {
    for _ in 0..SETTLE_PASSES {
        captures.note_pass();
    }
    let finished = captures.issue(&WindowHost::of(ctx), now, None);
    assert!(finished.is_empty());
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let mut callbacks: Vec<ScreenshotCallback> = output
        .viewport_output
        .get(&ViewportId::ROOT)
        .map(|out| {
            out.commands
                .iter()
                .filter_map(|command| if let ViewportCommand::Screenshot(callback) = command { Some(callback.clone()) } else { None })
                .collect()
        })
        .unwrap_or_default();
    callbacks.pop().expect("a capture command was sent")
}

/// More than `MAX_PENDING` or over the byte budgets is refused with the reply sender returned.
#[trial]
fn admission_budget_refuses_excess() {
    let now = Instant::now();
    let mut captures = Captures::default();
    captures.note_geometry(geometry(100.0, 100.0));
    let _keep: Vec<_> = (0..MAX_PENDING as u64).map(|token| admit(&mut captures, token, now).expect("admitted")).collect();
    assert_eq!(admit(&mut captures, 0, now).unwrap_err(), "screenshot token 0 is already pending");
    assert_eq!(admit(&mut captures, 99, now).unwrap_err(), format!("too many screenshots pending (at most {MAX_PENDING})"));
    assert_eq!(captures.pending(), MAX_PENDING);

    let mut captures = Captures::default();
    captures.note_geometry(geometry(12_000.0, 12_000.0));
    assert!(admit(&mut captures, 1, now).unwrap_err().contains("per-image limit"));
    // 5000 x 5000 x 4 bytes is 100 MB: two fit in 256 MiB, three do not.
    captures.note_geometry(geometry(5000.0, 5000.0));
    let _a = admit(&mut captures, 1, now).expect("admitted");
    let _b = admit(&mut captures, 2, now).expect("admitted");
    assert!(admit(&mut captures, 3, now).unwrap_err().starts_with("screenshot budget exhausted: 300000000 bytes"));
    assert_eq!(captures.reserved_bytes(), 200_000_000);
}

/// A geometry change between issue and collection retries up to `MAX_RETRIES`, then fails.
#[trial]
fn geometry_change_in_flight_retries() {
    let ctx = Context::default();
    let now = Instant::now();
    let mut captures = Captures::default();
    captures.note_geometry(geometry(100.0, 80.0));
    let rx = admit(&mut captures, 7, now).expect("admitted");

    for attempt in 0..=MAX_RETRIES {
        let callback = issue_one(&mut captures, &ctx, now);
        captures.note_geometry(geometry(100.0 + f32::from(attempt) + 1.0, 80.0));
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

/// A request before the first geometry waits and is issued once geometry is known.
#[trial]
fn early_request_waits_for_geometry() {
    let ctx = Context::default();
    let now = Instant::now();
    let mut captures = Captures::default();
    let _rx = admit(&mut captures, 1, now).expect("admitted");
    assert_eq!((captures.pending(), captures.any_unissued()), (1, true));

    let g = geometry(64.0, 48.0);
    captures.note_geometry(g);
    assert!(captures.drain(now, Some(g), None).is_empty());
    let callback = issue_one(&mut captures, &ctx, now);
    assert!(!captures.any_unissued());
    callback.complete(image(&g));
    let finished = captures.drain(now, Some(g), None);
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].result.as_ref().expect("an image").size, [64, 48]);
}

/// A finished image is encoded as a PNG that decodes to the same pixels, inlined as base64 or handed to the writer; an
/// encode-budget overflow and a write error are error replies.
#[trial]
fn screenshot_reply_encodes_a_png_and_writes_it() {
    use base64::Engine as _;
    let mut pixels = ColorImage::filled([3, 2], Color32::from_rgb(10, 20, 30));
    pixels.pixels[4] = Color32::from_rgb(200, 100, 50);
    let image = Arc::new(pixels);
    let decode = |bytes: &[u8]| {
        let mut reader = png::Decoder::new(bytes).read_info().expect("a PNG");
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("a frame");
        (info.width, info.height, buf)
    };
    let expected: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();

    let inline = screenshot_reply(5, image.clone(), None, &mut |_, _| Err("no path, no write".into()));
    assert_eq!((inline["result"]["width"].clone(), inline["result"]["mime"].clone()), (3.into(), "image/png".into()));
    let bytes = base64::engine::general_purpose::STANDARD.decode(inline["result"]["base64"].as_str().expect("base64")).expect("valid base64");
    assert_eq!(decode(&bytes), (3, 2, expected.clone()));

    let mut written = Vec::new();
    let reply = screenshot_reply(5, image.clone(), Some("out.png"), &mut |path, png| {
        written.push((path.to_owned(), png.to_vec()));
        Ok(())
    });
    assert_eq!(reply, serde_json::json!({"ok": true, "result": {"path": "out.png", "width": 3, "height": 2}}));
    assert_eq!(written[0].0, "out.png");
    assert_eq!(decode(&written[0].1), (3, 2, expected));

    let failed = screenshot_reply(5, image, Some("../out.png"), &mut |_, _| Err("path is outside the workspace".into()));
    assert_eq!(failed, serde_json::json!({"ok": false, "error": "path is outside the workspace"}));

    let huge = Arc::new(ColorImage::filled([8192, 4097], Color32::BLACK));
    let over = screenshot_reply(6, huge, None, &mut |_, _| Ok(()));
    assert!(over["error"].as_str().expect("an error").starts_with("screenshot 6 failed: the image exceeds the 128 MiB"), "{over}");
}
