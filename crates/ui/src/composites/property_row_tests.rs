use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::property_row::show_inspector;
use super::*;
use crate::controls::{IconButton, NumberField};
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::LocaleNumbers;
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

#[allow(clippy::expect_used)]
fn tokens_style() -> PropertyStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(PropertyRow::style(&Tokens::of(ui), ControlSize::Md))).drop_without_applying_deltas();
    style.expect("style")
}

/// A panel `width` wide at the top left.
fn panel(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        add(ui);
    });
}

fn number<'a>(id: &'static str, label: &'static str, value: f64, numbers: &'a LocaleNumbers) -> NumberField<'a> {
    NumberField::new(id, label, value, numbers)
}

fn spin<'a, State>(harness: &'a Harness<'_, State>, label: &'a str) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some(label))
}

fn text<'a, State>(harness: &'a Harness<'_, State>, value: &'a str) -> Option<Node<'a>> {
    harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value))
}

#[track_caller]
#[allow(clippy::panic)]
fn rect<State>(harness: &Harness<'_, State>, value: &str) -> Rect {
    text(harness, value).unwrap_or_else(|| panic!("no label {value}")).rect()
}

fn glyphs(ui: &mut Ui) {
    let numbers = LocaleNumbers::from_tag("en");
    panel(ui, 260.0, |ui| {
        PropertyGroup::new("g").show(ui, |ui| {
            PropertyRow::glyph("W").show(ui, |ui, _| number("a", "a", 1.0, &numbers).show(ui));
            PropertyRow::glyph("i").show(ui, |ui, _| number("b", "b", 2.0, &numbers).show(ui));
        });
    });
}

#[test]
fn glyph_labels_share_a_centre_with_the_widest_against_the_control() {
    let harness = harness(Direction::Ltr).build_ui(glyphs);
    let (wide, narrow) = (rect(&harness, "W"), rect(&harness, "i"));
    let (a, b) = (spin(&harness, "W").rect(), spin(&harness, "i").rect());
    let style = tokens_style();
    assert!(wide.width() > narrow.width());
    assert!((wide.center().x - narrow.center().x).abs() < 0.6, "{wide:?} {narrow:?}");
    assert!((a.left() - b.left()).abs() < 0.5, "controls start together: {a:?} {b:?}");
    assert!(a.left() - wide.right() - style.label_gap < 1.5, "the widest is flush with the control: {wide:?} {a:?}");
    assert!(wide.left() >= 0.0 && narrow.left() > wide.left());
}

#[test]
fn right_to_left_mirrors_the_label_column() {
    let harness = harness(Direction::Rtl).build_ui(glyphs);
    let (wide, narrow) = (rect(&harness, "W"), rect(&harness, "i"));
    let a = spin(&harness, "W").rect();
    assert!((wide.center().x - narrow.center().x).abs() < 0.6);
    assert!(a.right() < wide.left() && a.left() < 260.0 - 100.0, "{wide:?} {a:?}");
    assert!((a.right() - spin(&harness, "i").rect().right()).abs() < 0.5);
    assert!(wide.right() > 200.0, "the labels are at the right: {wide:?}");
}

#[test]
fn the_control_fills_the_row_and_keeps_its_minimum() {
    let style = tokens_style();
    for width in [260.0, 30.0] {
        let row = std::cell::Cell::new(Rect::NOTHING);
        let harness = harness(Direction::Ltr).build_ui(|ui| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, width, |ui| {
                let shown = PropertyRow::word("Opacity").redundant().show(ui, |ui, _| number("o", "Opacity", 50.0, &numbers).show(ui));
                row.set(shown.response.rect);
            });
        });
        let field = spin(&harness, "Opacity").rect();
        assert!(field.width() >= style.min_control - 0.5, "{field:?}");
        if width > 200.0 {
            assert!((field.right() - row.get().right()).abs() < 0.5, "fills the row: {field:?} {:?}", row.get());
            let label = rect(&harness, "Opacity");
            assert!((field.left() - row.get().left() - style.word_width - style.label_gap).abs() < 0.5, "after the word column: {label:?} {field:?}");
        } else {
            assert!((field.left() - row.get().left()).abs() < 0.5, "no label: {field:?}");
        }
    }
}

#[test]
fn a_pair_splits_the_width_equally_around_the_link_slot() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        let mut linked = true;
        panel(ui, 300.0, |ui| {
            let _ = PropertyRow::pair(PropertyLabel::glyph("W"), PropertyLabel::glyph("H")).show_linked(
                ui,
                |ui, _| number("w", "Width", 1.0, &numbers).show(ui),
                |ui| ui.add(IconButton::new(Icon::Lock, "Keep proportions").size(ControlSize::Sm).toggle(&mut linked)),
                |ui, _| number("h", "Height", 2.0, &numbers).show(ui),
            );
        });
    });
    let style = tokens_style();
    let (w, h) = (spin(&harness, "W").rect(), spin(&harness, "H").rect());
    let link = harness.get_by_label("Keep proportions").rect();
    assert!((w.width() - h.width()).abs() < 0.5, "{w:?} {h:?}");
    assert!((link.width() - style.link_width).abs() < 0.5, "{link:?}");
    assert!((link.left() - w.right() - style.column_gap).abs() < 0.5, "{w:?} {link:?}");
    let label_h = rect(&harness, "H");
    assert!(
        label_h.left() >= link.right() + style.column_gap && (h.left() - label_h.right() - style.label_gap).abs() < 1.5,
        "{link:?} {label_h:?} {h:?}"
    );
    assert!((h.right() - w.left() - 300.0).abs() < 30.0);
}

#[test]
fn an_essential_label_truncates_and_keeps_its_full_name() {
    let style = tokens_style();
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 100.0, |ui| {
            PropertyRow::word("Opacity of the layer").show(ui, |ui, _| number("o", "Opacity", 50.0, &numbers).show(ui));
        });
    });
    let field = harness.get_by(|n| n.role() == Role::SpinButton).rect();
    assert!(field.width() >= style.min_control - 0.5, "{field:?}");
    let label = rect(&harness, "Opacity of the layer");
    assert!(label.width() < 80.0, "{label:?}");
    let node = harness.get_by(|n| n.role() == Role::SpinButton);
    assert_eq!(node.accesskit_node().label().as_deref(), Some("Opacity of the layer"));
    a11y::assert_labelled(&harness);
}

#[test]
fn a_redundant_label_hides_when_narrow_and_the_control_keeps_its_name() {
    let row = |width: f32| {
        harness(Direction::Ltr).build_ui(move |ui| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, width, |ui| {
                PropertyRow::word("Opacity").redundant().show(ui, |ui, _| number("o", "Opacity", 50.0, &numbers).show(ui));
            });
        })
    };
    let wide = row(260.0);
    assert!(text(&wide, "Opacity").is_some());
    let narrow = row(90.0);
    assert!(text(&narrow, "Opacity").is_none(), "hidden");
    let field = spin(&narrow, "Opacity");
    assert!(field.accesskit_node().data().labelled_by().is_empty());
    assert!(field.rect().width() >= tokens_style().min_control, "{:?}", field.rect());
    a11y::assert_labelled(&narrow);
}

#[test]
fn the_control_is_labelled_by_the_visible_label() {
    let harness = harness(Direction::Ltr).build_ui(glyphs);
    let label = text(&harness, "W").expect("label").accesskit_node().locate().0;
    let node = spin(&harness, "W");
    assert_eq!(node.accesskit_node().data().labelled_by(), [label]);
    a11y::assert_labelled(&harness);
}

#[test]
fn a_scrubbing_label_changes_the_value() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, value: &mut f64| {
            let numbers = LocaleNumbers::from_tag("en");
            panel(ui, 260.0, |ui| {
                let edit = PropertyRow::word("Opacity")
                    .scrubs()
                    .show(ui, |ui, row| {
                        let field = number("o", "Opacity", *value, &numbers);
                        match row.scrub() {
                            Some(label) => field.scrub_by(label),
                            None => field,
                        }
                        .show(ui)
                    })
                    .inner;
                if let Some(EditEvent::Update { value: v, .. } | EditEvent::Commit { value: v, .. } | EditEvent::Set { value: v, .. }) = edit.event {
                    *value = v;
                }
            });
        },
        50.0,
    );
    let at = rect(&harness, "Opacity").center();
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.event(button(at, true));
    harness.run();
    for dx in [10.0, 30.0] {
        harness.event(Event::PointerMoved(at + vec2(dx, 0.0)));
        harness.run();
    }
    harness.event(button(at + vec2(30.0, 0.0), false));
    harness.run();
    assert_ne!(*harness.state(), 50.0, "dragging the label scrubbed the field");
}

#[test]
fn the_inspector_story_names_every_control_at_every_width() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for (width, mixed) in [(60.0, false), (150.0, true), (260.0, false)] {
            let mut args = Args::new().with_bool("mixed", mixed).with_number("width", width, 60.0, 400.0);
            let harness = harness(dir).build_ui(move |ui| show_inspector(ui, &mut args));
            a11y::assert_labelled(&harness);
        }
    }
}

#[test]
fn the_first_frame_already_shows_the_shared_centre() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let frame = |ctx: &Context| {
        let output = ctx.run_ui(egui::RawInput::default(), glyphs);
        let passes = output.platform_output.num_completed_passes;
        output.drop_without_applying_deltas();
        passes
    };
    assert_eq!(frame(&ctx), 2, "the pass that measured the labels is discarded");
    assert_eq!(frame(&ctx), 1, "an unchanged column lays out once");
}

#[test]
fn a_group_overrides_the_column_widths() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        panel(ui, 260.0, |ui| {
            PropertyGroup::new("g").word_width(120.0).show(ui, |ui| {
                PropertyRow::word("Angle").show(ui, |ui, _| number("a", "Angle", 1.0, &numbers).show(ui));
            });
        });
    });
    let style = tokens_style();
    let (label, field) = (rect(&harness, "Angle"), spin(&harness, "Angle").rect());
    assert!((field.left() - label.left() - 120.0 - style.label_gap).abs() < 1.5, "{label:?} {field:?}");
}
