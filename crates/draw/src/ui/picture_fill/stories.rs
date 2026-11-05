//! Stories of the picture fill editor. Registering them
//! adds `stories()` to craft-draw's stories, the component to `COMPONENTS` and
//! [`PictureStoryLabels`] to `WORDS`.
//!
//! Every story uses a fake `Files` that answers Insert… at once with one made-up picture, and the
//! fill starts on a fake source; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/picture-fill-editor/editor` | Stretch and Tile; Insert… through the fake `Files` |

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::Ui;

use super::{PictureFill, PictureFillEditor, PictureFillEvent, PictureMode};
use crate::ui::stroke::stories::EventKindLabels;
use craft_host::{FileError, Files, PickId, PickRequest, Picked};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, ensure_files, lock, reset_on, shared};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PictureFillEditor"];

craft_ui::words! {
    /// The captions of the picture fill stories, English by default.
    #[non_exhaustive]
    pub(crate) struct PictureStoryLabels {
        /// The fake source the fill starts on.
        harbour: "pictureStory.harbour" = "Harbour.png";
        /// The name of the picture Insert… answers with.
        inserted: "pictureStory.inserted" = "Meadow.jpg";
        /// The note after a picture arrived: `{name}`, `{size}` bytes.
        received: "pictureStory.received" = "Picture received: {name}, {size} bytes";
        /// The note after a failed read: `{message}`.
        failed: "pictureStory.failed" = "Failed: {message}";
        files_note: "pictureStory.filesNote" = "Files are fake: Insert… answers with one picture.";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/picture-fill-editor/editor",
        Level::Composites,
        "PictureFillEditor",
        "Editor",
        EditorArgs { mode: ModeArg::Stretch, source: true },
        show_editor
    )]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    /// The picture fills the shape, with offsets in percent.
    Stretch,
    /// The picture repeats, with offset, scale, alignment and mirror.
    Tile,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct EditorArgs {
    mode: ModeArg,
    /// Starts on a picture; off starts with no source.
    source: bool,
}

// --- fake files ---

/// A file service that answers every pick at once with one browser-style file.
#[derive(Default)]
struct StoryFiles {
    next: Mutex<u64>,
    /// The name of the picture the next answer carries.
    name: Mutex<String>,
    waiting: Mutex<Vec<PickId>>,
}

/// The made-up picture's bytes.
const PICTURE: [u8; 4] = [0x89, b'P', b'N', b'G'];

impl Files for StoryFiles {
    fn pick(&self, _request: PickRequest) -> PickId {
        let mut next = lock(&self.next);
        *next += 1;
        let id = PickId(*next);
        lock(&self.waiting).push(id);
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut waiting = lock(&self.waiting);
        let at = waiting.iter().position(|w| *w == id)?;
        waiting.remove(at);
        Some(Picked::Contents(vec![(lock(&self.name).clone(), PICTURE.to_vec())]))
    }

    fn forget(&self, id: PickId) {
        lock(&self.waiting).retain(|w| *w != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

// --- editor ---

#[derive(Clone)]
struct Kept {
    fill: PictureFill,
    note: String,
}

fn start(words: &PictureStoryLabels, args: &EditorArgs) -> Kept {
    let mut fill = if args.source { PictureFill::new(words.harbour.clone()) } else { PictureFill::default() };
    if args.mode == ModeArg::Tile {
        fill.mode = PictureMode::Tile {
            offset: [0.0, 0.0],
            scale: [100.0, 100.0],
            alignment: super::TileAlignment::TopLeft,
            mirror: super::TileMirror::None,
        };
    }
    Kept { fill, note: String::new() }
}

fn show_editor(ui: &mut Ui, args: &mut EditorArgs) {
    let words = PictureStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let files = shared(ui, "files", || Arc::new(StoryFiles::default()));
    let files: Arc<StoryFiles> = lock(&files).clone();
    *lock(&files.name) = words.inserted.clone();
    let _scope = ensure_files(ui.ctx(), files);
    reset_on(ui, "editor", &(args.mode, args.source));
    let kept = shared(ui, "editor", || start(&words, args));
    let mut kept = lock(&kept);
    let events = PictureFillEditor::new("picture-story", &kept.fill).show(ui).events;
    for event in events {
        match event {
            PictureFillEvent::Edit(edit) => {
                if let Some(kind) = EventKindLabels::of(ui).kind(&edit) {
                    kept.note = craft_i18n::fmt(&story.last_event, &[("event", &kind)]);
                }
                if let Some(fill) = edit.value() {
                    kept.fill = fill.clone();
                }
            }
            PictureFillEvent::Source { name, bytes } => {
                kept.note = craft_i18n::fmt(&words.received, &[("name", &name), ("size", &bytes.len().to_string())]);
                kept.fill.source = Some(name);
            }
            PictureFillEvent::SourceFailed(message) => {
                kept.note = craft_i18n::fmt(&words.failed, &[("message", &message)]);
            }
        }
    }
    let note = if kept.note.is_empty() { words.files_note.clone() } else { kept.note.clone() };
    ui.add(Text::new(note).content(Content::Muted));
}
