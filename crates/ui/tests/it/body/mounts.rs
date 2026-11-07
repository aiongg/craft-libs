//! The mount host: layers move and resize by pointer, a zoomed canvas body hits its members in canvas space,
//! and a viewport mount shows its body in the fallback layer where viewports are unavailable.

use super::kit::Slab;
use crate::contract::kit::{Probe, Seen};
use craft_behavior::CanvasTransform;
use craft_ui::contract::{ContainerProps, DragBand, HeaderRegion, Mount, Overlay, Props as _, Show as _, ShowContainer as _, Sizing};
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

/// What the pointer drags a layer by, and the most its position may differ from the drag.
const MOVE: Vec2 = Vec2::new(40.0, 30.0);
const GROW: Vec2 = Vec2::new(30.0, 20.0);
const SLACK: f32 = 1.5;
/// The body the layers show.
const BODY: Vec2 = Vec2::new(120.0, 60.0);

fn layer_props(overlay: Overlay) -> ContainerProps {
    ContainerProps::new().mount(Mount::Layer(overlay)).sizing(Sizing::Content).header(HeaderRegion::new().band(DragBand::Move))
}

/// Shows a layer container for `frames`, recording its rect.
fn show_layer(rect: &Cell<Rect>, overlay: &Overlay) -> impl FnMut(&mut Ui) {
    let overlay = overlay.clone();
    move |ui| {
        let out = Slab::new(layer_props(overlay.clone())).id_salt("layer").show(ui, |ui, _| {
            Block::new(BODY).show(ui);
        });
        rect.set(out.output.rect);
    }
}

#[trial]
fn a_layer_container_moves_by_its_header_band() {
    let rect = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let overlay = Overlay::floating();
    let mut show = show_layer(&rect, &overlay);
    frames.idle(4, &mut show);
    let before = rect.get();
    assert!(before.is_positive(), "the layer shows its body: {before:?}");
    let header = Pos2::new(before.center().x, before.min.y + 8.0);
    pointer::drag(&mut frames, header, MOVE, &mut show);
    frames.idle(2, &mut show);
    let moved = rect.get().min - before.min;
    assert!((moved - MOVE).abs().max_elem() < SLACK, "the layer followed the pointer: {moved:?}");
}

#[trial]
fn a_resizable_layer_resizes_by_its_corner_grip() {
    let rect = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let overlay = Overlay::floating().resizable(true);
    let mut show = show_layer(&rect, &overlay);
    frames.idle(4, &mut show);
    let before = rect.get();
    pointer::drag(&mut frames, before.max - Vec2::splat(1.0), GROW, &mut show);
    frames.idle(2, &mut show);
    let grown = rect.get().size() - before.size();
    assert!((grown - GROW).abs().max_elem() < SLACK, "the layer grew by the drag: {grown:?}");
    assert!((rect.get().min - before.min).abs().max_elem() < SLACK, "the opposite corner stayed: {:?} {before:?}", rect.get());
}

#[trial]
fn a_zoomed_canvas_body_hits_its_members_in_canvas_space() {
    let seen = RefCell::new(Seen::default());
    let body = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(400.0, 300.0)), |ui| {
            let props = ContainerProps::new().arena(true).transform(CanvasTransform::new(2.0, Vec2::ZERO));
            Slab::new(props).id_salt("canvas").show(ui, |ui, cx| {
                body.set(cx.rect);
                Probe::new(&seen).size(Vec2::new(40.0, 20.0)).show(ui);
            });
        });
    };
    frames.idle(4, &mut show);
    let origin = body.get().min;
    // The member is 40 x 20 in canvas space and 80 x 40 on screen.
    pointer::click(&mut frames, origin + Vec2::new(70.0, 30.0), &mut show);
    assert_eq!(seen.borrow().activations, 1, "a click inside the scaled member activates it");
    pointer::click(&mut frames, origin + Vec2::new(100.0, 30.0), &mut show);
    assert_eq!(seen.borrow().activations, 1, "a click outside the scaled member does not");
}

#[trial]
fn a_viewport_mount_shows_its_body_in_the_fallback_layer_under_test() {
    let (ran, rect) = (Cell::new(0), Cell::new(Rect::NOTHING));
    let mut frames = Frames::new();
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    frames.idle(4, |ui| {
        let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
        let out = Slab::new(props).id_salt("window").show(ui, |ui, _| {
            ran.set(ran.get() + 1);
            Block::new(BODY).show(ui);
        });
        rect.set(out.output.rect);
    });
    assert!(ran.get() > 0, "the body ran in the fallback layer");
    assert!(screen.contains_rect(rect.get()) && rect.get().width() >= BODY.x, "the fallback layer sits inside the window: {:?}", rect.get());
}
