//! Stories of `BezierOverlay`.

use super::{
    ANCHOR, AnchorsShown, BezierOutput, BezierOverlay, BezierTool, EditPath, EditSubpath, HandlesShown, Knot, KnotRef, PathSelection, index,
};
use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_theme::Srgba;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// An open subpath of three anchors, the last smooth with handles, and a closed triangle.
fn paths() -> Vec<EditPath> {
    let curve = Knot { anchor: Point::new(260.0, 60.0), in_ctrl: Point::new(230.0, 20.0), out_ctrl: Point::new(290.0, 100.0), smooth: true };
    let open = vec![Knot::corner(Point::new(60.0, 60.0)), Knot::corner(Point::new(160.0, 60.0)), curve];
    let triangle = [(60.0, 160.0), (160.0, 160.0), (110.0, 240.0)].map(|(x, y)| Knot::corner(Point::new(x, y))).to_vec();
    let subpaths = vec![EditSubpath { knots: triangle, closed: true }];
    vec![EditPath { key: 1, subpaths: vec![EditSubpath { knots: open, closed: false }], color: None }, EditPath { key: 2, subpaths, color: None }]
}

/// Shows `overlay` as the only canvas control of a canvas whose view has the document origin at its top left.
fn on_canvas(ui: &mut Ui, overlay: BezierOverlay<'_>) -> StoryOutput {
    let room = ui.available_rect_before_wrap();
    let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
    let mut overlay = Some(overlay);
    let shown = Canvas::new("Document")
        .view(view)
        .show(ui, |ui, body| -> Option<BezierOutput> { overlay.take().map(|overlay| overlay.show_at(ui, body.rect)) });
    shown.inner.flatten().map(StoryOutput::from).unwrap_or_default()
}

/// Drags the first anchor right by twice the drag threshold.
fn drag_first(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(anchor) = fixture.part(id, ANCHOR.at(index(0, KnotRef::new(0, 0)))) else { return };
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&anchor, Vec2::new(travel, 0.0));
}

fn selected() -> PathSelection {
    PathSelection { path: Some(1), anchors: vec![KnotRef::new(0, 2)], segment: None }
}

#[story(BezierOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn direct_selection_with_a_selected_anchor(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paths = data.value("paths", paths());
    on_canvas(ui, BezierOverlay::new("Paths", Bind::bound(&mut *paths), &selected()))
}

#[story(BezierOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn all_handles_shown(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paths = data.value("paths", paths());
    on_canvas(ui, BezierOverlay::new("Paths", Bind::bound(&mut *paths), &PathSelection::default()).handles(HandlesShown::All))
}

#[story(BezierOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn curvature_tool(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paths = data.value("paths", paths());
    on_canvas(ui, BezierOverlay::new("Paths", Bind::bound(&mut *paths), &selected()).tool(BezierTool::Curvature))
}

#[story(BezierOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn add_anchor_tool(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paths = data.value("paths", paths());
    on_canvas(ui, BezierOverlay::new("Paths", Bind::bound(&mut *paths), &PathSelection::default()).tool(BezierTool::AddAnchor))
}

#[story(BezierOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn outlines_only(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paths = data.value("paths", paths());
    on_canvas(
        ui,
        BezierOverlay::new("Paths", Bind::bound(&mut *paths), &PathSelection::default()).tool(BezierTool::Anchors).anchors(AnchorsShown::Hidden),
    )
}

#[story(BezierOverlay<'static>, covers = [Default], drive = Drive::Custom(drag_first))]
fn a_path_in_its_own_colour(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let coloured = || {
        let mut paths = paths();
        paths[0].color = Some(Srgba::rgb(0.9, 0.43, 0.08));
        paths
    };
    let paths = data.value("paths", coloured());
    on_canvas(ui, BezierOverlay::new("Paths", Bind::bound(&mut *paths), &selected()))
}

#[story(BezierOverlay<'static>, covers = [Invalid], drive = Drive::Never)]
fn paths_the_host_refuses(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paths = data.value("refused", paths());
    let refuse_many = |paths: &Vec<EditPath>| if paths.len() > 1 { Validity::Refuse(Message::new("Only one path")) } else { Validity::Ok };
    on_canvas(ui, BezierOverlay::new("Paths", Bind::bound(&mut *paths).check(refuse_many), &selected()))
}
