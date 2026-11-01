//! Split views.

use egui::{Event, Modifiers, PointerButton, RawInput, Rect as ScreenRect, Sense, pos2, vec2};
use kurbo::Point;

use super::*;
use crate::theme::{Base, install};

const GAP: f32 = 2.0;

fn full() -> ScreenRect {
    ScreenRect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))
}

#[test]
fn two_views_put_the_main_view_on_the_right() {
    let rects = SplitLayout::Two.rects(GAP, full());
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x - rects[1].max.x, GAP);
    assert_eq!((rects[0].max, rects[1].min), (full().max, full().min));
    let four = SplitLayout::Four.rects(GAP, full());
    assert_eq!(four.len(), 4);
    assert_eq!(four[0].max, full().max, "the main view is at the bottom right");
    assert_eq!(four[0].min.x - four[1].max.x, GAP);
    assert_eq!(four[0].min.y - four[1].max.y, GAP);
    assert_eq!(SplitLayout::One.rects(GAP, full()), vec![full()]);
}

fn press(at: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[test]
fn a_press_on_an_inactive_pane_activates_it() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    // Pane 1 is the left one; pane 0 (right) is active.
    let at = pos2(50.0, 150.0);
    let run = |events: Vec<Event>| {
        let input = RawInput { screen_rect: Some(full()), events, ..RawInput::default() };
        let mut out = (SplitResponse::default(), [false; 2]);
        ctx.run_ui(input, |ui| {
            out.0 = SplitViews::new("split", SplitLayout::Two).show(ui, full(), |ui, pane, rect| {
                out.1[pane] = ui.interact(rect, ui.scope_id().with("tool"), Sense::drag()).dragged();
            });
        })
        .drop_without_applying_deltas();
        out
    };
    run(vec![Event::PointerMoved(at)]);
    let (response, tool_saw) = run(vec![press(at, true)]);
    assert_eq!(response.activated, Some(1));
    assert_eq!(tool_saw, [false, false], "the press does not reach the pane's tool");
    run(vec![press(at, false)]);
    // Once active, the pane's tool gets the press.
    let at = pos2(300.0, 150.0);
    run(vec![Event::PointerMoved(at)]);
    let (response, tool_saw) = run(vec![press(at, true)]);
    assert_eq!(response.activated, None, "the active pane is not reported");
    assert!(tool_saw[0], "the active pane's tool gets the press");
}

#[test]
fn linked_panes_follow_the_main_view() {
    let rect = |w| ScreenRect::from_min_size(pos2(0.0, 0.0), vec2(w, 100.0));
    let mut main = CanvasView::new(rect(400.0), Point::new(30.0, 40.0), 2.5);
    main.rotation = 90.0;
    let mut own = CanvasView::new(rect(150.0), Point::new(1.0, 2.0), 1.0);
    own.rotation = 15.0;
    let mut panes = [SplitPane { view: own, linked: true }, SplitPane { view: own, linked: false }];
    SplitViews::follow(&main, &mut panes);
    assert_eq!((panes[0].view.center, panes[0].view.zoom), (main.center, main.zoom));
    assert_eq!((panes[0].view.rect, panes[0].view.rotation), (own.rect, 15.0));
    assert_eq!(panes[1].view, own, "an unlinked pane stays");
}

// An inactive pane is a Tab stop: Enter on it activates it; the active pane is not a stop.
#[test]
fn enter_on_a_focused_inactive_pane_activates_it() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let group = |ui: &egui::Ui, pane: usize| ui.make_persistent_id(egui::IdSalt::new("split")).with(("group", pane));
    let run = |events: Vec<Event>, focus: Option<usize>| {
        let input = RawInput { screen_rect: Some(full()), events, ..RawInput::default() };
        let mut out = SplitResponse::default();
        ctx.run_ui(input, |ui| {
            if let Some(pane) = focus {
                let id = group(ui, pane);
                ui.memory_mut(|m| m.request_focus(id));
            }
            out = SplitViews::new("split", SplitLayout::Two).show(ui, full(), |_, _, _| {});
        })
        .drop_without_applying_deltas();
        out
    };
    let enter = |pressed| Event::Key { key: egui::Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    // Pane 0 (right) is active; pane 1 (left) is inactive.
    run(vec![], Some(1));
    assert_eq!(run(vec![enter(true)], None).activated, Some(1));
    run(vec![enter(false)], None);
    run(vec![], Some(0));
    assert_eq!(run(vec![enter(true)], None).activated, None, "the active pane is no Tab stop");
}

// `.labels(..)` names the panes instead of the installed words.
#[test]
fn labels_name_the_panes() {
    use egui_kittest::{Harness, kittest::Queryable};
    let labels = SplitLabels { view: "Ansicht {n}".into() };
    let harness = Harness::builder().with_size(vec2(400.0, 300.0)).build_ui(move |ui| {
        install(ui.ctx(), &Base::suite_default());
        SplitViews::new("split", SplitLayout::Two).labels(labels.clone()).show(ui, full(), |_, _, _| {});
    });
    assert_eq!(harness.query_all_by_label("Ansicht 1").count(), 1);
    assert_eq!(harness.query_all_by_label("View 1").count(), 0);
}

#[test]
fn the_first_pane_is_drawn_first_and_keeps_its_place() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let run = |first: Option<usize>| {
        let mut calls = Vec::new();
        let input = RawInput { screen_rect: Some(full()), ..RawInput::default() };
        ctx.run_ui(input, |ui| {
            let mut split = SplitViews::new("order", SplitLayout::Two);
            if let Some(f) = first {
                split = split.first(f);
            }
            split.show(ui, full(), |_, pane, rect| calls.push((pane, rect)));
        })
        .drop_without_applying_deltas();
        calls
    };
    let by_default = run(None);
    assert_eq!(by_default.iter().map(|c| c.0).collect::<Vec<_>>(), [1, 0]);
    let right_first = run(Some(0));
    assert_eq!(right_first.iter().map(|c| c.0).collect::<Vec<_>>(), [0, 1]);
    let mut sorted = right_first.clone();
    sorted.reverse();
    assert_eq!(sorted, by_default, "the rects do not move");
}
