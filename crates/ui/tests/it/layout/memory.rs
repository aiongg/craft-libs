//! `LayoutMemory`: the tolerance rule and size tweens.

use craft_behavior::ColumnWidth;
use craft_ui::layout::BodyLayout;
use craft_ui::testing::layout::{body, is_animating, measured_block, read_size, start_tween};
use craft_ui::testing::{Frames, trial};
use egui::Vec2;

static COLUMNS: [ColumnWidth; 2] = [ColumnWidth::Fixed(60.0), ColumnWidth::Content];

/// Skeleton: `frames_state::end_aligned_readout_changing_every_frame_runs_one_pass`.
///
/// One rule covers every measured block: a first appearance and a change after a settled frame run one
/// discard pass, and a size that changes every frame is placed from last frame's value and runs none.
#[trial]
fn continuous_change_runs_no_discard_and_a_settled_change_runs_one() {
    let mut frames = Frames::new();
    let grid = BodyLayout::grid(&COLUMNS);
    let show = |frames: &mut Frames, width: f32| {
        frames.frame(vec![], |ui| {
            measured_block(ui, "readout", Vec2::new(width, 20.0));
            body(ui, "grid", &grid, &[Vec2::new(60.0, 20.0), Vec2::new(width, 20.0)]);
        })
    };
    assert_eq!(show(&mut frames, 20.0), 2, "the first appearance runs a discard pass");
    let growing: Vec<usize> = (1..10).map(|step| show(&mut frames, 20.0 + 30.0 * step as f32)).collect();
    assert!(growing[1..].iter().all(|passes| *passes == 1), "continuous change runs one pass per frame: {growing:?}");
    for _ in 0..3 {
        assert!(show(&mut frames, 290.0) <= 2, "settling takes at most one extra pass");
    }
    assert_eq!(show(&mut frames, 290.0), 1, "a settled block runs one pass");
    assert_eq!(show(&mut frames, 400.0), 2, "a change after a settled frame runs a discard pass, for both blocks together");
    assert_eq!(show(&mut frames, 400.4), 1, "a change under half a point is the same size");
}

/// 6.3: a size tween is evaluated at the reader's clock and asks for no discard pass while it runs.
#[trial]
fn a_size_tween_runs_without_a_discard_pass() {
    let mut frames = Frames::new();
    let mut seen = Vec::new();
    for frame in 0..30 {
        let passes = frames.frame(vec![], |ui| {
            if frame == 0 {
                start_tween(ui, "collapse", Vec2::ZERO, Vec2::new(100.0, 80.0), 0.25);
            }
            seen.push((read_size(ui, "collapse"), is_animating(ui, "collapse")));
        });
        assert_eq!(passes, 1, "frame {frame}");
    }
    let sizes: Vec<f32> = seen.iter().filter_map(|(size, _)| size.map(|size| size.y)).collect();
    assert!(sizes.windows(2).all(|pair| pair[0] <= pair[1]) && sizes[0] < 20.0, "the height rises from the start: {sizes:?}");
    assert_eq!(seen.last(), Some(&(Some(Vec2::new(100.0, 80.0)), false)), "and ends at the target");
    assert!(seen.iter().any(|(_, animating)| *animating));
}
