//! The suite's right-to-left case compares a component's own rect between directions: a component that changes its
//! own width under right-to-left fails it.

use craft_ui::buttons::Button;
use craft_ui::contract::Show as _;
use craft_ui::runtime::env::Env;
use craft_ui::testing::story::{Story, StoryData, StoryOutput};
use craft_ui::testing::suite::rtl_case;
use craft_ui::testing::trial;
use egui::Ui;

/// A button whose label, and so its width, depends on the layout direction.
fn wider_under_rtl(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let label = if Env::of(ui).view().direction().is_rtl() { "A much longer label under right-to-left" } else { "Short" };
    Button::new(label).show(ui).into()
}

fn same_in_both(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Button::new("Same").show(ui).into()
}

#[trial]
fn the_rtl_case_fails_a_component_whose_own_width_changes_under_rtl() {
    let failed = rtl_case(&Story::new("wider-under-rtl", wider_under_rtl)).expect_err("the width changes");
    assert!(failed.contains("width differs"), "{failed}");
}

#[trial]
fn the_rtl_case_passes_a_component_that_keeps_its_width() {
    assert_eq!(rtl_case(&Story::new("same", same_in_both)), Ok(()));
}
