//! `TextureCache`: states, in-place updates, eviction, reduction and region updates, on a bare
//! `egui::Context` (`max_texture_side` set in the raw input where a test needs it).

use std::cell::Cell;

use egui::{Color32, ColorImage, Context, RawInput};

use super::{Revision, TextureBudget, TextureCache, TextureCacheOptions, TextureState, fit_image};

fn pass_output(ctx: &Context, side: usize) -> egui::FullOutput {
    ctx.run_ui(RawInput { max_texture_side: Some(side), ..RawInput::default() }, |_| {})
}

/// A pass whose texture deltas nobody inspects.
fn pass_with(ctx: &Context, side: usize) {
    pass_output(ctx, side).drop_without_applying_deltas();
}

fn pass(ctx: &Context) {
    pass_with(ctx, 2048);
}

fn passes(ctx: &Context, n: usize) {
    for _ in 0..n {
        pass(ctx);
    }
}

fn square(side: usize, color: Color32) -> ColorImage {
    ColorImage::filled([side, side], color)
}

fn cache_with(ctx: &Context, max_bytes: usize, max_entries: usize) -> TextureCache<u32> {
    let budget = TextureBudget { max_bytes, max_entries, ..TextureCacheOptions::thumbnails().budget };
    TextureCache::in_context(ctx, "test", TextureCacheOptions::thumbnails().budget(budget))
}

// A put at a new revision keeps the entry's TextureId (`TextureHandle::set` in place).
#[test]
fn cache_updates_texture_in_place_when_revision_changes() {
    let ctx = Context::default();
    pass(&ctx);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::thumbnails());
    let first = cache.put(&ctx, 1, Revision(1), square(4, Color32::RED));
    pass(&ctx);
    let second = cache.put(&ctx, 1, Revision(2), square(8, Color32::BLUE));
    assert_eq!(first.id, second.id);
    assert_eq!(second.size, [8, 8]);
    assert_eq!(cache.state(&ctx, &1, Revision(2)), TextureState::Ready(second));
    assert_eq!(cache.statistics().entries, 1);
    assert_eq!(cache.statistics().bytes, 8 * 8 * 4);
}

// After a put at revision 1, `state` at revision 2 is Loading with the revision-1 picture as stand-in.
#[test]
fn cache_shows_stand_in_while_loading() {
    let ctx = Context::default();
    pass(&ctx);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::thumbnails());
    assert_eq!(cache.state(&ctx, &1, Revision(1)), TextureState::Missing);
    let shown = cache.put(&ctx, 1, Revision(1), square(4, Color32::RED));
    assert_eq!(cache.state(&ctx, &1, Revision(1)), TextureState::Ready(shown));
    let state = cache.state(&ctx, &1, Revision(2));
    assert_eq!(state, TextureState::Loading { stand_in: Some(shown) });
    assert_eq!(state.shown(), Some(shown));
}

// Over budget, the least recently seen entries go first, by bytes and by count.
#[test]
fn cache_evicts_oldest_unseen_over_budget() {
    let ctx = Context::default();
    pass(&ctx);
    let by_count = cache_with(&ctx, usize::MAX, 2);
    for key in 0..4 {
        by_count.put(&ctx, key, Revision(1), square(2, Color32::RED));
        passes(&ctx, 3);
    }
    assert_eq!(by_count.statistics().entries, 2);
    assert_eq!(by_count.statistics().evicted, 2);
    assert_eq!(by_count.state(&ctx, &0, Revision(1)), TextureState::Missing);
    assert_eq!(by_count.state(&ctx, &1, Revision(1)), TextureState::Missing);
    assert!(matches!(by_count.state(&ctx, &2, Revision(1)), TextureState::Ready(_)));
    assert!(matches!(by_count.state(&ctx, &3, Revision(1)), TextureState::Ready(_)));

    let by_bytes = TextureCache::<u32>::in_context(
        &ctx,
        "bytes",
        TextureCacheOptions::thumbnails().budget(TextureBudget { max_bytes: 2 * 4 * 4 * 4, ..TextureCacheOptions::thumbnails().budget }),
    );
    for key in 0..3 {
        by_bytes.put(&ctx, key, Revision(1), square(4, Color32::RED));
        passes(&ctx, 3);
    }
    assert_eq!(by_bytes.statistics().bytes, 2 * 4 * 4 * 4);
    assert_eq!(by_bytes.state(&ctx, &0, Revision(1)), TextureState::Missing);
}

// Entries seen in this pass or the previous one stay even past both caps.
#[test]
fn cache_never_evicts_entries_seen_this_or_last_pass() {
    let ctx = Context::default();
    pass(&ctx);
    let cache = cache_with(&ctx, 1, 1);
    for key in 0..3 {
        cache.put(&ctx, key, Revision(1), square(2, Color32::RED));
    }
    assert_eq!(cache.statistics().entries, 3);
    assert_eq!(cache.statistics().evicted, 0);
    pass(&ctx);
    cache.put(&ctx, 3, Revision(1), square(2, Color32::RED));
    assert_eq!(cache.statistics().entries, 4, "seen in the previous pass");
    passes(&ctx, 2);
    cache.put(&ctx, 4, Revision(1), square(2, Color32::RED));
    assert_eq!(cache.statistics().entries, 1, "only this pass's remains");
}

// A 9000 px wide image is stored at most `texture_side` wide (by the whole factor 3 at 4096).
#[test]
fn cache_shrinks_images_past_device_limit() {
    let ctx = Context::default();
    pass_with(&ctx, 4096);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::pages());
    let shown = cache.put(&ctx, 1, Revision(1), ColorImage::filled([9000, 2], Color32::WHITE));
    assert_eq!(shown.size, [3000, 1]);
    assert_eq!(cache.statistics().bytes, 3000 * 4);

    // Each reduced texel is the premultiplied mean of its block, short blocks at the edge.
    let image = ColorImage::new([3, 1], vec![Color32::from_rgba_premultiplied(100, 0, 0, 100), Color32::TRANSPARENT, Color32::WHITE]);
    let reduced = fit_image(image, 2);
    assert_eq!(reduced.size, [2, 1]);
    assert_eq!(reduced.pixels, vec![Color32::from_rgba_premultiplied(50, 0, 0, 50), Color32::WHITE]);
}

// `put_empty` and `put_failure` answer Empty and Failed for their revision and Missing for the next.
#[test]
fn cache_remembers_empty_and_failed() {
    let ctx = Context::default();
    pass(&ctx);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::thumbnails());
    cache.put_empty(&ctx, 1, Revision(1));
    cache.put_failure(&ctx, 2, Revision(1), "bad file");
    assert_eq!(cache.state(&ctx, &1, Revision(1)), TextureState::Empty);
    assert_eq!(cache.state(&ctx, &2, Revision(1)), TextureState::Failed { message: "bad file".to_owned() });
    assert_eq!(cache.state(&ctx, &1, Revision(2)), TextureState::Missing);
    assert_eq!(cache.state(&ctx, &2, Revision(2)), TextureState::Missing);
    // A picture replaced by an empty answer frees its texture.
    cache.put(&ctx, 3, Revision(1), square(4, Color32::RED));
    cache.put_empty(&ctx, 3, Revision(2));
    assert_eq!(cache.statistics().bytes, 0);
}

// `get_or_put` calls `make` once per key and revision.
#[test]
fn cache_get_or_put_makes_once_per_revision() {
    let ctx = Context::default();
    pass(&ctx);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::thumbnails());
    let calls = Cell::new(0);
    let make = || {
        calls.set(calls.get() + 1);
        square(4, Color32::RED)
    };
    let first = cache.get_or_put(&ctx, 1, Revision(1), make);
    let again = cache.get_or_put(&ctx, 1, Revision(1), make);
    assert_eq!((first, calls.get()), (again, 1));
    let next = cache.get_or_put(&ctx, 1, Revision(2), make);
    assert_eq!((next.id, calls.get()), (first.id, 2));
    // An empty entry of the same revision is made again.
    cache.put_empty(&ctx, 1, Revision(3));
    cache.get_or_put(&ctx, 1, Revision(3), make);
    assert_eq!(calls.get(), 3);
}

// `put_region` keeps the TextureId and changes the region's texels; on an image reduced by 2 an
// aligned region lands at origin / 2 and an unaligned one returns None.
#[test]
fn cache_put_region_updates_part_of_a_texture() {
    let ctx = Context::default();
    pass_with(&ctx, 1024);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::pages());
    let whole = cache.put(&ctx, 1, Revision(1), ColorImage::filled([2048, 4], Color32::BLACK));
    assert_eq!(whole.size, [1024, 2]);
    pass_with(&ctx, 1024);

    assert!(cache.put_region(&ctx, &1, Revision(2), [1, 0], square(2, Color32::WHITE)).is_none(), "origin off the grid");
    assert!(cache.put_region(&ctx, &1, Revision(2), [0, 0], square(3, Color32::WHITE)).is_none(), "far edge off the grid");
    assert!(cache.put_region(&ctx, &1, Revision(2), [2046, 2], square(4, Color32::WHITE)).is_none(), "past the image");
    assert!(cache.put_region(&ctx, &9, Revision(2), [0, 0], square(2, Color32::WHITE)).is_none(), "no picture");
    assert_eq!(cache.state(&ctx, &1, Revision(1)), TextureState::Ready(whole));

    let updated = cache.put_region(&ctx, &1, Revision(2), [2, 2], square(2, Color32::WHITE)).expect("aligned");
    assert_eq!(updated, whole);
    assert_eq!(cache.state(&ctx, &1, Revision(2)), TextureState::Ready(whole));
    let mut output = pass_output(&ctx, 1024);
    let deltas = output.textures_delta.set.get(&whole.id).expect("a texture delta");
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].pos, Some([1, 1]));
    assert_eq!(deltas[0].image.size(), [1, 1]);
    output.textures_delta.clear();
}

// The first upload of a pass is allowed even past the byte allowance; the 17th of a pass is not.
#[test]
fn cache_upload_allowance_resets_each_pass() {
    let ctx = Context::default();
    pass(&ctx);
    let cache = TextureCache::<u32>::in_context(&ctx, "test", TextureCacheOptions::thumbnails());
    assert!(cache.allow_upload(&ctx, 1 << 30), "the first upload always lands");
    for key in 0..16 {
        assert!(cache.allow_upload(&ctx, 16), "upload {key}");
        cache.put(&ctx, key, Revision(1), square(2, Color32::RED));
    }
    assert!(!cache.allow_upload(&ctx, 16), "the 17th");
    assert_eq!(cache.statistics().uploads, 16);
    pass(&ctx);
    assert!(cache.allow_upload(&ctx, 16));
}
