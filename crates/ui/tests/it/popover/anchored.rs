//! `Popover`: the close reason and the open binding written when a close rule fires.

use craft_ui::buttons::Button;
use craft_ui::contract::overlay::{Dismiss, Outside};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::popover::Popover;
use craft_ui::runtime::focus::{FocusTarget, Initial, request};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers, Pos2};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    open: Rc<Cell<bool>>,
    closed: Rc<Cell<Option<CloseReason>>>,
}

fn rig() -> Rig {
    let (open, closed) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(None)));
    let (shown_open, shown_closed) = (open.clone(), closed.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut is_open = shown_open.get();
        let shown = Popover::new("Brush options", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            Button::new("Reset").show(ui);
        });
        shown_open.set(is_open);
        if shown.output.closed.is_some() {
            shown_closed.set(shown.output.closed);
        }
    });
    fixture.settle();
    Rig { fixture, open, closed }
}

/// Escape with the focus inside closes the popover, clears the caller's binding and reports `Escape` once.
#[trial]
fn escape_closes_a_popover_and_reports_why() {
    let mut rig = rig();
    assert!(rig.open.get(), "the popover starts open");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    rig.fixture.idle(30);
    assert!(!rig.open.get(), "Escape clears the open binding");
    assert_eq!(rig.closed.get(), Some(CloseReason::Escape));
}

/// 6.3: a container layer opens at its final width on its first visible frame and asks for at most one discard
/// pass: the first sizing pass measures the content's ideal width instead of growing from none.
#[trial]
fn a_popover_opens_at_its_final_width_with_one_discard_pass() {
    let open = Rc::new(Cell::new(false));
    let shown_open = open.clone();
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Brush").show(ui);
        let mut is_open = shown_open.get();
        Popover::new("Brush options", Bind::bound(&mut is_open)).opened_by(&opener).show(ui, |ui, _| {
            Button::new("Reset the brush to its defaults").show(ui);
            Button::new("Apply").show(ui);
        });
        shown_open.set(is_open);
    });
    fixture.settle();
    open.set(true);
    let layer_width = |fixture: &Fixture| craft_ui::testing::frames::widest_layer(fixture.ctx());
    let tolerance = 0.5;
    let mut widths = Vec::new();
    let mut passes = Vec::new();
    for _ in 0..4 {
        fixture.frame(Vec::new());
        widths.push(layer_width(&fixture));
        passes.push(fixture.passes());
    }
    assert!(widths[0] > 0.0, "the layer shows on the first frame: {widths:?} {passes:?}");
    assert!(
        widths.iter().all(|width| (width - widths[0]).abs() < tolerance),
        "the layer keeps one width from its first frame: {widths:?} {passes:?}"
    );
    assert!(passes[0] <= 2, "the opening frame asks for at most one discard pass: {passes:?}");
}

/// A popover over two buttons: the opener "Brush" and another control "Other" beside it.
struct Scene {
    fixture: Fixture,
    open: Rc<Cell<bool>>,
    closed: Rc<Cell<Option<CloseReason>>>,
    pressed_outside: Rc<Cell<bool>>,
    opener_enabled: Rc<Cell<bool>>,
    opener: Rc<Cell<Id>>,
}

/// What the scene's popover is built from: its setters and whether its body has a focusable control.
struct Build {
    setters: fn(Popover<'_>) -> Popover<'_>,
    focusable_body: bool,
    at_point: Option<Pos2>,
}

fn open_scene(build: Build) -> Scene {
    let (open, closed) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(None)));
    let (pressed_outside, opener_enabled, opener_id) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(true)), Rc::new(Cell::new(Id::NULL)));
    let (shown_open, shown_closed, shown_outside, shown_enabled, shown_opener) =
        (open.clone(), closed.clone(), pressed_outside.clone(), opener_enabled.clone(), opener_id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let opener = ui
            .horizontal(|ui| {
                let opener = ui.add_enabled_ui(shown_enabled.get(), |ui| Button::new("Brush").show(ui)).inner;
                Button::new("Other").show(ui);
                opener
            })
            .inner;
        shown_opener.set(opener.id);
        let mut is_open = shown_open.get();
        let popover = Popover::new("Brush options", Bind::bound(&mut is_open));
        let popover = match build.at_point {
            Some(at) => popover.at_point(at),
            None => popover.opened_by(&opener),
        };
        let shown = (build.setters)(popover).show(ui, |ui, _| {
            if build.focusable_body {
                Button::new("Reset").show(ui);
            }
        });
        shown_open.set(is_open);
        if shown.output.closed.is_some() {
            shown_closed.set(shown.output.closed);
        }
        if shown.output.pressed_outside {
            shown_outside.set(true);
        }
    });
    fixture.settle();
    Scene { fixture, open, closed, pressed_outside, opener_enabled, opener: opener_id }
}

impl Scene {
    fn focus_other(&mut self) {
        let other = self.fixture.by_label("Other").expect("the other control is shown");
        self.fixture.access(AccessActionKind::Focus, &other);
        self.fixture.idle(2);
    }

    fn focus_opener(&mut self) {
        request(self.fixture.ctx(), FocusTarget::Id(self.opener.get()));
        self.fixture.idle(2);
    }
}

/// A popover whose body has nothing to focus, opened while another control has the focus, stays open: the focus was
/// never inside it, so it cannot have left.
#[trial]
fn a_popover_without_a_focusable_body_stays_open_over_passes() {
    let mut scene = open_scene(Build { setters: |popover| popover, focusable_body: false, at_point: None });
    scene.focus_other();
    scene.open.set(true);
    scene.fixture.idle(10);
    assert!(scene.open.get(), "the popover is still open");
    assert_eq!(scene.closed.get(), None, "no close rule fired");
}

/// A popover that keeps the focus on its opener stays open while the focus is there, closes with `FocusLeft` once
/// it moves to another control, and Escape on the opener closes it.
#[trial]
fn a_keep_anchor_popover_follows_the_focus_of_its_opener() {
    let keep_anchor = Build { setters: |popover| popover.initial_focus(Initial::KeepAnchor), focusable_body: true, at_point: None };
    let mut scene = open_scene(keep_anchor);
    scene.focus_opener();
    scene.open.set(true);
    scene.fixture.idle(10);
    assert!(scene.open.get(), "the popover stays open while the focus is on its opener");
    scene.focus_other();
    scene.fixture.idle(3);
    assert!(!scene.open.get(), "the focus moving to another control closes it");
    assert_eq!(scene.closed.get(), Some(CloseReason::FocusLeft));

    let keep_anchor = Build { setters: |popover| popover.initial_focus(Initial::KeepAnchor), focusable_body: true, at_point: None };
    let mut scene = open_scene(keep_anchor);
    scene.focus_opener();
    scene.open.set(true);
    scene.fixture.idle(10);
    scene.fixture.key(Key::Escape, Modifiers::NONE);
    scene.fixture.idle(30);
    assert!(!scene.open.get(), "Escape with the focus on the opener closes the popover");
    assert_eq!(scene.closed.get(), Some(CloseReason::Escape));
}

/// A disabled opener closes its popover with `AnchorGone`.
#[trial]
fn disabling_the_opener_closes_its_popover() {
    let mut scene = open_scene(Build { setters: |popover| popover, focusable_body: true, at_point: None });
    scene.open.set(true);
    scene.fixture.idle(5);
    assert!(scene.open.get(), "the popover is open");
    scene.opener_enabled.set(false);
    scene.fixture.idle(5);
    assert!(!scene.open.get(), "the opener turned disabled");
    assert_eq!(scene.closed.get(), Some(CloseReason::AnchorGone));
}

/// Under `Outside::Keep` a press outside leaves the popover open and says so in the output.
#[trial]
fn a_kept_popover_reports_a_press_outside() {
    let keep = Build { setters: |popover| popover.dismiss(Dismiss::new().outside(Outside::Keep)), focusable_body: true, at_point: None };
    let mut scene = open_scene(keep);
    scene.open.set(true);
    scene.fixture.idle(5);
    assert!(!scene.pressed_outside.get(), "nothing was pressed yet");
    let other = scene.fixture.by_label("Other").expect("the other control is shown");
    scene.fixture.click(&other);
    scene.fixture.idle(3);
    assert!(scene.pressed_outside.get(), "the press outside is reported");
    assert!(scene.open.get(), "the popover stays open for the caller to decide");
    assert_eq!(scene.closed.get(), None);
}

/// A popover at a point hangs from that point.
#[trial]
fn a_popover_at_a_point_opens_there() {
    let at = Pos2::new(120.0, 140.0);
    let mut scene = open_scene(Build { setters: |popover| popover, focusable_body: true, at_point: Some(at) });
    scene.open.set(true);
    scene.fixture.idle(5);
    let layer = scene.fixture.by_role(Role::DIALOG).first().copied().expect("the popover is shown");
    let tolerance = 2.0;
    assert!((layer.rect.min.x - at.x).abs() < tolerance, "the popover starts at the point's x: {:?}", layer.rect);
    assert!(layer.rect.min.y >= at.y - tolerance && layer.rect.min.y < at.y + 40.0, "the popover hangs below the point: {:?}", layer.rect);
}
