//! Stories of the file list picker and the path field. The Host's file service answers every
//! picker with Cancelled, so Browse opens nothing.

use std::path::{Path, PathBuf};

use craft_host::{AppInfo, Host, PickRequest};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice};
use craft_ui::typed_story;
use egui::{Context, Ui};

use super::{FileList, FileListMode, FileListPicker, PathField};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["FileListPicker", "PathField"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    Files,
    Folder,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FileListArgs {
    mode: ModeArg,
    #[arg(range = 0..=8)]
    files: usize,
    #[arg(name = "info lines")]
    info: bool,
    #[arg(name = "open documents")]
    open_documents: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PathArg {
    Existing,
    Missing,
    Empty,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PathFieldArgs {
    path: PathArg,
    #[arg(name = "must exist")]
    must_exist: bool,
    enabled: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/file-list-picker/picker",
            Level::Patterns,
            "FileListPicker",
            "Picker",
            FileListArgs { mode: ModeArg::Files, files: 4, info: true, open_documents: true },
            show_file_list
        ),
        typed_story!(
            "composites/path-field/field",
            Level::Composites,
            "PathField",
            "Field",
            PathFieldArgs { path: PathArg::Missing, must_exist: true, enabled: true },
            show_path_field
        ),
    ]
}

/// Installs a Host with in-memory settings and a file service that cancels every picker, when
/// the story runs without one.
fn ensure_host(ctx: &Context) {
    if Host::try_of_ctx(ctx).is_none() {
        Host::builder(AppInfo { id: "craft-dialogs-stories", name: "Craft Stories", version: "0" }).install(ctx);
    }
}

const NAMES: [&str; 8] = ["IMG_0412.jpg", "IMG_0413.jpg", "cover-final.psd", "portrait.tif", "sky-study.png", "logo.svg", "scan-07.tif", "notes.txt"];
const FOLDER: &str = "/home/studio/Pictures/shoot-2026-10";

fn paths(count: usize) -> Vec<PathBuf> {
    NAMES.iter().take(count).map(|name| Path::new(FOLDER).join(name)).collect()
}

/// The info line the app would read from each file's header.
fn info(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    if name.ends_with(".txt") {
        None
    } else if name.ends_with(".svg") {
        Some("Vector, 2 artboards".to_owned())
    } else {
        Some("6000 \u{d7} 4000 px, 300 ppi, RGB".to_owned())
    }
}

/// The picker over a list kept in egui memory; the args choose the starting list.
fn show_file_list(ui: &mut Ui, args: &mut FileListArgs) {
    ensure_host(ui.ctx());
    let mode = match args.mode {
        ModeArg::Files => FileListMode::Files,
        ModeArg::Folder => FileListMode::Folder,
    };
    let key = ui.make_persistent_id("craft_dialogs::ui::file-list-story");
    let start = (args.mode, args.files);
    let (shown, mut list): (Option<(ModeArg, usize)>, FileList) = ui.data(|d| d.get_temp(key)).unwrap_or((None, FileList::default()));
    if shown != Some(start) {
        list = match mode {
            FileListMode::Files => FileList { mode, files: paths(args.files), ..FileList::default() },
            FileListMode::Folder => FileList { mode, folder: Some(PathBuf::from(FOLDER)), ..FileList::default() },
        };
    }
    let open = [Path::new("/home/studio/Documents/brochure.psd").to_path_buf(), Path::new("/home/studio/Documents/map.psd").to_path_buf()];
    let folder_files = paths(args.files.max(1));
    let mut picker =
        FileListPicker::new("story-file-list", &mut list).folder_mode(true).folder_files(&folder_files).request(PickRequest::open_many("stack"));
    if args.open_documents {
        picker = picker.open_files(&open);
    }
    if args.info {
        picker = picker.info(&info);
    }
    picker.show(ui);
    let state = if list.is_valid() { "OK is enabled" } else { "OK is disabled" };
    ui.label(state);
    ui.data_mut(|d| d.insert_temp(key, (Some(start), list)));
}

/// The path field over a value kept in egui memory; typing a path and Enter replace it.
fn show_path_field(ui: &mut Ui, args: &mut PathFieldArgs) {
    ensure_host(ui.ctx());
    let key = ui.make_persistent_id("craft_dialogs::ui::path-field-story");
    let (shown, mut value): (Option<PathArg>, String) = ui.data(|d| d.get_temp(key)).unwrap_or((None, String::new()));
    if shown != Some(args.path) {
        value = match args.path {
            PathArg::Existing => "/".to_owned(),
            PathArg::Missing => "/home/studio/Documents/brochure-v3.pdf".to_owned(),
            PathArg::Empty => String::new(),
        };
    }
    let request = PickRequest::open("print").title("Choose a PDF").filter("PDF", &["pdf"]);
    let response = PathField::new("story-path", "PDF file", &value, request).must_exist(args.must_exist).enabled(args.enabled).show(ui);
    if let Some(path) = response.edit.committed() {
        value.clone_from(path);
    }
    ui.data_mut(|d| d.insert_temp(key, (Some(args.path), value)));
}
