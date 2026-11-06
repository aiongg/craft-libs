//! `measure_slot`: an invisible sizing pass at a width.

use craft_ui::layout::measure_slot;
use craft_ui::testing::layout::wrapped_text;
use craft_ui::testing::{Frames, trial};

#[trial]
fn measure_slot_wraps_at_the_width_and_takes_nothing_from_the_parent() {
    let mut frames = Frames::new();
    let text = "a sentence that is long enough to wrap in a narrow slot but not in a wide one";
    frames.frame(vec![], |ui| {
        let before = (ui.cursor().min, ui.min_rect());
        let narrow = measure_slot(ui, 100.0, |ui| wrapped_text(ui, text));
        let wide = measure_slot(ui, 600.0, |ui| wrapped_text(ui, text));
        assert!(narrow.x <= 100.0 && wide.x > 100.0, "the width bounds the content: {narrow:?} {wide:?}");
        assert!(narrow.y > wide.y, "a narrower slot wraps to more lines: {narrow:?} {wide:?}");
        assert_eq!(before, (ui.cursor().min, ui.min_rect()), "nothing is allocated in the parent");
    });
}
