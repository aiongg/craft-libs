//! Tests of the hover outline and lookup (task K10), over `ManualJobs` so each job finishes when
//! the test says.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// The pointer rests 120 ms on one pixel before `job` is called; one job at a time, no second job
// while one runs; `pending` is true until its answer arrives.
#[test]
#[ignore = "skeleton"]
fn hover_waits_debounce_then_looks_up_once() {
    todo!()
}

// A Found answer is shown; moving to points it `covers` shows it again with no job.
#[test]
#[ignore = "skeleton"]
fn hover_cache_serves_points_inside_found_mask() {
    todo!()
}

// A Nothing answer suppresses lookups within 2 px on each axis; only the newest 64 misses count.
#[test]
#[ignore = "skeleton"]
fn hover_miss_suppresses_nearby_points() {
    todo!()
}

// A button down, `enabled` false, the pointer off the canvas, `clear`, and a revision or key change
// each clear the shown result; a job answering after a key change is dropped.
#[test]
#[ignore = "skeleton"]
fn hover_clears_on_press_tool_change_leave() {
    todo!()
}

// `empty_here` gives the hint, no shown result and no job; the hint is gone on the next point.
#[test]
#[ignore = "skeleton"]
fn hover_empty_layer_gives_the_hint() {
    todo!()
}

// A 2000 × 500 mask samples to 667 × 167 (3 units per texel); a 300 × 200 one stays 300 × 200;
// `is_edge` marks the border and the texels beside uncovered ones.
#[test]
#[ignore = "skeleton"]
fn hover_mask_texture_is_reduced_to_768() {
    todo!()
}
