//! Count trials of focus and keys on craft-ui-rt: Tab, roving moves and keys run only what they
//! must, the stops are kept with their nodes, and traps hold. Relations only, no times.

use crate::skeleton::{IconButton, Stack};
use craft_behavior::{IconId, NavShape};
use craft_ui::contract::overlay::{Overlay, show_dialog};
use craft_ui::contract::{
    BodyCx, Common, Component, Cx, MeasureCx, MemoView, NoValue, Outbox, Outcome, OverlayUi, Overlays as _, Response, Role, Show as _, Ui,
};
use craft_ui::runtime::focus::Roving;
use craft_ui::runtime::windows::{WindowEnv, window_root_with};
use craft_ui::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Key, Mods, Rect, Sense, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

/// Past the theme's hover fade, which paint reads from the clock.
const FADE_END: f64 = 0.25;

/// A button, a Tab stop, that toggles a popover of two rows when activated.
#[derive(craft_ui::contract::Component)]
#[component(unregistered)]
struct MenuButton {
    #[common]
    common: Common,
    #[key(skip)]
    rows: Option<Vec<Rect>>,
    #[key(skip)]
    open: Option<bool>,
}

struct MenuOut {
    response: Response,
    rows: Option<Vec<Rect>>,
    open: Option<bool>,
}

impl Outcome for MenuOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl Component for MenuButton {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = MenuOut;

    fn style(_: &StyleCx<'_>, _: MenuButtonStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.response.clicked() {
            let menu = cx.popup("menu");
            if menu.is_open(cx) {
                menu.close(cx);
            } else {
                menu.open(cx);
            }
        }
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let menu = cx.popup("menu");
        let shown = ui.show(menu, Overlay::popover(), |ui: &mut Ui, _: &mut BodyCx<'_>| {
            let cut = IconButton::new(IconId::new("cut"), "Cut").focusable().show(ui);
            let copy = IconButton::new(IconId::new("copy"), "Copy").focusable().show(ui);
            (vec![cut.rect, copy.rect], cut.clicked() || copy.clicked())
        });
        if let Some((rows, picked)) = shown {
            self.rows = Some(rows);
            if picked {
                menu.close(cx);
            }
        }
        self.open = Some(menu.is_open(cx));
    }

    fn finish(self, cx: Cx<'_, Self>) -> MenuOut {
        MenuOut { response: cx.response.clone(), rows: self.rows, open: self.open }
    }
}

/// What the view holding C and D reports to the app.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ViewEvent {
    Ran,
    CClicked,
}

fn view(out: &mut Outbox<'_, ViewEvent>, ui: &mut Ui) -> (Rect, Rect) {
    out.send(ViewEvent::Ran);
    Stack::new().show(ui, |ui| {
        let c = button(ui, "C");
        if c.clicked() {
            out.send(ViewEvent::CClicked);
        }
        (c.rect, button(ui, "D").rect)
    })
}

/// A window with buttons A and B, a `MemoView` holding C and D, a roving group of R1 to R3, a menu
/// button and, while `modal_open`, a dialog with OK and Cancel.
struct App {
    a: Rect,
    b: Rect,
    c: Rect,
    d: Rect,
    r: Vec<Rect>,
    menu: Rect,
    rows: Vec<Rect>,
    menu_open: bool,
    modal_open: bool,
    ok: Rect,
    cancel: Rect,
    c_clicks: u32,
    view_runs: u32,
}

impl Default for App {
    fn default() -> App {
        let none = Rect::NOTHING;
        App {
            a: none,
            b: none,
            c: none,
            d: none,
            r: Vec::new(),
            menu: none,
            rows: Vec::new(),
            menu_open: false,
            modal_open: false,
            ok: none,
            cancel: none,
            c_clicks: 0,
            view_runs: 0,
        }
    }
}

fn button(ui: &mut Ui, label: &str) -> Response {
    IconButton::new(IconId::new("icon"), label).focusable().show(ui)
}

fn app_ui(ui: &mut Ui, app: &mut App) {
    app.a = button(ui, "A").rect;
    app.b = button(ui, "B").rect;
    let mut sent = Vec::new();
    if let Some(shown) = MemoView::new("view", Outbox::new(&mut sent), view).show(ui) {
        (app.c, app.d) = shown.value;
    }
    for event in sent {
        match event {
            ViewEvent::Ran => app.view_runs += 1,
            ViewEvent::CClicked => app.c_clicks += 1,
        }
    }
    app.r = Stack::new().roving(Roving::new(NavShape::Flat)).show(ui, |ui| ["R1", "R2", "R3"].map(|l| button(ui, l).rect).to_vec());
    let out = MenuButton { common: Common::new(), rows: None, open: None }.show(ui);
    app.menu = out.response.rect;
    if let Some(rows) = out.rows {
        app.rows = rows;
    }
    if let Some(open) = out.open {
        app.menu_open = open;
    }
    if app.modal_open {
        match show_dialog(ui, 1, &Overlay::modal(), |ui| (button(ui, "OK").rect, button(ui, "Cancel").rect)) {
            Ok(shown) => (app.ok, app.cancel) = shown.value,
            Err(_) => app.modal_open = false,
        }
    }
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    rig.step_with(&mut |_, cx| window_root_with(cx, WindowEnv::default(), |ui| app_ui(ui, app)))
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

fn rig(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 600.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    rig.driver.runtime.set_access_enabled(main, true);
    step(&mut rig, app);
    settle(&mut rig, app);
    rig
}

/// Presses and releases `key`; the report of the press's frame.
fn press(rig: &mut Rig<RecordingRenderer>, app: &mut App, key: Key, mods: Mods) -> StepReport {
    let main = rig.main;
    let [down, up] = Input::key_press(key, mods);
    rig.inject(main, down);
    let report = step(rig, app);
    rig.inject(main, up);
    step(rig, app);
    report
}

fn tab(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    press(rig, app, Key::Tab, Mods::NONE)
}

/// The rect of the focused node.
fn focused(rig: &Rig<RecordingRenderer>) -> Option<Rect> {
    let rt = &rig.driver.runtime;
    rt.focused(rig.main).and_then(|n| rt.window_rect(n))
}

fn access_id(rig: &Rig<RecordingRenderer>, name: &str) -> accesskit::NodeId {
    let rt = &rig.driver.runtime;
    rt.access_snapshot(rig.main).into_iter().find(|e| e.node.label() == Some(name)).unwrap_or_else(|| panic!("no node named {name}")).id
}

#[test]
fn tab_moves_focus_through_the_window_s_stops_repainting_two_nodes_and_running_none() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    tab(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.a), "the first Tab focuses the first stop");
    let moved = tab(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.b));
    assert_eq!((moved.frame.runs, moved.frame.scopes, moved.frame.built), (0, 0, 0), "a focus move runs nothing: {moved:?}");
    assert_eq!(moved.frame.painted, 2, "A loses its ring and B gains it: {moved:?}");
    assert_eq!(rig.driver.runtime.access_focus(rig.main), access_id(&rig, "B"), "assistive technology follows the focus");
    let mut order = vec![focused(&rig)];
    for _ in 0..4 {
        tab(&mut rig, &mut app);
        order.push(focused(&rig));
    }
    assert_eq!(order, vec![Some(app.b), Some(app.c), Some(app.d), Some(app.r[0]), Some(app.menu)], "tree order; the roving group is one stop");
    tab(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.a), "Tab wraps");
    press(&mut rig, &mut app, Key::Tab, Mods::SHIFT);
    assert_eq!(focused(&rig), Some(app.menu), "Shift+Tab goes back");
}

#[test]
fn a_key_on_the_focused_node_runs_that_node_and_the_scopes_on_its_path() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    for _ in 0..3 {
        tab(&mut rig, &mut app);
    }
    assert_eq!(focused(&rig), Some(app.c));
    let views = app.view_runs;
    let enter = press(&mut rig, &mut app, Key::Enter, Mods::NONE);
    assert_eq!(app.c_clicks, 1, "Enter activates the focused button in the frame of the key");
    assert_eq!((enter.frame.runs, enter.frame.scopes), (1, 2), "C runs, and the root and the view on its path: {enter:?}");
    assert_eq!(app.view_runs, views + 1, "the view's closure ran to reach C");
    let other = press(&mut rig, &mut app, Key::Q, Mods::NONE);
    assert_eq!((other.frame.runs, other.frame.scopes), (0, 0), "a key no node takes runs nothing: {other:?}");
}

#[test]
fn a_kept_node_keeps_its_stop_through_frames_that_do_not_run_it() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    let walk = |rig: &mut Rig<RecordingRenderer>, app: &mut App| {
        (0..6)
            .map(|_| {
                tab(rig, app);
                focused(rig)
            })
            .collect::<Vec<_>>()
    };
    let first = walk(&mut rig, &mut app);
    // A click on A runs A and the root; the view and the group keep their nodes.
    let main = rig.main;
    let views = app.view_runs;
    for input in [Input::mouse_move(app.a.center()), Input::mouse_down(app.a.center()), Input::mouse_up(app.a.center())] {
        rig.inject(main, input);
        step(&mut rig, &mut app);
    }
    settle(&mut rig, &mut app);
    assert_eq!(app.view_runs, views, "the view did not run");
    assert_eq!(focused(&rig), Some(app.a), "the press focused A");
    let idle = step(&mut rig, &mut app);
    assert!(!idle.ran, "nothing runs");
    let again = walk(&mut rig, &mut app);
    assert_eq!(&again[..5], &first[1..6], "the Tab order is the same, from A");
}

#[test]
fn tab_inside_a_modal_stays_in_it_and_requests_outside_it_are_refused() {
    let mut app = App { modal_open: true, ..App::default() };
    let mut rig = rig(&mut app);
    assert_eq!(focused(&rig), Some(app.ok), "the dialog took focus to its first stop when it appeared");
    let mut seen = Vec::new();
    for _ in 0..3 {
        tab(&mut rig, &mut app);
        seen.push(focused(&rig));
    }
    assert_eq!(seen, vec![Some(app.cancel), Some(app.ok), Some(app.cancel)], "Tab wraps inside the dialog");
    press(&mut rig, &mut app, Key::Tab, Mods::SHIFT);
    assert_eq!(focused(&rig), Some(app.ok));
    let main = rig.main;
    let a = access_id(&rig, "A");
    rig.inject(main, Input::Access { action: accesskit::Action::Focus, target: a, data: None });
    step(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.ok), "a Focus request below the modal is refused");
    let cancel = access_id(&rig, "Cancel");
    rig.inject(main, Input::Access { action: accesskit::Action::Focus, target: cancel, data: None });
    step(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.cancel), "a Focus request inside it goes through");
}

#[test]
fn escape_with_a_popup_open_dismisses_it_runs_only_its_opener_and_returns_focus() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    for _ in 0..6 {
        tab(&mut rig, &mut app);
    }
    assert_eq!(focused(&rig), Some(app.menu));
    press(&mut rig, &mut app, Key::Enter, Mods::NONE);
    settle(&mut rig, &mut app);
    assert!(app.menu_open, "Enter opened the menu");
    assert_eq!(focused(&rig), Some(app.rows[0]), "the popover took focus to its first row");
    tab(&mut rig, &mut app);
    tab(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.rows[0]), "Tab stays in the popover");
    let escape = press(&mut rig, &mut app, Key::Escape, Mods::NONE);
    assert!(!app.menu_open, "Escape dismissed the popover");
    assert_eq!((escape.frame.runs, escape.frame.scopes), (1, 1), "only the opener runs, under the root: {escape:?}");
    assert_eq!(focused(&rig), Some(app.menu), "focus went back to the opener");
}

#[test]
fn arrows_move_focus_within_a_roving_group_and_tab_returns_to_its_lead() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    for _ in 0..5 {
        tab(&mut rig, &mut app);
    }
    assert_eq!(focused(&rig), Some(app.r[0]), "Tab enters the group at its first member");
    let down = press(&mut rig, &mut app, Key::Down, Mods::NONE);
    assert_eq!(focused(&rig), Some(app.r[1]));
    assert_eq!((down.frame.runs, down.frame.scopes), (0, 0), "a roving move runs nothing: {down:?}");
    press(&mut rig, &mut app, Key::Down, Mods::NONE);
    press(&mut rig, &mut app, Key::Down, Mods::NONE);
    assert_eq!(focused(&rig), Some(app.r[2]), "the group does not wrap");
    tab(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.menu), "Tab leaves the group");
    press(&mut rig, &mut app, Key::Tab, Mods::SHIFT);
    assert_eq!(focused(&rig), Some(app.r[2]), "Shift+Tab comes back to the member that had focus");
}

/// Secondary clicks (production: a context request on a secondary release over the hovered component; focus only
/// from a primary press). A click no shape sensing `CONTEXT_CLICK` takes reaches the component under the pointer with
/// no press; a secondary press moves no focus to its shape, and one away from a focused text field releases it.
mod secondary {
    use std::cell::RefCell;

    use crate::skeleton::IconButton;
    use crate::text_field::TextField;
    use craft_behavior::IconId;
    use craft_ui_contract::contract::{Bind, Common, Component, Cx, MeasureCx, Response, Role, Show as _, Ui};
    use craft_ui_contract::style::StyleCx;
    use craft_ui_contract::testing::Harness;
    use craft_ui_platform_headless::{Contact, Input};
    use craft_ui_rt::{PointerButton, PointerPhase, Pos2, Rect, Sense, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;

    thread_local! {
        static RECTS: RefCell<Vec<(&'static str, Rect)>> = const { RefCell::new(Vec::new()) };
        static RUNS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        static CLICKS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }

    /// A pad sensing `sense` that records its runs and its secondary clicks.
    #[derive(craft_ui_contract::contract::Component)]
    #[component(unregistered)]
    struct Pad {
        #[common]
        common: Common,
        #[key(skip)]
        name: &'static str,
        #[key(skip)]
        sense: Sense,
    }

    impl Component for Pad {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: PadStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(120.0, 40.0)
        }

        fn sense(&self) -> Sense {
            self.sense
        }

        fn update(&mut self, cx: &mut Cx<'_, Self>) {
            RUNS.with(|runs| runs.borrow_mut().push(self.name));
            if cx.response.secondary_clicked() {
                CLICKS.with(|clicks| clicks.borrow_mut().push(self.name));
            }
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    fn pad(ui: &mut Ui, name: &'static str, sense: Sense) {
        let rect = Pad { common: Common::new(), name, sense }.show(ui).rect;
        RECTS.with(|rects| {
            let mut rects = rects.borrow_mut();
            rects.retain(|(n, _)| *n != name);
            rects.push((name, rect));
        });
    }

    fn rect(name: &str) -> Rect {
        RECTS.with(|rects| rects.borrow().iter().find(|(n, _)| *n == name).map(|(_, r)| *r)).unwrap_or_else(|| panic!("no pad {name}"))
    }

    fn runs(name: &str) -> usize {
        RUNS.with(|runs| runs.borrow().iter().filter(|n| **n == name).count())
    }

    fn clicks() -> Vec<&'static str> {
        CLICKS.with(|clicks| clicks.borrow().clone())
    }

    fn harness() -> Harness {
        let mut h = Harness::new_ui(|ui| {
            let mut text = String::new();
            let _ = IconButton::new(IconId::new("plus"), "A").focusable().show(ui);
            let field = TextField::new(Bind::bound(&mut text)).show(ui).response.rect;
            RECTS.with(|rects| {
                let mut rects = rects.borrow_mut();
                rects.retain(|(n, _)| *n != "field");
                rects.push(("field", field));
            });
            // A slider-like shape that drags, one that only hovers (a label), one that takes its secondary press.
            pad(ui, "drag", Sense::HOVER | Sense::CLICK | Sense::DRAG);
            pad(ui, "label", Sense::HOVER);
            pad(ui, "context", Sense::HOVER | Sense::CLICK | Sense::CONTEXT_CLICK | Sense::FOCUS | Sense::TAKE_FOCUS);
        });
        h.run();
        h
    }

    fn secondary(h: &mut Harness, phase: PointerPhase, at: Pos2) {
        h.send(Input::pointer(Contact { button: PointerButton::Secondary, ..Contact::MOUSE }, phase, at)).run();
    }

    fn secondary_click(h: &mut Harness, at: Pos2) {
        h.send(Input::mouse_move(at)).run();
        secondary(h, PointerPhase::Down, at);
        secondary(h, PointerPhase::Up, at);
    }

    #[test]
    fn a_secondary_click_no_shape_takes_reaches_the_component_under_the_pointer_with_no_press() {
        let mut h = harness();
        let at = rect("drag").center();
        h.send(Input::mouse_move(at)).run();
        let before = runs("drag");
        secondary(&mut h, PointerPhase::Down, at);
        assert_eq!(runs("drag"), before, "a secondary press no shape takes runs nothing");
        secondary(&mut h, PointerPhase::Up, at);
        assert_eq!(clicks(), ["drag"], "its release is the dragging shape's secondary click");
        assert_eq!(runs("drag"), before + 1, "which runs it once");

        secondary(&mut h, PointerPhase::Down, at);
        h.send(Input::mouse_move(at + Vec2::new(30.0, 0.0))).run();
        secondary(&mut h, PointerPhase::Up, at + Vec2::new(30.0, 0.0));
        assert_eq!(clicks(), ["drag"], "a secondary drag is no click");

        secondary_click(&mut h, rect("label").center());
        assert_eq!(clicks(), ["drag", "label"], "a shape that only hovers gets its secondary click too");
    }

    #[test]
    fn a_secondary_press_moves_no_focus_and_one_away_from_a_focused_field_releases_it() {
        let mut h = harness();
        let a = h.get_by_label("A");
        h.click(&a).run();
        assert_eq!(h.focused().map(|t| t.label), Some("A".to_owned()), "a primary click focuses A");
        secondary_click(&mut h, rect("context").center());
        assert_eq!(clicks(), ["context"], "the shape that senses CONTEXT_CLICK takes its secondary click");
        assert_eq!(h.focused().map(|t| t.label), Some("A".to_owned()), "a secondary press does not move focus to its shape");

        let bounds = rect("field");
        let field = bounds.center();
        h.click_at(field).run();
        assert_eq!(h.focused().map(|t| t.bounds), Some(bounds), "a primary click enters the field");
        secondary_click(&mut h, field);
        assert_eq!(h.focused().map(|t| t.bounds), Some(bounds), "a secondary press on the focused field keeps its focus");
        secondary_click(&mut h, rect("drag").center());
        assert_eq!(h.focused(), None, "a secondary press away from the focused field releases it");
    }
}
