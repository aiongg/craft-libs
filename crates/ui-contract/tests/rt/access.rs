//! Count trials of accessibility on craft-ui-rt: nodes are node-owned and incremental, an action
//! runs only the node it names and the scopes on its path, announcements go to their window, and
//! no node builds while the access tree is off.

use std::cell::Cell;
use std::rc::Rc;

use craft_ui_contract::contract::{
    Access, AccessAction, Common, Component, Container, ContainerProps, Cx, Gap, MeasureCx, MemoView, NodeInput, Numeric, PaintUi, Politeness,
    Response, Role, Scroll, Show as _, ShowContainer as _, Sizing, Styled as _, Ui,
};
use craft_ui_contract::primitives::PaintShape;
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::style::{StyleColor, UiTokens, rgba_color};
use craft_ui_platform_headless::{HeadlessPlatform, Input};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Platform, Pos2, Rect, Runtime, Sense, StepReport, Vec2, WindowId, WindowSpec};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::{Rig, test_tokens};

/// A leaf that names itself by `label`, counts its `access` calls and what reached it.
#[derive(Component)]
struct Item {
    #[common]
    common: Common,
    label: String,
    /// `number`'s bits, which the key holds.
    bits: Option<u64>,
    #[key(skip)]
    number: Option<f64>,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
    #[key(skip)]
    accessed: Rc<Cell<u32>>,
    #[key(skip)]
    seen: Rc<std::cell::RefCell<Vec<AccessAction>>>,
}

impl Component for Item {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ItemStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(60.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
        for i in cx.input() {
            if let NodeInput::Access(a) = i {
                self.seen.borrow_mut().push(a.clone());
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let ink = ui.env.tokens.theme().color.text.faint;
        if cx.state().state != craft_theme::State::Rest {
            ui.draw().fill(&PaintShape::Rect(cx.rect, UiTokens::corner(0.0)), StyleColor(rgba_color(ink)));
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        self.accessed.set(self.accessed.get() + 1);
        access.label(self.label.as_str());
        if let Some(v) = self.number {
            access.numeric(Numeric::new(v, 0.0..=10.0).step(1.0));
        }
        access.custom_action(7, "Reset");
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct App {
    label_a: String,
    other_label: String,
    /// What a click on Other does: announce this text, and rename A.
    announce: Option<String>,
    rename: bool,
    clicks: u32,
    a: Rect,
    other: Rect,
    runs: Rc<Cell<u32>>,
    accessed: Rc<Cell<u32>>,
    seen: Rc<std::cell::RefCell<Vec<AccessAction>>>,
    view_runs: u32,
}

impl App {
    fn new() -> App {
        App {
            label_a: String::new(),
            other_label: String::new(),
            announce: None,
            rename: false,
            clicks: 0,
            a: Rect::NOTHING,
            other: Rect::NOTHING,
            runs: Rc::default(),
            accessed: Rc::default(),
            seen: Rc::default(),
            view_runs: 0,
        }
    }
}

fn item(ui: &mut Ui<'_, '_>, app: &App, label: &str, number: Option<f64>) -> Response {
    Item {
        common: Common::new(),
        label: label.into(),
        bits: number.map(f64::to_bits),
        number,
        runs: app.runs.clone(),
        accessed: app.accessed.clone(),
        seen: app.seen.clone(),
    }
    .show(ui)
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let other_label = app.other_label.clone();
    let o = item(ui, app, &other_label, None);
    app.other = o.rect;
    if o.clicked() {
        app.clicks += 1;
        if let Some(text) = app.announce.clone() {
            ui.announce(text, Politeness::Polite);
        }
        if app.rename {
            app.label_a = format!("A{}", app.clicks);
        }
    }
    let label = app.label_a.clone();
    let rect = MemoView::new(label).show(ui, |ui, label| {
        app.view_runs += 1;
        item(ui, app, label, Some(3.0)).rect
    });
    app.a = rect;
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    rig.step_with(&mut |_, cx| window_root(cx, |ui| app_ui(ui, app)))
}

fn rig(app: &mut App, access: bool) -> Rig<RecordingRenderer> {
    app.label_a = "A".into();
    app.other_label = "Other".into();
    // The headless platform reports an assistive technology at window creation unless told not to.
    let mut platform = HeadlessPlatform::new();
    platform.set_access_active(access);
    let main = platform.create_window(&WindowSpec { size: Some(Vec2::new(400.0, 300.0)), ..WindowSpec::new("main") });
    let mut runtime = Runtime::new(test_tokens());
    runtime.set_inline_root(main);
    let mut rig = Rig::from_parts(runtime, platform, RecordingRenderer::default());
    for _ in 0..6 {
        if !step(&mut rig, app).ran {
            return rig;
        }
    }
    panic!("the rig did not settle");
}

fn send(rig: &mut Rig<RecordingRenderer>, app: &mut App, request: accesskit::ActionRequest) -> StepReport {
    let main = rig.main;
    rig.inject(main, Input::Access { action: request.action, target: request.target_node, data: request.data });
    step(rig, app)
}

fn request(action: accesskit::Action, target: accesskit::NodeId, data: Option<accesskit::ActionData>) -> accesskit::ActionRequest {
    accesskit::ActionRequest { action, target_tree: accesskit::TreeId::ROOT, target_node: target, data }
}

/// The access id of the node under `at`, found by its name in the settled tree.
fn node_named(rig: &Rig<RecordingRenderer>, name: &str) -> (accesskit::NodeId, accesskit::Node) {
    let main = rig.main;
    let rt = &rig.driver.runtime;
    let entry = rt.access_snapshot(main).into_iter().find(|e| e.node.label() == Some(name)).unwrap_or_else(|| panic!("no node named {name}"));
    (entry.id, entry.node.clone())
}

fn settle(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    for _ in 0..6 {
        // A pointer look fades over the theme's hover time, read in paint: the clock moves on.
        rig.driver.platform.advance(1.0);
        if !step(rig, app).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn pointer(rig: &mut Rig<RecordingRenderer>, app: &mut App, to: Pos2) -> StepReport {
    let main = rig.main;
    rig.inject(main, Input::mouse_move(to));
    step(rig, app)
}

#[test]
fn a_hover_that_changes_no_access_property_sends_no_access_node() {
    let mut app = App::new();
    let mut rig = rig(&mut app, true);
    let at = app.a.center();
    let hover = pointer(&mut rig, &mut app, at);
    assert_eq!(hover.frame.painted, 1, "the hover look repainted: {hover:?}");
    assert_eq!((hover.frame.runs, hover.frame.access_nodes), (0, 0), "no hook ran and no node was sent: {hover:?}");
}

#[test]
fn a_label_change_sends_exactly_the_changed_node() {
    let mut app = App::new();
    let mut rig = rig(&mut app, true);
    let before = app.accessed.get();
    app.rename = true;
    let (other, _) = node_named(&rig, "Other");
    let changed = send(&mut rig, &mut app, request(accesskit::Action::Click, other, None));
    assert_eq!(app.accessed.get(), before + 2, "Other (clicked) and A (renamed) ran their access hooks");
    assert_eq!(changed.frame.access_nodes, 1, "only A's node differs from what was sent: {changed:?}");
    let (_, node) = node_named(&rig, "A1");
    assert_eq!(node.role(), accesskit::Role::Button);
}

#[test]
fn an_access_click_runs_only_the_button_and_the_scopes_on_its_path() {
    let mut app = App::new();
    let mut rig = rig(&mut app, true);
    let (id, node) = node_named(&rig, "A");
    assert!(node.supports_action(accesskit::Action::Click), "a pressable node declares Click");
    let views = app.view_runs;
    let clicked = send(&mut rig, &mut app, request(accesskit::Action::Click, id, None));
    assert_eq!(clicked.frame.runs, 1, "only A runs: {clicked:?}");
    assert_eq!(clicked.frame.scopes, 2, "the root and the view on A's path");
    assert_eq!(app.view_runs, views + 1);
    assert!(app.seen.borrow().iter().any(|a| a.action == accesskit::Action::Click));
}

#[test]
fn set_value_increment_and_custom_actions_arrive_on_the_node_they_name() {
    let mut app = App::new();
    let mut rig = rig(&mut app, true);
    let (id, node) = node_named(&rig, "A");
    for action in [accesskit::Action::SetValue, accesskit::Action::Increment, accesskit::Action::Decrement, accesskit::Action::CustomAction] {
        assert!(node.supports_action(action), "{action:?} declared");
    }
    assert_eq!(node.numeric_value(), Some(3.0));
    let set = send(&mut rig, &mut app, request(accesskit::Action::SetValue, id, Some(accesskit::ActionData::NumericValue(8.0))));
    assert_eq!(set.frame.runs, 1, "{set:?}");
    send(&mut rig, &mut app, request(accesskit::Action::Increment, id, None));
    send(&mut rig, &mut app, request(accesskit::Action::CustomAction, id, Some(accesskit::ActionData::CustomAction(7))));
    let seen = app.seen.borrow();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[0].numeric_value(), Some(8.0));
    assert_eq!(seen[1].action, accesskit::Action::Increment);
    assert_eq!(seen[2].data, Some(accesskit::ActionData::CustomAction(7)));
}

#[test]
fn no_access_node_is_built_while_the_tree_is_off_and_turning_it_on_builds_each_once() {
    let mut app = App::new();
    let mut rig = rig(&mut app, false);
    assert_eq!(app.accessed.get(), 0, "no access hook ran with the tree off");
    assert!(!rig.driver.runtime.access_enabled(rig.main));
    let at = app.a.center();
    pointer(&mut rig, &mut app, at);
    assert_eq!(app.accessed.get(), 0);
    let main: WindowId = rig.main;
    rig.driver.runtime.set_access_enabled(main, true);
    settle(&mut rig, &mut app);
    assert_eq!(app.accessed.get(), 2, "each of the two items built its node once");
    let (_, node) = node_named(&rig, "Other");
    assert_eq!(node.role(), accesskit::Role::Button);
}

#[test]
fn an_announcement_is_one_live_status_node_cleared_in_the_next_frame() {
    let mut app = App::new();
    let mut rig = rig(&mut app, true);
    app.announce = Some("Saved".into());
    let (other, _) = node_named(&rig, "Other");
    let click = |rig: &mut Rig<RecordingRenderer>, app: &mut App| send(rig, app, request(accesskit::Action::Click, other, None));
    let said = click(&mut rig, &mut app);
    assert!(said.frame.access_nodes >= 1, "{said:?}");
    let status = |rig: &Rig<RecordingRenderer>| {
        let main = rig.main;
        rig.driver.runtime.access_status(main)
    };
    assert_eq!(status(&rig).as_deref(), Some("Saved"));
    settle(&mut rig, &mut app);
    assert_eq!(status(&rig).as_deref(), Some(""), "the status is empty again once read");
    // The same text again reads again.
    click(&mut rig, &mut app);
    assert_eq!(status(&rig).as_deref(), Some("Saved"));
}

/// A vertical scroll container.
#[derive(Component)]
#[component(container, unregistered)]
struct Pane {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Component for Pane {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PaneStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Pane {}

fn rows_ui(ui: &mut Ui<'_, '_>, app: &App) {
    let mut props = ContainerProps::new();
    props.body_gap = Gap::None;
    let props = props.scroll(Scroll::vertical()).sizing(Sizing::Fill);
    Pane { common: Common::new(), props }.show(ui, |ui, _| {
        for i in 0..300 {
            item(ui, app, &format!("row {i}"), None);
        }
    });
}

#[test]
fn rows_out_of_view_are_not_in_the_tree_and_scrolling_sends_only_those_that_enter() {
    let app = App::new();
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    rig.driver.runtime.set_access_enabled(main, true);
    let step = |rig: &mut Rig<RecordingRenderer>| rig.step_with(&mut |_, cx| window_root(cx, |ui| rows_ui(ui, &app)));
    for _ in 0..6 {
        if !step(&mut rig).ran {
            break;
        }
    }
    let rows = |rig: &Rig<RecordingRenderer>| -> Vec<String> {
        rig.driver
            .runtime
            .access_snapshot(rig.main)
            .iter()
            .filter_map(|e| e.node.label().map(str::to_owned))
            .filter(|l| l.starts_with("row "))
            .collect()
    };
    let before = rows(&rig);
    assert!(!before.is_empty() && before.len() < 20, "only the rows in view are in the tree: {}", before.len());
    rig.inject(main, Input::Wheel { pos: Pos2::new(200.0, 150.0), delta: Vec2::new(0.0, -400.0), mods: craft_ui_rt::Mods::NONE });
    let scrolled = step(&mut rig);
    let after = rows(&rig);
    assert!(after.iter().all(|r| !before.contains(r)), "rows that left view are gone, rows that entered are in");
    assert!(scrolled.frame.access_nodes as usize <= after.len() + 3, "only entering rows and their parent are sent: {scrolled:?}");
    assert_eq!(scrolled.frame.runs, 0, "no hook ran: the rows were built whole and moved by a transform");
}

/// A control shown with `PaintUi::inner` folds into its owner's node: with the tree off no fold is asked for, and
/// turning the tree on runs the owner again, so the owner's node carries the control's value text.
mod fold {
    use craft_ui_contract::contract::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Response, Role, Show as _};
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_rt::{Rect, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use craft_ui_test_kit::rt::Rig;

    #[derive(Component)]
    struct Value {
        #[common]
        common: Common,
    }

    impl Component for Value {
        const ROLE: Role = Role::SPIN_BUTTON;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: ValueStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(40.0, 20.0)
        }

        fn value_text(&self, _: &Cx<'_, Self>) -> Option<String> {
            Some("42".into())
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    #[derive(Component)]
    struct Owner {
        #[common]
        common: Common,
    }

    impl Component for Owner {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: OwnerStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(120.0, 20.0)
        }

        fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
            let rect = Rect::from_min_size(cx.rect.min, Vec2::new(40.0, 20.0));
            let _ = ui.inner("value", rect, Value { common: Common::new() });
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    thread_local! {
        /// The window's fold requests as of the last step.
        static REQUESTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    fn step(rig: &mut Rig<RecordingRenderer>) -> bool {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                let _ = Owner { common: Common::new() }.label("Size").show(ui);
                REQUESTS.with(|requests| requests.set(craft_ui_contract::testing::access::fold_requests(ui.ctx())));
            })
        })
        .ran
    }

    #[test]
    fn a_folded_control_asks_nothing_with_the_tree_off_and_folds_once_it_is_on() {
        let mut rig = Rig::inline(Vec2::new(300.0, 200.0), 1.0, RecordingRenderer::default());
        for _ in 0..4 {
            step(&mut rig);
        }
        let main = rig.main;
        assert!(rig.driver.runtime.access_snapshot(main).is_empty(), "no access node with the tree off");
        assert_eq!(REQUESTS.with(std::cell::Cell::get), 0, "no fold request with the tree off");
        rig.driver.runtime.set_access_enabled(main, true);
        for _ in 0..4 {
            step(&mut rig);
        }
        let snapshot = rig.driver.runtime.access_snapshot(main);
        let owner = snapshot.iter().find(|e| e.node.label() == Some("Size")).expect("the owner's node");
        assert_eq!(owner.node.value(), Some("42"), "the owner carries its folded control's value");
        assert!(!snapshot.iter().any(|e| e.node.role() == accesskit::Role::SpinButton), "the folded control keeps no node of its own");
        assert!(REQUESTS.with(std::cell::Cell::get) >= 1, "the owner asks for the fold once the tree is on");
    }
}
