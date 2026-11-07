//! `ToolFixture` (22.4, the tool row): drives a [`CanvasTool`] on a [`Canvas`] through press, drag, release and
//! Escape, and checks what every tool owes the canvas.

use super::{Fixture, Target, pointer};
use crate::canvas::{Canvas, CanvasTool, ToolHost, ToolPath};
use crate::contract::{BodyCx, ShowContainer as _};
use crate::runtime::access::Role;
use craft_behavior::GesturePhase;
use egui::{Event, Key, Modifiers, Pos2, Ui, Vec2};
use std::cell::{Ref, RefCell, RefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The rule of the `probe` entry that names a freehand tool in `design/exceptions/<crate>/<family>.toml`; the
/// entry's `name` is the fixture's label.
pub const FREEHAND_RULE: &str = "probe/exempt/freehand-tool";

/// A tool shown on a canvas that fills the fixture's room, with the gesture phases the host reported.
#[non_exhaustive]
pub struct ToolFixture<T: CanvasTool + 'static> {
    fixture: Fixture,
    tool: Rc<RefCell<T>>,
    label: &'static str,
    gestures: Rc<RefCell<Vec<GesturePhase>>>,
}

impl<T: CanvasTool + 'static> ToolFixture<T> {
    /// A canvas named `label` whose active tool is `tool`.
    pub fn new(label: &'static str, tool: T) -> Self {
        Self::with_body(label, tool, |_, _| {})
    }

    /// A canvas named `label` whose body `body` fills: canvas controls shown at the body's rect join the canvas's
    /// arena, and a press on one of their parts never reaches the tool.
    pub fn with_body(label: &'static str, tool: T, mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'static) -> Self {
        let tool = Rc::new(RefCell::new(tool));
        let gestures = Rc::new(RefCell::new(Vec::new()));
        let (shown, seen) = (Rc::clone(&tool), Rc::clone(&gestures));
        let mut fixture = Fixture::new(move |ui| {
            let path = shown.borrow().command_form();
            let mut host = ToolHost::new(label);
            let canvas = Canvas::new(label, path).show(ui, |ui, cx| {
                host.capture_values(ui);
                body(ui, cx);
            });
            let reply = host.show(ui, &canvas.output, &mut *shown.borrow_mut());
            if let Some(phase) = reply.gesture {
                seen.borrow_mut().push(phase);
            }
        });
        fixture.settle();
        ToolFixture { fixture, tool, label, gestures }
    }

    /// The fixture underneath, for input the helpers here do not cover.
    pub fn fixture(&mut self) -> &mut Fixture {
        &mut self.fixture
    }

    /// The tool.
    pub fn tool(&self) -> Ref<'_, T> {
        self.tool.borrow()
    }

    /// The tool, to change its options between gestures.
    pub fn tool_mut(&self) -> RefMut<'_, T> {
        self.tool.borrow_mut()
    }

    /// The canvas's node.
    pub fn canvas(&self) -> Target {
        self.fixture.by_role(Role::CANVAS).into_iter().next().expect("the fixture shows a canvas")
    }

    /// The gesture phases the host reported, one per pass that began, moved or ended a gesture, since the last
    /// [`ToolFixture::forget`].
    pub fn gestures(&self) -> Vec<GesturePhase> {
        self.gestures.borrow().clone()
    }

    /// Forgets the phases reported so far.
    pub fn forget(&mut self) -> &mut Self {
        self.gestures.borrow_mut().clear();
        self
    }

    fn count(&self, phase: GesturePhase) -> usize {
        self.gestures.borrow().iter().filter(|seen| **seen == phase).count()
    }

    /// Hover and press at `at` with the primary button, in separate frames.
    pub fn press_at(&mut self, at: Pos2) -> &mut Self {
        self.fixture.frame(vec![pointer::moved(at)]);
        self.fixture.frame(vec![pointer::press(at)]);
        self
    }

    /// Moves to `to` in one frame.
    pub fn move_to(&mut self, to: Pos2) -> &mut Self {
        self.fixture.frame(vec![pointer::moved(to)]);
        self
    }

    /// Releases the primary button at `at`, then one idle frame.
    pub fn release_at(&mut self, at: Pos2) -> &mut Self {
        self.fixture.frame(vec![pointer::release(at)]);
        self.fixture.frame(Vec::new());
        self
    }

    /// Presses at `from`, moves by `by` in four steps and releases.
    pub fn drag_from(&mut self, from: Pos2, by: Vec2) -> &mut Self {
        self.drag_with(from, by, |at, down| if down { pointer::press(at) } else { pointer::release(at) })
    }

    /// [`ToolFixture::drag_from`] from the middle of the canvas.
    pub fn drag(&mut self, by: Vec2) -> &mut Self {
        let from = self.canvas().center();
        self.drag_from(from, by)
    }

    /// [`ToolFixture::drag_from`] with the middle button, which a canvas keeps for panning.
    pub fn middle_drag_from(&mut self, from: Pos2, by: Vec2) -> &mut Self {
        self.drag_with(from, by, pointer::middle)
    }

    fn drag_with(&mut self, from: Pos2, by: Vec2, button: fn(Pos2, bool) -> Event) -> &mut Self {
        self.fixture.frame(vec![pointer::moved(from)]);
        self.fixture.frame(vec![button(from, true)]);
        for step in 1..=4 {
            self.fixture.frame(vec![pointer::moved(from + by * (step as f32 / 4.0))]);
        }
        self.fixture.frame(vec![button(from + by, false)]);
        self.fixture.frame(Vec::new());
        self
    }

    /// Presses at `from`, moves halfway by `by`, presses Escape, moves the rest of the way and releases.
    pub fn drag_and_escape_from(&mut self, from: Pos2, by: Vec2) -> &mut Self {
        self.press_at(from);
        self.move_to(from + by * 0.25);
        self.move_to(from + by * 0.5);
        self.escape();
        self.move_to(from + by);
        self.release_at(from + by)
    }

    /// Presses Escape.
    pub fn escape(&mut self) -> &mut Self {
        self.fixture.key(Key::Escape, Modifiers::NONE);
        self
    }

    /// The checks every tool owes the canvas, on a drag from the middle of the canvas:
    ///
    /// - the tool has a command form: `ToolPath::Command` with a text, or `ToolPath::Freehand` with its named
    ///   exception entry (checked when the lint configuration is readable);
    /// - a drag reaches the tool as one `Begin` and one `Commit`, with updates between;
    /// - Escape during a drag cancels it once, and a second Escape cancels nothing.
    pub fn check(&mut self) {
        self.check_command_form();
        let by = Vec2::new(60.0, 40.0);
        self.forget().drag(by);
        let seen = self.gestures();
        assert_eq!(self.count(GesturePhase::Begin), 1, "a drag begins once: {seen:?}");
        assert_eq!(self.count(GesturePhase::Commit), 1, "a drag commits once: {seen:?}");
        assert_eq!(self.count(GesturePhase::Cancel), 0, "a drag that ends in a release is not cancelled: {seen:?}");
        assert_eq!(seen.first(), Some(&GesturePhase::Begin), "a drag begins first: {seen:?}");
        assert_eq!(seen.last(), Some(&GesturePhase::Commit), "a drag commits last: {seen:?}");

        let from = self.canvas().center();
        self.forget().drag_and_escape_from(from, by);
        let seen = self.gestures();
        assert_eq!(self.count(GesturePhase::Begin), 1, "the drag began once: {seen:?}");
        assert_eq!(self.count(GesturePhase::Cancel), 1, "Escape cancels the drag once: {seen:?}");
        assert_eq!(self.count(GesturePhase::Commit), 0, "the release after Escape does not commit: {seen:?}");
        self.escape();
        assert_eq!(self.count(GesturePhase::Cancel), 1, "a second Escape cancels nothing: {:?}", self.gestures());
    }

    fn check_command_form(&self) {
        match self.tool.borrow().command_form() {
            ToolPath::Command(text) => assert!(!text.trim().is_empty(), "{}: the command form names its command", self.label),
            ToolPath::Freehand => match freehand_entry(self.label) {
                Entry::Found => {}
                Entry::Missing => panic!(
                    "{}: a freehand tool needs a `{FREEHAND_RULE}` entry named \"{}\" in design/exceptions/<crate>/<family>.toml",
                    self.label, self.label
                ),
                Entry::Unreadable => {
                    eprintln!("{}: freehand; design/exceptions was not readable, so its exception entry was not checked", self.label)
                }
            },
            _ => panic!("{}: the command form is neither a command nor freehand", self.label),
        }
    }
}

enum Entry {
    Found,
    Missing,
    Unreadable,
}

/// Looks for the `FREEHAND_RULE` entry named `name` under the `design/exceptions` of the workspace that holds the
/// test package.
fn freehand_entry(name: &str) -> Entry {
    let Some(root) = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .and_then(|dir| dir.ancestors().map(|dir| dir.join("design").join("exceptions")).find(|dir| dir.is_dir()))
    else {
        return Entry::Unreadable;
    };
    let mut files = Vec::new();
    toml_files(&root, &mut files);
    let found = files.iter().filter_map(|file| std::fs::read_to_string(file).ok()).filter_map(|text| text.parse::<toml::Table>().ok()).any(|table| {
        table.get("exception").and_then(|list| list.as_array()).is_some_and(|list| {
            list.iter().filter_map(|entry| entry.as_table()).any(|entry| {
                let text = |key: &str| entry.get(key).and_then(|value| value.as_str());
                text("kind") == Some("probe") && text("rule") == Some(FREEHAND_RULE) && text("name") == Some(name)
            })
        })
    });
    if found { Entry::Found } else { Entry::Missing }
}

fn toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            toml_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "toml") {
            out.push(path);
        }
    }
}
