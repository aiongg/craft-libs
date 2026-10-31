//! The LUT picker in a kittest harness: its rows, the wheel's previewed steps and Load 3D LUT….

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::accesskit::Role;
use egui::{Event, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{LutChoice, LutPicker};
use craft_cms::lutfile::{LutError, LutFile};
use craft_host::{AppInfo, FileError, Files, InlineJobs, Jobs, NoPickFiles, PickId, PickRequest, Picked};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

struct Host {
    value: LutChoice,
    folder: Option<PathBuf>,
    loaded: Vec<Result<LutFile, LutError>>,
    events: Vec<EditEvent<LutChoice>>,
}

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

/// A file service whose picker answers with `answer` at once and remembers what it was asked.
pub(super) struct Scripted {
    answer: Picked,
    pub(super) asked: Mutex<Vec<PickRequest>>,
    answered: Mutex<Vec<PickId>>,
    /// The files written through the service.
    pub(super) written: Mutex<Vec<(PathBuf, Vec<u8>)>>,
}

impl Scripted {
    pub(super) fn new(answer: Picked) -> Arc<Scripted> {
        Arc::new(Scripted { answer, asked: Mutex::new(Vec::new()), answered: Mutex::new(Vec::new()), written: Mutex::new(Vec::new()) })
    }
}

impl Files for Scripted {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut asked = self.asked.lock().unwrap();
        asked.push(request);
        let id = PickId(asked.len() as u64);
        self.answered.lock().unwrap().push(id);
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut answered = self.answered.lock().unwrap();
        let at = answered.iter().position(|a| *a == id)?;
        answered.remove(at);
        Some(self.answer.clone())
    }

    fn forget(&self, id: PickId) {
        self.answered.lock().unwrap().retain(|a| *a != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FileError> {
        Ok(std::fs::read(path)?)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), FileError> {
        self.written.lock().unwrap().push((path.to_owned(), bytes.to_vec()));
        Ok(())
    }

    fn last_folder(&self, _: &str) -> Option<PathBuf> {
        None
    }
}

fn picker(value: LutChoice, folder: Option<PathBuf>) -> Harness<'static, Host> {
    picker_on(value, folder, None)
}

/// The picker with `files` as the installed host's file service (no host without it).
fn picker_on(value: LutChoice, folder: Option<PathBuf>, files: Option<Arc<Scripted>>) -> Harness<'static, Host> {
    let mut builder = harness();
    if let Some(files) = files {
        builder = builder.with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(AppInfo { id: "test", name: "Test", version: "1" }).files(files.clone()).build().install(ctx);
        });
    }
    builder.build_ui_state(
        |ui, host: &mut Host| {
            let shown = LutPicker::new("lut", &host.value).folder(host.folder.clone()).show(ui);
            host.loaded.extend(shown.loaded);
            if let Some(event) = shown.edit.event {
                match &event {
                    EditEvent::Set { value, .. } | EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } => host.value = value.clone(),
                    EditEvent::Begin { value: Some(value), .. } => host.value = value.clone(),
                    _ => {}
                }
                host.events.push(event);
            }
        },
        Host { value, folder, loaded: Vec::new(), events: Vec::new() },
    )
}

fn options(harness: &Harness<'_, Host>) -> Vec<String> {
    harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect()
}

/// The rows are Load, None, the built-ins, the folder files, then the loaded LUT.
#[test]
fn rows_follow_photocraft() {
    let dir = std::env::temp_dir().join(format!("craft-lut-rows-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["b.cube", "A.3dl", "notes.txt", ".hidden.cube"] {
        std::fs::write(dir.join(name), "x").unwrap();
    }
    let mut harness = picker(LutChoice::Loaded { name: "Film".into(), size: 33 }, Some(dir.clone()));
    // The scan runs on a worker; wait for its files to show.
    for _ in 0..200 {
        harness.run_steps(2);
        harness.get_by_role(Role::ComboBox);
        if options_after_open(&mut harness).contains(&"b.cube".to_owned()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let rows = options_after_open(&mut harness);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        rows,
        [
            "Load 3D LUT…",
            "None",
            "Warm Filter",
            "Cool Filter",
            "Teal & Orange",
            "Bleach Bypass",
            "Faded Film",
            "Day for Night",
            "Mono High Contrast",
            "Cross Process",
            "A.3dl",
            "b.cube",
            "Film (33³)",
        ]
    );
}

fn options_after_open(harness: &mut Harness<'_, Host>) -> Vec<String> {
    if options(harness).is_empty() {
        harness.get_by_role(Role::ComboBox).click();
        harness.run_steps(2);
    }
    options(harness)
}

/// The wheel steps through None and the built-ins only, as one previewed gesture.
#[test]
fn the_wheel_previews_built_ins_only() {
    let mut harness = picker(LutChoice::None, None);
    harness.run_steps(2);
    let at = harness.get_by_role(Role::ComboBox).rect().center();
    harness.hover_at(at);
    harness.run_steps(2);
    for _ in 0..2 {
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -1.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
        });
        harness.run_steps(2);
    }
    let events = &harness.state().events;
    assert!(
        matches!(
            events[..],
            [EditEvent::Begin { value: Some(LutChoice::Builtin("warm")), .. }, EditEvent::Update { value: LutChoice::Builtin("cool"), .. }]
        ),
        "{events:?}"
    );
    // Rest: the gesture commits with the last step.
    std::thread::sleep(std::time::Duration::from_millis(900));
    harness.run_steps(3);
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Commit { value: LutChoice::Builtin("cool"), changed: true, .. })), "{events:?}");
}

/// Load 3D LUT… asks the file service for a LUT file (purpose "lut"), leaves the choice unchanged,
/// and hands the parsed file back in the response once the job has read it.
#[test]
fn load_picks_a_file_and_returns_its_lut() {
    let dir = std::env::temp_dir().join(format!("craft-lut-load-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("film.cube");
    std::fs::write(&path, craft_cms::lutfile::write_cube(&LutFile::identity(2).unwrap())).unwrap();
    let files = Scripted::new(Picked::Paths(vec![path.clone()]));
    let mut harness = picker_on(LutChoice::Builtin("warm"), None, Some(files.clone()));
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Load 3D LUT…").click();
    for _ in 0..200 {
        harness.run_steps(2);
        if !harness.state().loaded.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = std::fs::remove_dir_all(&dir);
    let asked = files.asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 1);
    assert_eq!((asked[0].purpose.as_str(), asked[0].filters[0].extensions.as_slice()), ("lut", ["cube", "3dl", "look"].map(String::from).as_slice()));
    let loaded = &harness.state().loaded;
    assert_eq!(loaded.len(), 1, "the lut is reported once");
    assert_eq!(loaded[0].as_ref().map(|lut| lut.size), Ok(2));
    assert_eq!(harness.state().value, LutChoice::Builtin("warm"));
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);
}

/// A file that is not a LUT comes back as the error; a cancelled picker reports nothing.
#[test]
fn a_bad_file_is_an_error_and_a_cancel_is_nothing() {
    let dir = std::env::temp_dir().join(format!("craft-lut-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.cube");
    std::fs::write(&path, "not a lut").unwrap();
    let mut harness = picker_on(LutChoice::None, None, Some(Scripted::new(Picked::Paths(vec![path]))));
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Load 3D LUT…").click();
    for _ in 0..200 {
        harness.run_steps(2);
        if !harness.state().loaded.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(harness.state().loaded[..], [Err(_)]), "{:?}", harness.state().loaded);

    let mut harness = picker_on(LutChoice::None, None, Some(Scripted::new(Picked::Cancelled)));
    harness.run();
    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Load 3D LUT…").click();
    harness.run_steps(10);
    assert!(harness.state().loaded.is_empty());
}

/// Runs each job at once and records its name.
#[derive(Default)]
struct NamedJobs(Mutex<Vec<String>>);

impl Jobs for NamedJobs {
    fn spawn(&self, name: &str, job: Box<dyn FnOnce() + Send>) {
        self.0.lock().unwrap().push(name.to_owned());
        InlineJobs.spawn(name, job);
    }
}

/// The folder scan is a job of the installed host, and its files show without a timer.
#[test]
fn the_folder_scan_runs_as_a_host_job() {
    let dir = std::env::temp_dir().join(format!("craft-lut-job-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.cube"), "x").unwrap();
    let jobs = Arc::new(NamedJobs::default());
    let installed = jobs.clone();
    let mut harness = harness()
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(AppInfo { id: "test", name: "Test", version: "1" })
                .files(Arc::new(NoPickFiles::default()))
                .jobs(installed.clone())
                .build()
                .install(ctx);
        })
        .build_ui_state(
            |ui, host: &mut Host| {
                let _ = LutPicker::new("lut", &host.value).folder(host.folder.clone()).show(ui);
            },
            Host { value: LutChoice::None, folder: Some(dir.clone()), loaded: Vec::new(), events: Vec::new() },
        );
    harness.run_steps(3);
    let rows = options_after_open(&mut harness);
    let _ = std::fs::remove_dir_all(&dir);
    let names = jobs.0.lock().unwrap().clone();
    assert!(!names.is_empty() && names.iter().all(|n| n == "lut-folder-scan"), "{names:?}");
    assert!(rows.contains(&"a.cube".to_owned()), "{rows:?}");
}
