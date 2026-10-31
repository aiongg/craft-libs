use egui::accesskit::{Invalid, Role};
use egui::{Context, Rect, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::field::{DialogArgs, show_dialog};
use super::*;
use crate::controls::{Checkbox, NumberField, Select, Slider, TextField};
use crate::direction::{self, Direction};
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, install};

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

fn text_input<'a, State>(harness: &'a Harness<'_, State>, value: &'a str) -> Node<'a> {
    harness.get_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some(value))
}

/// A text node (its text runs carry the same value).
fn text<'a, State>(harness: &'a Harness<'_, State>, value: &'a str) -> Option<Node<'a>> {
    harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value))
}

#[track_caller]
fn rect<State>(harness: &Harness<'_, State>, value: &str) -> Rect {
    harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(value)).rect()
}

/// A short and a long label, then a check box without one.
fn form(ui: &mut Ui) {
    FieldGroup::new("form").show(ui, |ui| {
        let _ = Field::new("Name:").show(ui, |ui, _| TextField::new("name", "Name", "short").show(ui));
        let _ = Field::new("Resolution:").show(ui, |ui, _| TextField::new("res", "Resolution", "long").show(ui));
        let mut clip = false;
        let _ = Field::unlabelled().show(ui, |ui, _| Checkbox::bool("Clip", &mut clip).show(ui));
    });
}

#[test]
fn a_group_lines_its_controls_up_after_one_label_column() {
    let harness = harness(Direction::Ltr).build_ui(form);
    let (short, long) = (text_input(&harness, "short").rect(), text_input(&harness, "long").rect());
    let clip = harness.get_by_label("Clip").rect();
    assert!((short.left() - long.left()).abs() < 0.5, "{short:?} {long:?}");
    assert!((clip.left() - short.left()).abs() < 0.5, "the unlabelled row starts at the control column: {clip:?}");
    let (name, res) = (rect(&harness, "Name:"), rect(&harness, "Resolution:"));
    assert!((name.right() - res.right()).abs() < 0.5, "labels sit against the control: {name:?} {res:?}");
    assert!(res.right() < long.left() && name.width() < res.width());
    a11y::assert_labelled(&harness);
    assert_eq!(text_input(&harness, "short").accesskit_node().label().as_deref(), Some("Name:"));
    assert_eq!(text_input(&harness, "long").accesskit_node().label().as_deref(), Some("Resolution:"));
    assert_eq!(harness.get_by(|n| n.role() == Role::CheckBox).accesskit_node().label().as_deref(), Some("Clip"));
}

#[test]
fn right_to_left_the_label_column_is_at_the_right() {
    let harness = harness(Direction::Rtl).build_ui(form);
    let (short, long) = (text_input(&harness, "short").rect(), text_input(&harness, "long").rect());
    let clip = harness.get_by_label("Clip").rect();
    assert!((short.right() - long.right()).abs() < 0.5, "{short:?} {long:?}");
    assert!((clip.right() - short.right()).abs() < 0.5, "{clip:?}");
    let (name, res) = (rect(&harness, "Name:"), rect(&harness, "Resolution:"));
    assert!((name.left() - res.left()).abs() < 0.5, "{name:?} {res:?}");
    assert!(res.left() > long.right());
}

/// A short and a long label with the labels at the reading start.
fn start_form(ui: &mut Ui) {
    FieldGroup::new("form").show(ui, |ui| {
        let _ = Field::new("Name:").label_align(LabelAlign::Start).show(ui, |ui, _| TextField::new("name", "Name", "short").show(ui));
        let _ = Field::new("Resolution:").label_align(LabelAlign::Start).show(ui, |ui, _| TextField::new("res", "Resolution", "long").show(ui));
    });
}

#[test]
fn start_aligned_labels_share_the_reading_start_of_the_column() {
    let ltr = harness(Direction::Ltr).build_ui(start_form);
    let (name, res) = (rect(&ltr, "Name:"), rect(&ltr, "Resolution:"));
    assert!((name.left() - res.left()).abs() < 0.5, "{name:?} {res:?}");
    assert!(name.right() < res.right());
    let rtl = harness(Direction::Rtl).build_ui(start_form);
    let (name, res) = (rect(&rtl, "Name:"), rect(&rtl, "Resolution:"));
    assert!((name.right() - res.right()).abs() < 0.5, "{name:?} {res:?}");
}

#[test]
fn above_puts_the_label_over_the_control_at_the_reading_start() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let _ = Field::new("Name:").above().show(ui, |ui, _| TextField::new("name", "Name", "x").show(ui));
    });
    let (label, field) = (rect(&harness, "Name:"), text_input(&harness, "x").rect());
    assert!(label.bottom() <= field.top() && (label.left() - field.left()).abs() < 0.5, "{label:?} {field:?}");
}

#[test]
fn an_error_replaces_the_help_text_and_the_help_comes_back() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, error: &mut bool| {
            let _ = Field::new("Name:")
                .help("Shown in the Layers panel.")
                .error(error.then_some("Enter a name."))
                .show(ui, |ui, invalid| TextField::new("name", "Name", "x").invalid(invalid).show(ui));
        },
        false,
    );
    let shown = |harness: &Harness<'_, bool>| (text(harness, "Shown in the Layers panel.").is_some(), text(harness, "Enter a name.").is_some());
    assert_eq!(shown(&harness), (true, false));
    *harness.state_mut() = true;
    harness.run();
    assert_eq!(shown(&harness), (false, true));
    let help = rect(&harness, "Enter a name.");
    assert!(help.top() >= text_input(&harness, "x").rect().bottom(), "under the control");
    *harness.state_mut() = false;
    harness.run();
    assert_eq!(shown(&harness), (true, false));
}

#[test]
fn the_control_is_labelled_described_and_given_its_error_message() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, error: &mut bool| {
            let _ = Field::new("Name:")
                .help("Shown in the Layers panel.")
                .error(error.then_some("Enter a name."))
                .show(ui, |ui, invalid| TextField::new("name", "Name", "x").invalid(invalid).show(ui));
            let mut on = true;
            let _ = Field::unlabelled().show(ui, |ui, _| Checkbox::bool("Clip", &mut on).show(ui));
        },
        false,
    );
    let id = |harness: &Harness<'_, bool>, value: &str| text(harness, value).expect("text node").accesskit_node().locate().0;
    {
        let node = text_input(&harness, "x");
        let node = node.accesskit_node();
        assert_eq!(node.data().labelled_by(), [id(&harness, "Name:")]);
        assert_eq!(node.label().as_deref(), Some("Name:"), "the visible label names it");
        assert_eq!(node.data().described_by(), [id(&harness, "Shown in the Layers panel.")]);
        assert_eq!((node.data().error_message(), node.data().invalid()), (None, None));
        let clip = harness.get_by_label("Clip");
        assert!(clip.accesskit_node().data().labelled_by().is_empty(), "a row without a label keeps the control's name");
    }
    *harness.state_mut() = true;
    harness.run();
    let error = id(&harness, "Enter a name.");
    let node = text_input(&harness, "x");
    let node = node.accesskit_node();
    assert_eq!((node.data().error_message(), node.data().invalid()), (Some(error), Some(Invalid::True)));
    assert_eq!(node.data().described_by(), [error]);
}

#[test]
fn every_kind_of_control_is_labelled_described_and_given_its_error() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let numbers = LocaleNumbers::from_tag("en");
        let (mut mode, mut snap) = (0usize, false);
        let _ = Field::new("Opacity:")
            .help("Opacity help.")
            .show(ui, |ui, invalid| NumberField::new("opacity", "Opacity", 50.0, &numbers).invalid(invalid).show(ui));
        let _ = Field::new("Mode:")
            .error(Some("Mode error."))
            .show(ui, |ui, _| Select::new("mode", "Mode", &mut mode).option(0, "Normal").option(1, "Multiply").show(ui));
        let _ = Field::new("Snapping:").help("Snap help.").show(ui, |ui, _| Checkbox::bool("Snap", &mut snap).show(ui));
        let _ = Field::new("Size:").error(Some("Size error.")).show(ui, |ui, _| Slider::new("size", "Size", 5.0, 0.0..=10.0).show(ui));
    });
    let id = |value: &str| text(&harness, value).expect("text node").accesskit_node().locate().0;
    for (role, label, message, invalid) in [
        (Role::SpinButton, "Opacity:", "Opacity help.", false),
        (Role::ComboBox, "Mode:", "Mode error.", true),
        (Role::CheckBox, "Snapping:", "Snap help.", false),
        (Role::Slider, "Size:", "Size error.", true),
    ] {
        let node = harness.get_by(|n| n.role() == role).accesskit_node();
        assert_eq!(node.label().as_deref(), Some(label), "{role:?}");
        assert_eq!(node.data().labelled_by(), [id(label)], "{role:?}");
        assert_eq!(node.data().described_by(), [id(message)], "{role:?}");
        assert_eq!(node.data().error_message(), invalid.then(|| id(message)), "{role:?}");
        assert_eq!(node.data().invalid(), invalid.then_some(Invalid::True), "{role:?}");
    }
}

#[test]
fn the_dialog_story_names_every_control() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for (above, error) in [(false, false), (true, true)] {
            let mut args = DialogArgs { above, help: true, error };
            let harness = harness(dir).build_ui(move |ui| show_dialog(ui, &mut args));
            a11y::assert_labelled(&harness);
            assert_eq!(text_input(&harness, "Layer 1").accesskit_node().label().as_deref(), Some("Name:"));
        }
    }
}

#[test]
fn the_first_frame_already_shows_the_measured_column() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let frame = |ctx: &Context| {
        let mut lefts = Vec::new();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            lefts.clear();
            FieldGroup::new("form").show(ui, |ui| {
                for (label, value) in [("Name:", "short"), ("Resolution:", "long")] {
                    let edit = Field::new(label).show(ui, |ui, _| TextField::new(value, label, value).show(ui)).inner;
                    lefts.push(edit.response.rect.left());
                }
            });
        });
        let passes = output.platform_output.num_completed_passes;
        output.drop_without_applying_deltas();
        (passes, lefts)
    };
    let (passes, lefts) = frame(&ctx);
    assert_eq!(passes, 2, "the pass that measured the labels is discarded");
    assert!((lefts[0] - lefts[1]).abs() < 0.5, "{lefts:?}");
    assert_eq!(frame(&ctx).0, 1, "an unchanged column lays out once");
}

#[test]
fn a_help_text_keeps_its_paragraphs_each_under_the_one_before() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let _ = Field::new("Name:").help("First paragraph.\nSecond paragraph.").show(ui, |ui, _| TextField::new("name", "Name", "x").show(ui));
    });
    let (first, second) = (rect(&harness, "First paragraph."), rect(&harness, "Second paragraph."));
    assert!(second.top() >= first.bottom(), "{first:?} {second:?}");
    let node = text_input(&harness, "x");
    let described = node.accesskit_node().data().described_by().len();
    assert_eq!(described, 2, "both paragraphs describe the control");
}

#[test]
fn a_multi_paragraph_error_is_one_error_message_node_reading_every_paragraph() {
    let harness = harness(Direction::Ltr).build_ui(|ui| {
        let _ = Field::new("Name:")
            .error(Some("First problem.\nSecond problem."))
            .show(ui, |ui, invalid| TextField::new("name", "Name", "x").invalid(invalid).show(ui));
    });
    let node = text_input(&harness, "x");
    let node = node.accesskit_node();
    let message = node.data().error_message().expect("an error message");
    let group = harness.get_by_role_and_label(Role::Group, "First problem.\nSecond problem.");
    assert_eq!(group.accesskit_node().locate().0, message);
}
