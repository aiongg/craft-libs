//! `TextureCache`: states, in-place updates, eviction, reduction and region updates, on a bare
//! `egui::Context` (`max_texture_side` set in the raw input where a test needs it).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A put at a new revision keeps the entry's TextureId (`TextureHandle::set` in place).
#[test]
#[ignore = "skeleton"]
fn cache_updates_texture_in_place_when_revision_changes() {
    todo!()
}

// After a put at revision 1, `state` at revision 2 is Loading with the revision-1 picture as stand-in.
#[test]
#[ignore = "skeleton"]
fn cache_shows_stand_in_while_loading() {
    todo!()
}

// Over budget, the least recently seen entries go first, by bytes and by count.
#[test]
#[ignore = "skeleton"]
fn cache_evicts_oldest_unseen_over_budget() {
    todo!()
}

// Entries seen in this pass or the previous one stay even past both caps.
#[test]
#[ignore = "skeleton"]
fn cache_never_evicts_entries_seen_this_or_last_pass() {
    todo!()
}

// A 9000 px wide image is stored at most `texture_side` wide (by the whole factor 3 at 4096).
#[test]
#[ignore = "skeleton"]
fn cache_shrinks_images_past_device_limit() {
    todo!()
}

// `put_empty` and `put_failure` answer Empty and Failed for their revision and Missing for the next.
#[test]
#[ignore = "skeleton"]
fn cache_remembers_empty_and_failed() {
    todo!()
}

// `get_or_put` calls `make` once per key and revision.
#[test]
#[ignore = "skeleton"]
fn cache_get_or_put_makes_once_per_revision() {
    todo!()
}

// `put_region` keeps the TextureId and changes the region's texels; on an image reduced by 2 an
// aligned region lands at origin / 2 and an unaligned one returns None.
#[test]
#[ignore = "skeleton"]
fn cache_put_region_updates_part_of_a_texture() {
    todo!()
}

// The first upload of a pass is allowed even past the byte allowance; the 17th of a pass is not.
#[test]
#[ignore = "skeleton"]
fn cache_upload_allowance_resets_each_pass() {
    todo!()
}
