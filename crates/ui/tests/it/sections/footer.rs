//! Footer actions and buttons: drop rules, the enabled look under a drag, letters on dialog buttons and popup facts.

use crate::contract::kit::PlainStyle;
use craft_behavior::{IconId, PartKey, PressPhase, Sense};
use craft_host::target::Os;
use craft_ui::contract::{Common, Component, Cx, DragPreview, MeasureCx, PartsCx, Pointer as _, PopupKind, Props as _, Role, Show as _};
use craft_ui::sections::{DialogRole, FooterAction, FooterBar, FooterButton, FooterOutput};
use craft_ui::style::StyleCx;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Id, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

const CHIP: Rect = Rect { min: Pos2::new(10.0, 100.0), max: Pos2::new(60.0, 130.0) };
const GRIP: PartKey = PartKey::new("grip");

/// A row dragged out of the list `list`.
#[derive(Clone, Debug)]
struct Row {
    list: u8,
}

/// Starts a drag of a row of its list.
#[derive(craft_ui::contract::Component)]
struct Chip {
    #[common]
    common: Common,
    list: u8,
}

impl Component for Chip {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ChipStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        CHIP.size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(GRIP, rect).sense(Sense::DRAG).pointer_only("the grip is dragged with the pointer");
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if matches!(cx.part(GRIP).press.phase, PressPhase::DragStart { .. }) {
            cx.start_drag(Row { list: self.list }, DragPreview::None);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// What the footer showed in the last pass.
#[derive(Default)]
struct Seen {
    dropped_on: Option<usize>,
    hint: Option<String>,
    delete: Option<Id>,
}

/// A footer with one Delete action that takes the rows of list 1, shown beside a chip that drags a row of `from`.
fn scene<'a>(from: u8, enabled: bool, flagged: bool, seen: &'a RefCell<Seen>) -> impl FnMut(&mut Ui) + 'a {
    move |ui: &mut Ui| {
        Chip { common: Common::new(), list: from }.id_salt("chip").show_at(ui, CHIP);
        let list = 1u8;
        let delete = FooterAction::new(IconId::new("trash"), "Delete")
            .accepts_drops::<Row>(move |row| row.list == list)
            .enabled(enabled)
            .takes_drops_disabled(flagged)
            .drop_hint("Delete the rows");
        let out: FooterOutput = FooterBar::actions([delete]).show_footer(ui);
        let mut seen = seen.borrow_mut();
        seen.dropped_on = out.dropped.as_ref().map(|drop| drop.index);
        seen.hint = out.drop_hint.as_ref().map(|hint| hint.as_str().to_owned());
        seen.delete = out.responses.first().map(|response| response.id);
    }
}

/// Drags the chip over the first action and returns the pointer position there; the button stays down.
fn drag_over_delete(frames: &mut Frames, show: &mut impl FnMut(&mut Ui), seen: &RefCell<Seen>) -> Pos2 {
    frames.idle(3, &mut *show);
    let tree = frames.tree();
    let id = seen.borrow().delete.expect("the action showed");
    let key = craft_ui::testing::access::NodeKey::of(id);
    let over = tree.bounds_of(key).expect("the action has bounds").center();
    let from = CHIP.center();
    frames.frame(vec![moved(from)], &mut *show);
    frames.frame(vec![press(from)], &mut *show);
    for at in [from + Vec2::new(30.0, 0.0), from + Vec2::new(60.0, 10.0), over] {
        frames.frame(vec![moved(at)], &mut *show);
    }
    frames.idle(1, &mut *show);
    over
}

/// A disabled action takes no drop; one flagged `takes_drops_disabled` takes it and reports enabled while the drag is over it.
#[trial]
fn a_disabled_action_takes_a_drop_only_when_flagged_and_then_reports_enabled() {
    let plain = RefCell::new(Seen::default());
    let mut show = scene(1, false, false, &plain);
    let mut frames = Frames::new();
    let over = drag_over_delete(&mut frames, &mut show, &plain);
    let id = plain.borrow().delete.expect("the action showed");
    let tree = frames.tree();
    assert!(tree.is_disabled(id), "the disabled action stays disabled under a drag");
    frames.frame(vec![release(over)], &mut show);
    let first = plain.borrow().dropped_on;
    frames.idle(2, &mut show);
    assert!(first.is_none() && plain.borrow().dropped_on.is_none(), "a disabled action takes no drop by default");

    let flagged = RefCell::new(Seen::default());
    let mut show = scene(1, false, true, &flagged);
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    let id = flagged.borrow().delete.expect("the action showed");
    assert!(frames.tree().is_disabled(id), "it is disabled before the drag");
    let over = drag_over_delete(&mut frames, &mut show, &flagged);
    assert!(!frames.tree().is_disabled(id), "it reports enabled while the accepted drag is over it");
    frames.frame(vec![release(over)], &mut show);
    let first = flagged.borrow().dropped_on;
    frames.idle(2, &mut show);
    assert_eq!(first.or(flagged.borrow().dropped_on), Some(0), "the flagged action takes the drop");
}

/// A flagged action whose parent container is disabled is disabled by the parent, not by its own state: it takes no drop.
#[trial]
fn a_flagged_action_under_a_disabled_parent_takes_no_drop() {
    let seen = RefCell::new(Seen::default());
    let mut inner = scene(1, false, true, &seen);
    let mut show = |ui: &mut Ui| {
        ui.add_enabled_ui(false, |ui| inner(ui));
    };
    let mut frames = Frames::new();
    let over = drag_over_delete(&mut frames, &mut show, &seen);
    let id = seen.borrow().delete.expect("the action showed");
    assert!(frames.tree().is_disabled(id), "it stays disabled under the disabled parent while the drag is over it");
    assert!(seen.borrow().hint.is_none(), "the disabled parent shows no drop hint");
    frames.frame(vec![release(over)], &mut show);
    let first = seen.borrow().dropped_on;
    frames.idle(2, &mut show);
    assert!(first.is_none() && seen.borrow().dropped_on.is_none(), "the flagged action takes no drop under a disabled parent");
}

