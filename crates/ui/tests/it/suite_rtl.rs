//! The suite's right-to-left case compares a component's own rect between directions: a component that changes its
//! own width under right-to-left fails it, and so does content that does not follow the component's rect when that
//! rect moves.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::runtime::env::Env;
use craft_ui::testing::story::{Story, StoryData, StoryOutput};
use craft_ui::testing::suite::rtl_case;
use craft_ui::testing::trial;
use craft_ui::text::TextField;
use egui::{Layout, Rect, Ui, UiBuilder, pos2, vec2};

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

const FIELD_WIDTH: f32 = 200.0;
const SHIFT: f32 = 40.0;

/// A search field with text, so that it has a clear part at its end, in the rect starting at `at`.
fn search_field(ui: &mut Ui, at: egui::Pos2) -> StoryOutput {
    let rect = Rect::from_min_size(at, vec2(FIELD_WIDTH, 28.0));
    let mut text = "text".to_owned();
    let builder = UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(egui::Align::Min));
    ui.scope_builder(builder, |ui| TextField::search("Search", Bind::bound(&mut text)).show(ui)).inner.into()
}

/// The field's rect moves right by `SHIFT` under right-to-left and its content mirrors inside it.
fn moves_with_its_content(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let shift = if Env::of(ui).view().direction().is_rtl() { SHIFT } else { 0.0 };
    search_field(ui, pos2(shift, 0.0))
}

/// As above, plus a second field that keeps its place while the first one moves.
fn leaves_content_behind(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let shift = if Env::of(ui).view().direction().is_rtl() { SHIFT } else { 0.0 };
    let shown = search_field(ui, pos2(shift, 0.0));
    search_field(ui, pos2(0.0, 40.0));
    shown
}

#[trial]
fn the_rtl_case_passes_a_component_whose_rect_moves_and_whose_content_mirrors_inside_it() {
    assert_eq!(rtl_case(&Story::new("moves-with-content", moves_with_its_content)), Ok(()));
}

#[trial]
fn the_rtl_case_fails_content_that_keeps_its_ltr_offsets_while_the_rect_moves() {
    let failed = rtl_case(&Story::new("leaves-content", leaves_content_behind)).expect_err("the second field does not move");
    assert!(failed.contains("not its reflection"), "{failed}");
}
