//! Stories of [`StartScreen`]: six recent files in Grid and List, two pinned folders and three
//! templates; and the empty state.

use std::path::PathBuf;

use egui::{Ui, vec2};

use super::{RecentFile, RecentSort, RecentView, StartScreen, Template};
use crate::story::{Level, Story, StoryArgs, StoryChoice, Surface};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["StartScreen"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/shell/start",
            Level::Patterns,
            "StartScreen",
            "Start",
            StartArgs { view: ViewArg::Grid, sort: SortArg::Opened, empty: false },
            show_start
        )
        .on(Surface::Own),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ViewArg {
    Grid,
    List,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SortArg {
    Opened,
    Name,
    Created,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StartArgs {
    view: ViewArg,
    sort: SortArg,
    /// No recent files, pinned folders or templates.
    empty: bool,
}

const NOW: i64 = 1_800_000_000;

fn recent() -> Vec<RecentFile> {
    let file = |name: &str, age: i64| RecentFile {
        path: PathBuf::from(format!("/home/user/Pictures/{name}")),
        name: name.to_owned(),
        opened: Some(NOW - age),
        created: Some(NOW - age - 86_400),
        thumbnail: None,
    };
    vec![
        file("Harbour.psd", 30),
        file("Poster draft.psd", 3_600 * 2),
        file("Logo.psd", 86_400 * 2),
        file("a10.psd", 86_400 * 9),
        file("a2.psd", 86_400 * 40),
        file("Scan 0007.psd", 86_400 * 400),
    ]
}

fn show_start(ui: &mut Ui, args: &mut StartArgs) {
    let files = recent();
    let pinned = [PathBuf::from("/home/user/Pictures"), PathBuf::from("/home/user/Projects")];
    let templates = [
        Template { id: "hd".into(), name: "Full HD".into(), detail: "1920 × 1080 px".into(), thumbnail: None },
        Template { id: "a4".into(), name: "A4".into(), detail: "210 × 297 mm".into(), thumbnail: None },
        Template { id: "square".into(), name: "Square".into(), detail: "1080 × 1080 px".into(), thumbnail: None },
    ];
    let view = match args.view {
        ViewArg::Grid => RecentView::Grid,
        ViewArg::List => RecentView::List,
    };
    let sort = match args.sort {
        SortArg::Opened => RecentSort::Opened,
        SortArg::Name => RecentSort::Name,
        SortArg::Created => RecentSort::Created,
    };
    ui.set_min_size(vec2(ui.available_width(), 420.0));
    let mut screen = StartScreen::new("start-story").view(view, sort).now(NOW);
    if !args.empty {
        screen = screen.recent(&files).pinned(&pinned).templates(&templates);
    }
    screen.show(ui);
}
