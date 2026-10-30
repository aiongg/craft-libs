use craft_theme::ControlSize;
use egui::accesskit::Role;
use egui::{Context, Rect, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::form_grid::{ImageSizeArgs, show_image_size};
use super::*;
use crate::controls::{Checkbox, NumberField, TextField};
use crate::direction::{self, Direction};
use crate::localized::LocaleNumbers;
use crate::primitives::{Gap, Row};
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf, install};

fn harness(dir: Direction) -> HarnessBuilder<Option<FormStyle>> {
    Harness::<Option<FormStyle>>::builder().with_size([640.0, 320.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

fn text_input<'a, State>(harness: &'a Harness<'_, State>, value: &'a str) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some(value))
}

#[track_caller]
fn label<State>(harness: &Harness<'_, State>, value: &str) -> Rect {
    harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value)).rect()
}

/// A short and a long label, a read-only value and a check box without a label.
fn form(grid: FormGrid) -> impl Fn(&mut Ui, &mut Option<FormStyle>) {
    move |ui, style| {
        *style = Some(FormGrid::style(&Tokens::of(ui), false));
        grid.show(ui, |form| {
            let _ = form.row("Name:", |ui, _| TextField::new("name", "Name", "short").show(ui));
            let _ = form.row("Resolution:", |ui, _| TextField::new("res", "Resolution", "long").show(ui));
            let _ = form.value("Size:", "3.7M");
            let mut on = false;
            let _ = form.unlabelled(|ui| Checkbox::bool("Resample", &mut on).show(ui));
        });
    }
}

fn shown(dir: Direction, grid: FormGrid) -> Harness<'static, Option<FormStyle>> {
    harness(dir).build_ui_state(form(grid), None)
}

#[test]
fn labels_end_against_one_control_column_a_column_gap_away() {
    let harness = shown(Direction::Ltr, FormGrid::new("form"));
    let style = harness.state().expect("style");
    let (short, long) = (text_input(&harness, "short").rect(), text_input(&harness, "long").rect());
    let (name, res) = (label(&harness, "Name:"), label(&harness, "Resolution:"));
    assert!((short.left() - long.left()).abs() < 0.5, "{short:?} {long:?}");
    assert!((res.right() + style.column_gap - long.left()).abs() < 1.0, "column gap {}: {res:?} {long:?}", style.column_gap);
    assert!((name.right() - res.right()).abs() < 0.5 && name.width() < res.width(), "{name:?} {res:?}");
    let (value, check) = (label(&harness, "3.7M"), harness.get_by_label("Resample").rect());
    assert!((value.left() - short.left()).abs() < 0.5 && (check.left() - short.left()).abs() < 0.5, "{value:?} {check:?}");
    assert!((long.top() - short.bottom() - style.row_gap).abs() < 0.5, "rows are `row_gap` ({}) apart: {short:?} {long:?}", style.row_gap);
}

#[test]
fn a_label_starts_at_the_reading_start_when_asked() {
    let harness = shown(Direction::Ltr, FormGrid::new("form").label_align(LabelAlign::Start));
    let (name, res) = (label(&harness, "Name:"), label(&harness, "Resolution:"));
    assert!((name.left() - res.left()).abs() < 0.5, "{name:?} {res:?}");
}

#[test]
fn the_roomy_grid_has_the_wider_gaps_and_the_label_column_minimum() {
    let harness = harness(Direction::Ltr).build_ui_state(
        |ui, style| {
            *style = Some(FormGrid::style(&Tokens::of(ui), true));
            FormGrid::new("form").roomy(true).show(ui, |form| {
                let _ = form.row("Name:", |ui, _| TextField::new("name", "Name", "short").show(ui));
            });
        },
        None,
    );
    let style = harness.state().expect("style");
    let (name, field) = (label(&harness, "Name:"), text_input(&harness, "short").rect());
    assert!(style.label_min >= 150.0 && style.column_gap > 12.0 && style.row_gap > 6.0, "{style:?}");
    assert!(
        (field.left() - name.right() - style.column_gap).abs() < 1.0 && field.left() >= style.label_min + style.column_gap - 1.0,
        "{name:?} {field:?}"
    );
}

#[test]
fn the_min_label_width_widens_the_label_column() {
    let narrow = shown(Direction::Ltr, FormGrid::new("form"));
    let wide = shown(Direction::Ltr, FormGrid::new("form").min_label_width(200.0));
    let (a, b) = (text_input(&narrow, "short").rect(), text_input(&wide, "short").rect());
    assert!(b.left() > a.left() + 100.0 && b.left() >= 200.0, "{a:?} {b:?}");
}

#[test]
fn right_to_left_the_label_column_is_at_the_right() {
    let harness = shown(Direction::Rtl, FormGrid::new("form"));
    let (short, long) = (text_input(&harness, "short").rect(), text_input(&harness, "long").rect());
    let (name, res) = (label(&harness, "Name:"), label(&harness, "Resolution:"));
    let check = harness.get_by_label("Resample").rect();
    assert!((short.right() - long.right()).abs() < 0.5 && (check.right() - short.right()).abs() < 0.5, "{short:?} {long:?} {check:?}");
    assert!((name.left() - res.left()).abs() < 0.5 && res.left() > long.right(), "{name:?} {res:?}");
}

#[test]
fn a_label_is_centred_on_its_control() {
    let harness = shown(Direction::Ltr, FormGrid::new("form"));
    let (name, short) = (label(&harness, "Name:"), text_input(&harness, "short").rect());
    assert!((name.center().y - short.center().y).abs() < 0.5, "{name:?} {short:?}");
}

#[test]
fn a_label_is_centred_on_a_small_control_and_on_a_control_pushed_down_by_a_taller_sibling() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui_state(
            |ui, _| {
                FormGrid::new("form").show(ui, |form| {
                    let _ = form.row("Small:", |ui, _| TextField::new("small", "Small", "sm").size(ControlSize::Sm).show(ui));
                    let _ = form.row("Mixed:", |ui, _| {
                        let first = TextField::new("first", "First", "first").size(ControlSize::Sm).show(ui);
                        let _ = TextField::new("second", "Second", "second").show(ui);
                        first
                    });
                });
            },
            None,
        );
        for (name, value) in [("Small:", "sm"), ("Mixed:", "first")] {
            let (label, field) = (label(&harness, name), text_input(&harness, value).rect());
            assert!((label.center().y - field.center().y).abs() < 0.5, "{dir:?} {name} {label:?} {field:?}");
        }
    }
}

#[test]
fn the_label_names_its_control_and_a_value_names_nothing() {
    let harness = shown(Direction::Ltr, FormGrid::new("form"));
    let id = |value: &str| harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value)).accesskit_node().locate().0;
    let node = text_input(&harness, "short");
    let node = node.accesskit_node();
    assert_eq!(node.data().labelled_by(), [id("Name:")]);
    assert_eq!(node.label().as_deref(), Some("Name:"));
    let check = harness.get_by_label("Resample");
    assert!(check.accesskit_node().data().labelled_by().is_empty(), "a row without a label keeps the control's name");
    let value = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("3.7M"));
    assert!(value.accesskit_node().data().labelled_by().is_empty());
    a11y::assert_labelled(&harness);
}

#[test]
fn the_image_size_story_is_labelled_and_mirrors() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = Harness::builder()
            .with_size([640.0, 320.0])
            .with_context_setup(move |ctx: &Context| {
                install(ctx, &Base::suite_default());
                direction::apply(ctx, dir);
            })
            .build_ui_state(|ui, args: &mut ImageSizeArgs| show_image_size(ui, args), ImageSizeArgs { roomy: false, labels_at_start: false });
        a11y::assert_labelled(&harness);
        let width = label(&harness, "Width:");
        let field = harness.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Width:")).rect();
        match dir {
            Direction::Ltr => assert!(field.left() > width.right(), "{width:?} {field:?}"),
            Direction::Rtl => assert!(field.right() < width.left(), "{width:?} {field:?}"),
        }
    }
}

#[test]
fn a_label_scrubs_its_field_in_the_grid() {
    let mut harness =
        Harness::<f64>::builder().with_size([640.0, 120.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, value: &mut f64| {
                let numbers = LocaleNumbers::from_tag("en");
                FormGrid::new("form").show(ui, |form| {
                    let _ = form.row("Radius:", |ui, label| {
                        let field = NumberField::new("radius", "Radius", *value, &numbers).range(0.0..=500.0);
                        let edit = match label.scrub() {
                            Some(label) => field.scrub_by(label),
                            None => field,
                        }
                        .show(ui);
                        if let Some(event) = edit.event.as_ref().and_then(|e| e.value().copied()) {
                            *value = event;
                        }
                        edit
                    });
                });
            },
            4.0,
        );
    let from = label(&harness, "Radius:").center();
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    harness.event(egui::Event::PointerMoved(from));
    harness.event(button(from, true));
    harness.run();
    for dx in [10.0, 20.0] {
        harness.event(egui::Event::PointerMoved(from + egui::vec2(dx, 0.0)));
        harness.run();
    }
    harness.event(button(from + egui::vec2(20.0, 0.0), false));
    harness.run();
    assert!(*harness.state() > 4.0, "dragging the label scrubbed the value: {}", harness.state());
}

#[test]
fn a_cell_of_several_controls_is_a_group_named_by_the_label() {
    let harness =
        Harness::<()>::builder().with_size([640.0, 120.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(|ui| {
            FormGrid::new("form").show(ui, |form| {
                let _ = form.row("Actions:", |ui, _| {
                    Row::new(Gap::Inline)
                        .show(ui, |ui| {
                            let _ = ui.button("One");
                            let _ = ui.button("Two");
                        })
                        .response
                });
                let _ = form.row("Name:", |ui, _| TextField::new("name", "Name", "short").show(ui));
            });
        });
    let id = |value: &str| harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value)).accesskit_node().locate().0;
    let group = harness.get_by_role(Role::Group);
    assert_eq!(group.accesskit_node().data().labelled_by(), [id("Actions:")]);
    assert_eq!(harness.query_all_by_role(Role::Group).count(), 1, "a row of one control adds no group");
    assert_eq!(text_input(&harness, "short").accesskit_node().data().labelled_by(), [id("Name:")]);
}

#[test]
fn a_cell_of_several_controls_puts_them_in_a_row_and_names_the_first() {
    let gap = std::cell::Cell::new(0.0);
    let harness =
        Harness::<()>::builder().with_size([640.0, 120.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(|ui| {
            gap.set(ui.spacing().item_spacing.x);
            FormGrid::new("form").show(ui, |form| {
                let _ = form.row("Mode:", |ui, _| {
                    (TextField::new("mode", "Mode", "first").show(ui), TextField::new("opacity", "Opacity", "second").show(ui))
                });
            });
        });
    let (first, second) = (text_input(&harness, "first"), text_input(&harness, "second"));
    assert!((second.rect().left() - first.rect().right() - gap.get()).abs() < 0.5, "{:?} {:?}", first.rect(), second.rect());
    let label = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Mode:")).accesskit_node().locate().0;
    assert_eq!(first.accesskit_node().data().labelled_by(), [label]);
    assert!(second.accesskit_node().data().labelled_by().is_empty());
}
