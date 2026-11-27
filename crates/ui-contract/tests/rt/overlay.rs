//! Count trials of a component's popups on craft-ui-rt layers, through the real `Component` trait: opening a popup
//! runs only its opener and its content, and a press outside that dismisses it runs only the opener; a hover popup
//! waits for its delay with no frame; a popup's body memory outlasts a close; a modal blocks the content below it; a
//! later wake does not replace an earlier one; a fading popup takes no input and goes at its wake; a context opener
//! offers its menu on its own node; a drag preview follows the pointer with no run; a panel popup closes on a row
//! click; Space held lets a press through a modal to the canvas it keeps open. Relations only, no times.

use std::cell::Cell;
use std::rc::Rc;

use craft_ui_contract::contract::overlay::{Overlay, OverlayUi, Overlays as _, Trigger};
use craft_ui_contract::contract::{
    Common, Component, Container, ContainerProps, Cx, DragPreview, DragSource, MeasureCx, Mount, Props as _, Response, Role, Show as _,
    ShowContainer, Ui,
};
use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
use craft_ui_contract::style::StyleCx;
use craft_ui_platform_headless::{HeadlessPlatform, Input};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Key, Mods, Platform as _, Pos2, Rect, Runtime, Sense, StepReport, Vec2, WindowSpec};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::{Rig, test_tokens};

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
    #[key(skip)]
    trigger: Trigger,
}

/// What the menu reports to the trial.
struct Menu {
    open: Cell<bool>,
    rows: Cell<[Rect; 3]>,
    picked: Cell<Option<usize>>,
    /// A count the menu's body keeps in its memory, raised at every run of the body.
    seen: Cell<u32>,
}

impl Default for Menu {
    fn default() -> Menu {
        Menu { open: Cell::new(false), rows: Cell::new([Rect::NOTHING; 3]), picked: Cell::new(None), seen: Cell::new(0) }
    }
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
        popup.trigger(self.trigger, cx);
        self.menu.open.set(popup.is_open(cx));
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        let menu = self.menu.clone();
        ui.show(popup, Overlay::menu(), move |ui, body| {
            let seen = body.memory.shown::<u32>(body.id.with("seen"));
            *seen += 1;
            menu.seen.set(*seen);
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
    trigger: Trigger,
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let button = MenuButton { common: Common::new(), runs: app.runs.clone(), menu: app.menu.clone(), trigger: app.trigger }.label("Edit".to_owned());
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
    assert!(settled(rig, &mut |rig| step(rig, app, None)), "the rig did not settle");
}

/// Steps `rig` until a step runs nothing. The rig's clock stands still, so a step that only repaints (a fade reading
/// the frame time) moves it on by a frame first, as frames do in an app; a build wake waits for the clock.
fn settled(rig: &mut Rig<RecordingRenderer>, step: &mut dyn FnMut(&mut Rig<RecordingRenderer>) -> StepReport) -> bool {
    let mut fading = false;
    for _ in 0..16 {
        if fading {
            rig.driver.platform.advance(1.0 / 30.0);
        }
        let report = step(rig);
        if !report.ran {
            return true;
        }
        let f = &report.frame;
        fading = f.runs == 0 && f.scopes == 0 && f.built == 0 && f.painted > 0;
    }
    false
}

fn setup() -> (Rig<RecordingRenderer>, App) {
    setup_with(Trigger::Click)
}

fn setup_with(trigger: Trigger) -> (Rig<RecordingRenderer>, App) {
    let mut app = App { button: Rect::NOTHING, beside: Rect::NOTHING, runs: Rc::default(), menu: Rc::default(), trigger };
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
    assert!(settled(&mut rig, &mut |rig| toast_step(rig, &mut app)), "the rig did not settle");
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

#[test]
fn a_hover_popup_waits_for_its_delay_with_no_frame_and_opens_at_it() {
    use craft_ui_contract::contract::tooltip::DELAY;
    let (mut rig, mut app) = setup_with(Trigger::Hover);
    let over = app.button.center();
    step(&mut rig, &mut app, Some(Input::mouse_move(over)));
    settle(&mut rig, &mut app);
    assert!(!app.menu.open.get(), "the menu waits for the hover delay");
    let runs = app.runs.get();
    for _ in 0..3 {
        let idle = step(&mut rig, &mut app, None);
        assert!(!idle.ran, "the delay is a build wake, not a frame per step: {idle:?}");
    }
    rig.driver.platform.advance(DELAY / 2.0);
    step(&mut rig, &mut app, None);
    assert!(!app.menu.open.get(), "half the delay opens nothing");
    rig.driver.platform.advance(DELAY / 2.0 + 0.05);
    step(&mut rig, &mut app, None);
    settle(&mut rig, &mut app);
    assert!(app.menu.open.get(), "the menu opens when the delay ends");
    assert!(app.runs.get() <= runs + 3, "the opener runs at the deadline, not per frame: {} after {runs}", app.runs.get());
}

#[test]
fn a_popup_s_body_memory_outlasts_its_close_while_its_owner_shows() {
    let (mut rig, mut app) = setup();
    open(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let first = app.menu.seen.get();
    assert!(first >= 1, "the body ran");
    let [down, up] = Input::key_press(Key::Escape, Mods::NONE);
    step(&mut rig, &mut app, Some(down));
    step(&mut rig, &mut app, Some(up));
    settle(&mut rig, &mut app);
    assert!(!app.menu.open.get(), "Escape closed the menu");
    open(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    assert!(app.menu.seen.get() > first, "the reopened body found what it kept: {} after {first}", app.menu.seen.get());
}

/// A chip under a modal dialog; it records whether a click reached it.
struct Modal {
    under: Rect,
    under_clicked: bool,
    ok: Rect,
}

fn modal_ui(ui: &mut Ui<'_, '_>, app: &mut Modal) {
    use craft_ui_contract::contract::overlay::show_dialog;
    let under = Chip::new("Under").show(ui);
    app.under = under.rect;
    app.under_clicked |= under.clicked();
    let shown = show_dialog(ui, 7, &Overlay::modal(), |ui| Chip::new("OK").show(ui).rect);
    if let Ok(shown) = shown {
        app.ok = shown.value;
    }
}

fn modal_step(rig: &mut Rig<RecordingRenderer>, app: &mut Modal, input: Option<Input>) -> StepReport {
    if let Some(input) = input {
        let main = rig.main;
        rig.inject(main, input);
    }
    rig.step_with(&mut |_, cx| window_root_with(cx, WindowEnv::default(), |ui| modal_ui(ui, app)))
}

#[test]
fn a_modal_blocks_pointer_input_to_the_content_below_it() {
    let mut app = Modal { under: Rect::NOTHING, under_clicked: false, ok: Rect::NOTHING };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    for _ in 0..6 {
        if !modal_step(&mut rig, &mut app, None).ran {
            break;
        }
    }
    assert!(app.ok != Rect::NOTHING, "the modal shows its body");
    let at = app.under.center();
    assert!(!app.ok.contains(at), "the chip below lies outside the dialog's body");
    modal_step(&mut rig, &mut app, Some(Input::mouse_move(at)));
    let down = modal_step(&mut rig, &mut app, Some(Input::mouse_down(at)));
    let up = modal_step(&mut rig, &mut app, Some(Input::mouse_up(at)));
    assert!(!app.under_clicked, "the click below the modal reached nothing");
    assert!(down.frame.runs <= 1 && up.frame.runs <= 1, "no node below the modal runs for the click: {down:?} {up:?}");
}

/// A leaf that asks for a build in one second and then in five, in that order or the other: the earlier wake stands.
#[derive(Component)]
struct Waits {
    #[common]
    common: Common,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
    #[key(skip)]
    later_first: bool,
}

impl Component for Waits {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: WaitsStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(20.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
        let (soon, late) = (std::time::Duration::from_secs(1), std::time::Duration::from_secs(5));
        let order = if self.later_first { [late, soon] } else { [soon, late] };
        for after in order {
            cx.context().request_repaint_after(after);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_later_wake_does_not_replace_an_earlier_one() {
    for later_first in [false, true] {
        let runs = Rc::new(Cell::new(0));
        let mut rig = Rig::inline(Vec2::new(200.0, 100.0), 1.0, RecordingRenderer::default());
        let frame = |rig: &mut Rig<RecordingRenderer>| {
            let runs = runs.clone();
            rig.step_with(&mut |_, cx| {
                window_root_with(cx, WindowEnv::default(), |ui| {
                    Waits { common: Common::new(), runs: runs.clone(), later_first }.show(ui);
                })
            })
        };
        for _ in 0..6 {
            if !frame(&mut rig).ran {
                break;
            }
        }
        let before = runs.get();
        assert!(!frame(&mut rig).ran, "a waiting leaf runs no frame");
        rig.driver.platform.advance(1.1);
        frame(&mut rig);
        assert_eq!(runs.get(), before + 1, "the one-second wake ran the leaf once (later first: {later_first})");
    }
}

/// The nodes on the window's layers above the content: popups, menus, dialogs, toasts, the drag ghost.
fn layer_children(rig: &Rig<RecordingRenderer>) -> Vec<craft_ui_rt::NodeId> {
    let rt = rig.runtime();
    let Some(root) = rt.window_node(rig.main) else { return Vec::new() };
    rt.children(root).iter().skip(1).flat_map(|&layer| rt.children(layer).to_vec()).collect()
}

/// Runs `rig` past the end of every fade and every wake it asked for.
fn past_fades(rig: &mut Rig<RecordingRenderer>, step: &mut dyn FnMut(&mut Rig<RecordingRenderer>) -> StepReport) {
    rig.driver.platform.advance(1.0);
    assert!(settled(rig, step), "the rig did not settle");
}

/// Clicks at `at` through `frame`, which injects an input and steps.
fn click_through(rig: &mut Rig<RecordingRenderer>, frame: &mut dyn FnMut(&mut Rig<RecordingRenderer>, Option<Input>) -> StepReport, at: Pos2) {
    frame(rig, Some(Input::mouse_move(at)));
    frame(rig, Some(Input::mouse_down(at)));
    frame(rig, Some(Input::mouse_up(at)));
}

#[test]
fn a_fading_popup_takes_no_click_or_key_and_is_gone_after_its_wake() {
    let (mut rig, mut app) = setup();
    open(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let rows = app.menu.rows.get();
    let [down, up] = Input::key_press(Key::Escape, Mods::NONE);
    step(&mut rig, &mut app, Some(down));
    step(&mut rig, &mut app, Some(up));
    // The clock stands still, so the close fade is still running: settling would move the clock past it.
    step(&mut rig, &mut app, None);
    assert!(!app.menu.open.get(), "Escape closed the menu");
    let fading = layer_children(&rig);
    assert_eq!(fading.len(), 1, "the closed menu stays while it fades out");
    // No key focus stays in it, and Tab finds none of its rows.
    let focus_outside = |rig: &Rig<RecordingRenderer>| {
        let mut at = rig.runtime().focused(rig.main);
        while let Some(node) = at {
            assert!(!fading.contains(&node), "focus is not in the fading menu");
            at = rig.runtime().parent(node);
        }
    };
    focus_outside(&rig);
    for _ in 0..3 {
        let [down, up] = Input::key_press(Key::Tab, Mods::NONE);
        step(&mut rig, &mut app, Some(down));
        step(&mut rig, &mut app, Some(up));
        focus_outside(&rig);
    }
    // A click where a row shows reaches none of its rows. One more step, not a settle: settling moves the clock on,
    // past the close fade this inspects.
    click(&mut rig, &mut app, rows[1].center());
    step(&mut rig, &mut app, None);
    assert_eq!(app.menu.picked.get(), None, "the fading menu's rows took nothing");
    assert!(!app.menu.open.get(), "nothing opened it again");
    assert_eq!(layer_children(&rig), fading, "a click inside a fading menu does not dismiss it again or reopen it");
    let runs = app.runs.get();
    past_fades(&mut rig, &mut |rig| step(rig, &mut app, None));
    assert!(layer_children(&rig).is_empty(), "the fade's end woke the owner, which dropped the menu");
    assert_eq!(app.runs.get(), runs + 1, "the owner ran once, at the fade's end");
}

/// A container shown as a layer: the trials' panel popups and modal.
#[derive(Component)]
#[component(container, unregistered)]
struct Panel {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Panel {
    fn new(overlay: Overlay) -> Panel {
        Panel { common: Common::new(), props: ContainerProps::new().mount(Mount::Layer(overlay)) }
    }
}

impl Component for Panel {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PanelStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Panel {}

#[test]
fn a_context_opener_offers_the_context_menu_on_its_own_node() {
    let shown = Rc::new(Cell::new(0_u32));
    let mut platform = HeadlessPlatform::new();
    platform.set_access_active(true);
    let main = platform.create_window(&WindowSpec { size: Some(Vec2::new(400.0, 300.0)), ..WindowSpec::new("main") });
    let mut runtime = Runtime::new(test_tokens());
    runtime.set_inline_root(main);
    let mut rig = Rig::from_parts(runtime, platform, RecordingRenderer::default());
    let mut frame = |rig: &mut Rig<RecordingRenderer>| {
        let shown = shown.clone();
        rig.step_with(&mut |_, cx| {
            window_root_with(cx, WindowEnv::default(), |ui| {
                let target = Chip::new("Target").show(ui);
                let shown = shown.clone();
                let _ = ShowContainer::show(Panel::new(Overlay::menu().opened_by(&target, Trigger::Context)), ui, move |ui, _| {
                    shown.set(shown.get() + 1);
                    Chip::new("Rename").show(ui);
                });
            })
        })
    };
    past_fades(&mut rig, &mut frame);
    let (target, offers) = {
        let entry = rig.runtime().access_snapshot(main).into_iter().find(|e| e.node.label() == Some("Target")).expect("the target's access node");
        (entry.id, entry.node.supports_action(accesskit::Action::ShowContextMenu))
    };
    assert!(offers, "the opener's own node offers the context menu");
    assert_eq!(shown.get(), 0, "the menu waits for its request");
    rig.inject(main, Input::action(accesskit::Action::ShowContextMenu, target));
    past_fades(&mut rig, &mut frame);
    assert!(shown.get() >= 1, "the assistive request on the opener's node opened its menu");
}

/// A leaf that drags a payload with a badge beside the pointer; it counts its runs.
#[derive(Component)]
struct Source {
    #[common]
    common: Common,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
}

impl Component for Source {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SourceStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 40.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::DRAG
    }

    fn drag_source(&self) -> Option<DragSource> {
        Some(DragSource::new(3_u32, DragPreview::Badge("3 files".to_owned().into())))
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_drag_preview_follows_the_pointer_with_no_run_or_build() {
    let runs = Rc::new(Cell::new(0_u32));
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let frame = |rig: &mut Rig<RecordingRenderer>, input: Option<Input>| {
        if let Some(input) = input {
            let main = rig.main;
            rig.inject(main, input);
        }
        let (runs, rect) = (runs.clone(), rect.clone());
        rig.step_with(&mut |_, cx| {
            window_root_with(cx, WindowEnv::default(), |ui| rect.set(Source { common: Common::new(), runs: runs.clone() }.show(ui).rect))
        })
    };
    for _ in 0..6 {
        if !frame(&mut rig, None).ran {
            break;
        }
    }
    let from = rect.get().center();
    frame(&mut rig, Some(Input::mouse_move(from)));
    frame(&mut rig, Some(Input::mouse_down(from)));
    frame(&mut rig, Some(Input::mouse_move(from + Vec2::new(30.0, 0.0))));
    frame(&mut rig, None);
    assert_eq!(layer_children(&rig).len(), 1, "the drag shows its ghost on the drag layer");
    let before = runs.get();
    for at in 1..=4 {
        let moved = frame(&mut rig, Some(Input::mouse_move(from + Vec2::new(30.0 + 15.0 * at as f32, 10.0))));
        assert_eq!((moved.frame.runs, moved.frame.scopes, moved.frame.built), (0, 0, 0), "a move only moves the ghost: {moved:?}");
    }
    assert_eq!(runs.get(), before, "the source does not run while its ghost follows the pointer");
    frame(&mut rig, Some(Input::mouse_up(from + Vec2::new(90.0, 10.0))));
}

#[test]
fn a_panel_popup_stays_open_while_used_and_closes_on_a_row_click() {
    let rows = Rc::new(Cell::new([Rect::NOTHING; 2]));
    let picked = Rc::new(Cell::new(None::<usize>));
    let opener = Rc::new(Cell::new(Rect::NOTHING));
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let mut frame = |rig: &mut Rig<RecordingRenderer>, input: Option<Input>| {
        if let Some(input) = input {
            let main = rig.main;
            rig.inject(main, input);
        }
        let (rows, picked, opener) = (rows.clone(), picked.clone(), opener.clone());
        rig.step_with(&mut |_, cx| {
            window_root_with(cx, WindowEnv::default(), |ui| {
                let open = Chip::new("Open").show(ui);
                opener.set(open.rect);
                let (rows, picked) = (rows.clone(), picked.clone());
                let _ = ShowContainer::show(Panel::new(Overlay::menu().opened_by(&open, Trigger::Click)), ui, move |ui, _| {
                    let mut at = [Rect::NOTHING; 2];
                    for (row, name) in ["One", "Two"].iter().enumerate() {
                        let shown = Chip::new(name).show(ui);
                        at[row] = shown.rect;
                        if shown.clicked() {
                            picked.set(Some(row));
                        }
                    }
                    rows.set(at);
                });
            })
        })
    };
    past_fades(&mut rig, &mut |rig| frame(rig, None));
    click_through(&mut rig, &mut frame, opener.get().center());
    past_fades(&mut rig, &mut |rig| frame(rig, None));
    assert_eq!(layer_children(&rig).len(), 1, "the opener's click opened the panel");
    // Moving over its rows, and time passing, keep it open.
    frame(&mut rig, Some(Input::mouse_move(rows.get()[0].center())));
    past_fades(&mut rig, &mut |rig| frame(rig, None));
    assert_eq!(layer_children(&rig).len(), 1, "the panel stays open while the pointer is on it");
    click_through(&mut rig, &mut frame, rows.get()[1].center());
    assert_eq!(picked.get(), Some(1), "the row took the click");
    past_fades(&mut rig, &mut |rig| frame(rig, None));
    assert!(layer_children(&rig).is_empty(), "the row click closed the panel, and its fade ended");
}

/// A canvas under a modal; it counts the clicks that reach it.
#[derive(Component)]
struct Canvas {
    #[common]
    common: Common,
    #[key(skip)]
    clicks: Rc<Cell<u32>>,
}

impl Component for Canvas {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: CanvasStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(200.0, 150.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::DRAG
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.response.clicked() {
            self.clicks.set(self.clicks.get() + 1);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_space_held_press_pans_the_canvas_a_modal_keeps_open() {
    use craft_ui_contract::contract::Sampler as _;
    let clicks = Rc::new(Cell::new(0_u32));
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let mut frame = |rig: &mut Rig<RecordingRenderer>, input: Option<Input>| {
        if let Some(input) = input {
            let main = rig.main;
            rig.inject(main, input);
        }
        let (clicks, rect) = (clicks.clone(), rect.clone());
        rig.step_with(&mut |_, cx| {
            window_root_with(cx, WindowEnv::default(), |ui| {
                let canvas = Canvas { common: Common::new(), clicks: clicks.clone() }.show(ui);
                rect.set(canvas.rect);
                let id = canvas.id;
                let _ = ShowContainer::show(Panel::new(Overlay::modal()), ui, move |ui, body| {
                    body.keep_view_open(id);
                    Chip::new("OK").show(ui);
                });
            })
        })
    };
    past_fades(&mut rig, &mut |rig| frame(rig, None));
    let at = rect.get().min + Vec2::new(10.0, 10.0);
    click_through(&mut rig, &mut frame, at);
    assert_eq!(clicks.get(), 0, "a plain primary press stays blocked by the modal");
    let [space_down, space_up] = Input::key_press(Key::Space, Mods::NONE);
    frame(&mut rig, Some(space_down));
    click_through(&mut rig, &mut frame, at);
    frame(&mut rig, Some(space_up));
    assert_eq!(clicks.get(), 1, "with Space held the primary press reaches the canvas, for its pan");
}
