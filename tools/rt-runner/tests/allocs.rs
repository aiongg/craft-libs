//! A frame's allocations on the real epaint renderer, counted as a relation to the experiment's
//! v4 runtime (ui-experiment b91c16c, `runner --bench`, same panel and renderer options).

use craft_rt_runner::backend;
use craft_rt_runner::run;
use craft_ui_test_kit::rt::alloc::CountingAlloc;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

/// The experiment's median allocations of one hover frame on a settled 20-layer panel: 197. This
/// runtime made 251 until the renderer joined consecutive shapes of one clip and texture into one
/// mesh the way epaint does and moved the last damage rect's mesh instead of cloning it; it makes
/// about 170 now. The bound is the experiment's count, so it holds while a frame costs no more than
/// the experiment's did.
const HOVER_ALLOCS_MAX: f64 = 197.0;

#[test]
fn a_hover_frame_on_a_settled_panel_allocates_no_more_than_the_experiments() {
    let rows = run::bench(backend::epaint, &[20], &[1.0], Some(&["hover".to_owned()]));
    let row = &rows[0].row;
    assert!(row.alloc_counted, "the counting allocator is installed");
    assert!(row.ran_frames > 0, "hover frames ran");
    assert!(row.allocs.median <= HOVER_ALLOCS_MAX, "a hover frame made {} allocations (limit {HOVER_ALLOCS_MAX})", row.allocs.median);
}
