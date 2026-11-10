//! Stories of the before and after view. The two pictures are fakes the story paints (a gradient and the same
//! gradient toned), so every mode shows without a renderer.

use super::{BeforeAfter, BeforeAfterMode, BeforeAfterPane, BeforeAfterSide, DIVIDER};
use crate::contract::{Bind, Show as _, Validity};
use crate::primitives::ContentPainterCx;
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, Grip, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::{Color32, Mesh, Shape, Ui, Vec2};

/// The picture's aspect ratio: story-local.
const ASPECT: f32 = 1.4;
/// Strength of the tone the "after" fake adds to the gradient.
const TONE: f32 = 0.45;
/// The most the divider may be moved toward the after side in the invalid story.
const MOST: f32 = 0.8;

fn paint_fake(painter: &ContentPainterCx<'_>, pane: &BeforeAfterPane) {
    let (from, to, tone) = (Color32::from_gray(220), Color32::from_gray(30), Color32::from_rgb(70, 130, 220));
    let blend = |c: Color32| if pane.side == BeforeAfterSide::After { c.lerp_to_gamma(tone, TONE) } else { c };
    let corners = [(pane.image.left_top(), 0.0), (pane.image.right_top(), 0.5), (pane.image.right_bottom(), 1.0), (pane.image.left_bottom(), 0.5)];
    let mut mesh = Mesh::default();
    for (pos, t) in corners {
        mesh.colored_vertex(pos, blend(from.lerp_to_gamma(to, t)));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.painter.add(Shape::mesh(mesh));
}

fn show(ui: &mut Ui, mode: BeforeAfterMode, at: Bind<'_, f32>, captions: bool, anywhere: bool) -> StoryOutput {
    BeforeAfter::new("Compare", mode, at).aspect(ASPECT).captions(captions).drag_anywhere(anywhere).pictures(paint_fake).show(ui).into()
}

fn drag_knob(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(grip) = fixture.part(id, DIVIDER.at(0)) else { return };
    let travel = 2.0 * crate::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&grip, Vec2::new(travel, 0.0));
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Custom(drag_knob))]
fn split_view(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::Split, Bind::bound(data.value("fraction", 0.5_f32)), false, true)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Drag(Grip::new(DIVIDER.at(0).key)))]
fn split_stacked(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::SplitStacked, Bind::bound(data.value("fraction", 0.5_f32)), false, true)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Custom(drag_knob))]
fn split_view_with_captions(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::Split, Bind::bound(data.value("fraction", 0.5_f32)), true, true)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Custom(drag_knob))]
fn drag_near_the_line_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::Split, Bind::bound(data.value("fraction", 0.5_f32)), false, false)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Never)]
fn side_by_side(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::SideBySide, Bind::bound(data.value("fraction", 0.5_f32)), true, true)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Never)]
fn stacked(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::Stacked, Bind::bound(data.value("fraction", 0.5_f32)), true, true)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Never)]
fn after_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::After, Bind::bound(data.value("fraction", 0.5_f32)), true, true)
}

#[story(BeforeAfter<'static>, covers = [Default], drive = Drive::Never)]
fn original_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::Original, Bind::bound(data.value("fraction", 0.5_f32)), true, true)
}

#[story(BeforeAfter<'static>, covers = [Mixed], drive = Drive::Custom(drag_knob))]
fn positions_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show(ui, BeforeAfterMode::Split, Bind::bound_mixed(data.value("fraction", Mixed::<f32>::Mixed)), false, true)
}

#[story(BeforeAfter<'static>, covers = [Invalid], drive = Drive::Custom(drag_knob))]
fn divider_too_far_right(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let too_far = |at: &f32| if *at > MOST { Validity::Refuse(Message::new("The divider is too far right")) } else { Validity::Ok };
    show(ui, BeforeAfterMode::Split, Bind::bound(data.value("fraction", MOST + 0.1)).check(too_far), false, true)
}
