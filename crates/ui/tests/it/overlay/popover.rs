//! `Popover`: the close reason and the open binding written when a close rule fires.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::popover::Popover;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    open: Rc<Cell<bool>>,
    closed: Rc<Cell<Option<CloseReason>>>,
}

fn rig() -> Rig {
    let (open, closed) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(None)));
    let (shown_open, shown_closed) = (open.clone(), closed.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut is_open = shown_open.get();
        let shown = Popover::new("Brush options", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            Button::new("Reset").show(ui);
        });
        shown_open.set(is_open);
        if shown.output.closed.is_some() {
            shown_closed.set(shown.output.closed);
        }
    });
    fixture.settle();
    Rig { fixture, open, closed }
}

/// Escape with the focus inside closes the popover, clears the caller's binding and reports `Escape` once.
#[trial]
fn escape_closes_a_popover_and_reports_why() {
    let mut rig = rig();
    assert!(rig.open.get(), "the popover starts open");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    rig.fixture.idle(30);
    assert!(!rig.open.get(), "Escape clears the open binding");
    assert_eq!(rig.closed.get(), Some(CloseReason::Escape));
}

/// 6.3: a container layer opens at its final width on its first visible frame and asks for at most one discard
/// pass: the first sizing pass measures the content's ideal width instead of growing from none.
#[trial]
fn a_popover_opens_at_its_final_width_with_one_discard_pass() {
    let open = Rc::new(Cell::new(false));
    let shown_open = open.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut is_open = shown_open.get();
        Popover::new("Brush options", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            Button::new("Reset the brush to its defaults").show(ui);
            Button::new("Apply").show(ui);
        });
        shown_open.set(is_open);
    });
    fixture.settle();
    open.set(true);
    let layer_width = |fixture: &Fixture| {
        fixture.ctx().memory(|memory| {
            memory
                .layer_ids()
                .filter(|layer| layer.order != egui::Order::Background)
                .filter_map(|layer| memory.area_rect(layer.id))
                .map(|rect| rect.width())
                .fold(0.0_f32, f32::max)
        })
    };
    let mut widths = Vec::new();
    let mut passes = Vec::new();
    for _ in 0..4 {
        fixture.frame(Vec::new());
        widths.push(layer_width(&fixture));
        passes.push(fixture.passes());
    }
    assert!(widths[0] > 0.0, "the layer shows on the first frame: {widths:?} {passes:?}");
    assert!(widths.iter().all(|width| (width - widths[0]).abs() < 0.5), "the layer keeps one width from its first frame: {widths:?} {passes:?}");
    assert!(passes[0] <= 2, "the opening frame asks for at most one discard pass: {passes:?}");
}
