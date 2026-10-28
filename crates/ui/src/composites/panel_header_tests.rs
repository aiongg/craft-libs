//! Panel header tests: buttons, Tab and AccessKit order, dragging, truncation and mirroring.

use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::panel_header::show_flyout;
use super::*;
use crate::controls::IconButton;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};
use craft_theme::ControlSize;

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

/// A panel `width` wide at the top left.
fn panel(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        add(ui);
    });
}

#[allow(clippy::expect_used)]
fn style() -> PanelHeaderStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(PanelHeader::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

fn title<'a, State>(harness: &'a Harness<'_, State>, text: &'a str) -> Option<Node<'a>> {
    harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(text))
}

fn header_node<'a, State>(harness: &'a Harness<'_, State>) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::Header)
}

/// The buttons' names in AccessKit tree order.
fn buttons_in_tree<State>(harness: &Harness<'_, State>) -> Vec<String> {
    harness.root().children_recursive().map(|n| n.accesskit_node()).filter(|n| n.role() == Role::Button).filter_map(|n| n.label()).collect()
}

/// What the host saw: whether the header was dragged, and the buttons clicked.
#[derive(Default)]
struct Seen {
    dragged: bool,
    clicks: Vec<&'static str>,
}

/// A header with the float and close buttons, and an "Add preset" action when `actions`, in a
/// panel `width` wide.
fn flyout(width: f32, actions: bool) -> impl FnMut(&mut Ui, &mut Seen) {
    move |ui, seen| {
        panel(ui, width, |ui| {
            let shown = PanelHeader::new("brush", "Brush settings").float(true).close(true).show(ui, |ui| {
                if actions && ui.add(IconButton::new(Icon::Plus, "Add preset").size(ControlSize::Sm)).clicked() {
                    seen.clicks.push("action");
                }
            });
            seen.dragged |= shown.header.dragged();
            for (name, response) in [("menu", shown.menu), ("float", shown.float), ("close", shown.close)] {
                if response.is_some_and(|r| r.clicked()) {
                    seen.clicks.push(name);
                }
            }
        })
    }
}

/// Presses at `from`, moves by `dx` in steps past the drag threshold, and releases.
fn drag(harness: &mut Harness<'_, Seen>, from: Pos2, dx: f32) {
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(from));
    harness.run();
    harness.event(button(from, true));
    harness.run();
    for step in [dx / 3.0, dx] {
        harness.event(Event::PointerMoved(from + vec2(step, 0.0)));
        harness.run();
    }
    harness.event(button(from + vec2(dx, 0.0), false));
    harness.run();
}

#[test]
fn the_buttons_are_named_and_clickable() {
    let mut harness = harness(Direction::Ltr).build_ui_state(flyout(280.0, false), Seen::default());
    for (label, name) in [("Panel menu", "menu"), ("Float panel", "float"), ("Close panel", "close")] {
        let node = harness.get_by_label(label);
        assert_eq!(node.accesskit_node().role(), Role::Button);
        node.click();
        harness.run();
        assert_eq!(harness.state().clicks.last(), Some(&name));
    }
    a11y::assert_labelled(&harness);
    assert_eq!(header_node(&harness).accesskit_node().label().as_deref(), Some("Brush settings"));
}

#[test]
fn tab_accesskit_and_layout_follow_reading_order() {
    let style = style();
    for dir in [Direction::Ltr, Direction::Rtl] {
        for actions in [false, true] {
            let mut harness = harness(dir).build_ui_state(flyout(280.0, actions), Seen::default());
            let order: Vec<&str> = if actions {
                vec!["Add preset", "Panel menu", "Float panel", "Close panel"]
            } else {
                vec!["Panel menu", "Float panel", "Close panel"]
            };
            let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
            assert_eq!(stops, order, "{dir:?}: the background is not a Tab stop");
            assert_eq!(buttons_in_tree(&harness), order, "{dir:?}");

            let header = header_node(&harness).rect();
            assert!((header.height() - style.height).abs() < 0.5, "{header:?}");
            let rects: Vec<Rect> = order.iter().map(|label| harness.get_by_label(label).rect()).collect();
            let text = title(&harness, "Brush settings").expect("title").rect();
            let close = rects[rects.len() - 1];
            for pair in rects.windows(2) {
                let gap = if dir == Direction::Ltr { pair[1].left() - pair[0].right() } else { pair[0].left() - pair[1].right() };
                assert!((gap - style.gap).abs() < 0.5, "{dir:?}: {pair:?}");
            }
            for (label, rect) in order.iter().zip(&rects).skip(usize::from(actions)) {
                assert!((rect.width() - style.button).abs() < 0.5 && (rect.height() - style.button).abs() < 0.5, "{label}: {rect:?}");
            }
            if dir == Direction::Ltr {
                assert!(text.right() <= rects[0].left(), "{text:?} {rects:?}");
                assert!((header.right() - style.pad - close.right()).abs() < 0.5, "the close button ends the row: {close:?} {header:?}");
            } else {
                assert!(text.left() >= rects[0].right(), "{text:?} {rects:?}");
                assert!((close.left() - header.left() - style.pad).abs() < 0.5, "{close:?} {header:?}");
            }
        }
    }
}

#[test]
fn the_first_frame_already_places_the_actions() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let pad = style().pad;
    let frame = |ctx: &Context| {
        let mut close = Rect::NOTHING;
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            panel(ui, 280.0, |ui| {
                let shown = PanelHeader::new("brush", "Brush").close(true).show(ui, |ui| {
                    ui.add(IconButton::new(Icon::Plus, "Add preset").size(ControlSize::Sm));
                });
                close = shown.close.map_or(Rect::NOTHING, |r| r.rect);
            });
        });
        let passes = output.platform_output.num_completed_passes;
        output.drop_without_applying_deltas();
        (passes, close)
    };
    let (passes, close) = frame(&ctx);
    assert_eq!(passes, 2, "the pass that measured the actions is discarded");
    assert!((close.right() - (280.0 - pad)).abs() < 0.5, "{close:?}");
    assert_eq!(frame(&ctx).0, 1, "unchanged actions lay out once");
}

#[test]
fn a_label_set_before_showing_a_button_is_kept() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let _ = PanelHeader::new("brush", "Brush")
            .float_label("Detach")
            .float(true)
            .close_label("Dismiss")
            .close(false)
            .close(true)
            .menu_label("Options")
            .menu(true)
            .show(ui, |_| {});
    });
    assert_eq!(buttons_in_tree(&harness), ["Options", "Detach", "Dismiss"]);
}

#[test]
fn a_long_title_is_cut_but_keeps_its_full_name() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 140.0, |ui| {
            let _ = PanelHeader::new("long", "A rather long panel title indeed").float(true).close(true).show(ui, |_| {});
        })
    });
    let text = title(&harness, "A rather long panel title indeed").expect("title").rect();
    let menu = harness.get_by_label("Panel menu").rect();
    assert!(text.right() <= menu.left() + 0.5, "{text:?} {menu:?}");
    assert_eq!(header_node(&harness).accesskit_node().label().as_deref(), Some("A rather long panel title indeed"));
}

#[test]
fn a_header_too_narrow_for_its_title_draws_none() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui_state(flyout(70.0, true), Seen::default());
        assert!(title(&harness, "Brush settings").is_none(), "{dir:?}: the buttons reach the title's start");
        assert_eq!(header_node(&harness).accesskit_node().label().as_deref(), Some("Brush settings"));
    }
}

#[test]
fn dragging_the_background_reports_a_drag() {
    let mut harness = harness(Direction::Ltr).build_ui_state(flyout(280.0, false), Seen::default());
    drag(&mut harness, Pos2::new(60.0, 10.0), 30.0);
    assert!(harness.state().dragged);
}

#[test]
fn a_press_on_a_button_or_an_action_never_drags_the_panel() {
    for label in ["Close panel", "Panel menu", "Add preset"] {
        let mut harness = harness(Direction::Ltr).build_ui_state(flyout(280.0, true), Seen::default());
        let at = harness.get_by_label(label).rect().center();
        drag(&mut harness, at, -40.0);
        assert!(!harness.state().dragged, "{label}: the press belongs to the button");
        assert!(harness.state().clicks.is_empty(), "{label}: released away from the button, no click");
    }
}

#[test]
fn the_flyout_story_names_every_control_in_both_directions() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for width in [60.0, 150.0, 260.0] {
            let mut a = Args::new()
                .with_bool("float", true)
                .with_bool("close", true)
                .with_bool("menu", true)
                .with_bool("actions", true)
                .with_number("width", width, 60.0, 400.0);
            let harness = harness(dir).build_ui(move |ui| show_flyout(ui, &mut a));
            a11y::assert_labelled(&harness);
        }
    }
}
