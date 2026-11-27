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

/// A window root that shows one toast queue; it posts one card with a one-second timeout on its first run and counts
/// its own runs and the cards it drew.
struct Toasts {
    queue: craft_ui_contract::contract::toast::ToastQueue,
    posted: Option<craft_ui_contract::runtime::toast::ToastId>,
    root_runs: u32,
    drawn: u32,
}

fn toast_ui(ui: &mut Ui<'_, '_>, app: &mut Toasts) {
    use craft_ui_contract::contract::toast::{CardOutcome, ToastHost};
    use craft_ui_contract::runtime::toast::Toast;
    app.root_runs += 1;
    if app.posted.is_none() {
        app.posted = Some(app.queue.post(Toast::new("Saved").timeout(Some(1.0))));
    }
    let body = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
    let drawn = &mut app.drawn;
    ToastHost::show(ui, &mut app.queue, body, &mut |ui, _, salt| {
        *drawn += 1;
        Chip::new("Saved").id_source(craft_ui_contract::contract::IdSource::Salt(salt)).show(ui);
        CardOutcome::new(false, None)
    });
}

fn toast_step(rig: &mut Rig<RecordingRenderer>, app: &mut Toasts) -> StepReport {
    rig.step_with(&mut |_, cx| window_root_with(cx, WindowEnv::default(), |ui| toast_ui(ui, app)))
}

#[test]
fn a_waiting_toast_runs_nothing_until_its_timeout_ends_it() {
    let mut app = Toasts { queue: Default::default(), posted: None, root_runs: 0, drawn: 0 };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    for _ in 0..6 {
        if !toast_step(&mut rig, &mut app).ran {
            break;
        }
    }
    let id = app.posted.expect("the root posted its card");
    assert!(app.drawn >= 1, "the card was drawn");
    let (runs, drawn) = (app.root_runs, app.drawn);
    for _ in 0..3 {
        let idle = toast_step(&mut rig, &mut app);
        assert!(!idle.ran, "a waiting card's timer is a build wake, not a frame per step: {idle:?}");
    }
    assert_eq!((app.root_runs, app.drawn), (runs, drawn), "nothing ran while the card waited");
    rig.driver.platform.advance(1.1);
    toast_step(&mut rig, &mut app);
    assert_eq!(app.root_runs, runs + 1, "the deadline woke the showing node once");
    assert_eq!(app.queue.take_end(id), Some(craft_ui_contract::runtime::toast::ToastEnd::Expired), "the timeout ended the card");
    assert!(app.queue.is_empty());
}
