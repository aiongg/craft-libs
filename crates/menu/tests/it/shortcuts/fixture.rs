//! The catalog the dialog tests use, and a rig that shows the dialog pass by pass.

use std::cell::RefCell;
use std::rc::Rc;

use craft_keymap::{Bindable, Binding, CommandRef, ImportOptions, Keymap, Kind, Overrides, Platform, ProfileError, Profiles, Scope, Scopes};
use craft_menu::shortcuts::{DialogState, Options, Outcome, ShortcutsDialog};
use craft_ui::testing::{Fixture, Target};
use egui::{Key, Modifiers, Ui};

fn binding(text: &str) -> Binding {
    Binding::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// A small app: File and Object commands, two tools on `M`, a Timeline panel command and a Text context command.
pub(crate) fn keymap(platform: Platform) -> Keymap {
    let tool = Kind::Tool { group: Some("Marquee".into()) };
    let path = |p: &[&str]| p.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let timeline = Scope::new("Timeline").unwrap_or_else(Scope::app);
    let catalog = vec![
        Bindable::new("file.save", "Save", Scope::app()).with_path(path(&["File", "Save"])).with_default(binding("Cmd+S")),
        Bindable::new("file.saveAs", "Save As…", Scope::app()).with_path(path(&["File", "Save As…"])).with_default(binding("Cmd+Shift+S")),
        Bindable::new("file.export", "Export…", Scope::app()).with_path(path(&["File", "Export…"])).with_default(binding("Cmd+E")),
        Bindable::new("object.front", "Bring to Front", Scope::app()).with_path(path(&["Object", "Arrange", "Bring to Front"])),
        Bindable::new("tool.marquee", "Marquee Tool", Scope::app()).with_kind(tool.clone()).with_default(binding("M")),
        Bindable::new("tool.ellipse", "Elliptical Marquee Tool", Scope::app()).with_kind(tool).with_default(binding("M")),
        Bindable::new("timeline.loop", "Loop", timeline).with_default(binding("L")),
        Bindable::new("type.bold", "Bold", Scope::app()).with_default(binding("Text: Cmd+B")),
    ];
    Keymap::new(catalog, Scopes::new().with_panel("Timeline").with_context("Text"), platform)
}

/// `km` with a command whose label and menu path are far too long for a row.
pub(crate) fn with_long_label(km: Keymap) -> Keymap {
    let long = "Distribute Selected Objects Evenly Along the Active Guide Path and Keep Their Spacing".to_string();
    let mut catalog = km.catalog().to_vec();
    catalog.insert(
        3,
        Bindable::new("object.distribute", long.as_str(), Scope::app())
            .with_path(vec!["Object".into(), "Arrange and Distribute Along a Path".into(), long.clone()])
            .with_default(binding("Cmd+Alt+Shift+D"))
            .with_default(binding("Ctrl+Alt+Shift+F12")),
    );
    Keymap::new(catalog, Scopes::new().with_panel("Timeline").with_context("Text"), km.platform())
}

/// The options a test starts from.
pub(crate) fn options() -> Options {
    Options::default()
}

/// The built-in sets of the tests' profiles.
pub(crate) fn builtins() -> Vec<(&'static str, Overrides)> {
    vec![("Fixture Defaults", Overrides::new())]
}

/// A step the dialog takes between passes, with the pass's `Ui` to read the words from.
type Job = Box<dyn for<'a> FnOnce(ShortcutsDialog<'a>, &Ui, &mut DialogState)>;

/// What the dialog edits, and what it reported.
pub(crate) struct Shared {
    pub km: Keymap,
    pub state: DialogState,
    pub options: Options,
    pub sets: Option<(Profiles, Vec<(&'static str, Overrides)>)>,
    pub outcomes: Vec<Outcome>,
    jobs: Vec<Job>,
}

/// The dialog shown in a fixture, one pass per frame.
pub(crate) struct Rig {
    pub fixture: Fixture,
    pub shared: Rc<RefCell<Shared>>,
}

impl Rig {
    /// A dialog over `km` with `options`, and named sets when `profiles` is given.
    pub(crate) fn new(km: Keymap, options: Options, profiles: Option<Profiles>) -> Rig {
        Rig::with_builtins(km, options, profiles.map(|p| (p, builtins())))
    }

    /// Like [`Rig::new`], with the built-in sets given too.
    pub(crate) fn with_builtins(km: Keymap, options: Options, sets: Option<(Profiles, Vec<(&'static str, Overrides)>)>) -> Rig {
        let shared = Rc::new(RefCell::new(Shared { km, state: DialogState::new(), options, sets, outcomes: Vec::new(), jobs: Vec::new() }));
        let pass = shared.clone();
        let mut fixture = Fixture::new(move |ui| {
            let mut guard = pass.borrow_mut();
            let Shared { km, state, options, sets, outcomes, jobs } = &mut *guard;
            for job in std::mem::take(jobs) {
                let mut dialog = ShortcutsDialog::new(km).options(options);
                if let Some((p, b)) = sets.as_mut() {
                    dialog = dialog.sets(p, b);
                }
                job(dialog, ui, state);
            }
            let mut dialog = ShortcutsDialog::new(km).options(options);
            if let Some((p, b)) = sets.as_mut() {
                dialog = dialog.sets(p, b);
            }
            outcomes.extend(dialog.show(ui, state).outcome);
        });
        fixture.room(egui::vec2(1000.0, 700.0));
        fixture.settle();
        Rig { fixture, shared }
    }

    /// A Mac dialog over the fixture catalog with default options and no sets.
    pub(crate) fn mac() -> Rig {
        Rig::new(keymap(Platform::Mac), options(), None)
    }

    /// A Mac dialog over the fixture catalog with sets.
    pub(crate) fn mac_with_sets(profiles: Profiles) -> Rig {
        Rig::new(keymap(Platform::Mac), options(), Some(profiles))
    }

    /// The control with this label; panics when there is none.
    pub(crate) fn find(&self, label: &str) -> Target {
        self.fixture.by_label(label).unwrap_or_else(|| panic!("no control labelled {label:?}"))
    }

    pub(crate) fn has(&self, label: &str) -> bool {
        self.fixture.by_label(label).is_some()
    }

    pub(crate) fn click(&mut self, label: &str) {
        let target = self.find(label);
        self.fixture.click(&target);
        self.fixture.idle(3);
    }

    pub(crate) fn press(&mut self, modifiers: Modifiers, key: Key) {
        self.fixture.key(key, modifiers);
        self.fixture.idle(3);
    }

    /// The bindings of a command as text.
    pub(crate) fn bindings(&self, id: &str) -> Vec<String> {
        self.shared.borrow().km.bindings(&CommandRef::from(id)).iter().map(ToString::to_string).collect()
    }

    pub(crate) fn outcomes(&self) -> Vec<Outcome> {
        self.shared.borrow().outcomes.clone()
    }

    pub(crate) fn has_outcome(&self, wanted: impl Fn(&Outcome) -> bool) -> bool {
        self.shared.borrow().outcomes.iter().any(wanted)
    }

    pub(crate) fn active_set(&self) -> Option<String> {
        self.shared.borrow().sets.as_ref().map(|(p, _)| p.active().to_string())
    }

    /// Runs `job` with the dialog in the next pass and returns what it returns.
    pub(crate) fn import_json(&mut self, json: &str) -> Result<String, ProfileError> {
        let result = Rc::new(RefCell::new(None));
        let (out, text) = (result.clone(), json.to_owned());
        let job: Job = Box::new(move |dialog, ui, state| *out.borrow_mut() = Some(dialog.import_json(ui, state, &text)));
        self.shared.borrow_mut().jobs.push(job);
        self.fixture.idle(2);
        result.take().unwrap_or_else(|| panic!("the job did not run"))
    }

    /// Opens the review of `set` in the next pass.
    pub(crate) fn review_import(&mut self, set: craft_keymap::ForeignSet, options: ImportOptions) {
        let cell = RefCell::new(Some((set, options)));
        let job: Job = Box::new(move |dialog, ui, state| {
            if let Some((set, options)) = cell.take() {
                dialog.review_import(ui, state, set, options);
            }
        });
        self.shared.borrow_mut().jobs.push(job);
        self.fixture.idle(3);
    }
}
