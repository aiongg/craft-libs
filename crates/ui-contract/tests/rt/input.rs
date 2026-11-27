//! Count trials of the input half: presses on nested content, value gestures, the wheel, toggle
//! sweeps and interaction reads from responses.

use crate::skeleton::{IconButton, ScrollArea, Slider, Stack, Toggle};
use craft_behavior::IconId;
use craft_ui_contract::contract::Bind;
use craft_ui_contract::contract::{MemoView, Outbox, Ui};
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Mods, Pos2, Rect, StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;

type View<A> = fn(&mut Ui<'_, '_>, &mut A);

/// An app on a rig with access on, run until it settles. The app is boxed: a binding's key holds
/// the bound field's address, which stays put across frames in a real app.
struct Run<A> {
    rig: Rig<RecordingRenderer>,
    app: Box<A>,
    view: View<A>,
}

impl<A> Run<A> {
    fn new(app: A, view: View<A>) -> Run<A> {
        let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
        let main = rig.main;
        rig.driver.runtime.set_access_enabled(main, true);
        let mut run = Run { rig, app: Box::new(app), view };
        for _ in 0..5 {
            if !run.step().ran {
                return run;
            }
        }
        panic!("the rig did not settle");
    }

    fn step(&mut self) -> StepReport {
        let (app, view) = (&mut *self.app, self.view);
        self.rig.step_with(&mut |_, cx| window_root(cx, |ui| view(ui, app)))
    }

    fn input(&mut self, input: Input) -> StepReport {
        let main = self.rig.main;
        self.rig.inject(main, input);
        self.step()
    }

    fn counts(report: &StepReport) -> (usize, usize) {
        (report.frame.runs as usize, report.frame.scopes as usize)
    }
}

/// Two views deep: root, outer view, a stack, inner view, a stack of X and Y; Z beside the inner view.
struct Nested {
    x: Rect,
    x_clicks: u32,
}

impl Default for Nested {
    fn default() -> Nested {
        Nested { x: Rect::NOTHING, x_clicks: 0 }
    }
}

fn nested(ui: &mut Ui<'_, '_>, app: &mut Nested) {
    IconButton::new(IconId::new("plus"), "Top").show(ui);
    let mut clicks = Vec::new();
    if let Some(shown) = MemoView::new(1u32, Outbox::new(&mut clicks), outer_view).show(ui) {
        app.x = shown.value;
    }
    app.x_clicks += u32::try_from(clicks.len()).unwrap_or(u32::MAX);
}

/// The outer view: Z, and the inner view; it passes on the inner view's clicks on X.
fn outer_view(clicks: &mut Outbox<'_, ()>, ui: &mut Ui) -> Rect {
    Stack::new().show(ui, |ui| {
        IconButton::new(IconId::new("brush"), "Z").show(ui);
        let mut inner = Vec::new();
        let x = MemoView::new(2u32, Outbox::new(&mut inner), inner_view).show(ui).map_or(Rect::NOTHING, |shown| shown.value);
        for click in inner {
            clicks.send(click);
        }
        x
    })
}

/// The inner view: X and Y; it sends each click on X.
fn inner_view(clicks: &mut Outbox<'_, ()>, ui: &mut Ui) -> Rect {
    Stack::new().show(ui, |ui| {
        let x = IconButton::new(IconId::new("brush"), "X").show(ui);
        IconButton::new(IconId::new("eraser"), "Y").show(ui);
        if x.clicked() {
            clicks.send(());
        }
        x.rect
    })
}

#[test]
fn a_click_runs_the_clicked_node_and_the_scopes_on_its_path_only() {
    let mut run = Run::new(Nested::default(), nested);
    let at = run.app.x.center();
    run.input(Input::mouse_move(at));
    let down = run.input(Input::mouse_down(at));
    assert_eq!(Run::<Nested>::counts(&down), (0, 0), "the press look is a kept variant");
    let up = run.input(Input::mouse_up(at));
    assert_eq!(run.app.x_clicks, 1, "show returned the click in the frame of the release");
    assert_eq!(Run::<Nested>::counts(&up), (1, 3), "X runs; root, outer and inner view run their closures; Top, Z, Y and both stacks are kept");
}

/// A slider bound to `value` under a button.
struct Sliding {
    value: f64,
    track: Rect,
    commits: u32,
}

impl Default for Sliding {
    fn default() -> Sliding {
        Sliding { value: 0.0, track: Rect::NOTHING, commits: 0 }
    }
}

fn sliding(ui: &mut Ui<'_, '_>, app: &mut Sliding) {
    IconButton::new(IconId::new("plus"), "Other").show(ui);
    let out = Slider::new(Bind::bound(&mut app.value), 0.0..=100.0).show(ui);
    app.track = out.response.rect;
    if out.events.committed().is_some() {
        app.commits += 1;
    }
}

#[test]
fn a_slider_drag_runs_only_the_slider_per_move() {
    let mut run = Run::new(Sliding::default(), sliding);
    let track = run.app.track;
    let at = |f: f32| Pos2::new(track.min.x + f * track.width(), track.center().y);
    run.input(Input::mouse_move(at(0.25)));
    let down = run.input(Input::mouse_down(at(0.25)));
    assert_eq!(Run::<Sliding>::counts(&down), (1, 1), "a press on the track sets the value: the slider and the root run");
    assert!((run.app.value - 25.0).abs() < 1.0, "the press set the value under it: {}", run.app.value);
    for f in [0.5, 0.6, 0.75] {
        let moved = run.input(Input::mouse_move(at(f)));
        assert_eq!(Run::<Sliding>::counts(&moved), (1, 1), "each move runs the slider alone: {moved:?}");
        assert!((run.app.value - f64::from(f) * 100.0).abs() < 1.0, "the value follows the pointer: {}", run.app.value);
    }
    let up = run.input(Input::mouse_up(at(0.75)));
    assert_eq!(Run::<Sliding>::counts(&up), (1, 1));
    assert_eq!(run.app.commits, 1, "the release commits the gesture");
    let idle = run.step();
    assert!(!idle.ran, "nothing is left to run after the release");
}

/// Thirty buttons in a scroll area as tall as the window: more than it shows.
#[derive(Default)]
struct Scrolling {
    rows: Vec<Rect>,
    clicked: Option<usize>,
}

fn scrolling(ui: &mut Ui<'_, '_>, app: &mut Scrolling) {
    let rows = ScrollArea::new().show(ui, |ui| {
        (0..30)
            .map(|i| {
                let r = IconButton::new(IconId::new("brush"), &format!("{i}")).show(ui);
                if r.clicked() {
                    app.clicked = Some(i);
                }
                r.rect
            })
            .collect::<Vec<_>>()
    });
    app.rows = rows;
}

#[test]
fn a_wheel_over_a_scroll_area_runs_no_component() {
    let mut run = Run::new(Scrolling::default(), scrolling);
    let rows = run.app.rows.clone();
    let step = rows[1].min.y - rows[0].min.y;
    let wheel = run.input(Input::Wheel { pos: rows[1].center(), delta: Vec2::new(0.0, -step), mods: Mods::NONE });
    assert_eq!((wheel.frame.runs, wheel.frame.scopes, wheel.frame.built), (0, 0, 0), "the scroll node moves its content alone: {wheel:?}");
    let at = rows[1].center();
    run.input(Input::mouse_move(at));
    run.input(Input::mouse_down(at));
    run.input(Input::mouse_up(at));
    assert_eq!(run.app.clicked, Some(2), "the content moved up one row: a click where row 1 was reaches row 2");
    assert!((run.app.rows[2].center().y - at.y).abs() < 0.5, "the rebuilt rects carry the scroll offset");
}

/// Five toggles one under another.
struct Sweeping {
    on: [bool; 5],
    rects: [Rect; 5],
}

impl Default for Sweeping {
    fn default() -> Sweeping {
        Sweeping { on: [false; 5], rects: [Rect::NOTHING; 5] }
    }
}

fn sweeping(ui: &mut Ui<'_, '_>, app: &mut Sweeping) {
    for i in 0..5 {
        let out = Toggle::new(Bind::bound(&mut app.on[i])).show(ui);
        app.rects[i] = out.response.rect;
    }
}

#[test]
fn a_toggle_sweep_across_five_toggles_runs_each_once() {
    let mut run = Run::new(Sweeping::default(), sweeping);
    let rects = run.app.rects;
    let mut runs = 0;
    let mut total = |r: StepReport| {
        runs += r.frame.runs;
        r
    };
    total(run.input(Input::mouse_move(rects[0].center())));
    total(run.input(Input::mouse_down(rects[0].center())));
    let started = total(run.input(Input::mouse_move(rects[0].center() + Vec2::new(0.0, rects[0].height() / 3.0))));
    assert_eq!(started.frame.runs, 1, "the drag start runs the press owner, which takes the swept value");
    assert_eq!(run.app.on, [true, false, false, false, false]);
    total(run.input(Input::mouse_move(rects[1].center())));
    assert_eq!(run.app.on, [true, true, false, false, false]);
    let jump = total(run.input(Input::mouse_move(rects[4].center())));
    assert_eq!(jump.frame.runs, 3, "a move across three toggles marks each one it crosses");
    let back = total(run.input(Input::mouse_move(rects[0].center())));
    assert_eq!(back.frame.runs, 0, "a toggle the sweep set is not set again");
    let up = total(run.input(Input::mouse_up(rects[0].center())));
    assert_eq!(up.frame.runs, 0, "the release ends the sweep without a run");
    assert_eq!(run.app.on, [true; 5]);
    assert_eq!(runs, 5, "each toggle ran once over the whole sweep");
}

/// A view that reads its button's hover, beside a button outside it.
struct Reading {
    inside: Rect,
    outside: Rect,
    hovered: bool,
    view_runs: u32,
}

impl Default for Reading {
    fn default() -> Reading {
        Reading { inside: Rect::NOTHING, outside: Rect::NOTHING, hovered: false, view_runs: 0 }
    }
}

/// The view holding Inside: its rect and whether it reads hovered; it sends each run.
fn inside_view(ran: &mut Outbox<'_, ()>, ui: &mut Ui) -> (Rect, bool) {
    ran.send(());
    let r = IconButton::new(IconId::new("brush"), "Inside").show(ui);
    (r.rect, r.hovered())
}

fn reading(ui: &mut Ui<'_, '_>, app: &mut Reading) {
    app.outside = IconButton::new(IconId::new("plus"), "Outside").show(ui).rect;
    let mut ran = Vec::new();
    if let Some(shown) = MemoView::new(0u32, Outbox::new(&mut ran), inside_view).show(ui) {
        (app.inside, app.hovered) = shown.value;
    }
    app.view_runs += u32::try_from(ran.len()).unwrap_or(u32::MAX);
}

#[test]
fn a_hover_read_reruns_its_scope_only_when_the_hover_flips() {
    let mut run = Run::new(Reading::default(), reading);
    let (inside, outside) = (run.app.inside.center(), run.app.outside.center());
    let views = run.app.view_runs;
    let entered = run.input(Input::mouse_move(inside));
    assert_eq!(Run::<Reading>::counts(&entered), (0, 2), "the reading view and the root above it run; the button is kept: {entered:?}");
    assert!(run.app.hovered);
    assert_eq!(run.app.view_runs, views + 1);
    let within = run.input(Input::mouse_move(inside + Vec2::new(1.0, 1.0)));
    assert_eq!(Run::<Reading>::counts(&within), (0, 0), "no read flipped");
    let down = run.input(Input::mouse_down(inside));
    assert_eq!(Run::<Reading>::counts(&down), (0, 0), "the press flips a fact the view did not read");
    run.input(Input::mouse_up(inside));
    let left = run.input(Input::mouse_move(outside));
    assert_eq!(Run::<Reading>::counts(&left), (0, 2), "leaving flips the read again");
    assert!(!run.app.hovered);
    let away = run.input(Input::mouse_move(outside + Vec2::new(1.0, 1.0)));
    assert_eq!(Run::<Reading>::counts(&away), (0, 0), "a move over a button outside the view runs nothing");
}

/// Where a text field's focus goes on a press that would take it: a keeper above commits the draft and keeps the
/// focus, a pane takes it, and a canvas above keeps only its own focus.
mod press_release {
    use std::cell::Cell;
    use std::rc::Rc;

    use craft_ui_contract::contract::overlay::{Overlay, show_dialog};
    use craft_ui_contract::contract::{
        Bind, Common, Component, Container, ContainerProps, Cx, MeasureCx, Props as _, Response, Role, Show as _, ShowContainer, Ui,
    };
    use craft_ui_contract::runtime::focus::{FocusPolicy, KeyUse};
    use craft_ui_contract::style::StyleCx;
    use craft_ui_contract::testing::Harness;
    use craft_ui_rt::{Rect, Sense, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;

    use crate::text_field::TextField;

    /// A container that is a pane (`pane`) or the window's document canvas: a stop that keeps the focus on presses
    /// no shape takes.
    #[derive(Component)]
    #[component(container, unregistered)]
    struct Area {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        pane: bool,
    }

    impl Component for Area {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: AreaStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn sense(&self) -> Sense {
            if self.pane { Sense::empty() } else { Sense::HOVER | Sense::CLICK | Sense::FOCUS }
        }

        fn keys(&self) -> KeyUse {
            let mut keys = KeyUse::of(craft_ui_contract::contract::Input::Display);
            keys.chrome_key = !self.pane;
            keys
        }

        fn focus_scope(&self) -> FocusPolicy {
            if self.pane { FocusPolicy::PANE } else { FocusPolicy::NONE }
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    impl Container for Area {}

    /// A shape a press acts on that takes no focus (a swatch).
    #[derive(Component)]
    struct Tile {
        #[common]
        common: Common,
    }

    impl Component for Tile {
        const ROLE: Role = Role::GROUP;
        const STOPS: bool = false;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: TileStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(4.0 * cx.tokens.size.button, cx.tokens.size.button)
        }

        fn sense(&self) -> Sense {
            Sense::HOVER | Sense::CLICK
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    /// A field inside an [`Area`] and a tile that takes no focus beside it; the area's, the field's and the tile's
    /// rects.
    fn in_area(pane: bool) -> (Harness, Rc<Cell<bool>>, Rc<Cell<[Rect; 3]>>) {
        let committed = Rc::new(Cell::new(false));
        let rects = Rc::new(Cell::new([Rect::NOTHING; 3]));
        let (c, r) = (Rc::clone(&committed), Rc::clone(&rects));
        let mut text = String::new();
        let mut h = Harness::builder().with_size(Vec2::new(400.0, 600.0)).build_ui(move |ui: &mut Ui<'_, '_>| {
            let text = &mut text;
            let area = Area { common: Common::new(), props: ContainerProps::new(), pane };
            let shown = ShowContainer::show(area.label("Area".to_owned()), ui, |ui, _| {
                let field = TextField::new(Bind::bound(&mut *text)).show(ui);
                c.set(c.get() || field.events.committed().is_some());
                field.response.rect
            });
            let tile = Tile { common: Common::new() }.show(ui).rect;
            if let Some(field) = shown.inner {
                r.set([shown.output.rect, field, tile]);
            }
        });
        h.run();
        (h, committed, rects)
    }

    #[test]
    fn a_press_a_trapping_scope_keeps_commits_the_field_s_draft_and_keeps_its_focus() {
        let committed = Rc::new(Cell::new(false));
        let rects = Rc::new(Cell::new([Rect::NOTHING; 2]));
        let (c, r) = (Rc::clone(&committed), Rc::clone(&rects));
        let mut text = String::new();
        let mut h = Harness::builder().with_size(Vec2::new(400.0, 600.0)).build_ui(move |ui: &mut Ui<'_, '_>| {
            let text = &mut text;
            let _ = show_dialog(ui, 1, &Overlay::modal(), |ui| {
                let field = TextField::new(Bind::bound(&mut *text)).show(ui);
                c.set(c.get() || field.events.committed().is_some());
                r.set([field.response.rect, Tile { common: Common::new() }.show(ui).rect]);
            });
        });
        h.run();
        let [field, tile] = rects.get();
        h.click_at(field.center());
        h.type_text("x");
        h.run();
        let focus = h.focused().map(|target| target.id);
        assert!(focus.is_some() && !committed.get(), "the dialog's field types a draft");
        h.click_at(tile.center());
        h.run();
        assert!(committed.get(), "the press the dialog kept commits the draft");
        assert_eq!(h.focused().map(|target| target.id), focus, "and the field keeps the focus");
    }

    #[test]
    fn a_field_gives_its_focus_to_its_pane_on_a_press_that_takes_it() {
        let (mut h, committed, rects) = in_area(true);
        let [pane, field, tile] = rects.get();
        h.click_at(field.center());
        h.type_text("x");
        h.run();
        h.click_at(tile.center());
        h.run();
        assert!(committed.get(), "the field commits through its focus loss");
        assert_eq!(h.focused().map(|target| target.bounds), Some(pane), "the pane holds the focus");
    }

    #[test]
    fn a_field_on_a_canvas_gives_its_focus_up_on_a_press_outside_it() {
        let (mut h, committed, rects) = in_area(false);
        let [_, field, tile] = rects.get();
        h.click_at(field.center());
        h.type_text("x");
        h.run();
        assert!(h.focused().is_some_and(|target| target.bounds == field), "the field holds the focus");
        h.click_at(tile.center());
        h.run();
        assert!(committed.get(), "the field commits through its focus loss");
        assert!(h.focused().is_none(), "the canvas above keeps only its own focus");
    }
}
