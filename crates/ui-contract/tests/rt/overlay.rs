//! Count trials of overlays on craft-ui-rt layers: popups, tooltips, drags and modals run only what
//! they must. Relations only, no times.

use craft_behavior::IconId;
use craft_ui_contract::contract::overlay::Overlay;
use craft_ui_contract::contract::rt::skeleton::{IconButton, Stack};
use craft_ui_contract::contract::rt::{
    Component, ComponentKey, Container, Cx, DragSource, MeasureCx, MemoView, NodeInput, Outcome, OverlayUi, PopupHandle, Response, TOOLTIP_DELAY,
    ToastHost, ToastQueue, Ui, show_container, show_dialog, show_leaf, window_root,
};
use craft_ui_contract::contract::{ContainerProps, MemoKey, MemoProps, NoValue, Pad};
use craft_ui_contract::runtime::drag::DragPreview;
use craft_ui_contract::runtime::overlay::CloseReason;
use craft_ui_contract::runtime::toast::{Toast, ToastEnd, ToastId};
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Key, Mods, Pos2, Rect, Sense, StepReport, Vec2, WindowId, WindowSpec};
use craft_ui_test_kit::rt::Rig;

/// One simulated 60 Hz frame, in seconds.
const FRAME: f64 = 1.0 / 60.0;
/// Past the theme's hover fade, which paint reads from the clock: a step this much later ends the
/// pointer looks' fades.
const FADE_END: f64 = 0.25;

const ROWS: [&str; 3] = ["Cut", "Copy", "Paste"];

fn list() -> PopupHandle {
    PopupHandle::new("list")
}

/// What `PopupButton::show` returns.
struct PopupOut {
    response: Response,
    /// The popup's open state and its rows' rects, when its overlay pass ran.
    open: Option<bool>,
    rows: Option<Vec<Rect>>,
    picked: Option<usize>,
}

impl Outcome for PopupOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A button that toggles a popover listing `rows`; a row's click picks it and closes the popover.
struct PopupButton {
    toggle: bool,
    open: Option<bool>,
    rows: Option<Vec<Rect>>,
    picked: Option<usize>,
}

impl PopupButton {
    fn show(self, ui: &mut Ui<'_, '_>) -> PopupOut {
        show_leaf(self, ui)
    }
}

impl ComponentKey for PopupButton {
    fn key(&self, key: &mut MemoKey) {
        for row in ROWS {
            row.key(key);
        }
    }
}

impl Component for PopupButton {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = PopupOut;

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.toggle = cx.clicked();
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_, '_>) {
        if self.toggle {
            ui.toggle(list());
        }
        let shown = ui.show(list(), &Overlay::popover(), |ui| {
            let mut rects = Vec::new();
            let mut picked = None;
            for (i, row) in ROWS.iter().enumerate() {
                let out = IconButton::new(IconId::new("row"), row).show(ui);
                rects.push(out.rect);
                if out.clicked() {
                    picked = Some(i);
                }
            }
            (rects, picked)
        });
        if let Some((rects, picked)) = shown {
            self.rows = Some(rects);
            if picked.is_some() {
                self.picked = picked;
                ui.close(list(), CloseReason::Activated);
            }
        }
        self.open = Some(ui.is_open(list()));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> PopupOut {
        PopupOut { response: cx.response.clone(), open: self.open, rows: self.rows, picked: self.picked }
    }
}

/// A button with a tooltip.
struct TipButton;

impl Component for TipButton {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = Response;

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn tooltip(&self) -> Option<String> {
        Some("Brush size".into())
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl ComponentKey for TipButton {
    fn key(&self, _: &mut MemoKey) {}
}

/// A chip that drags a number, with a badge beside the pointer.
struct DragChip;

/// What `DragChip` returns: whether its run heard its drag end.
struct DragOut {
    response: Response,
    ended: bool,
}

impl Outcome for DragOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl Component for DragChip {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = DragOut;

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::DRAG
    }

    fn drag_source(&self) -> Option<DragSource> {
        Some(DragSource::new(7_u32, DragPreview::Badge("1 layer".into())))
    }

    fn finish(self, cx: &Cx<'_, Self>) -> DragOut {
        DragOut { response: cx.response.clone(), ended: cx.input().iter().any(|i| matches!(i, NodeInput::DragEnd(_))) }
    }
}

impl ComponentKey for DragChip {
    fn key(&self, _: &mut MemoKey) {}
}

struct App {
    other: Rect,
    other_clicks: u32,
    opener: Rect,
    popup_open: bool,
    rows: Vec<Rect>,
    picked: Option<usize>,
    view_runs: u32,
    tip: Rect,
    chip: Rect,
    drag_ended: bool,
    modal_open: bool,
    ok: Rect,
    ok_clicks: u32,
    second: Option<WindowId>,
    second_button: Rect,
    second_clicks: u32,
    /// A click on Other posts a toast that stays this many seconds.
    toast_on_other: Option<f32>,
    toasts: ToastQueue,
    toast: Option<ToastId>,
}

impl Default for App {
    fn default() -> App {
        App {
            other: Rect::NOTHING,
            other_clicks: 0,
            opener: Rect::NOTHING,
            popup_open: false,
            rows: Vec::new(),
            picked: None,
            view_runs: 0,
            tip: Rect::NOTHING,
            chip: Rect::NOTHING,
            drag_ended: false,
            modal_open: false,
            ok: Rect::NOTHING,
            ok_clicks: 0,
            second: None,
            second_button: Rect::NOTHING,
            second_clicks: 0,
            toast_on_other: None,
            toasts: ToastQueue::new(),
            toast: None,
        }
    }
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let other = IconButton::new(IconId::new("plus"), "Other").show(ui);
    app.other = other.rect;
    app.other_clicks += u32::from(other.clicked());
    if let (true, Some(timeout)) = (other.clicked(), app.toast_on_other) {
        app.toast = Some(app.toasts.post(Toast::new("Saved").timeout(Some(timeout))));
    }
    let out = PopupButton { toggle: false, open: None, rows: None, picked: None }.show(ui);
    app.opener = out.response.rect;
    if let Some(open) = out.open {
        app.popup_open = open;
    }
    if let Some(rows) = out.rows {
        app.rows = rows;
    }
    app.picked = out.picked.or(app.picked);
    MemoView::new(()).show(ui, |ui, _| {
        app.view_runs += 1;
        Stack::new().show(ui, |ui| {
            IconButton::new(IconId::new("brush"), "A").show(ui);
            IconButton::new(IconId::new("eraser"), "B").show(ui);
        })
    });
    app.tip = show_leaf(TipButton, ui).rect;
    let chip = show_leaf(DragChip, ui);
    app.chip = chip.response.rect;
    app.drag_ended |= chip.ended;
    let body = ui.max_rect();
    ToastHost::show(ui, &mut app.toasts, body);
    if app.modal_open {
        match show_dialog(ui, 1, &Overlay::modal(), |ui| IconButton::new(IconId::new("check"), "OK").show(ui)) {
            Ok(ok) => {
                app.ok = ok.rect;
                if ok.clicked() {
                    app.ok_clicks += 1;
                    app.modal_open = false;
                }
            }
            Err(_) => app.modal_open = false,
        }
    }
}

fn second_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let button = IconButton::new(IconId::new("plus"), "Second").show(ui);
    app.second_button = button.rect;
    app.second_clicks += u32::from(button.clicked());
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    let main = rig.main;
    rig.step_with(&mut |window, cx| {
        if window == main {
            window_root(cx, |ui| app_ui(ui, app));
        } else {
            window_root(cx, |ui| second_ui(ui, app));
        }
    })
}

fn settle(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    for _ in 0..6 {
        rig.driver.platform.advance(FADE_END);
        if !step(rig, app).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn finish_fades(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    rig.driver.platform.advance(FADE_END);
    step(rig, app);
}

fn rig(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 400.0), 1.0, RecordingRenderer::default());
    step(&mut rig, app);
    settle(&mut rig, app);
    rig
}

fn input_to(rig: &mut Rig<RecordingRenderer>, app: &mut App, window: WindowId, input: Input) -> StepReport {
    rig.inject(window, input);
    step(rig, app)
}

fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
    let main = rig.main;
    input_to(rig, app, main, input)
}

fn click(rig: &mut Rig<RecordingRenderer>, app: &mut App, at: Pos2) -> (StepReport, StepReport) {
    input(rig, app, Input::mouse_move(at));
    let down = input(rig, app, Input::mouse_down(at));
    let up = input(rig, app, Input::mouse_up(at));
    (down, up)
}

fn open_popup(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    let at = app.opener.center();
    let (_, up) = click(rig, app, at);
    assert!(app.popup_open, "the click opened the popup");
    up
}

#[test]
fn opening_a_popup_builds_only_the_popup_and_its_rows() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    let views = app.view_runs;
    let up = open_popup(&mut rig, &mut app);
    assert_eq!(up.frame.runs, 1 + ROWS.len() as u32, "the opener and its rows run, nothing else: {up:?}");
    assert_eq!(up.frame.scopes, 2, "the root on the opener's path and the popup run their closures");
    assert_eq!(app.view_runs, views, "the view beside the opener is kept");
    assert_eq!(app.rows.len(), ROWS.len());
    assert!(app.rows[0].min.y >= app.opener.max.y, "the popup hangs below its opener: {:?} under {:?}", app.rows[0], app.opener);
    assert!(app.rows[1].min.y > app.rows[0].max.y - 0.5, "rows lie one under another, in window coordinates");
    settle(&mut rig, &mut app);
}

#[test]
fn hover_inside_an_open_popup_repaints_only_the_rows_it_crosses() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    open_popup(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let (first, second) = (app.rows[0].center(), app.rows[1].center());
    input(&mut rig, &mut app, Input::mouse_move(first));
    let moved = input(&mut rig, &mut app, Input::mouse_move(second));
    assert_eq!(moved.frame.painted, 2, "the row left and the row entered swap their looks: {moved:?}");
    assert_eq!((moved.frame.runs, moved.frame.scopes, moved.frame.built), (0, 0, 0), "no hook, scope or build runs");
    finish_fades(&mut rig, &mut app);
    let within = input(&mut rig, &mut app, Input::mouse_move(second + Vec2::new(1.0, 1.0)));
    assert_eq!(within.frame.painted, 0);
}

#[test]
fn a_row_click_picks_and_closes_and_an_outside_press_or_escape_dismisses() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    open_popup(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let row = app.rows[2].center();
    let (_, up) = click(&mut rig, &mut app, row);
    settle(&mut rig, &mut app);
    assert_eq!(app.picked, Some(2), "the row's click reached the opener through the popup: {up:?}");
    assert!(!app.popup_open, "picking closed the popup");

    open_popup(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let at = app.other.center();
    input(&mut rig, &mut app, Input::mouse_move(at));
    input(&mut rig, &mut app, Input::mouse_down(at));
    assert!(!app.popup_open, "a press outside dismissed the popup in the frame of the press");
    input(&mut rig, &mut app, Input::mouse_up(at));
    assert_eq!(app.other_clicks, 1, "the press went on to what it hit");

    open_popup(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    for key in Input::key_press(Key::Escape, Mods::NONE) {
        input(&mut rig, &mut app, key);
    }
    settle(&mut rig, &mut app);
    assert!(!app.popup_open, "Escape no node took dismissed the popup");
}

#[test]
fn moving_a_drag_preview_runs_nothing() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    let at = app.chip.center();
    input(&mut rig, &mut app, Input::mouse_move(at));
    input(&mut rig, &mut app, Input::mouse_down(at));
    input(&mut rig, &mut app, Input::mouse_move(at + Vec2::new(30.0, 0.0)));
    settle(&mut rig, &mut app);
    assert!(rig.runtime().drag().is_some(), "the press past slop started a session");
    for dx in [40.0, 60.0, 90.0] {
        let moved = input(&mut rig, &mut app, Input::mouse_move(at + Vec2::new(dx, 10.0)));
        assert_eq!(
            (moved.frame.runs, moved.frame.scopes, moved.frame.built, moved.frame.painted),
            (0, 0, 0, 0),
            "the preview moves by translation; nothing runs, builds or paints: {moved:?}"
        );
    }
    input(&mut rig, &mut app, Input::mouse_up(at + Vec2::new(90.0, 10.0)));
    settle(&mut rig, &mut app);
    assert!(rig.runtime().drag().is_none());
    assert!(app.drag_ended, "the source heard the end in its next run");
}

#[test]
fn a_tooltip_waiting_for_its_delay_runs_no_frame_until_the_delay_ends() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    let at = app.tip.center();
    let entered = input(&mut rig, &mut app, Input::mouse_move(at));
    assert_eq!(entered.frame.runs, 0, "entering starts the delay without a run");
    let start = rig.platform().now();
    let wake = rig.platform().pending_wake().expect("the delay asked for a wake");
    assert!((wake - (start + TOOLTIP_DELAY)).abs() < 1e-9, "the wake is at the end of the delay");
    let frames = ((TOOLTIP_DELAY / FRAME) as usize).saturating_sub(2);
    let main = rig.main;
    for _ in 0..frames {
        let idle = rig.frame_with(FRAME, &mut |window, cx| {
            if window == main {
                window_root(cx, |ui| app_ui(ui, &mut app));
            }
        });
        assert!(!idle.ran, "no frame runs while the tooltip waits");
    }
    rig.platform().advance_to_wake();
    let shown = step(&mut rig, &mut app);
    assert_eq!(shown.frame.runs, 2, "at the wake the tipped button and the tooltip's text run: {shown:?}");
    assert_eq!(shown.frame.scopes, 2, "the root on the button's path and the tooltip layer");
    let left = input(&mut rig, &mut app, Input::mouse_move(Pos2::new(390.0, 390.0)));
    assert_eq!(left.frame.runs, 1, "leaving runs the button once, without its tooltip");
    assert_eq!(rig.runtime().next_wake(), None, "no wake is left");
}

#[test]
fn a_modal_blocks_pointer_input_to_its_window_and_the_app_s_other_windows() {
    let mut app = App { modal_open: true, ..App::default() };
    let mut rig = rig(&mut app);
    let second = rig.driver.open_inline_window(&WindowSpec { size: Some(Vec2::new(200.0, 200.0)), ..WindowSpec::new("Second") });
    app.second = Some(second);
    settle(&mut rig, &mut app);

    let at = app.other.center();
    assert!(app.ok.is_positive() && app.ok.min.y > 0.0, "the dialog shows, centred: {:?}", app.ok);
    assert!(!app.ok.contains(at), "the dialog does not cover Other");
    let hover = input(&mut rig, &mut app, Input::mouse_move(at));
    assert_eq!(hover.frame.painted, 0, "the modal layer hides Other from the pointer");
    input(&mut rig, &mut app, Input::mouse_down(at));
    input(&mut rig, &mut app, Input::mouse_up(at));
    assert_eq!(app.other_clicks, 0, "a click under the modal reaches nothing");

    let in_second = app.second_button.center();
    input_to(&mut rig, &mut app, second, Input::mouse_move(in_second));
    input_to(&mut rig, &mut app, second, Input::mouse_down(in_second));
    input_to(&mut rig, &mut app, second, Input::mouse_up(in_second));
    assert_eq!(app.second_clicks, 0, "the app-modal dialog blocks the other window");
    assert_eq!(rig.platform().focused(), Some(rig.main), "a press in the blocked window focuses the dialog's window");

    for key in Input::key_press(Key::Escape, Mods::NONE) {
        input(&mut rig, &mut app, key);
    }
    settle(&mut rig, &mut app);
    assert!(!app.modal_open, "Escape dismissed the dialog");
    input_to(&mut rig, &mut app, second, Input::mouse_move(in_second + Vec2::new(1.0, 0.0)));
    input_to(&mut rig, &mut app, second, Input::mouse_down(in_second));
    input_to(&mut rig, &mut app, second, Input::mouse_up(in_second));
    assert_eq!(app.second_clicks, 1, "with the dialog gone the other window takes input again");
    let (_, up) = click(&mut rig, &mut app, at);
    assert_eq!(app.other_clicks, 1, "and so does content under it: {up:?}");
}

#[test]
fn a_toast_waiting_for_its_timeout_runs_no_frame_and_leaves_at_its_deadline() {
    let mut app = App { toast_on_other: Some(2.0), ..App::default() };
    let mut rig = rig(&mut app);
    let at = app.other.center();
    let (_, up) = click(&mut rig, &mut app, at);
    assert_eq!(app.toasts.len(), 1);
    assert_eq!((up.frame.runs, up.frame.scopes), (2, 2), "Other and the card run; the root and the stack run their closures: {up:?}");
    let posted = rig.platform().now();
    finish_fades(&mut rig, &mut app);
    let main = rig.main;
    for _ in 0..((1.5 / FRAME) as usize) {
        let idle = rig.frame_with(FRAME, &mut |window, cx| {
            if window == main {
                window_root(cx, |ui| app_ui(ui, &mut app));
            }
        });
        assert!(!idle.ran, "no frame runs while the card waits");
    }
    rig.platform().advance_to_wake();
    assert!((rig.platform().now() - (posted + 2.0)).abs() < 1e-6, "the wake is the card's deadline");
    let gone = step(&mut rig, &mut app);
    assert!(gone.ran);
    assert!(app.toasts.is_empty());
    assert_eq!(app.toasts.take_end(app.toast.expect("posted")), Some(ToastEnd::Expired));
    assert_eq!(gone.frame.runs, 0, "the expired card's stack goes without a run: {gone:?}");
}

/// What `PopupPanel` returns: its popup's open state and rows, as `PopupOut`.
type PanelOut = PopupOut;

/// A padded container whose own click (on its padding, outside its children) toggles a popover
/// listing `ROWS`; a row's click picks it and closes the popover.
struct PopupPanel {
    props: ContainerProps,
    toggle: bool,
    open: Option<bool>,
    rows: Option<Vec<Rect>>,
    picked: Option<usize>,
}

impl PopupPanel {
    fn new() -> PopupPanel {
        let mut props = ContainerProps::new();
        props.pad = Pad::Roomy;
        PopupPanel { props, toggle: false, open: None, rows: None, picked: None }
    }
}

impl ComponentKey for PopupPanel {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for PopupPanel {
    const ROLE: accesskit::Role = accesskit::Role::Group;
    type Output = PanelOut;

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.toggle = cx.clicked();
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_, '_>) {
        if self.toggle {
            ui.toggle(list());
        }
        let shown = ui.show(list(), &Overlay::popover(), |ui| {
            let mut rects = Vec::new();
            let mut picked = None;
            for (i, row) in ROWS.iter().enumerate() {
                let out = IconButton::new(IconId::new("row"), row).show(ui);
                rects.push(out.rect);
                if out.clicked() {
                    picked = Some(i);
                }
            }
            (rects, picked)
        });
        if let Some((rects, picked)) = shown {
            self.rows = Some(rects);
            if picked.is_some() {
                self.picked = picked;
                ui.close(list(), CloseReason::Activated);
            }
        }
        self.open = Some(ui.is_open(list()));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> PanelOut {
        PopupOut { response: cx.response.clone(), open: self.open, rows: self.rows, picked: self.picked }
    }
}

impl Container for PopupPanel {
    fn props(&self) -> &ContainerProps {
        &self.props
    }
}

struct PanelApp {
    panel: Rect,
    inside: Rect,
    inside_clicks: u32,
    popup_open: bool,
    rows: Vec<Rect>,
    picked: Option<usize>,
}

impl Default for PanelApp {
    fn default() -> PanelApp {
        PanelApp { panel: Rect::NOTHING, inside: Rect::NOTHING, inside_clicks: 0, popup_open: false, rows: Vec::new(), picked: None }
    }
}

fn panel_ui(ui: &mut Ui<'_, '_>, app: &mut PanelApp) {
    let (inside, out) = show_container(PopupPanel::new(), ui, |ui| IconButton::new(IconId::new("brush"), "Inside").show(ui));
    app.panel = out.response.rect;
    app.inside = inside.rect;
    app.inside_clicks += u32::from(inside.clicked());
    if let Some(open) = out.open {
        app.popup_open = open;
    }
    if let Some(rows) = out.rows {
        app.rows = rows;
    }
    app.picked = out.picked.or(app.picked);
}

fn panel_input(rig: &mut Rig<RecordingRenderer>, app: &mut PanelApp, input: Input) -> StepReport {
    let main = rig.main;
    rig.inject(main, input);
    rig.step_with(&mut |window, cx| {
        if window == main {
            window_root(cx, |ui| panel_ui(ui, app));
        }
    })
}

fn panel_settle(rig: &mut Rig<RecordingRenderer>, app: &mut PanelApp) {
    let main = rig.main;
    for _ in 0..6 {
        rig.driver.platform.advance(FADE_END);
        let report = rig.step_with(&mut |window, cx| {
            if window == main {
                window_root(cx, |ui| panel_ui(ui, app));
            }
        });
        if !report.ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn panel_click(rig: &mut Rig<RecordingRenderer>, app: &mut PanelApp, at: Pos2) -> StepReport {
    panel_input(rig, app, Input::mouse_move(at));
    panel_input(rig, app, Input::mouse_down(at));
    panel_input(rig, app, Input::mouse_up(at))
}

#[test]
fn a_container_opens_a_popup_that_lives_through_its_body_s_runs_and_closes_on_a_row_click() {
    let mut app = PanelApp::default();
    let mut rig = Rig::inline(Vec2::new(400.0, 400.0), 1.0, RecordingRenderer::default());
    panel_settle(&mut rig, &mut app);
    assert!(app.panel.min.x < app.inside.min.x, "the panel pads its child: {:?} around {:?}", app.panel, app.inside);

    let pad = app.panel.min + Vec2::splat(2.0);
    let up = panel_click(&mut rig, &mut app, pad);
    assert!(app.popup_open, "a click on the panel's padding opened its popup: {up:?}");
    assert_eq!(up.frame.runs, 1 + ROWS.len() as u32, "the panel and its rows run; its child is kept: {up:?}");
    assert_eq!(app.rows.len(), ROWS.len());
    assert!(app.rows[0].min.y >= app.panel.max.y, "the popup hangs below the panel: {:?} under {:?}", app.rows[0], app.panel);
    panel_settle(&mut rig, &mut app);

    // A click on the child runs the panel's body while the panel keeps its own outputs: the popup
    // is reached, kept, and stays open.
    let inside = app.inside.center();
    let up = panel_click(&mut rig, &mut app, inside);
    panel_settle(&mut rig, &mut app);
    assert_eq!(app.inside_clicks, 1, "the child took its click: {up:?}");
    assert!(app.popup_open, "the popup outlived a run of the panel's body");
    assert_eq!(up.frame.runs, 1, "only the clicked child runs; the panel and the popup's rows are kept: {up:?}");

    // A row's click reaches the panel through PATH while it keeps its outputs; picking closes.
    let row = app.rows[1].center();
    let up = panel_click(&mut rig, &mut app, row);
    panel_settle(&mut rig, &mut app);
    assert_eq!(app.picked, Some(1), "the row's click reached the panel through its popup: {up:?}");
    assert!(!app.popup_open, "picking closed the popup");
}
