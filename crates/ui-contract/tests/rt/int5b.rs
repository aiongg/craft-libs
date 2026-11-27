//! Trials of the id map and the popup frame on craft-ui-rt (INT-5b): an access relation names a
//! sibling shown after its owner, and a popup's content reads the popup's own frame and the
//! owner's direction. Relations only, no times.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_i18n::Direction;
use craft_ui_contract::contract::overlay::Overlay;
use craft_ui_contract::contract::rt::access::Access;
use craft_ui_contract::contract::rt::{
    Component, ComponentKey, Cx, EnvScope, LayerFrame, MeasureCx, Paint, PopupHandle, Response, Ui, show_leaf, window_root,
};
use craft_ui_contract::contract::{Id, IdSource, MemoKey};
use craft_ui_platform_headless::HeadlessPlatform;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Platform, Rect, Runtime, StepReport, Vec2, WindowSpec};
use craft_ui_test_kit::rt::{Rig, test_tokens};

fn name_id() -> Id {
    Id::NULL.with("int5b-name")
}

/// A leaf named `label`, keyed by `id` when it has one, and labelled by `labelled_by`.
struct Node {
    label: &'static str,
    id: Option<Id>,
    labelled_by: Option<Id>,
    runs: Rc<Cell<u32>>,
}

impl ComponentKey for Node {
    fn key(&self, key: &mut MemoKey) {
        key.feed_hash(&(self.label, self.id.map(|i| i.value()), self.labelled_by.map(|i| i.value())));
    }
}

impl Component for Node {
    const ROLE: accesskit::Role = accesskit::Role::TextInput;
    type Output = Response;

    fn id_source(&self) -> IdSource {
        self.id.map_or(IdSource::Auto, IdSource::Id)
    }

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(60.0, 20.0)
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.label(self.label);
        if let Some(id) = self.labelled_by {
            access.labelled_by(id);
        }
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Labels {
    show_name: bool,
    field_runs: Rc<Cell<u32>>,
}

fn labels_ui(ui: &mut Ui<'_, '_>, app: &mut Labels) {
    show_leaf(Node { label: "Field", id: None, labelled_by: Some(name_id()), runs: app.field_runs.clone() }, ui);
    if app.show_name {
        show_leaf(Node { label: "Name", id: Some(name_id()), labelled_by: None, runs: Rc::default() }, ui);
    }
}

fn labels_step(rig: &mut Rig<RecordingRenderer>, app: &mut Labels) -> StepReport {
    rig.step_with(&mut |_, cx| window_root(cx, |ui| labels_ui(ui, app)))
}

fn labels_rig(app: &mut Labels) -> Rig<RecordingRenderer> {
    let mut platform = HeadlessPlatform::new();
    platform.set_access_active(true);
    let main = platform.create_window(&WindowSpec { size: Some(Vec2::new(400.0, 300.0)), ..WindowSpec::new("main") });
    let mut runtime = Runtime::new(test_tokens());
    runtime.set_inline_root(main);
    let mut rig = Rig::from_parts(runtime, platform, RecordingRenderer::default());
    for _ in 0..6 {
        if !labels_step(&mut rig, app).ran {
            return rig;
        }
    }
    panic!("the rig did not settle");
}

/// The nodes `label`'s access node names as its labels, by their own names.
fn labelled_by(rig: &Rig<RecordingRenderer>, label: &str) -> Vec<String> {
    let snapshot = rig.driver.runtime.access_snapshot(rig.main);
    let node = snapshot.iter().find(|e| e.node.label() == Some(label)).unwrap_or_else(|| panic!("no node named {label}"));
    node.node
        .labelled_by()
        .iter()
        .map(|id| snapshot.iter().find(|e| e.id == *id).and_then(|e| e.node.label().map(str::to_owned)).unwrap_or_else(|| "<none>".into()))
        .collect()
}

#[test]
fn a_label_shown_after_its_field_names_it_and_one_that_appears_later_does_too() {
    let mut app = Labels { show_name: true, field_runs: Rc::default() };
    let rig = labels_rig(&mut app);
    assert_eq!(labelled_by(&rig, "Field"), ["Name"], "a later sibling resolves through the id map");

    let mut app = Labels { show_name: false, field_runs: Rc::default() };
    let mut rig2 = labels_rig(&mut app);
    assert!(labelled_by(&rig2, "Field").is_empty(), "no node answers to the id yet");
    let before = app.field_runs.get();
    app.show_name = true;
    labels_step(&mut rig2, &mut app);
    assert_eq!(app.field_runs.get(), before, "the field did not run again");
    assert_eq!(labelled_by(&rig2, "Field"), ["Name"], "its access node was rebuilt when the label appeared");
}

thread_local! {
    static SEEN: RefCell<Option<(Option<LayerFrame>, Direction, Rect)>> = const { RefCell::new(None) };
}

/// Records what the popup's content reads in paint.
struct Probe;

impl ComponentKey for Probe {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for Probe {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(80.0, 24.0)
    }

    fn paint(&mut self, paint: &Paint<'_>, cx: &Cx<'_, Self>) {
        let _ = cx;
        SEEN.with(|s| *s.borrow_mut() = Some((paint.frame(), paint.direction(), paint.bounds())));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Opens a popup with a [`Probe`] on its first run.
struct Opener;

impl ComponentKey for Opener {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for Opener {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = Response;

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::splat(cx.tokens.size.button)
    }

    fn overlay(&mut self, ui: &mut craft_ui_contract::contract::rt::OverlayUi<'_, '_>) {
        let popup = PopupHandle::new("probe");
        ui.set_open(popup, true);
        ui.show(popup, &Overlay::popover(), |ui| {
            show_leaf(Probe, ui);
        });
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_popup_gives_its_content_its_own_frame_and_the_owners_direction() {
    let mut rig = Rig::inline(Vec2::new(400.0, 400.0), 1.0, RecordingRenderer::default());
    for _ in 0..6 {
        let step = rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                EnvScope::new().direction(Direction::Rtl).show(ui, |ui| {
                    show_leaf(Opener, ui);
                });
            })
        });
        if !step.ran {
            break;
        }
    }
    let (frame, direction, bounds) = SEEN.with(|s| s.borrow().clone()).expect("the popup's content painted");
    assert_eq!(direction, Direction::Rtl, "the popup reads the direction of the scope its owner is in");
    let frame = frame.expect("the popup's content is inside the popup's rounded frame");
    assert!(frame.rect.contains_rect(bounds), "the frame {:?} holds the content {:?}", frame.rect, bounds);
    assert!(frame.radii.top_left > 0.0, "the frame carries the popup surface's radii");
}
