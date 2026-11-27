//! Count trials of a component's popups on craft-ui-rt layers, through the real `Component` trait: opening a popup
//! runs only its opener and its content, and a press outside that dismisses it runs only the opener. Relations only,
//! no times.

use std::cell::Cell;
use std::rc::Rc;

use craft_ui_contract::contract::overlay::{Overlay, OverlayUi, Overlays as _, Trigger};
use craft_ui_contract::contract::{Common, Component, Cx, MeasureCx, Props as _, Response, Role, Show as _, Ui};
use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
use craft_ui_contract::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Key, Mods, Pos2, Rect, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

use crate::skeleton::Chip;

const ROWS: [&str; 3] = ["Cut", "Copy", "Paste"];

/// A button whose click toggles a menu of `ROWS`; it counts its runs and the rows its menu showed.
#[derive(Component)]
struct MenuButton {
    #[common]
    common: Common,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
    #[key(skip)]
    menu: Rc<Menu>,
}

/// What the menu reports to the trial.
#[derive(Default)]
struct Menu {
    open: Cell<bool>,
    rows: Cell<[Rect; 3]>,
    picked: Cell<Option<usize>>,
}

impl Component for MenuButton {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: MenuButtonStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(60.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
        let popup = cx.popup("menu");
        popup.trigger(Trigger::Click, cx);
        self.menu.open.set(popup.is_open(cx));
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        let menu = self.menu.clone();
        ui.show(popup, Overlay::menu(), move |ui, _| {
            let mut rows = [Rect::NOTHING; 3];
            for (at, row) in ROWS.iter().enumerate() {
                let shown = Chip::new(row).show(ui);
                rows[at] = shown.rect;
                if shown.clicked() {
                    menu.picked.set(Some(at));
                }
            }
            menu.rows.set(rows);
        });
        if ui.closing(popup).is_some() {
            self.menu.open.set(false);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The menu button and an unrelated chip beside it.
struct App {
    button: Rect,
    beside: Rect,
    runs: Rc<Cell<u32>>,
    menu: Rc<Menu>,
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let button = MenuButton { common: Common::new(), runs: app.runs.clone(), menu: app.menu.clone() }.label("Edit".to_owned());
    app.button = button.show(ui).rect;
    app.beside = Chip::new("Beside").show(ui).rect;
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Option<Input>) -> StepReport {
    if let Some(input) = input {
        let main = rig.main;
        rig.inject(main, input);
    }
    rig.step_with(&mut |_, cx| window_root_with(cx, WindowEnv::default(), |ui| app_ui(ui, app)))
}

fn settle(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    for _ in 0..6 {
        if !step(rig, app, None).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn setup() -> (Rig<RecordingRenderer>, App) {
    let mut app = App { button: Rect::NOTHING, beside: Rect::NOTHING, runs: Rc::default(), menu: Rc::default() };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    settle(&mut rig, &mut app);
    (rig, app)
}

/// Clicks at `at`; the report of the release.
fn click(rig: &mut Rig<RecordingRenderer>, app: &mut App, at: Pos2) -> StepReport {
    step(rig, app, Some(Input::mouse_move(at)));
    step(rig, app, Some(Input::mouse_down(at)));
    step(rig, app, Some(Input::mouse_up(at)))
}

fn open(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    let at = app.button.center();
    let up = click(rig, app, at);
    assert!(app.menu.open.get(), "the click opened the menu");
    up
}

#[test]
fn opening_a_popup_runs_only_its_opener_and_its_content() {
    let (mut rig, mut app) = setup();
    let runs = app.runs.get();
    let up = open(&mut rig, &mut app);
    assert_eq!(app.runs.get(), runs + 1, "the opener runs once for its click");
    assert_eq!(up.frame.runs, 1 + ROWS.len() as u32, "the opener and the menu's rows run; the chip beside is kept: {up:?}");
    let rows = app.menu.rows.get();
    assert!(rows[0].min.y >= app.button.max.y, "the menu hangs below its opener: {:?} under {:?}", rows[0], app.button);
    settle(&mut rig, &mut app);
}

#[test]
fn a_dismissing_press_outside_runs_only_the_popup_s_owner() {
    let (mut rig, mut app) = setup();
    open(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let outside = Pos2::new(390.0, 290.0);
    step(&mut rig, &mut app, Some(Input::mouse_move(outside)));
    let down = step(&mut rig, &mut app, Some(Input::mouse_down(outside)));
    assert!(!app.menu.open.get(), "the press outside closed the menu");
    assert_eq!(down.frame.runs, 1, "only the menu's owner runs to close it; no row, nothing unrelated: {down:?}");
    let up = step(&mut rig, &mut app, Some(Input::mouse_up(outside)));
    assert_eq!(up.frame.runs, 0, "the release outside reaches nothing: {up:?}");
}

#[test]
fn escape_closes_the_menu_and_a_row_click_reaches_its_row() {
    let (mut rig, mut app) = setup();
    open(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let [down, up] = Input::key_press(Key::Escape, Mods::NONE);
    let esc = step(&mut rig, &mut app, Some(down));
    step(&mut rig, &mut app, Some(up));
    assert!(!app.menu.open.get(), "Escape closed the menu");
    assert_eq!(esc.frame.runs, 1, "only the owner runs: {esc:?}");
    open(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let row = app.menu.rows.get()[1].center();
    click(&mut rig, &mut app, row);
    settle(&mut rig, &mut app);
    assert_eq!(app.menu.picked.get(), Some(1), "the row took the click");
}
