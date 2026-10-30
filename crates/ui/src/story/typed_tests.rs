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
    assert_eq!(AllKinds::read(&args), sample());
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
    assert!(!args.bool("on"));
    assert_eq!(args.choice("mode"), "last");
    assert_eq!(args.number("amount"), 42.5);
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
