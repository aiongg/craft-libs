//! `WatermarkEditor` and `OutputPathRow` in a harness.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{FileError, Files, Host, PickAnswer, PickId, PickKind, PickRequest, Picked};
use egui::accesskit::Role;
use egui::{Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::dialog::Anchor;
use crate::theme::{Base, install};

/// Records every request; answers are put in by the test.
#[derive(Default)]
struct StubFiles {
    state: Mutex<StubState>,
}

#[derive(Default)]
struct StubState {
    requests: Vec<PickRequest>,
    answered: Vec<PickAnswer>,
}

impl StubFiles {
    fn answer(&self, picked: Picked) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let id = PickId(state.requests.len() as u64);
        let request = state.requests[state.requests.len() - 1].clone();
        state.answered.push(PickAnswer { id, request, picked });
    }

    fn requests(&self) -> Vec<PickRequest> {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).requests.clone()
    }
}

impl Files for StubFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.requests.push(request);
        PickId(state.requests.len() as u64)
    }

    fn take_pick(&self) -> Option<PickAnswer> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.answered.is_empty() { None } else { Some(state.answered.remove(0)) }
    }

    fn return_pick(&self, answer: PickAnswer) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).answered.insert(0, answer);
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

fn builder<S>(files: &Arc<StubFiles>) -> HarnessBuilder<S> {
    let files = files.clone();
    HarnessBuilder::default().with_size(vec2(640.0, 560.0)).with_context_setup(move |ctx| {
        install(ctx, &Base::suite_default());
        Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).files(files.clone()).install(ctx);
    })
}

fn checked(node: &egui_kittest::Node<'_>) -> bool {
    node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

// Switching Style to Graphic shows the image path with Choose…; Position is the 3×3 anchor grid;
// Size, Opacity and the insets stay in their ranges.
#[test]
fn watermark_rows_follow_the_style_and_ranges() {
    let files = Arc::new(StubFiles::default());
    let mut harness: Harness<'_, Watermark> = builder(&files).build_ui_state(
        |ui, wm: &mut Watermark| {
            let _ = WatermarkEditor::new("wm", wm).show(ui);
        },
        Watermark { style: WatermarkStyle::Text("(c) me".into()), ..Watermark::default() },
    );
    harness.run();
    assert!(harness.query_by_role_and_label(Role::MultilineTextInput, "Text").is_some());
    assert!(harness.query_by_label("Choose…").is_none());
    assert_eq!(harness.get_all_by_role(Role::RadioButton).count(), 9);

    // Position: a click picks; the cell of the default is marked.
    assert!(checked(&harness.get_by_role_and_label(Role::RadioButton, "Bottom right")));
    harness.get_by_role_and_label(Role::RadioButton, "Top left").click();
    harness.run();
    assert_eq!(harness.state().position, Anchor::TopLeft);

    // Keyboard: the arrow keys move the choice between cells, Space picks.
    harness.get_by_role_and_label(Role::RadioButton, "Top left").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state().position, Anchor::Top);

    // Centred along x: the horizontal inset has no effect and is disabled.
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Inset X").accesskit_node().is_disabled());
    assert!(!harness.get_by_role_and_label(Role::SpinButton, "Inset Y").accesskit_node().is_disabled());

    // Typed values clamp to the ranges.
    for (label, typed, expected) in [("Size", "99", 15.0), ("Opacity", "1", 5.0), ("Inset Y", "90", 40.0)] {
        harness.get_by_role_and_label(Role::SpinButton, label).focus();
        harness.run();
        harness.event(egui::Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
        harness.event(egui::Event::Text(typed.into()));
        harness.key_press(Key::Enter);
        harness.run();
        let wm = harness.state();
        let got = match label {
            "Size" => wm.size,
            "Opacity" => wm.opacity,
            _ => wm.inset[1],
        };
        assert_eq!(got, expected, "{label}");
    }

    // Graphic: the image path and Choose… replace the text rows; the answer sets the path.
    harness.get_by_role_and_label(Role::ComboBox, "Style").click();
    harness.run();
    harness.get_by_label("Graphic").click();
    harness.run();
    assert!(matches!(harness.state().style, WatermarkStyle::Graphic(ref p) if p.is_empty()));
    assert!(harness.query_by_role_and_label(Role::MultilineTextInput, "Text").is_none());
    harness.get_by_label("Choose…").click();
    harness.run();
    let requests = files.requests();
    assert_eq!((requests.len(), requests[0].kind, requests[0].purpose.as_str()), (1, PickKind::Open, "watermark"));
    files.answer(Picked::Paths(vec![PathBuf::from("/tmp/logo.png")]));
    harness.run();
    harness.run();
    assert_eq!(harness.state().style, WatermarkStyle::Graphic("/tmp/logo.png".into()));

    // Back to Text: the text typed before comes back.
    harness.get_by_role_and_label(Role::ComboBox, "Style").click();
    harness.run();
    harness.get_by_label("Text").click();
    harness.run();
    assert_eq!(harness.state().style, WatermarkStyle::Text("(c) me".into()));
}

struct RowState {
    path: String,
    templates: bool,
}

fn row_harness(files: &Arc<StubFiles>, path: &str) -> Harness<'static, RowState> {
    builder(files).build_ui_state(
        |ui, state: &mut RowState| {
            let mut row = OutputPathRow::new("out", &state.path, PickRequest::save("render").suggested_extension("mov"));
            if state.templates {
                row = row.templates(vec![
                    ("[compName].[fileExtension]".into(), "Comp name".into()),
                    ("[projectName]_[compName].[fileExtension]".into(), "Project and comp".into()),
                ]);
            }
            if let Some(path) = row.show(ui).edit.committed() {
                state.path = path.clone();
            }
        },
        RowState { path: path.into(), templates: true },
    )
}

// Picking a template commits the path; a click on the file name queues the save picker and its
// answer commits; Alt-click edits the template, Esc restores it.
#[test]
fn output_path_row_templates_picker_and_alt_edit() {
    let files = Arc::new(StubFiles::default());
    let mut harness = row_harness(&files, "[compName].[fileExtension]");
    harness.run();

    harness.get_by_role_and_label(Role::ComboBox, "Output To").click();
    harness.run();
    harness.get_by_label("Project and comp").click();
    harness.run();
    assert_eq!(harness.state().path, "[projectName]_[compName].[fileExtension]");

    harness.get_by_role_and_label(Role::Link, "[projectName]_[compName].[fileExtension]").click();
    harness.run();
    let requests = files.requests();
    assert_eq!((requests.len(), requests[0].kind, requests[0].purpose.as_str()), (1, PickKind::Save, "render"));
    files.answer(Picked::Paths(vec![PathBuf::from("/out/shot.mov")]));
    harness.run();
    harness.run();
    assert_eq!(harness.state().path, "/out/shot.mov");

    // Alt-click edits in place; Esc leaves the path as it was.
    harness.get_by_role_and_label(Role::Link, "/out/shot.mov").click_modifiers(Modifiers::ALT);
    harness.run();
    harness.run();
    assert_eq!(files.requests().len(), 1);
    let field = harness.get_by_role_and_label(Role::TextInput, "Alt-click to edit the template");
    field.type_text("/out/other.mov");
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().path, "/out/shot.mov");
    assert!(harness.query_by_role_and_label(Role::Link, "/out/shot.mov").is_some());

    // Enter commits the edited template.
    harness.get_by_role_and_label(Role::Link, "/out/shot.mov").click_modifiers(Modifiers::ALT);
    harness.run();
    harness.run();
    harness.get_by_role_and_label(Role::TextInput, "Alt-click to edit the template").type_text("/out/other.mov");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().path, "/out/other.mov");
}
