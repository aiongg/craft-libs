//! The screenshot slot: one completion per slot, a bounded image, and a wake of the owning viewport only.

use std::sync::{Arc, Mutex};

use craft_ui::runtime::egui::screenshot::{ScreenshotError, ScreenshotSlot};
use craft_ui::runtime::windows::WindowHost;
use craft_ui::testing::trial;
use egui::{Color32, ColorImage, Context, ScreenshotCallback, ViewportCommand, ViewportId};

fn image(size: [usize; 2]) -> Arc<ColorImage> {
    Arc::new(ColorImage::filled(size, Color32::default()))
}

/// The viewports the context was asked to repaint.
fn observe_repaints(ctx: &Context) -> Arc<Mutex<Vec<ViewportId>>> {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let observed = requests.clone();
    ctx.set_repaint_observer(move |info| observed.lock().unwrap().push(info.viewport_id));
    requests
}

#[trial]
fn callback_clones_share_one_completion_and_take_is_single_use() {
    let ctx = Context::default();
    let (slot, callback) = ScreenshotSlot::detached(&ctx, ViewportId::ROOT, 1024);
    let duplicate = callback.clone();
    let first_image = image([1, 1]);
    let second_image = image([1, 1]);
    let weak_loser = Arc::downgrade(&second_image);

    callback.complete(first_image.clone());
    duplicate.complete(second_image);
    assert!(weak_loser.upgrade().is_none(), "the losing completion releases its image");

    let taken = slot.take().unwrap().unwrap();
    assert!(Arc::ptr_eq(&taken, &first_image), "the first completion wins");
    assert!(slot.take().is_none(), "the result is taken once");
}

#[trial]
fn cancel_releases_late_images_without_waking() {
    let ctx = Context::default();
    let requests = observe_repaints(&ctx);
    let (slot, callback) = ScreenshotSlot::detached(&ctx, ViewportId::from_hash_of("cancelled-window"), 1024);
    slot.cancel();

    let late_image = image([2, 2]);
    let weak_image = Arc::downgrade(&late_image);
    callback.complete(late_image.clone());
    drop(late_image);

    assert!(weak_image.upgrade().is_none(), "a cancelled slot releases the late image");
    assert!(slot.take().is_none());
    assert!(requests.lock().unwrap().is_empty(), "cancellation suppresses the wakeup");
}

#[trial]
fn rejects_oversized_malformed_and_overflowing_images() {
    let ctx = Context::default();

    let (slot, callback) = ScreenshotSlot::detached(&ctx, ViewportId::ROOT, 16);
    callback.complete(image([3, 3]));
    assert!(matches!(slot.take(), Some(Err(ScreenshotError::ImageTooLarge { bytes: 36, max_bytes: 16, .. }))));
    assert!(slot.take().is_none(), "a completion error is reported once");

    let (slot, callback) = ScreenshotSlot::detached(&ctx, ViewportId::ROOT, 1024);
    callback.complete(Arc::new(ColorImage { size: [2, 2], source_size: egui::Vec2::ZERO, pixels: vec![Color32::default(); 3] }));
    assert!(matches!(slot.take(), Some(Err(ScreenshotError::MalformedImage { width: 2, height: 2, pixels: 3, .. }))));

    let (slot, callback) = ScreenshotSlot::detached(&ctx, ViewportId::ROOT, 1024);
    callback.complete(Arc::new(ColorImage { size: [usize::MAX, 2], source_size: egui::Vec2::ZERO, pixels: Vec::new() }));
    assert!(matches!(slot.take(), Some(Err(ScreenshotError::DimensionsOverflow { width, height, .. })) if width == usize::MAX && height == 2));

    // The pixel count fits but the byte count does not.
    let wide = usize::MAX / 2;
    let (slot, callback) = ScreenshotSlot::detached(&ctx, ViewportId::ROOT, 1024);
    callback.complete(Arc::new(ColorImage { size: [wide, 1], source_size: egui::Vec2::ZERO, pixels: Vec::new() }));
    assert!(matches!(slot.take(), Some(Err(ScreenshotError::DimensionsOverflow { width, height, .. })) if width == wide && height == 1));
}

#[trial]
fn another_thread_completes_and_wakes_only_the_owning_viewport() {
    let ctx = Context::default();
    let requests = observe_repaints(&ctx);
    let viewport = ViewportId::from_hash_of("child-window");
    let (slot, callback) = ScreenshotSlot::detached(&ctx, viewport, 1024);
    let expected = image([1, 1]);
    let sent = expected.clone();

    std::thread::spawn(move || callback.complete(sent)).join().unwrap();

    let requested = requests.lock().unwrap();
    assert!(!requested.is_empty(), "completion wakes the owning viewport");
    assert!(requested.iter().all(|requested_viewport| *requested_viewport == viewport), "wakes only the owning viewport: {requested:?}");
    drop(requested);
    assert!(Arc::ptr_eq(&slot.take().unwrap().unwrap(), &expected));
}

/// Each screenshot request is one slot and one screenshot command for its viewport, and the renderer's completion reaches
/// that slot.
#[trial]
fn issue_sends_one_screenshot_command_for_its_viewport() {
    let ctx = Context::default();
    let viewport = ViewportId::ROOT;
    let slot = WindowHost::of(&ctx).screenshot(viewport, 1024);
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let mut callbacks: Vec<ScreenshotCallback> = output
        .viewport_output
        .get(&viewport)
        .map(|out| {
            out.commands
                .iter()
                .filter_map(|command| if let ViewportCommand::Screenshot(callback) = command { Some(callback.clone()) } else { None })
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(callbacks.len(), 1, "one command per call");
    assert!(slot.take().is_none());
    let rendered = image([1, 1]);
    callbacks.remove(0).complete(rendered.clone());
    assert!(Arc::ptr_eq(&slot.take().unwrap().unwrap(), &rendered));
}
