//! Count trials of the real leaf pipeline on craft-ui-rt: a leaf with a label, a face and a surface
//! (and the veil of a faded disabled one) keeps its outputs on hover and runs once on a click; a
//! fired command reruns only the node that takes it, and only once.

use std::cell::Cell;
use std::rc::Rc;

use craft_keymap::{Binding, Chord, CommandDef, Keymap, Scope, Scopes};
use craft_ui_contract::contract::{CommandSet, Commands as _, Common, Component, Cx, MeasureCx, Props as _, Response, Role, Show as _, Ui};
use craft_ui_contract::runtime::commands::CommandBoard;
use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
use craft_ui_contract::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Key, Mods, Rect, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

use crate::skeleton::{Chip, IconButton};

/// Two chips: one enabled, one disabled (faded, so it paints under its veil).
struct Chips {
    on: Rect,
    off: Rect,
    clicks: u32,
}

fn chips_ui(ui: &mut Ui<'_, '_>, app: &mut Chips) {
    let on = Chip::new("Chip").show(ui);
    app.on = on.rect;
    if on.clicked() {
        app.clicks += 1;
    }
    app.off = Chip::new("Off").enabled(false).show(ui).rect;
}

fn chips_step(rig: &mut Rig<RecordingRenderer>, app: &mut Chips, input: Option<Input>) -> StepReport {
    if let Some(input) = input {
        let main = rig.main;
        rig.inject(main, input);
    }
    rig.step_with(&mut |_, cx| window_root_with(cx, WindowEnv::default(), |ui| chips_ui(ui, app)))
}

fn settle<A>(rig: &mut Rig<RecordingRenderer>, app: &mut A, step: fn(&mut Rig<RecordingRenderer>, &mut A, Option<Input>) -> StepReport) {
    for _ in 0..6 {
        if !step(rig, app, None).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

#[test]
fn a_real_step_leaf_keeps_its_outputs_on_hover_and_runs_once_on_a_click() {
    let mut app = Chips { on: Rect::NOTHING, off: Rect::NOTHING, clicks: 0 };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    rig.driver.runtime.set_access_enabled(main, true);
    settle(&mut rig, &mut app, chips_step);
    let at = app.on.center();
    let hover = chips_step(&mut rig, &mut app, Some(Input::mouse_move(at)));
    assert_eq!((hover.frame.runs, hover.frame.scopes), (0, 0), "a hover runs no hook and no scope: {hover:?}");
    assert_eq!(hover.frame.painted, 1, "the chip swaps to its kept hover look: {hover:?}");
    let at = app.off.center();
    let faded = chips_step(&mut rig, &mut app, Some(Input::mouse_move(at)));
    assert_eq!(faded.frame.runs, 0, "{faded:?}");
    assert_eq!(faded.frame.painted, 1, "only the enabled chip leaves its hover look; the faded one keeps one look: {faded:?}");
    let at = app.on.center();
    chips_step(&mut rig, &mut app, Some(Input::mouse_move(at)));
    let down = chips_step(&mut rig, &mut app, Some(Input::mouse_down(at)));
    assert_eq!((down.frame.runs, down.frame.painted), (0, 1), "the press look is a kept look too: {down:?}");
    let up = chips_step(&mut rig, &mut app, Some(Input::mouse_up(at)));
    assert_eq!(app.clicks, 1, "the click reaches the app in the frame of the release");
    assert_eq!(up.frame.runs, 1, "only the clicked chip runs; the faded one is kept: {up:?}");
    assert_eq!(up.frame.access_nodes, 0, "the chip's run built the same access node: {up:?}");
}

/// The command set the taker takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "pipelineTrial")]
#[non_exhaustive]
enum Go {
    #[command(label = "Go")]
    Go,
}

/// A leaf that takes `Go` in its `update` and counts its runs and takes.
#[derive(Component)]
struct Taker {
    #[common]
    common: Common,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
    #[key(skip)]
    taken: Rc<Cell<u32>>,
}

impl Component for Taker {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TakerStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
        while cx.take_command::<Go>().is_some() {
            self.taken.set(self.taken.get() + 1);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Taking {
    board: CommandBoard,
    runs: Rc<Cell<u32>>,
    taken: Rc<Cell<u32>>,
}

fn taking_step(rig: &mut Rig<RecordingRenderer>, app: &mut Taking, input: Option<Input>) -> StepReport {
    if let Some(input) = input {
        let main = rig.main;
        rig.inject(main, input);
    }
    let env = WindowEnv { commands: Some(app.board.clone()), ..WindowEnv::default() };
    let (runs, taken) = (app.runs.clone(), app.taken.clone());
    rig.step_with(&mut |_, cx| {
        window_root_with(cx, env.clone(), |ui| {
            Taker { common: Common::new(), runs: runs.clone(), taken: taken.clone() }.show(ui);
            IconButton::new(craft_behavior::IconId::new("plus"), "Other").show(ui);
        })
    })
}

#[test]
fn a_fired_command_reruns_only_the_node_that_takes_it_and_only_once() {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let os = rig.driver.runtime.os();
    let command = Go::Go.id();
    let def = CommandDef::new(command, "Go", Scope::app()).with_default(Binding::new(Chord::new(Mods::CMD, Key::G)));
    let board = CommandBoard::new(Keymap::new(vec![def], Scopes::new(), os));
    let mut app = Taking { board, runs: Rc::new(Cell::new(0)), taken: Rc::new(Cell::new(0)) };
    settle(&mut rig, &mut app, taking_step);
    let runs = app.runs.get();
    let [down, up] = Input::key_press(Key::G, Mods::CMD);
    let fired = taking_step(&mut rig, &mut app, Some(down));
    taking_step(&mut rig, &mut app, Some(up));
    assert_eq!(app.taken.get(), 1, "the taker took the fired command once");
    assert_eq!(app.runs.get(), runs + 1, "the fire reran the taker");
    assert_eq!(fired.frame.runs, 1, "only the taker runs; its sibling is kept: {fired:?}");
    // A later run of the taker, for another reason, does not take the command again.
    let mut tokens = craft_ui_test_kit::rt::rig::test_tokens();
    tokens.color.state.hover = craft_theme::Rgba([1, 2, 3, 255]);
    rig.driver.runtime.set_tokens(tokens);
    taking_step(&mut rig, &mut app, None);
    assert!(app.runs.get() > runs + 1, "the token change reran the taker");
    assert_eq!(app.taken.get(), 1, "the consumed sequence holds: no second take");
}

/// Where `Labelled` draws its label: a part that senses nothing, which `finish` reads back.
const LABEL: craft_behavior::PartKey = craft_behavior::PartKey::new("label");

/// A leaf that declares its label rect as a part with no sense and reports it from `finish`, as a list row does.
#[derive(Component)]
struct Labelled {
    #[common]
    common: Common,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
}

struct LabelledOutput {
    response: Response,
    label: Option<Rect>,
}

impl craft_ui_contract::contract::Outcome for LabelledOutput {
    type Value = craft_ui_contract::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl Component for Labelled {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = LabelledOutput;

    fn style(_: &StyleCx<'_>, _: LabelledStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(120.0, 24.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut craft_ui_contract::contract::PartsCx<'_, Self>) {
        let label = Rect::from_min_max(rect.min + Vec2::new(30.0, 4.0), rect.max - Vec2::new(8.0, 4.0));
        let _ = cx.part(LABEL, label).sense(craft_behavior::Sense::empty());
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
    }

    fn finish(self, cx: Cx<'_, Self>) -> LabelledOutput {
        use craft_ui_contract::contract::Pointer as _;
        LabelledOutput { response: cx.response.clone(), label: cx.part_if_declared(LABEL).map(|part| part.rect) }
    }
}

struct Labelling {
    chip: Rect,
    clicks: u32,
    row: Rect,
    label: Option<Rect>,
    runs: Rc<Cell<u32>>,
}

fn labelling_step(rig: &mut Rig<RecordingRenderer>, app: &mut Labelling, input: Option<Input>) -> StepReport {
    if let Some(input) = input {
        let main = rig.main;
        rig.inject(main, input);
    }
    let runs = app.runs.clone();
    rig.step_with(&mut |_, cx| {
        window_root_with(cx, WindowEnv::default(), |ui| {
            let chip = Chip::new("Chip").show(ui);
            app.chip = chip.rect;
            app.clicks += u32::from(chip.clicked());
            let out = Labelled { common: Common::new(), runs: runs.clone() }.show(ui);
            (app.row, app.label) = (out.response.rect, out.label);
        })
    })
}

#[test]
fn a_kept_leaf_reports_the_part_rects_its_last_run_declared() {
    let mut app = Labelling { chip: Rect::NOTHING, clicks: 0, row: Rect::NOTHING, label: None, runs: Rc::new(Cell::new(0)) };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    settle(&mut rig, &mut app, labelling_step);
    let ran = app.runs.get();
    let row = app.row;
    let label = Rect::from_min_max(row.min + Vec2::new(30.0, 4.0), row.max - Vec2::new(8.0, 4.0));
    assert_eq!(app.label, Some(label), "a real run reports the rect its `parts` declared");
    let at = app.chip.center();
    labelling_step(&mut rig, &mut app, Some(Input::mouse_move(at)));
    labelling_step(&mut rig, &mut app, Some(Input::mouse_down(at)));
    app.label = None;
    labelling_step(&mut rig, &mut app, Some(Input::mouse_up(at)));
    assert_eq!(app.clicks, 1, "the click reran the window's build");
    assert_eq!(app.runs.get(), ran, "the leaf is kept");
    assert_eq!(app.label, Some(label), "the kept leaf's `finish` reads the rect its last run declared");
}
