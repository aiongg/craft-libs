//! Section and panel header tests: toggling, focus, AccessKit, truncation and mirroring.

use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::panel_header::show_flyout;
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
            let _ = SectionHeader::new("Transform").show(ui, |ui| ui.label("Body"));
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
            let _ = SectionHeader::new("Same").id_salt("a").show(ui, |ui| ui.label("Body A"));
            let _ = SectionHeader::new("Same").id_salt("b").show(ui, |ui| ui.label("Body B"));
        })
    });
    harness.get_all_by_label("Same").next().expect("first header").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Body A").is_none() && harness.query_by_label("Body B").is_some());
}

#[test]
fn a_host_owned_state_is_read_and_written() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, open: &mut bool| {
            panel(ui, 240.0, |ui| {
                let _ = SectionHeader::new("Transform").open(open).show(ui, |ui| ui.label("Body"));
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
            let _ = SectionHeader::new("Transform")
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
                let _ = SectionHeader::new("A rather long section title")
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

fn flyout(width: f32) -> impl FnMut(&mut Ui, &mut Vec<&'static str>) {
    move |ui, clicks| {
        panel(ui, width, |ui| {
            let shown = PanelHeader::new("Brush settings").float(true).close(true).show(ui, |_| {});
            for (name, response) in [("menu", shown.menu), ("float", shown.float), ("close", shown.close)] {
                if response.is_some_and(|r| r.clicked()) {
                    clicks.push(name);
                }
            }
        })
    }
}

#[test]
fn the_panel_header_buttons_are_named_and_clickable() {
    let mut harness = harness(Direction::Ltr).build_ui_state(flyout(280.0), Vec::new());
    for (label, name) in [("Panel menu", "menu"), ("Float panel", "float"), ("Close panel", "close")] {
        let node = harness.get_by_label(label);
        assert_eq!(node.accesskit_node().role(), Role::Button);
        node.click();
        harness.run();
        assert_eq!(harness.state().last(), Some(&name));
    }
    a11y::assert_labelled(&harness);
    let title = harness.get_by(|n| n.role() == Role::Header);
    assert_eq!(title.accesskit_node().label().as_deref(), Some("Brush settings"));
}

#[test]
fn the_panel_header_has_the_size_token_height_and_the_buttons_trail() {
    let height = PanelHeader::style(&tokens()).height;
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui_state(flyout(280.0), Vec::new());
        let header = harness.get_by(|n| n.role() == Role::Header).rect();
        assert!((header.height() - height).abs() < 0.5, "{header:?}");
        let (menu, float, close) =
            (harness.get_by_label("Panel menu").rect(), harness.get_by_label("Float panel").rect(), harness.get_by_label("Close panel").rect());
        let title = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Brush settings")).rect();
        if dir == Direction::Ltr {
            assert!(
                title.left() < menu.left() && menu.right() <= float.left() && float.right() <= close.left(),
                "{title:?} {menu:?} {float:?} {close:?}"
            );
            assert!(close.right() < header.right() + 0.5);
        } else {
            assert!(
                title.right() > menu.right() && menu.left() >= float.right() && float.left() >= close.right(),
                "{title:?} {menu:?} {float:?} {close:?}"
            );
            assert!(close.left() > header.left() - 0.5);
        }
    }
}

#[test]
fn a_long_panel_title_is_cut_but_keeps_its_full_name() {
    let harness = harness(Direction::Ltr).build_ui_state(
        |ui, _: &mut ()| {
            panel(ui, 140.0, |ui| {
                let _ = PanelHeader::new("A rather long panel title indeed").float(true).close(true).show(ui, |_| {});
            })
        },
        (),
    );
    let title = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("A rather long panel title indeed")).rect();
    let float = harness.get_by_label("Panel menu").rect();
    assert!(title.right() <= float.left() + 0.5, "{title:?} {float:?}");
    assert_eq!(harness.get_by(|n| n.role() == Role::Header).accesskit_node().label().as_deref(), Some("A rather long panel title indeed"));
}

#[test]
fn dragging_the_header_background_reports_a_drag() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, dragged: &mut bool| {
            panel(ui, 280.0, |ui| {
                *dragged |= PanelHeader::new("Brush").show(ui, |_| {}).header.dragged();
            })
        },
        false,
    );
    let at = Pos2::new(130.0, 10.0);
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.event(button(at, true));
    harness.run();
    for dx in [10.0, 30.0] {
        harness.event(Event::PointerMoved(at + vec2(dx, 0.0)));
        harness.run();
    }
    assert!(*harness.state());
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
            let mut a = Args::new()
                .with_bool("float", true)
                .with_bool("close", true)
                .with_bool("menu", true)
                .with_bool("actions", true)
                .with_number("width", width, 60.0, 400.0);
            let harness = self::harness(dir).build_ui(move |ui| show_flyout(ui, &mut a));
            a11y::assert_labelled(&harness);
        }
    }
}
