//! Rules of the pipeline's steps: the order the hooks run in, sizing passes, `measure_in`, activation, ids, focus
//! on a part that goes away, the body size and the ink and app values a body hands down.

use super::kit::{PartSpec, PlainStyle, Probe, Seen};
use craft_behavior::{Proposal, Sense};
use craft_theme::Stateful;
use craft_ui::contract::{
    Common, Component, Container, ContainerProps, Cx, Focus as _, Looks, MeasureCx, OverlayUi, PaintUi, PartsCx, Props as _, Role, Show as _,
    ShowContainer as _, Sizing,
};
use craft_ui::runtime::env::{AppValues, EnvValue};
use craft_ui::runtime::install;
use craft_ui::style::{ComponentStyle, Ink, StateSet, StyleCx, SurfaceKind, SurfaceLook};
use craft_ui::testing::contract::{allocate, clicked as is_clicked, clicked_by_primary, findings};
use craft_ui::testing::focus::named_id;
use craft_ui::testing::input::key_press;
use craft_ui::testing::memory::scoped;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Response, Ui, UiBuilder, Vec2};
use std::cell::RefCell;

type Log = RefCell<Vec<&'static str>>;

/// A style-affecting app value the body hands to its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnvValue)]
#[env(style, root = Tone(1))]
struct Tone(u8);

/// A body that logs every hook the pipeline calls and can hand `Tone` to its body.
#[derive(Component)]
#[component(container, overlay)]
struct Hooked<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    log: &'a Log,
    tone: Option<u8>,
}

impl<'a> Hooked<'a> {
    fn new(log: &'a Log) -> Self {
        Hooked { common: Common::new(), props: ContainerProps::new().sizing(Sizing::Content), log, tone: None }
    }

    fn mark(&self, hook: &'static str) {
        self.log.borrow_mut().push(hook);
    }
}

impl Component for Hooked<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HookedStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        self.mark("measure");
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, _: Rect, _: &mut PartsCx<'_, Self>) {
        self.mark("parts");
    }

    fn content_env(&self, _: &Cx<'_, Self>, values: &mut AppValues) {
        self.mark("content_env");
        if let Some(tone) = self.tone {
            values.set(Tone(tone));
        }
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        self.mark("update");
    }

    fn overlay(&mut self, _: &mut OverlayUi<'_>, _: &mut Cx<'_, Self>) {
        self.mark("overlay");
    }

    fn flags(&self) -> craft_ui::style::Flags {
        self.mark("flags");
        craft_ui::style::Flags::empty()
    }

    fn activated(&self, _: &Cx<'_, Self>) -> bool {
        self.mark("activated");
        false
    }

    fn paint(&mut self, _: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        self.mark("paint");
    }

    fn paint_over(&mut self, _: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        self.mark("paint_over");
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        self.mark("finish");
        cx.response.clone()
    }
}

impl Container for Hooked<'_> {}

#[trial]
fn hooks_run_in_pipeline_order_with_the_body_between_paint_and_paint_over() {
    let mut frames = Frames::new();
    let log: Log = RefCell::new(Vec::new());
    frames.frame(vec![], |ui| {
        log.borrow_mut().clear();
        Hooked::new(&log).show(ui, |_, _| log.borrow_mut().push("body"));
    });
    assert_eq!(
        *log.borrow(),
        ["measure", "parts", "update", "overlay", "flags", "content_env", "paint", "body", "paint_over", "activated", "finish"]
    );
}

/// A sizing pass of the parent `Ui` allocates the size and runs nothing else.
#[trial]
fn a_sizing_pass_registers_nothing_and_its_id_is_the_real_passs() {
    let sized_id = {
        let mut frames = Frames::new();
        let seen = RefCell::new(Seen::default());
        let mut id = None;
        frames.frame(vec![], |ui| {
            id = Some(ui.scope_builder(UiBuilder::new().sizing_pass(), |ui| Probe::new(&seen).show(ui)).inner.id);
        });
        assert_eq!(seen.borrow().updates, 0, "a sizing pass runs no update");
        id
    };
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut real_id = None;
    frames.frame(vec![], |ui| {
        real_id = Some(scoped(ui, |ui| Probe::new(&seen).show(ui)).id);
    });
    assert_eq!(sized_id, real_id, "the Auto ordinal of a sizing pass is the real pass's");

    // The id a sizing pass used is free for the real show in the same pass.
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let shared = named_id("shared");
    frames.frame(vec![], |ui| {
        ui.scope_builder(UiBuilder::new().sizing_pass(), |ui| Probe::new(&seen).id(shared).show(ui));
        Probe::new(&seen).id(shared).show(ui);
    });
    assert!(findings(&frames.ctx).is_empty(), "a sizing pass registers no id: {:?}", findings(&frames.ctx));
}

#[trial]
fn showing_one_id_twice_in_a_pass_is_a_duplicate_id_finding() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let shared = named_id("shared");
    // The second show also trips the debug assertion that one id opens one memory stage per pass; the finding is
    // recorded before it.
    let shown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        frames.frame(vec![], |ui| {
            Probe::new(&seen).id(shared).show(ui);
            Probe::new(&seen).id(shared).show(ui);
        });
    }));
    assert_eq!(shown.is_err(), cfg!(debug_assertions), "the second stage of one id is a debug assertion");
    let found = findings(&frames.ctx);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].rule, "duplicate-id");
}

/// A leaf that records the id its `measure` and `update` ran under.
#[derive(Component)]
struct Spy<'a> {
    #[common]
    common: Common,
    ids: &'a RefCell<(Vec<Id>, Vec<Id>)>,
}

impl Component for Spy<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SpyStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        self.ids.borrow_mut().0.push(cx.resolved.id);
        Vec2::new(20.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.ids.borrow_mut().1.push(cx.id);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn measure_in_uses_the_id_the_next_show_uses_and_does_not_advance_the_ordinal() {
    let mut frames = Frames::new();
    let ids = RefCell::new((Vec::new(), Vec::new()));
    frames.frame(vec![], |ui| {
        ids.borrow_mut().0.clear();
        ids.borrow_mut().1.clear();
        let mut spy = Spy { common: Common::new(), ids: &ids };
        spy.measure_in(ui, Proposal::Ideal);
        spy.measure_in(ui, Proposal::Ideal);
        spy.show(ui);
    });
    let (measured, updated) = &*ids.borrow();
    assert_eq!(measured.len(), 3);
    assert!(measured.iter().all(|id| *id == measured[0]), "both measurements and the show share one id: {measured:?}");
    assert_eq!(updated.last(), measured.last());
}

/// Step 17 rewrites the click flags: keyboard activation reports a click, a command-bound component none.
#[trial]
fn the_click_flags_follow_how_the_component_was_activated() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut clicked = false;
    let show = |ui: &mut Ui, command: bool, out: &mut bool, by_pointer: &mut bool| {
        let probe = Probe::new(&seen).id_salt("probe");
        let response = if command { probe.command("cmd").show(ui) } else { probe.show(ui) };
        *out = is_clicked(&response);
        *by_pointer = clicked_by_primary(&response);
    };
    let mut by_pointer = false;
    frames.frame(vec![], |ui| show(ui, false, &mut clicked, &mut by_pointer));
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], |ui| show(ui, false, &mut clicked, &mut by_pointer));
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], |ui| show(ui, false, &mut clicked, &mut by_pointer));
    assert!(clicked, "Enter on the focused component reports a click");
    assert!(!by_pointer, "and not a pointer click");

    let mut bound = Frames::new();
    let seen = RefCell::new(Seen::default());
    let at = Pos2::new(10.0, 10.0);
    let show_bound = |ui: &mut Ui, out: &mut bool, by_pointer: &mut bool| {
        let response = Probe::new(&seen).id_salt("probe").command("cmd").show(ui);
        *out = is_clicked(&response);
        *by_pointer = clicked_by_primary(&response);
    };
    bound.frame(vec![moved(at)], |ui| show_bound(ui, &mut clicked, &mut by_pointer));
    bound.frame(vec![press(at)], |ui| show_bound(ui, &mut clicked, &mut by_pointer));
    bound.frame(vec![release(at)], |ui| show_bound(ui, &mut clicked, &mut by_pointer));
    assert!(!clicked && !by_pointer, "a command-bound component reports no click; the command fires");
}

/// A focused part that is not declared the next pass is no longer active.
#[trial]
fn a_focused_part_that_disappears_is_no_longer_active() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let key = craft_behavior::PartKey::new("a");
    let parts = [PartSpec { key, at: Rect::from_min_size(Pos2::ZERO, Vec2::new(20.0, 20.0)), sense: Sense::CLICK | Sense::FOCUS, focusable: true }];
    let declared = std::cell::Cell::new(true);
    let mut show = |ui: &mut Ui| {
        let probe = Probe::new(&seen).sense(Sense::HOVER).id_salt("probe");
        let probe = if declared.get() { probe.parts(&parts) } else { probe };
        probe.show(ui);
    };
    frames.frame(vec![], &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert!(seen.borrow().active.is_some(), "the part is active while declared");
    declared.set(false);
    frames.frame(vec![], &mut show);
    assert_eq!(seen.borrow().active, None, "the part left, so no part is active");
}

/// Frames of a `Hooked` body with a minimum size larger than its padding; `fill` makes the body take all its room.
fn body_sizes(fill: bool) -> (Vec<Vec2>, Vec2) {
    let mut looks = Looks::new();
    looks.style::<Hooked<'static>>(|_, _| PlainStyle::min(Vec2::new(200.0, 100.0)));
    let mut frames = Frames::uninstalled();
    assert!(install(&frames.ctx, craft_ui::testing::install_spec().looks(looks)).is_ok());
    let log: Log = RefCell::new(Vec::new());
    let first_room = RefCell::new(None);
    let mut sizes = Vec::new();
    for _ in 0..6 {
        frames.frame(vec![], |ui| {
            let out = Hooked::new(&log).id_salt("panel").show(ui, |ui, _| {
                first_room.borrow_mut().get_or_insert(ui.available_size());
                let size = if fill { ui.available_size().max(Vec2::ZERO) } else { Vec2::new(50.0, 30.0) };
                allocate(ui, size);
            });
            sizes.push(out.output.rect.size());
        });
    }
    (sizes, first_room.borrow().expect("the body ran"))
}

#[trial]
fn a_body_that_fills_its_room_does_not_grow_past_the_room_it_first_had() {
    let (sizes, room) = body_sizes(true);
    assert!(room.x > 0.0 && room.y > 0.0, "the first frame's body had room: {room:?}");
    let last = sizes.len() - 1;
    assert_eq!(sizes[last], sizes[last - 1], "the size settled: {sizes:?}");
    assert!(sizes[last].x <= room.x && sizes[last].y <= room.y, "the body is no larger than its room {room:?}: {sizes:?}");
}

#[trial]
fn a_body_smaller_than_the_minimum_size_takes_the_minimum_size() {
    let (sizes, _) = body_sizes(false);
    assert_eq!(sizes[sizes.len() - 1], Vec2::new(200.0, 100.0), "{sizes:?}");
}

/// A leaf on a `Scrim` surface.
#[derive(Component)]
struct Inked<'a> {
    #[common]
    common: Common,
    ink: &'a RefCell<Option<Ink>>,
}

#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct InkStyle {
    #[style(surface)]
    look: SurfaceLook,
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

impl Component for Inked<'_> {
    const ROLE: Role = Role::None;
    type Style = InkStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: InkedStyleKey) -> InkStyle {
        InkStyle {
            look: SurfaceLook::new(Stateful::new(cx.ink(Ink::OnPanel))).kind(SurfaceKind::Scrim),
            metrics: craft_behavior::Metrics::new().min_size(Vec2::new(20.0, 20.0)),
        }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn paint(&mut self, _: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        use craft_ui::contract::Styled as _;
        *self.ink.borrow_mut() = Some(cx.ink());
        let _: &StateSet = cx.state();
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_component_on_a_scrim_surface_reads_the_scrims_ink() {
    let mut frames = Frames::new();
    let ink = RefCell::new(None);
    frames.frame(vec![], |ui| {
        Inked { common: Common::new(), ink: &ink }.show(ui);
    });
    assert_eq!(*ink.borrow(), Some(Ink::OnDark), "the root ink on a panel is {:?}", Ink::OnPanel);
}

/// A leaf that declares `PARTS = false` and then declares a part.
#[derive(Component)]
struct Liar {
    #[common]
    common: Common,
}

impl Component for Liar {
    const ROLE: Role = Role::None;
    const PARTS: bool = false;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: LiarStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::splat(10.0))
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(craft_behavior::PartKey::new("a"), Rect::from_min_size(Pos2::ZERO, Vec2::splat(4.0)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_component_that_declares_no_parts_and_declares_one_is_a_debug_assertion() {
    let mut frames = Frames::new();
    let shown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        frames.frame(vec![], |ui| {
            Liar { common: Common::new() }.id_salt("liar").show(ui);
        });
    }));
    assert_eq!(shown.is_err(), cfg!(debug_assertions), "the declaration `PARTS = false` is checked in debug builds");
}

/// A focusable leaf that takes the arrow moves routed to it and declares no `moves`.
#[derive(Component)]
struct Stepper {
    #[common]
    common: Common,
}

impl Component for Stepper {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: StepperStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::splat(10.0))
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        while cx.intents().take_move().is_some() {}
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_component_that_takes_an_arrow_move_without_declaring_moves_is_a_debug_assertion() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Stepper { common: Common::new() }.id_salt("stepper").show(ui);
    };
    frames.frame(vec![], &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    let moved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE)], &mut show);
    }));
    assert_eq!(moved.is_err(), cfg!(debug_assertions), "taking `Intent::Move` needs `moves` in the key use, checked in debug builds");
}

/// A leaf whose width follows `Tone`.
#[derive(Component)]
struct Chip {
    #[common]
    common: Common,
}

impl Component for Chip {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: ChipStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::splat(f32::from(cx.env::<Tone>().0) * 10.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_style_value_set_in_content_env_changes_a_body_childs_style() {
    let mut frames = Frames::new();
    let log: Log = RefCell::new(Vec::new());
    let (mut outside, mut inside) = (0.0, 0.0);
    frames.frame(vec![], |ui| {
        outside = Chip { common: Common::new() }.id_salt("outside").show(ui).rect.width();
        let mut body = Hooked::new(&log).id_salt("scoped");
        body.tone = Some(5);
        body.show(ui, |ui, _| inside = Chip { common: Common::new() }.id_salt("inside").show(ui).rect.width());
    });
    assert!(inside > outside, "the body's child read the body's value: {outside} -> {inside}");
}
