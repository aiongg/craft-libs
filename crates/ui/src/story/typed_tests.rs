use egui::{Context, RawInput};

use super::*;
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Mode {
    Plain,
    #[choice(name = "extra wide")]
    ExtraWide,
    Last,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AllKinds {
    on: bool,
    #[arg(name = "the label")]
    label: String,
    #[arg(range = 0.0..=100.0)]
    amount: f64,
    #[arg(range = -5.0..=5.0)]
    offset: f32,
    #[arg(range = 1..=40)]
    rows: usize,
    mode: Mode,
}

fn sample() -> AllKinds {
    AllKinds { on: true, label: "Hello".into(), amount: 42.5, offset: -2.5, rows: 7, mode: Mode::ExtraWide }
}

#[test]
fn derived_args_list_the_fields_in_order_and_round_trip() {
    let args = sample().to_args();
    let expected = Args::new()
        .with_bool("on", true)
        .with_text("the label", "Hello")
        .with_number("amount", 42.5, 0.0, 100.0)
        .with_number("offset", -2.5, -5.0, 5.0)
        .with_number("rows", 7.0, 1.0, 40.0)
        .with_choice("mode", &["plain", "extra wide", "last"], 1);
    assert_eq!(args, expected);
    assert_eq!(AllKinds::from_args(&args), sample());
}

#[test]
fn choice_names_and_indices() {
    assert_eq!(Mode::OPTIONS, &["plain", "extra wide", "last"]);
    assert_eq!(Mode::ExtraWide.index(), 1);
    assert_eq!(Mode::from_index(2), Mode::Last);
    assert_eq!(Mode::from_index(99), Mode::Last);
    assert_eq!(Mode::ExtraWide.name(), "extra wide");
    assert_eq!(SizeArg::OPTIONS, &["sm", "md"]);
}

#[test]
fn a_typed_story_keeps_what_it_changes() {
    fn toggle(_: &mut egui::Ui, args: &mut AllKinds) {
        args.on = !args.on;
        args.mode = Mode::Last;
    }
    let story = typed_story!("controls/x/y", Level::Controls, "X", "Y", sample(), toggle);
    let mut args = story.defaults();
    assert_eq!(args, sample().to_args().for_story("controls/x/y"));
    Context::default().run_ui(RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
    let after = AllKinds::from_args(&args);
    assert!(!after.on);
    assert_eq!(after.mode, Mode::Last);
    assert_eq!(after.amount, 42.5);
}

#[test]
fn migrated_stories_build_the_args_they_had_by_hand() {
    let args = |id: &str| registry().into_iter().find(|s| s.id == id).expect("story").defaults();
    let sizes = &["sm", "md"];
    assert_eq!(
        args("controls/button/playground"),
        Args::new()
            .with_choice("variant", &["primary", "secondary", "ghost", "danger"], 0)
            .with_choice("size", sizes, 1)
            .with_choice("icon", &["none", "save", "chevron_right"], 1)
            .with_bool("disabled", false)
            .with_bool("loading", false)
            .with_bool("narrow", false)
            .with_text("label", "Save the document as a copy")
            .for_story("controls/button/playground")
    );
    assert_eq!(
        args("controls/slider/values"),
        Args::new()
            .with_choice("size", sizes, 1)
            .with_bool("disabled", false)
            .with_bool("default value (50)", false)
            .with_bool("wheel unfocused", false)
            .for_story("controls/slider/values")
    );
}

fn set_number(args: &mut Args, name: &str, to: f64) {
    for arg in args.iter_mut().filter(|a| a.name == name) {
        if let ArgValue::Number { value, .. } = &mut arg.value {
            *value = to;
        }
    }
}

fn run(story: &Story, args: &mut Args) {
    Context::default().run_ui(RawInput::default(), |ui| (story.show)(ui, args)).drop_without_applying_deltas();
}

#[test]
fn a_story_that_writes_nothing_leaves_fractional_values_alone() {
    fn idle(_: &mut egui::Ui, _: &mut AllKinds) {}
    let story = typed_story!("controls/x/y", Level::Controls, "X", "Y", sample(), idle);
    let mut args = story.defaults();
    for arg in args.iter_mut() {
        if let ArgValue::Number { value, .. } = &mut arg.value {
            *value += 0.3;
        }
    }
    let before = args.clone();
    run(&story, &mut args);
    assert_eq!(args, before);
}

#[test]
fn a_story_that_writes_one_field_changes_only_that_field() {
    fn bump(_: &mut egui::Ui, args: &mut AllKinds) {
        args.rows = 9;
    }
    let story = typed_story!("controls/x/y", Level::Controls, "X", "Y", sample(), bump);
    let mut args = story.defaults();
    set_number(&mut args, "amount", 42.4);
    let mut expected = args.clone();
    run(&story, &mut args);
    set_number(&mut expected, "rows", 9.0);
    assert_eq!(args, expected);
}

#[test]
fn integer_fields_round_to_the_nearest_value() {
    let mut args = sample().to_args();
    set_number(&mut args, "rows", 7.6);
    assert_eq!(AllKinds::from_args(&args).rows, 8);
}

#[derive(StoryArgs, Debug, PartialEq)]
struct Shared {
    on: bool,
    #[arg(range = 0.0..=10.0)]
    gap: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct Flattened {
    label: String,
    #[arg(flatten)]
    shared: Shared,
    mode: Mode,
}

#[test]
fn a_flattened_struct_lists_its_args_in_place_and_round_trips() {
    let value = Flattened { label: "Hi".into(), shared: Shared { on: true, gap: 3.0 }, mode: Mode::Last };
    let args = value.to_args();
    let expected = Args::new().with_text("label", "Hi").with_bool("on", true).with_number("gap", 3.0, 0.0, 10.0).with_choice(
        "mode",
        &["plain", "extra wide", "last"],
        2,
    );
    assert_eq!(args, expected);
    assert_eq!(Flattened::from_args(&args), value);
    assert_eq!(<Flattened as StoryArgs>::ARG_COUNT, 4);
}

#[test]
#[should_panic(expected = "two args are named `on`")]
fn an_arg_named_like_an_earlier_flattened_arg_panics() {
    let _ = Args::new().with_args(&Shared { on: true, gap: 1.0 }.to_args()).with_bool("on", false);
}

#[test]
#[should_panic(expected = "two args are named `on`")]
fn a_flattened_arg_named_like_an_earlier_arg_panics() {
    let _ = Args::new().with_bool("on", false).with_args(&Shared { on: true, gap: 1.0 }.to_args());
}
