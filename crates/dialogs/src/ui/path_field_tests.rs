//! `PathField` in a harness with a `Files` stub installed in the Host.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{FileError, Files, Host, PickAnswer, PickId, PickRequest, Picked};
use craft_ui::localized::Labels;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{PathField, PathFieldLabels, PathFieldResponse};

/// Records every request; answers are put in by the test.
#[derive(Default)]
struct StubFiles {
    state: Mutex<StubState>,
}

#[derive(Default)]
struct StubState {
    next: u64,
    requests: Vec<PickRequest>,
    answered: Vec<PickAnswer>,
}

impl StubFiles {
    fn answer(&self, id: PickId, picked: Picked) {
        let mut state = self.state.lock().unwrap();
        let request = state.requests[(id.0 - 1) as usize].clone();
        state.answered.push(PickAnswer { id, request, picked });
    }

    fn requests(&self) -> Vec<PickRequest> {
        self.state.lock().unwrap().requests.clone()
    }
}

impl Files for StubFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap();
        state.next += 1;
        state.requests.push(request);
        PickId(state.next)
    }

    fn take_pick(&self) -> Option<PickAnswer> {
        let mut state = self.state.lock().unwrap();
        if state.answered.is_empty() { None } else { Some(state.answered.remove(0)) }
    }

    fn return_pick(&self, answer: PickAnswer) {
        self.state.lock().unwrap().answered.insert(0, answer);
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

struct Rig {
    ctx: Context,
    files: Arc<StubFiles>,
    value: String,
    must_exist: bool,
    field: Rect,
    /// What the last frame reported.
    committed: Option<String>,
    browsing: Option<PickId>,
    texts: Vec<String>,
    time: f64,
}

impl Rig {
    fn new(value: &str) -> Self {
        let ctx = Context::default();
        let files = Arc::new(StubFiles::default());
        Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).files(files.clone()).install(&ctx);
        let mut rig = Self {
            ctx,
            files,
            value: value.to_owned(),
            must_exist: false,
            field: Rect::NOTHING,
            committed: None,
            browsing: None,
            texts: Vec::new(),
            time: 0.0,
        };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, mut events: Vec<Event>) {
        let releases: Vec<Event> = events
            .iter()
            .filter_map(|e| match e {
                Event::Key { key, physical_key, pressed: true, modifiers, .. } => {
                    Some(Event::Key { key: *key, physical_key: *physical_key, pressed: false, repeat: false, modifiers: *modifiers })
                }
                _ => None,
            })
            .collect();
        events.extend(releases);
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let request = PickRequest::save("export").suggested_extension("png");
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let response: PathFieldResponse =
                PathField::new("export-path", "Save to", &self.value, request.clone()).must_exist(self.must_exist).show(ui);
            self.field = response.edit.response.rect;
            self.committed = response.edit.committed().cloned();
            self.browsing = response.browsing;
        });
        self.texts = output.shapes.iter().flat_map(|clipped| painted_text(&clipped.shape)).collect();
        output.drop_without_applying_deltas();
        if let Some(value) = self.committed.clone() {
            self.value = value;
        }
    }

    fn click(&mut self, at: Pos2) {
        let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
    }

    fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    }
}

fn painted_text(shape: &egui::Shape) -> Vec<String> {
    match shape {
        egui::Shape::Text(text) => vec![text.galley.text().to_owned()],
        egui::Shape::Vec(shapes) => shapes.iter().flat_map(painted_text).collect(),
        _ => Vec::new(),
    }
}

// Typing a path and pressing Enter commits it; the Browse button follows the field in Tab order.
#[test]
fn typed_path_commits_on_enter_and_browse_is_next_in_tab_order() {
    let mut rig = Rig::new("");
    let at = rig.field.center();
    rig.click(at);
    rig.frame(vec![Event::Text("/tmp/out.png".into())]);
    assert_eq!(rig.committed, None, "typing alone commits nothing");
    rig.key(Key::Enter);
    assert_eq!(rig.committed.as_deref(), Some("/tmp/out.png"));
    assert!(rig.files.requests().is_empty());

    rig.click(at);
    rig.key(Key::Tab);
    rig.key(Key::Enter);
    assert_eq!(rig.files.requests().len(), 1, "Enter on the focused Browse queues the picker");
}

// Browse queues the field's request; the answer's path fills the field on the next pass and
// commits once. An answer with another ticket is returned to the service.
#[test]
fn browse_answer_fills_the_field_and_other_tickets_are_returned() {
    let mut rig = Rig::new("/old.png");
    let browse = Pos2::new(rig.field.right() + 20.0, rig.field.center().y);
    rig.click(browse);
    let ticket = rig.files.requests().len();
    assert_eq!(ticket, 1);
    assert_eq!(rig.files.requests()[0], PickRequest::save("export").suggested_extension("png"));

    let ours = PickId(1);
    let other = PickId(99);
    rig.files.state.lock().unwrap().requests.resize(99, PickRequest::open("x"));
    rig.files.answer(other, Picked::Paths(vec![PathBuf::from("/theirs.png")]));
    rig.frame(vec![]);
    assert_eq!(rig.committed, None, "another ticket's answer is not ours");
    assert_eq!(rig.files.state.lock().unwrap().answered.len(), 1, "and goes back to the service");

    rig.files.state.lock().unwrap().answered.clear();
    rig.files.answer(ours, Picked::Paths(vec![PathBuf::from("/new.png"), PathBuf::from("/second.png")]));
    rig.frame(vec![]);
    assert_eq!(rig.value, "/new.png", "the first path fills the field");
    rig.frame(vec![]);
    assert_eq!(rig.committed, None, "and commits once");

    // A cancelled pick changes nothing.
    rig.click(browse);
    rig.files.answer(PickId(100), Picked::Cancelled);
    rig.frame(vec![]);
    assert_eq!(rig.value, "/new.png");
}

// With `must_exist`, a missing path shows the not-found message under the field.
#[test]
fn missing_path_shows_not_found_when_required() {
    let message = PathFieldLabels::english().not_found;
    let mut rig = Rig::new("/definitely/not/here.png");
    assert!(!rig.texts.contains(&message), "not required: no message");
    rig.must_exist = true;
    rig.frame(vec![]);
    assert!(rig.texts.contains(&message));

    rig.value = std::env::current_exe().expect("test binary").to_string_lossy().into_owned();
    rig.frame(vec![]);
    assert!(!rig.texts.contains(&message), "an existing path is fine");
}
