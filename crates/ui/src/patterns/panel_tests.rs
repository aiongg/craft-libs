//! Embedded panel tests: the body and footer filling the rect, the footer's height, padding,
//! scrolling, button order and AccessKit.

use std::cell::Cell;
use std::rc::Rc;

use egui::accesskit::{Orientation, Role};
use egui::{Context, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::panel::stories;
use super::*;
use crate::composites::PanelFooter;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

const SIZE: [f32; 2] = [300.0, 200.0];

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size(SIZE).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

/// The rects a panel run reports.
struct Seen {
    panel: Cell<Rect>,
    first: Cell<Rect>,
    last: Cell<Rect>,
    footer: Cell<Option<Rect>>,
    buttons: Cell<[Rect; 2]>,
}

fn run(dir: Direction, rows: usize, scroll: bool, footer: bool) -> (Harness<'static, ()>, Rc<Seen>) {
    let nothing = Cell::new(Rect::NOTHING);
    let seen = Rc::new(Seen {
        panel: nothing.clone(),
        first: nothing.clone(),
        last: nothing,
        footer: Cell::new(None),
        buttons: Cell::new([Rect::NOTHING; 2]),
    });
    let out = seen.clone();
    let mut harness = harness(dir).build_ui(move |ui| {
        let mut buttons = [Rect::NOTHING; 2];
        let panel = Panel::new("test").title("Layers").scroll(scroll);
        let panel = if footer {
            panel.footer_label("Layer actions").footer(|ui| {
                buttons[0] = PanelFooter::button(ui, Icon::Trash, "Delete layer").rect;
                buttons[1] = PanelFooter::button(ui, Icon::Plus, "New layer").rect;
            })
        } else {
            panel
        };
        let panel_rect = ui.available_rect_before_wrap();
        let output = panel.show(ui, |ui| {
            for row in 0..rows {
                let rect = ui.label(format!("Row {row}")).rect;
                if row == 0 {
                    out.first.set(rect);
                }
                out.last.set(rect);
            }
        });
        out.panel.set(panel_rect);
        out.footer.set(output.footer);
        out.buttons.set(buttons);
    });
    harness.run();
    (harness, seen)
}

fn footer_height() -> f32 {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut height = 0.0;
    ctx.run_ui(egui::RawInput::default(), |ui| height = Panel::footer_height(&Tokens::of(ui), ui.pixels_per_point())).drop_without_applying_deltas();
    height
}

fn pad() -> f32 {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut pad = 0.0;
    ctx.run_ui(egui::RawInput::default(), |ui| pad = Panel::style(&Tokens::of(ui), 1.0).pad).drop_without_applying_deltas();
    pad
}

#[test]
fn the_footer_sits_at_the_bottom_edge_to_edge_and_is_footer_height_high() {
    let (_h, seen) = run(Direction::Ltr, 2, true, true);
    let footer = seen.footer.get().expect("footer");
    let panel = seen.panel.get();
    assert_eq!(footer.bottom(), panel.bottom());
    assert_eq!((footer.left(), footer.right()), (panel.left(), panel.right()));
    assert!((footer.height() - footer_height()).abs() < 0.01, "{} vs {}", footer.height(), footer_height());
}

#[test]
fn the_body_has_padding_on_every_side_and_the_footer_takes_nothing_from_the_host() {
    let (_h, seen) = run(Direction::Ltr, 2, true, true);
    let panel = seen.panel.get();
    let first = seen.first.get();
    assert!((first.left() - (panel.left() + pad())).abs() < 0.5);
    assert!((first.top() - (panel.top() + pad())).abs() < 0.5);
    let (_h2, plain) = run(Direction::Ltr, 2, true, false);
    assert_eq!(plain.footer.get(), None);
}

#[test]
fn a_tall_body_scrolls_above_the_footer() {
    let (mut h, seen) = run(Direction::Ltr, 60, true, true);
    let footer = seen.footer.get().expect("footer");
    h.event(egui::Event::PointerMoved(egui::pos2(100.0, 60.0)));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, -500.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    assert!(seen.first.get().top() < seen.panel.get().top(), "the body scrolled");
    assert_eq!(seen.footer.get(), Some(footer), "the footer stays");
}

#[test]
fn footer_buttons_start_at_the_reading_end() {
    let (_h, ltr) = run(Direction::Ltr, 1, true, true);
    let [first, second] = ltr.buttons.get();
    assert!(first.left() > second.left());
    let (_h, rtl) = run(Direction::Rtl, 1, true, true);
    let [first, second] = rtl.buttons.get();
    assert!(first.left() < second.left());
}

#[test]
fn the_panel_is_a_named_group_and_the_footer_a_horizontal_toolbar() {
    let (h, _) = run(Direction::Ltr, 1, true, true);
    assert!(h.get_all_by_role(Role::Group).any(|n| n.accesskit_node().label().as_deref() == Some("Layers")));
    let toolbar = h.get_by_role(Role::Toolbar);
    assert_eq!(toolbar.accesskit_node().label().as_deref(), Some("Layer actions"));
    assert_eq!(toolbar.accesskit_node().orientation(), Some(Orientation::Horizontal));
}

#[test]
fn every_story_names_every_control() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for story in stories() {
            let mut args = (story.args)();
            let show = story.show;
            let mut harness = harness(dir).build_ui(move |ui: &mut Ui| show(ui, &mut args));
            harness.run();
            a11y::assert_labelled(&harness);
        }
    }
}
