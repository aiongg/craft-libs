//! The edge fade of a scroll region: a fade at each edge past which content is hidden, and none where nothing is.

use super::kit::Slab;
use crate::collection::kit::{List, Row, Words};
use craft_i18n::Direction;
use craft_ui::contract::{ContainerProps, Props as _, Scroll, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Shape, Ui, Vec2};

const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const LINE: Vec2 = Vec2::new(100.0, 20.0);
/// One wheel step, and enough steps to pass the end of forty lines in a 200-point view.
const STEP: f32 = 200.0;
const STEPS_TO_END: usize = 6;

/// An edge of the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// The edges the fades in `shapes` sit at: each fade is a gradient quad, opaque at the edge it sits at and clear
/// at the other side.
fn faded_edges(shapes: &[Shape]) -> Vec<Edge> {
    let mut edges: Vec<Edge> = shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Mesh(mesh) if mesh.vertices.len() == 4 => Some(mesh),
            _ => None,
        })
        .filter_map(|mesh| {
            let (clear, shown): (Vec<_>, Vec<_>) = mesh.vertices.iter().partition(|vertex| vertex.color.a() == 0);
            (clear.len() == 2 && shown.len() == 2).then(|| {
                let at = |vertices: &[&egui::epaint::Vertex]| vertices.iter().fold(Vec2::ZERO, |sum, vertex| sum + vertex.pos.to_vec2()) / 2.0;
                let toward = at(&shown) - at(&clear);
                match (toward.x.abs() > toward.y.abs(), toward.x > 0.0, toward.y > 0.0) {
                    (true, true, _) => Edge::Right,
                    (true, false, _) => Edge::Left,
                    (false, _, true) => Edge::Bottom,
                    (false, _, false) => Edge::Top,
                }
            })
        })
        .collect();
    edges.sort_by_key(|edge| *edge as u8);
    edges
}

/// A body scrolling on `scroll` over `count` children of `child` points.
fn body(ui: &mut Ui, scroll: Scroll, child: Vec2, count: usize) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        Slab::new(ContainerProps::new().scroll(scroll)).id_salt("body").show(ui, |ui, _| {
            for _ in 0..count {
                Block::new(child).show(ui);
            }
        });
    });
}

fn settled(frames: &mut Frames, show: &mut impl FnMut(&mut Ui)) -> Vec<Edge> {
    frames.idle(3, &mut *show);
    faded_edges(&frames.frame_shapes(&mut *show))
}

/// A body fades the edges its content runs past: at rest the end only, scrolled to the end the start only, under
/// right-to-left the mirrored side; content that fits fades nothing. A collection's rows fade the same way.
#[trial]
fn a_scroll_region_fades_the_edges_past_which_content_is_hidden() {
    let mut frames = Frames::new();
    let mut tall = |ui: &mut Ui| body(ui, Scroll::vertical(), LINE, 40);
    assert_eq!(settled(&mut frames, &mut tall), vec![Edge::Bottom], "at rest, only the end hides content");
    frames.frame(vec![pointer::moved(ORIGIN + LINE / 2.0)], &mut tall);
    for _ in 0..STEPS_TO_END {
        frames.frame(vec![pointer::wheel(-STEP)], &mut tall);
    }
    assert_eq!(settled(&mut frames, &mut tall), vec![Edge::Top], "scrolled to the end, only the start hides content");

    let mut fits = Frames::new();
    assert_eq!(settled(&mut fits, &mut |ui: &mut Ui| body(ui, Scroll::vertical(), LINE, 2)), vec![], "content that fits fades nothing");

    let mut narrow_row = |ui: &mut Ui| body(ui, Scroll::horizontal(), LINE, 1);
    let mut wide_row = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            Slab::new(ContainerProps::new().scroll(Scroll::horizontal())).id_salt("row").show(ui, |ui, _| {
                Block::new(Vec2::new(VIEW.x * 3.0, LINE.y)).show(ui);
            });
        });
    };
    let mut ltr = Frames::new();
    assert_eq!(settled(&mut ltr, &mut narrow_row), vec![], "a horizontal body that fits fades nothing");
    assert_eq!(settled(&mut ltr, &mut wide_row), vec![Edge::Right], "left to right, the end is the right edge");
    let mut rtl = Frames::new();
    rtl.root(RootEnv::new().direction(Direction::Rtl));
    assert_eq!(settled(&mut rtl, &mut wide_row), vec![Edge::Left], "right to left, the end is the left edge");

    let words: &'static Words = Box::leak(Box::new(Words::flat(&["row"; 40])));
    let mut list = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            List::new(SelectMode::Single).id_salt("list").show_items(ui, words, &mut |_, _| Row::new())
        });
    };
    let mut rows = Frames::new();
    assert_eq!(settled(&mut rows, &mut list), vec![Edge::Bottom], "a collection's rows fade the end they run past");
}
