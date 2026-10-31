//! Stories of the step 2 dialogs, one per kind. Not yet in [`stories`](super::stories::stories):
//! registering them would change `tests/dumps/stories/registry.txt` and run their bodies in
//! `every_story_shows_with_its_default_args`. The fill task that lands the bodies adds
//! `all.extend(super::framework_stories::stories())` and regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum LayoutArg {
    SideColumn,
    BottomFooter,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ParamsArgs {
    open: bool,
    layout: LayoutArg,
    preview: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FormArgs {
    open: bool,
    #[arg(name = "ok enabled")]
    ok_enabled: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizeKindArg {
    Image,
    Canvas,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SizeArgs {
    open: bool,
    kind: SizeKindArg,
    resample: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PromptKindArg {
    ConfirmDelete,
    DamagedFile,
    ChangedOnDisk,
    ReplaceFile,
    TrashMask,
    Unsaved,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PromptArgs {
    open: bool,
    kind: PromptKindArg,
    #[arg(name = "several documents")]
    several: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PropertiesArgs {
    open: bool,
    #[arg(name = "read only")]
    read_only: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ReportArgs {
    open: bool,
    #[arg(range = 0..=40)]
    notes: usize,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct QueueArgs {
    recovery: bool,
    warning: bool,
    update: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/dialog/params",
            Level::Patterns,
            "Dialog",
            "Parameters",
            ParamsArgs { open: true, layout: LayoutArg::SideColumn, preview: true },
            show_params
        ),
        typed_story!("patterns/dialog/command-form", Level::Patterns, "Dialog", "Command form", FormArgs { open: true, ok_enabled: true }, show_form),
        typed_story!(
            "patterns/dialog/size",
            Level::Patterns,
            "Dialog",
            "Image and canvas size",
            SizeArgs { open: true, kind: SizeKindArg::Image, resample: true },
            show_size
        ),
        typed_story!(
            "patterns/dialog/standard-prompt",
            Level::Patterns,
            "Dialog",
            "Standard prompts",
            PromptArgs { open: true, kind: PromptKindArg::ConfirmDelete, several: false },
            show_prompt
        ),
        typed_story!(
            "patterns/dialog/properties",
            Level::Patterns,
            "Dialog",
            "Properties",
            PropertiesArgs { open: true, read_only: false },
            show_properties
        ),
        typed_story!("patterns/dialog/report", Level::Patterns, "Dialog", "Report", ReportArgs { open: true, notes: 6 }, show_report),
        typed_story!(
            "patterns/dialog/startup-queue",
            Level::Patterns,
            "Dialog",
            "Startup queue",
            QueueArgs { recovery: true, warning: true, update: true },
            show_queue
        ),
    ]
}

/// An `Arc<Mutex<DialogHost>>` kept in egui memory under the story's id, opened with the dialog
/// when `open` flips on; the tile shows the result text after OK.
// new: the host in temp memory, as `stack_stories::show_stack` keeps its stack.
fn show_params(ui: &mut Ui, args: &mut ParamsArgs) {
    todo!()
}

fn show_form(ui: &mut Ui, args: &mut FormArgs) {
    todo!()
}

fn show_size(ui: &mut Ui, args: &mut SizeArgs) {
    todo!()
}

fn show_prompt(ui: &mut Ui, args: &mut PromptArgs) {
    todo!()
}

fn show_properties(ui: &mut Ui, args: &mut PropertiesArgs) {
    todo!()
}

fn show_report(ui: &mut Ui, args: &mut ReportArgs) {
    todo!()
}

/// Three alerts enqueued in the ticked stages; they open one at a time in stage order.
fn show_queue(ui: &mut Ui, args: &mut QueueArgs) {
    todo!()
}
