//! Section header tests: toggling, focus, AccessKit, the controls slot, truncation and mirroring.

use egui::accesskit::Role;
use egui::{Context, Key, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::section_header::{show_always_open, show_collapsible};
use super::*;
use crate::controls::IconButton;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};
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

fn header<'a, State>(harness: &'a Harness<'_, State>, title: &'a str) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::DisclosureTriangle && n.label().as_deref() == Some(title))
}

/// The title's text node.
fn title<'a, State>(harness: &'a Harness<'_, State>, text: &'a str) -> Option<Node<'a>> {
    harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(text))
}

/// The buttons' names in AccessKit tree order.
fn buttons_in_tree<State>(harness: &Harness<'_, State>) -> Vec<String> {
    harness.root().children_recursive().map(|n| n.accesskit_node()).filter(|n| n.role() == Role::Button).filter_map(|n| n.label()).collect()
}

fn body_shown<State>(harness: &Harness<'_, State>) -> bool {
    harness.query_by_label("Body").is_some()
}

/// The tokens of the suite's default theme.
#[allow(clippy::expect_used)]
fn tokens() -> std::sync::Arc<crate::theme::UiTokens> {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut tokens = None;
    ctx.run_ui(egui::RawInput::default(), |ui| tokens = Some(Tokens::of(ui))).drop_without_applying_deltas();
    tokens.expect("tokens")
}

fn section() -> impl FnMut(&mut Ui) {
    |ui| {
        panel(ui, 240.0, |ui| {
            let _ = SectionHeader::new("transform", "Transform").show(ui, |ui| ui.label("Body"));
        })
    }
}

#[test]
fn a_click_opens_and_closes_the_section() {
    let mut harness = harness(Direction::Ltr).build_ui(section());
    assert!(body_shown(&harness), "open at first");
    header(&harness, "Transform").click();
    harness.run();
    assert!(!body_shown(&harness));
    header(&harness, "Transform").click();
    harness.run();
    assert!(body_shown(&harness));
}

#[test]
fn enter_and_space_toggle_the_focused_header() {
    let mut harness = harness(Direction::Ltr).build_ui(section());
    harness.key_press(Key::Tab);
    harness.run();
    assert!(header(&harness, "Transform").is_focused());
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!body_shown(&harness), "Enter closes");
    harness.key_press(Key::Space);
    harness.run();
    assert!(body_shown(&harness), "Space opens");
}

#[test]
fn a_focused_header_draws_the_focus_ring() {
    let mut harness = harness(Direction::Ltr).build_ui(section());
    harness.key_press(Key::Tab);
    harness.run();
    a11y::assert_focus_ring(&harness, tokens().color.line.focus.to_egui());
}

#[test]
fn the_header_is_a_disclosure_triangle_that_reports_expanded() {
    let mut harness = harness(Direction::Ltr).build_ui(section());
    assert_eq!(header(&harness, "Transform").accesskit_node().data().is_expanded(), Some(true));
    header(&harness, "Transform").click();
    harness.run();
    assert_eq!(header(&harness, "Transform").accesskit_node().data().is_expanded(), Some(false));
    a11y::assert_labelled(&harness);
}

#[test]
fn the_open_state_is_kept_per_header() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 240.0, |ui| {
            let _ = SectionHeader::new("a", "Same").show(ui, |ui| ui.label("Body A"));
            let _ = SectionHeader::new("b", "Same").show(ui, |ui| ui.label("Body B"));
        })
    });
    harness.get_all_by_label("Same").next().expect("first header").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Body A").is_none() && harness.query_by_label("Body B").is_some());
}

#[test]
fn sections_of_one_id_in_two_scopes_keep_separate_states() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 240.0, |ui| {
            for (scope, body) in [("left", "Body A"), ("right", "Body B")] {
                ui.push_id(scope, |ui| {
                    let _ = SectionHeader::new("transform", "Transform").show(ui, |ui| ui.label(body));
                });
            }
        })
    });
    harness.get_all_by_label("Transform").next().expect("first header").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Body A").is_none() && harness.query_by_label("Body B").is_some());
}

#[test]
fn the_open_state_does_not_depend_on_the_title() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, title: &mut &'static str| {
            panel(ui, 240.0, |ui| {
                let _ = SectionHeader::new("transform", *title).show(ui, |ui| ui.label("Body"));
            })
        },
        "Transform",
    );
    header(&harness, "Transform").click();
    harness.run();
    assert!(!body_shown(&harness));
    *harness.state_mut() = "Transformer";
    harness.run();
    assert!(!body_shown(&harness), "a new title (a language switch) keeps the state");
}

#[test]
fn a_host_owned_state_is_read_and_written() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, open: &mut bool| {
            panel(ui, 240.0, |ui| {
                let _ = SectionHeader::new("transform", "Transform").open(open).show(ui, |ui| ui.label("Body"));
            })
        },
        false,
    );
    assert!(!body_shown(&harness), "the host's state wins over the default");
    header(&harness, "Transform").click();
    harness.run();
    assert!(*harness.state(), "a click writes the host's state");
    assert!(body_shown(&harness));
    *harness.state_mut() = false;
    harness.run();
    assert!(!body_shown(&harness), "the host closes it");
}

#[test]
fn an_always_open_header_is_not_focusable_and_always_shows_its_body() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        panel(ui, 240.0, |ui| {
            let _ = SectionHeader::new("transform", "Transform")
                .collapsible(false)
                .controls(|ui| {
                    ui.add(IconButton::new(Icon::Plus, "Add transform").size(ControlSize::Sm));
                })
                .show(ui, |ui| ui.label("Body"));
        })
    });
    assert!(body_shown(&harness));
    assert!(harness.query_by(|n| n.role() == Role::DisclosureTriangle).is_none());
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
    assert_eq!(stops, ["Add transform"], "only the control is a Tab stop");
    harness.get_by_label("Transform").click();
    harness.run();
    assert!(body_shown(&harness));
}

#[test]
fn the_controls_sit_at_the_trailing_end_and_the_title_gives_way() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui(|ui| {
            panel(ui, 140.0, |ui| {
                let _ = SectionHeader::new("long", "A rather long section title")
                    .collapsible(false)
                    .controls(|ui| {
                        ui.add(IconButton::new(Icon::Plus, "Add").size(ControlSize::Sm));
                    })
                    .show(ui, |_| ());
            })
        });
        let button = harness.get_by_label("Add").rect();
        let title = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("A rather long section title")).rect();
        if dir == Direction::Ltr {
            assert!(button.right() > 130.0 && title.right() <= button.left() + 0.5, "{title:?} {button:?}");
        } else {
            assert!(button.left() < 10.0 && title.left() >= button.right() - 0.5, "{title:?} {button:?}");
        }
    }
}

/// A collapsible header with three controls in a panel `width` wide.
fn with_controls(width: f32) -> impl FnMut(&mut Ui) {
    move |ui| {
        panel(ui, width, |ui| {
            let _ = SectionHeader::new("transform", "Transform")
                .controls(|ui| {
                    for (icon, label) in [(Icon::Settings, "Settings"), (Icon::Copy, "Copy"), (Icon::Plus, "Add")] {
                        ui.add(IconButton::new(icon, label).size(ControlSize::Sm));
                    }
                })
                .show(ui, |ui| ui.label("Body"));
        })
    }
}

#[test]
fn the_controls_are_in_reading_order_for_tab_accesskit_and_layout() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut harness = harness(dir).build_ui(with_controls(240.0));
        let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
        assert_eq!(stops, ["Transform", "Settings", "Copy", "Add"], "{dir:?}");
        assert_eq!(buttons_in_tree(&harness), ["Settings", "Copy", "Add"], "{dir:?}");
        let xs: Vec<f32> = ["Settings", "Copy", "Add"].map(|label| harness.get_by_label(label).rect().center().x).to_vec();
        let row = header(&harness, "Transform").rect();
        let add = harness.get_by_label("Add").rect();
        if dir == Direction::Ltr {
            assert!(xs[0] < xs[1] && xs[1] < xs[2], "{xs:?}");
            assert!((add.right() - row.right()).abs() < 1.0, "the last control ends at the trailing edge: {add:?} {row:?}");
        } else {
            assert!(xs[0] > xs[1] && xs[1] > xs[2], "{xs:?}");
            assert!((add.left() - row.left()).abs() < 1.0, "{add:?} {row:?}");
        }
    }
}

#[test]
fn a_header_too_narrow_for_its_title_draws_none_and_keeps_its_name() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui(with_controls(70.0));
        assert!(title(&harness, "Transform").is_none(), "{dir:?}: the controls reach the title's start");
        assert_eq!(header(&harness, "Transform").accesskit_node().label().as_deref(), Some("Transform"));
    }
}

#[test]
fn the_title_starts_after_the_chevron_and_mirrors() {
    let offset = tokens().component.section.title_offset;
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui(section());
        let row = header(&harness, "Transform").rect();
        let title = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Transform")).rect();
        let gap = if dir == Direction::Ltr { title.left() - row.left() } else { row.right() - title.right() };
        assert!((gap - offset).abs() < 1.5, "{dir:?}: {title:?} in {row:?}");
        assert_eq!(row.width().round(), 240.0, "the whole row is the click target");
    }
}

#[test]
fn the_stories_name_every_control_in_both_directions() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for width in [60.0, 150.0, 260.0] {
            let args = Args::new().with_number("width", width, 60.0, 400.0);
            let mut a = args.clone();
            let harness = harness(dir).build_ui(move |ui| show_collapsible(ui, &mut a));
            a11y::assert_labelled(&harness);
            let mut a = args.clone();
            let harness = self::harness(dir).build_ui(move |ui| show_always_open(ui, &mut a));
            a11y::assert_labelled(&harness);
        }
    }
}
