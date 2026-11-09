//! The keyboard and wheel paths of a body: the notch, page and end keys, AccessKit scroll actions, the window
//! move and size modes, Escape through the scope's rule, and the collapse presence.

use crate::contract::kit::{PlainStyle, Probe, Seen};
use craft_behavior::CancelRule;
use craft_ui::contract::{
    BodyCx, Collapse, Common, Component, Container, ContainerProps, Cx, HeaderRegion, Mount, Overlay, Props as _, Role, Scroll, Show as _,
    ShowContainer as _, Sizing,
};
use craft_ui::runtime::commands::{self, CommandSet, FireSource, WindowCommand};
use craft_ui::runtime::focus::{FocusPolicy, FocusTarget, request};
use craft_ui::style::StyleCx;
use craft_ui::testing::access::Declared;
use craft_ui::testing::focus::{AccessActionKind, access_request};
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

const LINE: Vec2 = Vec2::new(100.0, 20.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const SAME: f32 = 0.5;
/// The views of content End has to pass.
const TWO_VIEWS: f32 = 2.0;
/// How much shorter than open a collapsed container is at least.
const COLLAPSED_BY: f32 = 70.0;

/// A container whose header closes it on request, so a trial closes a collapsible body.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Holder<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    close: &'a Cell<bool>,
}

impl<'a> Holder<'a> {
    fn new(props: ContainerProps, close: &'a Cell<bool>) -> Self {
        Holder { common: Common::new(), props, close }
    }
}

impl Component for Holder<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HolderStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Holder<'_> {
    fn header(&mut self, ui: &mut Ui, bcx: &mut BodyCx<'_>) {
        Block::new(Vec2::new(100.0, 10.0)).show(ui);
        if self.close.get() {
            bcx.request_close();
        }
    }
}

/// What a frame of `scrolling` saw.
#[derive(Default)]
struct Seen2 {
    first_top: Cell<f32>,
    view: Cell<Option<Rect>>,
    focus_id: Cell<Option<egui::Id>>,
    panel: Cell<Option<egui::Id>>,
}

/// A vertical scrolling panel of 40 focusable lines at `ORIGIN`.
fn scrolling(ui: &mut Ui, props: ContainerProps, seen: &Seen2, probes: &RefCell<Seen>) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let close = Cell::new(false);
        let out = Holder::new(props, &close).id_salt("panel").show(ui, |ui, cx| {
            seen.view.set(Some(cx.rect));
            for line in 0..40 {
                // The cursor, not the response: a response's rect is clipped to the view once the line scrolls out.
                let top = ui.next_widget_position().y;
                let response = Probe::new(probes).id_salt(line).size(LINE).show(ui);
                if line == 0 {
                    seen.first_top.set(top);
                    seen.focus_id.set(Some(response.id));
                }
            }
        });
        seen.panel.set(Some(out.output.id));
    });
}

fn moved_by(frames: &mut Frames, before: f32, seen: &Seen2, show: &mut impl FnMut(&mut Ui)) -> f32 {
    frames.idle(2, &mut *show);
    before - seen.first_top.get()
}

#[trial]
fn page_down_and_end_scroll_the_body_around_the_focused_component() {
    let (seen, probes) = (Seen2::default(), RefCell::new(Seen::default()));
    let props = ContainerProps::new().scroll(Scroll::vertical());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| scrolling(ui, props.clone(), &seen, &probes);
    frames.idle(3, &mut show);
    let top0 = seen.first_top.get();
    request(&frames.ctx, FocusTarget::Id(seen.focus_id.get().expect("a line ran")));
    frames.idle(2, &mut show);
    let after_focus = top0 - seen.first_top.get();
    frames.frame(vec![key_press(Key::PageDown, Modifiers::NONE)], &mut show);
    let page = moved_by(&mut frames, top0, &seen, &mut show) - after_focus;
    assert!((page - VIEW.y).abs() < SAME, "Page Down moved one view: {page}");
    frames.frame(vec![key_press(Key::End, Modifiers::NONE)], &mut show);
    let end = moved_by(&mut frames, top0, &seen, &mut show);
    assert!(end > TWO_VIEWS * VIEW.y, "End reached the end of the 800 points of content: {end}");
    frames.frame(vec![key_press(Key::Home, Modifiers::NONE)], &mut show);
    let home = moved_by(&mut frames, top0, &seen, &mut show);
    assert!(home.abs() < SAME, "Home returned to the start: {home}");
}

#[trial]
fn an_accesskit_scroll_action_scrolls_the_body_by_a_page() {
    let (seen, probes) = (Seen2::default(), RefCell::new(Seen::default()));
    let props = ContainerProps::new().scroll(Scroll::vertical());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| scrolling(ui, props.clone(), &seen, &probes);
    frames.idle(3, &mut show);
    let (top0, panel) = (seen.first_top.get(), seen.panel.get().expect("shown"));
    assert!(frames.tree().declares(panel, Declared::ScrollDown), "a scrolling body declares the scroll actions");
    frames.frame(vec![access_request(AccessActionKind::ScrollDown, panel)], &mut show);
    let moved = moved_by(&mut frames, top0, &seen, &mut show);
    assert!((moved - VIEW.y).abs() < SAME, "ScrollDown moved one view: {moved}");
}

fn layer_props() -> ContainerProps {
    ContainerProps::new()
        .mount(Mount::Layer(Overlay::floating().resizable(true)))
        .sizing(Sizing::Content)
        .header(HeaderRegion::new())
        .command_scope(<WindowCommand as CommandSet>::SCOPE)
}

#[trial]
fn a_click_ends_keyboard_move_mode_and_keeps_the_placement() {
    let (rect, probes) = (Cell::new(Rect::NOTHING), RefCell::new(Seen::default()));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let close = Cell::new(false);
        let out = Holder::new(layer_props(), &close).id_salt("window").show(ui, |ui, _| {
            Probe::new(&probes).size(LINE).show(ui);
        });
        rect.set(out.output.rect);
    };
    frames.idle(4, &mut show);
    let start = rect.get().min;
    request(&frames.ctx, FocusTarget::Id(probes.borrow().id.expect("the probe ran")));
    frames.idle(2, &mut show);
    assert!(commands::fire(&frames.ctx, WindowCommand::Move, FireSource::Palette), "the command has a live instance");
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    let moved = rect.get().min;
    assert!(moved.x > start.x, "an arrow moved the layer: {moved:?} from {start:?}");
    let at = rect.get().center();
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::press(at)], &mut show);
    frames.frame(vec![pointer::release(at)], &mut show);
    frames.idle(2, &mut show);
    assert!((rect.get().min - moved).abs().max_elem() < SAME, "the click kept the placement: {:?} {moved:?}", rect.get().min);
    frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert!((rect.get().min - moved).abs().max_elem() < SAME, "the click ended the mode: arrows no longer move the layer");
}

/// A floating container with commands of its own still takes the `window` commands.
#[trial]
fn a_layer_with_its_own_command_scope_still_takes_the_window_commands() {
    let (rect, probes) = (Cell::new(Rect::NOTHING), RefCell::new(Seen::default()));
    let props = layer_props().command_scope(craft_keymap::CommandScope::new("find"));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let close = Cell::new(false);
        let out = Holder::new(props.clone(), &close).id_salt("window").show(ui, |ui, _| {
            Probe::new(&probes).size(LINE).show(ui);
        });
        rect.set(out.output.rect);
    };
    frames.idle(4, &mut show);
    let start = rect.get().min;
    request(&frames.ctx, FocusTarget::Id(probes.borrow().id.expect("the probe ran")));
    frames.idle(2, &mut show);
    assert!(commands::fire(&frames.ctx, WindowCommand::Move, FireSource::Palette), "the window scope has a live instance");
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert!(rect.get().min.x > start.x, "an arrow moved the layer: {:?} from {start:?}", rect.get().min);
}

#[trial]
fn escape_never_collapses_a_collapsible_container_whatever_its_scope_rule() {
    let (ran, probes) = (Cell::new(false), RefCell::new(Seen::default()));
    let props = ContainerProps::new().collapse(Collapse::new().animated(false)).focus(FocusPolicy::PANE.cancel(CancelRule::Close));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let close = Cell::new(false);
        Holder::new(props.clone(), &close).id_salt("pane").show(ui, |ui, _| {
            ran.set(true);
            Probe::new(&probes).size(LINE).leaves_cancel().show(ui);
        });
    };
    frames.idle(3, &mut show);
    request(&frames.ctx, FocusTarget::Id(probes.borrow().id.expect("the probe ran")));
    frames.idle(2, &mut show);
    ran.set(false);
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    ran.set(false);
    frames.idle(2, &mut show);
    assert!(ran.get(), "Escape left the collapsible body open");
}

#[trial]
fn an_animated_collapse_keeps_the_body_while_it_closes() {
    let (height, ran) = (Cell::new(0.0), Cell::new(false));
    let close = Cell::new(false);
    let props = ContainerProps::new().header(HeaderRegion::new()).collapse(Collapse::new()).sizing(Sizing::Content);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        ran.set(false);
        let out = Holder::new(props.clone(), &close).id_salt("pane").show(ui, |ui, _| {
            ran.set(true);
            Block::new(Vec2::new(100.0, 80.0)).show(ui);
        });
        close.set(false);
        height.set(out.output.rect.height());
    };
    frames.idle(4, &mut show);
    let open = height.get();
    close.set(true);
    // The pass that requests the close shows the body open; the next one starts the presence from open.
    frames.frame(vec![], &mut show);
    frames.frame(vec![], &mut show);
    frames.advance(0.05);
    frames.frame(vec![], &mut show);
    assert!(ran.get() && height.get() < open && height.get() > 0.0, "partly closed: body still runs at {} of {open}", height.get());
    frames.advance(5.0);
    frames.idle(3, &mut show);
    assert!(!ran.get(), "the body stops once the presence reaches closed");
    assert!(height.get() < open - COLLAPSED_BY, "the container is as high as its header: {} of {open}", height.get());
}
