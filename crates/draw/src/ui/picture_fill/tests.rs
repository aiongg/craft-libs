//! Tests of the picture fill editor.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::accesskit::Role;
use egui::{Context, Event, Key, Ui};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, Node};

use super::{PictureFill, PictureFillEditor, PictureFillEvent, PictureMode, TileAlignment, TileMirror};
use craft_host::{AppInfo, FileError, Files, PickId, PickRequest, Picked};
use craft_ui::theme::{Base, install};

/// A host of one editor that applies every fill it reports.
struct EditorHost {
    fill: PictureFill,
    events: Vec<PictureFillEvent>,
}

impl EditorHost {
    fn show(&mut self, ui: &mut Ui) {
        for event in PictureFillEditor::new("picture", &self.fill).show(ui).events {
            if let PictureFillEvent::Edit(edit) = &event
                && let Some(fill) = edit.value()
            {
                self.fill = fill.clone();
            }
            self.events.push(event);
        }
    }
}

/// A file service whose picker answers `answer` at once.
struct Scripted {
    answer: Picked,
    asked: Mutex<Vec<PickRequest>>,
    waiting: Mutex<Vec<PickId>>,
}

impl Files for Scripted {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut asked = self.asked.lock().unwrap();
        asked.push(request);
        let id = PickId(asked.len() as u64);
        self.waiting.lock().unwrap().push(id);
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut waiting = self.waiting.lock().unwrap();
        let at = waiting.iter().position(|w| *w == id)?;
        waiting.remove(at);
        Some(self.answer.clone())
    }

    fn forget(&self, id: PickId) {
        self.waiting.lock().unwrap().retain(|w| *w != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FileError> {
        Ok(std::fs::read(path)?)
    }

    fn write(&self, _: &Path, _: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _: &str) -> Option<PathBuf> {
        None
    }
}

fn editor(fill: PictureFill, files: Option<Arc<Scripted>>) -> Harness<'static, EditorHost> {
    let harness = Harness::builder().with_size([480.0, 720.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        if let Some(files) = &files {
            craft_host::Host::builder(AppInfo { id: "test", name: "Test", version: "1" }).files(files.clone()).build().install(ctx);
        }
    });
    let mut harness = harness.build_ui_state(|ui, host: &mut EditorHost| host.show(ui), EditorHost { fill, events: Vec::new() });
    harness.run();
    harness
}

fn field<'a, S>(harness: &'a Harness<'_, S>, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name.as_str()))
}

/// Types `text` into the number field `name` and commits it with Tab.
fn type_into<S>(harness: &mut Harness<'_, S>, name: &str, text: &str) {
    field(harness, name).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
}

fn toggle_tile(harness: &mut Harness<'_, EditorHost>) {
    harness.get_by_label("Tile picture as texture").click();
    harness.run();
}

// Tile shows Offset X and Y, Scale X and Y, Alignment and Mirror type; Stretch shows the four
// offsets; turning Tile on and off keeps each mode's values.
#[test]
fn picture_editor_tile_fields_and_stretch_offsets() {
    let mut harness = editor(PictureFill::new("photo.png"), None);
    for name in ["Offset left", "Offset top", "Offset right", "Offset bottom"] {
        assert!(harness.query_by_label(name).is_some(), "{name}");
    }
    assert!(harness.query_by_label("Offset X").is_none());
    assert!(harness.query_by_label("Alignment").is_none());

    type_into(&mut harness, "Offset left", "10");
    assert_eq!(harness.state().fill.mode, PictureMode::Stretch { offsets: [10.0, 0.0, 0.0, 0.0] });

    toggle_tile(&mut harness);
    for name in ["Offset X", "Offset Y", "Scale X", "Scale Y", "Alignment", "Mirror type"] {
        assert!(harness.query_by_label(name).is_some(), "{name}");
    }
    assert!(harness.query_by_label("Offset left").is_none());
    assert!(matches!(
        harness.state().fill.mode,
        PictureMode::Tile { scale: [100.0, 100.0], alignment: TileAlignment::TopLeft, mirror: TileMirror::None, .. }
    ));

    type_into(&mut harness, "Offset X", "5");
    toggle_tile(&mut harness);
    assert_eq!(harness.state().fill.mode, PictureMode::Stretch { offsets: [10.0, 0.0, 0.0, 0.0] });
    toggle_tile(&mut harness);
    assert!(matches!(harness.state().fill.mode, PictureMode::Tile { offset: [5.0, 0.0], .. }));
    assert!(harness.state().events.iter().all(|e| matches!(e, PictureFillEvent::Edit(_))));
}

// A typed 0 or negative tile scale snaps to the floor, never to zero.
#[test]
fn picture_editor_tile_scale_has_a_floor() {
    let mut harness = editor(PictureFill::new("photo.png"), None);
    toggle_tile(&mut harness);
    type_into(&mut harness, "Scale X", "0");
    type_into(&mut harness, "Scale Y", "-50");
    let PictureMode::Tile { scale, .. } = harness.state().fill.mode else { panic!("tile mode") };
    assert_eq!(scale, [*PictureMode::SCALE.start(); 2]);
}

fn scripted(answer: Picked) -> Arc<Scripted> {
    Arc::new(Scripted { answer, asked: Mutex::new(Vec::new()), waiting: Mutex::new(Vec::new()) })
}

// Insert… with a fake `Files` answering "photo.png" reports `Source { name: "photo.png", bytes }`
// once the job reads it; a cancelled pick reports nothing.
#[test]
fn picture_editor_source_pick_reports_the_picked_image() {
    let dir = std::env::temp_dir().join(format!("craft-picture-fill-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("photo.png");
    std::fs::write(&path, [1u8, 2, 3]).unwrap();

    let files = scripted(Picked::Paths(vec![path]));
    let mut harness = editor(PictureFill::default(), Some(files.clone()));
    harness.get_by_label("Insert…").click();
    for _ in 0..200 {
        harness.run();
        if !harness.state().events.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let asked = files.asked.lock().unwrap().clone();
    assert!(
        matches!(asked.as_slice(), [request] if request.purpose == "picture" && request.filters.iter().any(|f| f.extensions.contains(&"png".to_owned())))
    );
    assert_eq!(harness.state().events, [PictureFillEvent::Source { name: "photo.png".into(), bytes: vec![1, 2, 3] }]);
    let _ = std::fs::remove_dir_all(&dir);

    let mut harness = editor(PictureFill::default(), Some(scripted(Picked::Cancelled)));
    harness.get_by_label("Insert…").click();
    harness.run_steps(10);
    assert!(harness.state().events.is_empty());
}
