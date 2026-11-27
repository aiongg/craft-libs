//! The app host over the headless platform: a bound key runs `App::logic` at event time and its `request_build`
//! builds the root in the same turn; a command fired from inside `App::ui` runs after that turn; an idle turn builds
//! nothing; `App::exit` runs once, when the last window has closed.

use std::cell::Cell;
use std::rc::Rc;

use craft_behavior::placement::Defaults;
use craft_keymap::{Binding, Chord, CommandDef, Keymap, Scope, Scopes};
use craft_ui_platform_headless::HeadlessPlatform;
use craft_ui_rt::{CloseReason, Driver, Key, Mods, RecordingRenderer, StepReport};
use craft_ui_shell::contract::Ui;
use craft_ui_shell::runtime::commands::{FireSource, Fired};
use craft_ui_shell::shell::app::{App, AppSpec, Host, LogicCx, WindowId, start};

const GRID: &str = "view.grid";

#[derive(Default)]
struct Seen {
    logic: Cell<u32>,
    builds: Cell<u32>,
    /// The count the last build showed.
    shown: Cell<u32>,
    exits: Cell<u32>,
    /// Fire the command once from inside the next build.
    fire: Cell<bool>,
}

struct Counter {
    /// App state outside cells: a build sees it only through `request_build`.
    count: u32,
    seen: Rc<Seen>,
}

impl App for Counter {
    fn logic(&mut self, fired: &Fired, cx: &mut LogicCx<'_>) {
        if fired.command.id == GRID {
            self.count += 1;
            self.seen.logic.set(self.seen.logic.get() + 1);
            cx.request_build(WindowId::ROOT);
        }
    }

    fn ui(&mut self, _: WindowId, ui: &mut Ui<'_, '_>) {
        self.seen.builds.set(self.seen.builds.get() + 1);
        self.seen.shown.set(self.count);
        if self.seen.fire.take() {
            ui.fire(GRID, FireSource::Key);
        }
    }

    fn exit(&mut self) {
        self.seen.exits.set(self.seen.exits.get() + 1);
    }
}

type Running = (Driver<HeadlessPlatform, RecordingRenderer>, Host<Counter>);

fn step((driver, host): &mut Running) -> StepReport {
    let now = driver.platform.advance(1.0 / 60.0);
    host.step(driver, now)
}

fn settle(running: &mut Running) {
    for _ in 0..20 {
        if !step(running).ran {
            return;
        }
    }
    panic!("the app does not settle");
}

fn running(seen: &Rc<Seen>) -> Running {
    let spec = AppSpec::new("host-trial", "Host", Defaults::new([320.0, 200.0], [160.0, 100.0]));
    let os = spec.shell_info.os;
    let def = CommandDef::new(GRID, "Show Grid", Scope::app()).with_default(Binding::new(Chord::new(Mods::CMD, Key::G)));
    let spec = spec.keymap(Keymap::new(vec![def], Scopes::new(), os));
    let seen = seen.clone();
    let started = start(spec, HeadlessPlatform::new(), RecordingRenderer::default(), move |_| Counter { count: 0, seen });
    started.unwrap_or_else(|error| panic!("the host starts: {error}"))
}

#[test]
fn key_runs_logic_and_builds_in_the_same_turn() {
    let seen = Rc::new(Seen::default());
    let mut running = running(&seen);
    settle(&mut running);
    let builds = seen.builds.get();
    assert!(!step(&mut running).ran, "an idle turn runs no frame");
    assert_eq!(seen.builds.get(), builds, "an idle turn builds nothing");

    running.0.platform.key_press(WindowId::ROOT, Key::G, Mods::CMD);
    step(&mut running);
    assert_eq!(seen.logic.get(), 1, "the bound key runs App::logic once");
    assert_eq!(seen.shown.get(), 1, "request_build builds the root in the turn of the key");
}

#[test]
fn a_fire_from_the_build_runs_after_the_turn() {
    let seen = Rc::new(Seen::default());
    let mut running = running(&seen);
    settle(&mut running);
    seen.fire.set(true);
    // The flag is outside cells: build the root for it to be read.
    running.0.runtime.mark_root(WindowId::ROOT);
    step(&mut running);
    assert_eq!(seen.logic.get(), 1, "the fire from the build reached App::logic after the turn");
    settle(&mut running);
    assert_eq!(seen.shown.get(), 1, "its request_build built the root in a later turn");
}

#[test]
fn exit_runs_once_when_the_last_window_closes() {
    let seen = Rc::new(Seen::default());
    let mut running = running(&seen);
    settle(&mut running);
    running.0.runtime.request_close(WindowId::ROOT, CloseReason::Window);
    for _ in 0..5 {
        step(&mut running);
    }
    assert_eq!(seen.exits.get(), 1, "App::exit runs once after the last window closed");
}
