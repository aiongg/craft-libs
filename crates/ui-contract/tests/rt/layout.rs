//! Count trials of containers, scopes and layout on craft-ui-rt: scroll offsets move kept content,
//! `Lazy` builds only rows entering view, a container's header change stays inside it, a measuring
//! run asks kept children for their size, a click in a footer reuses the body's plan, a scroll body
//! is as tall as its content until it outgrows the room, a kept `MemoView` repeats no event, and a column group
//! settles in its first frame.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use craft_behavior::{ColumnKey, Sense};
use craft_ui::contract::overlay::{PlacementChange, PlacementKind};
use craft_ui::contract::{
    BodyCx, Collapse, Common, Component, Container, ContainerProps, Cx, FooterRegion, Gap, HeaderRegion, Lazy, MeasureCx, MemoKey, MemoProps,
    MemoView, Mount, Overlay, PaneEdge, Response, RowAlign, Scroll, ShellPane, Show as _, ShowContainer as _, Sizing, Ui,
};
use craft_ui::layout::{BodyLayout, ColumnGroup};
use craft_ui::runtime::edit::OuterEdit;
use craft_ui::runtime::windows::{WindowEnv, window_root_with};
use craft_ui::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{BuildCx, Mods, Pos2, Rect, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

/// A leaf `h` high that counts its `measure` calls and reports clicks.
#[derive(craft_ui::contract::Component)]
#[component(unregistered)]
struct Chip {
    #[common]
    common: Common,
    label: String,
    h: u32,
    #[key(skip)]
    measures: Rc<Cell<u32>>,
}

impl Component for Chip {
    const ROLE: craft_ui::contract::Role = craft_ui::contract::Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ChipStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        self.measures.set(self.measures.get() + 1);
        Vec2::new(40.0, self.h as f32)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn chip(ui: &mut Ui, label: &str, h: f32, measures: &Rc<Cell<u32>>) -> Response {
    Chip { common: Common::new(), label: label.into(), h: h as u32, measures: measures.clone() }.show(ui)
}

/// A container with the given props, a header whose text sets its height (ten points a line) and a
/// footer with one chip when `footer` is set.
#[derive(craft_ui::contract::Component)]
#[component(container, unregistered)]
struct Boxed {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    title: Option<String>,
    footer: bool,
    #[key(skip)]
    measures: Rc<Cell<u32>>,
    #[key(skip)]
    footer_rect: Rc<Cell<Rect>>,
}

impl Boxed {
    fn new(props: ContainerProps, measures: &Rc<Cell<u32>>) -> Boxed {
        Boxed { common: Common::new(), props, title: None, footer: false, measures: measures.clone(), footer_rect: Rc::new(Cell::new(Rect::NOTHING)) }
    }
}

impl Component for Boxed {
    const ROLE: craft_ui::contract::Role = craft_ui::contract::Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BoxedStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Boxed {
    fn header(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        if let Some(title) = &self.title {
            let lines = title.lines().count() as f32;
            chip(ui, title, 10.0 * lines, &self.measures);
        }
    }

    fn footer(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        if self.footer {
            self.footer_rect.set(chip(ui, "Done", ROW, &self.measures).rect);
        }
    }
}

fn props(f: impl FnOnce(ContainerProps) -> ContainerProps) -> ContainerProps {
    let mut props = ContainerProps::new();
    props.body_gap = Gap::None;
    f(props)
}

fn rig() -> Rig<RecordingRenderer> {
    Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default())
}

fn root<R>(cx: &mut BuildCx<'_>, f: impl FnOnce(&mut Ui) -> R) -> R {
    window_root_with(cx, WindowEnv::default(), f)
}

fn settle(rig: &mut Rig<RecordingRenderer>, step: &mut dyn FnMut(&mut Rig<RecordingRenderer>) -> StepReport) {
    for _ in 0..6 {
        if !step(rig).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn click(rig: &mut Rig<RecordingRenderer>, at: Pos2, step: &mut dyn FnMut(&mut Rig<RecordingRenderer>) -> StepReport) -> StepReport {
    let main = rig.main;
    rig.inject(main, Input::mouse_move(at));
    step(rig);
    rig.inject(main, Input::mouse_down(at));
    step(rig);
    rig.inject(main, Input::mouse_up(at));
    step(rig)
}

const ROW: f32 = 20.0;
const ROWS: usize = 600;

/// The rows whose `Lazy` ran in the last step.
#[derive(Default)]
struct List {
    built: BTreeSet<usize>,
    measures: Rc<Cell<u32>>,
    lazy: bool,
    /// The list container's `Id`, from its last show.
    id: Option<craft_ui::contract::Id>,
    /// A glide velocity the next build of the list sets.
    glide: Option<Vec2>,
}

fn list_ui(ui: &mut Ui, list: &mut List) {
    list.built.clear();
    let props = props(|p| p.scroll(Scroll::vertical()).sizing(Sizing::Fill));
    let measures = list.measures.clone();
    let built = &mut list.built;
    let lazy = list.lazy;
    let shown = Boxed::new(props, &measures).show(ui, |ui, _| {
        for i in 0..ROWS {
            if lazy {
                if Lazy::new(i, ROW).show(ui, |ui| chip(ui, &format!("row {i}"), ROW, &measures)).is_some() {
                    built.insert(i);
                }
            } else {
                chip(ui, &format!("row {i}"), ROW, &measures);
            }
        }
    });
    list.id = Some(shown.output.id);
    if let Some(velocity) = list.glide.take() {
        ui.glide(shown.output.id, velocity);
    }
}

fn wheel(rig: &mut Rig<RecordingRenderer>, list: &mut List, dy: f32) -> StepReport {
    let main = rig.main;
    rig.inject(main, Input::Wheel { pos: Pos2::new(200.0, 150.0), delta: Vec2::new(0.0, dy), mods: Mods::NONE });
    rig.step_with(&mut |_, cx| root(cx, |ui| list_ui(ui, list)))
}

#[test]
fn a_wheel_step_over_a_600_row_lazy_list_builds_only_rows_entering_view() {
    let mut list = List { lazy: true, ..List::default() };
    let mut rig = rig();
    settle(&mut rig, &mut |rig| rig.step_with(&mut |_, cx| root(cx, |ui| list_ui(ui, &mut list))));
    let first = list.built.clone();
    assert!(!first.is_empty() && first.len() < ROWS / 10, "only the rows in the view and its overscan built: {}", first.len());

    // A step within the overscan: the kept rows move; nothing builds or runs.
    let inside = wheel(&mut rig, &mut list, -75.0);
    assert_eq!((inside.frame.built, inside.frame.runs, inside.frame.scopes), (0, 0, 0), "{inside:?}");
    assert_eq!(inside.frame.laid_out, 1, "only the scroll body lays out");

    // A step past the band: the body builds again; only rows entering the band run.
    let before = first;
    let past = wheel(&mut rig, &mut list, -300.0);
    let entering: Vec<usize> = list.built.difference(&before).copied().collect();
    assert!(!entering.is_empty(), "rows came into view");
    assert_eq!(past.frame.runs as usize, entering.len(), "only rows entering view run: {past:?}");
    assert!(list.built.len() < ROWS / 10);
}

#[test]
fn a_glide_moves_the_rows_with_no_run_per_frame() {
    let mut list = List::default();
    let mut rig = rig();
    settle(&mut rig, &mut |rig| rig.step_with(&mut |_, cx| root(cx, |ui| list_ui(ui, &mut list))));
    assert!(list.id.is_some());
    let main = rig.main;
    let frame = |rig: &mut Rig<RecordingRenderer>, list: &mut List| rig.frame_with(1.0 / 60.0, &mut |_, cx| root(cx, |ui| list_ui(ui, list)));

    // The build that sets the velocity is the only one: every later frame lays out the scroll body alone.
    list.glide = Some(Vec2::new(0.0, 600.0));
    rig.driver.runtime.mark_root(main);
    frame(&mut rig, &mut list);
    for _ in 0..4 {
        let step = frame(&mut rig, &mut list);
        assert!(step.ran, "the glide asks for the next frame");
        assert_eq!((step.frame.built, step.frame.runs, step.frame.scopes), (0, 0, 0), "a glide frame runs nothing: {step:?}");
        assert_eq!(step.frame.laid_out, 1, "only the scroll body lays out");
    }

    // A zero velocity stops it: after the frame that pays out the last wake, nothing runs.
    list.glide = Some(Vec2::ZERO);
    rig.driver.runtime.mark_root(main);
    frame(&mut rig, &mut list);
    let ran = (0..3).filter(|_| frame(&mut rig, &mut list).ran).count();
    assert!(ran <= 1, "a stopped glide asks for no frames: {ran} ran");
}

#[test]
fn a_scroll_offset_change_alone_runs_no_child() {
    let mut list = List::default();
    let mut rig = rig();
    settle(&mut rig, &mut |rig| rig.step_with(&mut |_, cx| root(cx, |ui| list_ui(ui, &mut list))));
    let step = wheel(&mut rig, &mut list, -200.0);
    assert!(step.ran);
    assert_eq!((step.frame.built, step.frame.runs, step.frame.scopes), (0, 0, 0), "no build: the content was built whole: {step:?}");
    assert_eq!(step.frame.laid_out, 1, "only the scroll body lays out");
    let rec = rig.driver.renderer.last().expect("recorded");
    assert_eq!(rec.moved.len(), 1, "one transform moves: the rows keep their display lists");
}

/// An "Edit" chip, a container whose header text Edit lengthens, and an "After" chip below it.
struct Page {
    title: String,
    edit: Rect,
    after: Rect,
    body_runs: u32,
    measures: Rc<Cell<u32>>,
}

fn page_ui(ui: &mut Ui, page: &mut Page) {
    let measures = page.measures.clone();
    let edit = chip(ui, "Edit", ROW, &measures);
    page.edit = edit.rect;
    if edit.clicked() {
        page.title.push_str("\nmore");
    }
    let boxed = Boxed { title: Some(page.title.clone()), ..Boxed::new(props(|p| p.header(HeaderRegion::new())), &measures) };
    let runs = &mut page.body_runs;
    let _ = boxed.show(ui, |ui, _| {
        *runs += 1;
        chip(ui, "one", ROW, &measures);
        chip(ui, "two", ROW, &measures);
    });
    page.after = chip(ui, "After", ROW, &measures).rect;
}

#[test]
fn a_header_text_change_lays_out_the_header_and_its_body_and_no_sibling_outside() {
    let mut page = Page { title: "Title".into(), edit: Rect::NOTHING, after: Rect::NOTHING, body_runs: 0, measures: Rc::default() };
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, page: &mut Page| rig.step_with(&mut |_, cx| root(cx, |ui| page_ui(ui, page)));
    settle(&mut rig, &mut |rig| step(rig, &mut page));
    let (edit, after) = (page.edit.center(), page.after);
    let up = click(&mut rig, edit, &mut |rig| step(rig, &mut page));
    assert_eq!(page.title, "Title\nmore");
    assert_eq!(up.frame.runs, 3, "Edit, the container's own hooks and its header's title run; its body and After are kept: {up:?}");
    assert!(page.after.min.y > after.min.y, "After moved down under the taller header without running");
}

/// An "Other" chip and a centred row of three chips.
struct Bar {
    other: Rect,
    first: Rect,
    body_runs: u32,
    measures: Rc<Cell<u32>>,
}

fn bar_ui(ui: &mut Ui, bar: &mut Bar) {
    let measures = bar.measures.clone();
    bar.other = chip(ui, "Other", ROW, &measures).rect;
    let props = props(|p| p.layout(BodyLayout::row().align(RowAlign::Center)).sizing(Sizing::FillWidth));
    let (runs, first) = (&mut bar.body_runs, &mut bar.first);
    let _ = Boxed::new(props, &measures).show(ui, |ui, _| {
        *runs += 1;
        *first = chip(ui, "a", ROW, &measures).rect;
        chip(ui, "b", ROW, &measures);
        chip(ui, "c", ROW, &measures);
    });
}

fn bar() -> Bar {
    Bar { other: Rect::NOTHING, first: Rect::NOTHING, body_runs: 0, measures: Rc::default() }
}

#[test]
fn an_unchanged_container_in_a_rerun_scope_costs_a_key_check() {
    let mut bar = bar();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, bar: &mut Bar| rig.step_with(&mut |_, cx| root(cx, |ui| bar_ui(ui, bar)));
    settle(&mut rig, &mut |rig| step(rig, &mut bar));
    assert!(bar.first.min.x > 100.0, "the row's block is centred: {:?}", bar.first);
    let at = bar.other.center();
    let main = rig.main;
    rig.inject(main, Input::mouse_move(at));
    step(&mut rig, &mut bar);
    rig.inject(main, Input::mouse_down(at));
    step(&mut rig, &mut bar);
    let (runs, measures) = (bar.body_runs, bar.measures.get());
    rig.inject(main, Input::mouse_up(at));
    let up = step(&mut rig, &mut bar);
    assert_eq!(up.frame.runs, 1, "only Other runs: {up:?}");
    assert_eq!(bar.body_runs, runs + 1, "the row's body runs once: its plan is reused, no measuring run");
    assert_eq!(bar.measures.get(), measures + 1, "only Other, which runs, measures; the row's chips keep their sizes");
    assert_eq!(up.frame.painted, 1, "nothing under the row repaints");
}

#[test]
fn a_measuring_run_asks_kept_children_for_their_size() {
    let mut bar = bar();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, bar: &mut Bar| rig.step_with(&mut |_, cx| root(cx, |ui| bar_ui(ui, bar)));
    settle(&mut rig, &mut |rig| step(rig, &mut bar));
    let at = bar.first.center();
    let main = rig.main;
    rig.inject(main, Input::mouse_move(at));
    step(&mut rig, &mut bar);
    rig.inject(main, Input::mouse_down(at));
    step(&mut rig, &mut bar);
    let (runs, measures) = (bar.body_runs, bar.measures.get());
    rig.inject(main, Input::mouse_up(at));
    let up = step(&mut rig, &mut bar);
    assert_eq!(up.frame.runs, 1, "only the clicked chip runs: {up:?}");
    assert_eq!(bar.body_runs, runs + 2, "a marked child makes the row measure again: a measuring run, then the real one");
    assert_eq!(bar.measures.get() - measures, 2, "only the marked chip measures (in both runs); its siblings answer from their size cache");
}

/// A dialog-like stack: a body with a fill child, and a footer with a Done chip.
struct Sheet {
    body_runs: u32,
    measures: Rc<Cell<u32>>,
    done: Rc<Cell<Rect>>,
}

fn sheet_ui(ui: &mut Ui, sheet: &mut Sheet) {
    let measures = sheet.measures.clone();
    let outer = Boxed {
        footer: true,
        footer_rect: sheet.done.clone(),
        ..Boxed::new(props(|p| p.footer(FooterRegion::new()).sizing(Sizing::Fill)), &measures)
    };
    let runs = &mut sheet.body_runs;
    let _ = outer.show(ui, |ui, _| {
        *runs += 1;
        chip(ui, "top", ROW, &measures);
        let _ = Boxed::new(props(|p| p.sizing(Sizing::Fill)), &measures).show(ui, |ui, _| chip(ui, "filler", ROW, &measures));
    });
}

#[test]
fn a_click_in_a_footer_reuses_the_body_plan() {
    let mut sheet = Sheet { body_runs: 0, measures: Rc::default(), done: Rc::new(Cell::new(Rect::NOTHING)) };
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, sheet: &mut Sheet| rig.step_with(&mut |_, cx| root(cx, |ui| sheet_ui(ui, sheet)));
    settle(&mut rig, &mut |rig| step(rig, &mut sheet));
    let at = sheet.done.get().center();
    assert!(at.y > 200.0, "the footer sits at the bottom of the filled sheet: {at:?}");
    let main = rig.main;
    rig.inject(main, Input::mouse_move(at));
    step(&mut rig, &mut sheet);
    rig.inject(main, Input::mouse_down(at));
    step(&mut rig, &mut sheet);
    let (runs, measures) = (sheet.body_runs, sheet.measures.get());
    rig.inject(main, Input::mouse_up(at));
    let up = step(&mut rig, &mut sheet);
    assert_eq!(up.frame.runs, 1, "only the Done chip runs; the sheet's own hooks and its body are kept: {up:?}");
    assert_eq!(sheet.body_runs, runs + 1, "the body runs once: its plan (the fill share) is reused, no measuring run");
    assert_eq!(sheet.measures.get() - measures, 2, "only Done measures, in the footer's measuring run and its real run");
}

#[test]
fn a_scroll_body_is_as_tall_as_its_content_until_it_outgrows_the_room() {
    let height = |rows: usize| {
        let mut rig = rig();
        let measures = Rc::new(Cell::new(0));
        let mut rect = Rect::NOTHING;
        settle(&mut rig, &mut |rig| {
            rig.step_with(&mut |_, cx| {
                root(cx, |ui| {
                    rect = Boxed::new(props(|p| p.scroll(Scroll::vertical())), &measures)
                        .show(ui, |ui, _| {
                            for i in 0..rows {
                                chip(ui, &format!("row {i}"), ROW, &measures);
                            }
                        })
                        .output
                        .rect;
                })
            })
        });
        rect.height()
    };
    assert_eq!(height(3), 3.0 * ROW, "three rows: the body takes their height");
    assert_eq!(height(40), 300.0, "forty rows outgrow the window: the body takes the room and scrolls");
}

/// What a memoized row returns: whether its chip was clicked in the build it ran.
struct RowProps {
    label: &'static str,
    measures: Rc<Cell<u32>>,
}

/// The label is the row's only input; the measure counter is the trial's, not the view's.
impl MemoProps for RowProps {
    fn key(&self, key: &mut MemoKey) {
        key.feed_hash(&self.label);
    }
}

fn memo_row(props: &mut RowProps, ui: &mut Ui) -> bool {
    chip(ui, props.label, ROW, &props.measures).clicked()
}

#[test]
fn a_kept_memo_view_returns_its_value_and_no_fresh_event() {
    let measures = Rc::new(Cell::new(0));
    let (row, other, seen) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING), std::cell::RefCell::new(Vec::new()));
    let mut rig = rig();
    let mut step = |rig: &mut Rig<RecordingRenderer>| {
        rig.step_with(&mut |_, cx| {
            root(cx, |ui| {
                let start = ui.cursor().min;
                let shown = MemoView::new("row", RowProps { label: "Row", measures: measures.clone() }, memo_row).show(ui).expect("in view");
                row.set(Rect::from_min_size(start, Vec2::new(40.0, ROW)));
                seen.borrow_mut().push((shown.value, shown.fresh, shown.clone().value_if_fresh()));
                other.set(chip(ui, "Other", ROW, &measures).rect);
            })
        })
    };
    settle(&mut rig, &mut step);
    let main = rig.main;
    for at in [row.get().center(), other.get().center()] {
        rig.inject(main, Input::mouse_move(at));
        step(&mut rig);
        rig.inject(main, Input::mouse_down(at));
        step(&mut rig);
        seen.borrow_mut().clear();
        rig.inject(main, Input::mouse_up(at));
        step(&mut rig);
        if at == row.get().center() {
            assert_eq!(seen.borrow().last(), Some(&(true, true, Some(true))), "the click runs the view: its event is fresh");
        } else {
            assert_eq!(seen.borrow().last(), Some(&(true, false, None)), "a click elsewhere keeps the view: the kept value, no event");
        }
    }
}

/// Builds of the window root in the first frame of a column group whose rows report `widths` in order, the widths
/// they read at the end of it, and the report of a second frame with nothing marked.
fn group_frames(widths: [f32; 3]) -> (u32, Vec<f32>, StepReport) {
    let mut rig = rig();
    let builds = Cell::new(0u32);
    let read = std::cell::RefCell::new(Vec::new());
    let mut step = |rig: &mut Rig<RecordingRenderer>| {
        rig.step_with(&mut |_, cx| {
            root(cx, |ui| {
                builds.set(builds.get() + 1);
                let key = ColumnKey::new("labels");
                let got = ColumnGroup::scope(ui, "group", |ui, group| widths.map(|w| group.width(ui, &key, w)));
                *read.borrow_mut() = got.to_vec();
            })
        })
    };
    step(&mut rig);
    let first = builds.get();
    let idle = step(&mut rig);
    (first, read.into_inner(), idle)
}

/// A column group whose widest row comes last builds its node once more in the same frame, so the rows before it
/// read the final width; one whose widest row comes first needs no second build. A frame with nothing marked builds
/// nothing.
#[test]
fn a_column_group_settles_in_its_first_frame_and_rebuilds_only_when_a_row_read_a_narrower_width() {
    let (first, read, idle) = group_frames([50.0, 40.0, 30.0]);
    assert_eq!(read, [50.0; 3], "every row reads the widest");
    assert!(!idle.ran, "a settled group builds nothing: {idle:?}");
    let (last, read, idle) = group_frames([30.0, 40.0, 50.0]);
    assert_eq!(read, [50.0; 3], "the rows before the widest read it after the second build");
    assert_eq!(last, first + 1, "one more build of the group's node in the same frame, none when the widest came first");
    assert!(!idle.ran, "a settled group builds nothing: {idle:?}");
}

/// A collapsing section whose header chip toggles its open state.
#[derive(craft_ui::contract::Component)]
#[component(container, unregistered)]
struct Fold {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    #[key(skip)]
    toggle: Rc<Cell<Rect>>,
    #[key(skip)]
    measures: Rc<Cell<u32>>,
}

impl Component for Fold {
    const ROLE: craft_ui::contract::Role = craft_ui::contract::Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FoldStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Fold {
    fn header(&mut self, ui: &mut Ui, bcx: &mut BodyCx<'_>) {
        let toggle = chip(ui, "Fold", ROW, &self.measures);
        self.toggle.set(toggle.rect);
        if toggle.clicked() {
            let open = bcx.is_open();
            *bcx.open() = !open;
        }
    }
}

/// A folding section with a body of three chips, and a chip after it.
struct Folds {
    toggle: Rc<Cell<Rect>>,
    after: Rect,
    measures: Rc<Cell<u32>>,
    /// The last body chip's rect and its clicks.
    last: Rect,
    last_clicks: u32,
}

impl Default for Folds {
    fn default() -> Folds {
        Folds { toggle: Rc::new(Cell::new(Rect::NOTHING)), after: Rect::NOTHING, measures: Rc::default(), last: Rect::NOTHING, last_clicks: 0 }
    }
}

fn folds_ui(ui: &mut Ui, folds: &mut Folds) {
    let measures = folds.measures.clone();
    let props = props(|p| p.collapse(Collapse::new()).header(HeaderRegion::new()));
    let fold = Fold { common: Common::new(), props, toggle: folds.toggle.clone(), measures: measures.clone() };
    let (last, last_clicks) = (&mut folds.last, &mut folds.last_clicks);
    let _ = fold.show(ui, |ui, _| {
        for name in ["a", "b", "c"] {
            let shown = chip(ui, name, ROW, &measures);
            if name == "c" {
                *last = shown.rect;
                *last_clicks += u32::from(shown.clicked());
            }
        }
    });
    folds.after = chip(ui, "After", ROW, &measures).rect;
}

#[test]
fn a_collapse_builds_its_section_once_per_frame_while_it_moves_and_never_after() {
    const FRAME: f64 = 1.0 / 60.0;
    let mut folds = Folds::default();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, folds: &mut Folds| rig.step_with(&mut |_, cx| root(cx, |ui| folds_ui(ui, folds)));
    settle(&mut rig, &mut |rig| step(rig, &mut folds));
    let open_after = folds.after;
    let up = click(&mut rig, folds.toggle.get().center(), &mut |rig| step(rig, &mut folds));
    assert!(up.ran, "the click toggles the section");
    // The collapse: the section runs once per frame while its height moves; its body's chips are kept and the chip
    // after it moves up without running.
    let mut moving = 0;
    for _ in 0..120 {
        rig.driver.platform.advance(FRAME);
        let frame = step(&mut rig, &mut folds);
        if !frame.ran {
            break;
        }
        moving += 1;
        assert!(frame.frame.runs <= 1, "only the section's own hooks run while it collapses: {frame:?}");
    }
    assert!(moving > 1, "the collapse takes more than one frame");
    assert!(folds.after.min.y < open_after.min.y, "After moved up under the closed section");
    let idle = step(&mut rig, &mut folds);
    assert!(!idle.ran, "nothing runs once the collapse has settled: {idle:?}");
}

#[test]
fn a_collapsing_body_clips_its_children_to_the_share_it_shows() {
    const FRAME: f64 = 1.0 / 60.0;
    let mut folds = Folds::default();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, folds: &mut Folds| rig.step_with(&mut |_, cx| root(cx, |ui| folds_ui(ui, folds)));
    let run_out = |rig: &mut Rig<RecordingRenderer>, folds: &mut Folds, limit: usize| {
        let mut moving = 0;
        while moving < limit {
            rig.driver.platform.advance(FRAME);
            if !step(rig, folds).ran {
                break;
            }
            moving += 1;
        }
        moving
    };
    settle(&mut rig, &mut |rig| step(rig, &mut folds));
    let last = folds.last;
    // One whole close and open to learn how many frames a collapse moves.
    click(&mut rig, folds.toggle.get().center(), &mut |rig| step(rig, &mut folds));
    let frames = run_out(&mut rig, &mut folds, 120);
    click(&mut rig, folds.toggle.get().center(), &mut |rig| step(rig, &mut folds));
    run_out(&mut rig, &mut folds, 120);
    assert!(frames > 2, "the collapse takes several frames");

    // Three quarters into a close, the body shows less than two of its three rows: the last row lies past the clip, so
    // a click on it reaches no chip of the body.
    click(&mut rig, folds.toggle.get().center(), &mut |rig| step(rig, &mut folds));
    run_out(&mut rig, &mut folds, (frames * 3).div_ceil(4));
    let clicks = folds.last_clicks;
    click(&mut rig, last.center(), &mut |rig| step(rig, &mut folds));
    assert_eq!(folds.last_clicks, clicks, "a row clipped by the moving collapse takes no click");
}

/// A chip before a dialog mounted on a layer (when `layered`) and a chip after it.
struct Layered {
    layered: bool,
    inside: Rect,
    after: Rect,
    measures: Rc<Cell<u32>>,
}

fn layered_ui(ui: &mut Ui, layered: &mut Layered) {
    let measures = layered.measures.clone();
    chip(ui, "Before", ROW, &measures);
    if layered.layered {
        let props = props(|p| p.mount(Mount::Layer(Overlay::dialog())));
        let inside = &mut layered.inside;
        let _ = Boxed::new(props, &measures).show(ui, |ui, _| {
            *inside = chip(ui, "Inside", ROW, &measures).rect;
        });
    }
    layered.after = chip(ui, "After", ROW, &measures).rect;
}

#[test]
fn a_layer_mount_shows_its_body_on_its_layer_and_takes_no_room_in_its_flow() {
    let run = |on: bool| {
        let mut layered = Layered { layered: on, inside: Rect::NOTHING, after: Rect::NOTHING, measures: Rc::default() };
        let mut rig = rig();
        let step = |rig: &mut Rig<RecordingRenderer>, layered: &mut Layered| rig.step_with(&mut |_, cx| root(cx, |ui| layered_ui(ui, layered)));
        settle(&mut rig, &mut |rig| step(rig, &mut layered));
        let idle = step(&mut rig, &mut layered);
        assert!(!idle.ran, "a shown layer mount settles: {idle:?}");
        layered
    };
    let (with, without) = (run(true), run(false));
    assert!(with.inside.width() > 0.0, "the dialog's body ran on its layer: {:?}", with.inside);
    assert_eq!(with.after, without.after, "After stands where it stands with no dialog: the layer mount took no room");
}

/// A chip and, while `open`, a container mounted in a window of its own (`Mount::viewport`) whose body counts its runs.
struct Viewported {
    open: bool,
    inside: Option<Rect>,
    body_runs: u32,
    /// The windows the host's `roots` closure was asked to build.
    roots: Vec<craft_ui_rt::WindowId>,
    measures: Rc<Cell<u32>>,
}

fn viewport_ui(ui: &mut Ui, viewported: &mut Viewported) {
    let measures = viewported.measures.clone();
    chip(ui, "Before", ROW, &measures);
    if viewported.open {
        let props = props(|p| p.mount(Mount::viewport(Overlay::dialog())));
        let (inside, runs) = (&mut viewported.inside, &mut viewported.body_runs);
        let _ = Boxed::new(props, &measures).show(ui, |ui, _| {
            *runs += 1;
            *inside = Some(chip(ui, "Inside", ROW, &measures).rect);
        });
    }
}

#[test]
fn a_viewport_mount_opens_a_window_built_in_the_showing_build_and_closes_it_with_the_container() {
    let mut viewported = Viewported { open: true, inside: None, body_runs: 0, roots: Vec::new(), measures: Rc::default() };
    let mut rig = rig();
    let main = rig.main;
    let mut step = |rig: &mut Rig<RecordingRenderer>, viewported: &mut Viewported| {
        rig.step_with(&mut |window, cx| {
            viewported.roots.push(window);
            if window == main {
                root(cx, |ui| viewport_ui(ui, viewported));
            }
        })
    };
    settle(&mut rig, &mut |rig| step(rig, &mut viewported));
    let windows: Vec<_> = rig.driver.runtime.windows().collect();
    assert_eq!(windows.len(), 2, "the viewport opened one window");
    assert!(viewported.roots.iter().all(|window| *window == main), "the host never builds the hosted window: {:?}", viewported.roots);
    assert!(viewported.inside.is_some_and(|rect| rect.width() > 0.0), "the body ran in the window, inside the showing build");
    let runs = viewported.body_runs;
    let idle = step(&mut rig, &mut viewported);
    assert!(!idle.ran && viewported.body_runs == runs, "an open viewport settles: {idle:?}");

    // A click on the body's chip in the hosted window reaches it through the showing node's build.
    let hosted = windows.into_iter().find(|window| *window != main).expect("the hosted window");
    let at = viewported.inside.expect("shown").center();
    rig.inject(hosted, Input::mouse_move(at));
    step(&mut rig, &mut viewported);
    rig.inject(hosted, Input::mouse_down(at));
    step(&mut rig, &mut viewported);
    assert!(viewported.roots.iter().all(|window| *window == main), "input in the hosted window builds the showing node, not a root of its own");

    // The container closes: the next build leaves the window out and it closes.
    viewported.open = false;
    rig.driver.runtime.mark_root(main);
    settle(&mut rig, &mut |rig| step(rig, &mut viewported));
    assert_eq!(rig.driver.runtime.windows().count(), 1, "the hosted window closed with its container");
}

/// A shell that notes a step of its outer gesture in the build where its open state changes, as the dialog shells do.
struct OuterShell {
    open: bool,
    noted: bool,
    gesture: Option<craft_behavior::GestureId>,
    ctx: Option<craft_ui::contract::Context>,
}

fn outer_ui(ui: &mut Ui, shell: &mut OuterShell) {
    shell.ctx = Some(ui.ctx().clone());
    if shell.open != shell.noted {
        shell.noted = shell.open;
        let edit = match shell.gesture.take() {
            None => {
                let id = ui.next_gesture_id();
                shell.gesture = Some(id);
                OuterEdit::Begin { id, label: "Image Size".into(), doc_key: None }
            }
            Some(id) => OuterEdit::Commit { id },
        };
        ui.ctx().note_outer_edit(edit);
    }
    chip(ui, "Field", ROW, &Rc::default());
}

#[test]
fn an_outer_edit_reaches_the_host_once_per_step() {
    let mut shell = OuterShell { open: false, noted: false, gesture: None, ctx: None };
    let mut rig = rig();
    let main = rig.main;
    let mut step = |rig: &mut Rig<RecordingRenderer>, shell: &mut OuterShell| {
        rig.driver.runtime.mark_root(main);
        let report = rig.step_with(&mut |_, cx| root(cx, |ui| outer_ui(ui, shell)));
        let taken = shell.ctx.as_ref().expect("built").take_outer_edits();
        (report, taken)
    };
    let (_, taken) = step(&mut rig, &mut shell);
    assert!(taken.is_empty());

    shell.open = true;
    let (_, taken) = step(&mut rig, &mut shell);
    assert!(matches!(taken[..], [OuterEdit::Begin { .. }]), "the open notes one Begin: {taken:?}");
    // A build that keeps the open state notes nothing; what the host took is gone.
    let (_, taken) = step(&mut rig, &mut shell);
    assert!(taken.is_empty(), "{taken:?}");

    shell.open = false;
    let (_, taken) = step(&mut rig, &mut shell);
    assert!(matches!(taken[..], [OuterEdit::Commit { .. }]), "the close notes one Commit: {taken:?}");
}

/// A leaf `w` wide and `w / 4` high.
#[derive(craft_ui::contract::Component)]
#[component(unregistered)]
struct Wide {
    #[common]
    common: Common,
    w: u32,
}

impl Component for Wide {
    const ROLE: craft_ui::contract::Role = craft_ui::contract::Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: WideStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(self.w as f32, self.w as f32 / 4.0)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A square picture: as wide as the room it is offered, from 20 to 400 points, and as high as it is wide.
#[derive(craft_ui::contract::Component)]
#[component(unregistered)]
struct Square {
    #[common]
    common: Common,
}

impl Component for Square {
    const ROLE: craft_ui::contract::Role = craft_ui::contract::Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SquareStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.picture_size(20.0, 400.0, |width| width)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A row whose unit is a content-sized column holding a `Wide` leaf and a `Square` under it.
struct Held {
    wide: u32,
    column: Rect,
    square: Rect,
}

fn held_ui(ui: &mut Ui, held: &mut Held) {
    let measures = Rc::new(Cell::new(0));
    let (wide, column, square) = (held.wide, &mut held.column, &mut held.square);
    let _ = Boxed::new(props(|p| p.layout(BodyLayout::row())), &measures).show(ui, |ui, _| {
        *column = Boxed::new(props(|p| p), &measures)
            .show(ui, |ui, _| {
                Wide { common: Common::new(), w: wide }.show(ui);
                *square = Square { common: Common::new() }.show(ui).rect;
            })
            .output
            .rect;
    });
}

#[test]
fn a_held_picture_shows_at_its_columns_width_and_never_holds_it_across_a_shrink() {
    let mut held = Held { wide: 120, column: Rect::NOTHING, square: Rect::NOTHING };
    let mut rig = rig();
    let main = rig.main;
    let mut step = |rig: &mut Rig<RecordingRenderer>, held: &mut Held| rig.step_with(&mut |_, cx| root(cx, |ui| held_ui(ui, held)));
    settle(&mut rig, &mut |rig| step(rig, &mut held));
    // The column takes its width from the wide leaf; the picture, which needs no width of its own, shows across it
    // and the column is as tall as the picture shows.
    assert_eq!(held.column.width(), 120.0, "the picture does not set the column's width: {:?}", held.column);
    assert_eq!(held.square.width(), 120.0, "the picture shows at the column's width: {:?}", held.square);
    assert!(held.column.height() >= 30.0 + 120.0 - 0.5, "the column measured the picture as tall as it shows: {:?}", held.column);

    // The wide leaf narrows: the column follows it, and the picture's earlier width holds nothing.
    held.wide = 80;
    rig.driver.runtime.mark_root(main);
    settle(&mut rig, &mut |rig| step(rig, &mut held));
    assert_eq!(held.column.width(), 80.0, "{:?}", held.column);
    assert_eq!(held.square.width(), 80.0, "{:?}", held.square);
}

#[test]
fn a_held_picture_keeps_its_columns_width_when_the_real_run_reuses_the_plan() {
    let mut held = Held { wide: 120, column: Rect::NOTHING, square: Rect::NOTHING };
    let mut rig = rig();
    let main = rig.main;
    let mut step = |rig: &mut Rig<RecordingRenderer>, held: &mut Held| rig.step_with(&mut |_, cx| root(cx, |ui| held_ui(ui, held)));
    settle(&mut rig, &mut |rig| step(rig, &mut held));
    let (column, square) = (held.column, held.square);
    // A build that changes nothing under the row: the row's real run shows the column without measuring it, and the
    // column lays its children out at the width its kept plan settled at.
    rig.driver.runtime.mark_root(main);
    settle(&mut rig, &mut |rig| step(rig, &mut held));
    assert_eq!(held.column, column, "the column keeps its settled rect");
    assert_eq!(held.square.width(), 120.0, "the picture shows at the column's width in the real run: {:?}", held.square);
    assert_eq!(held.square, square);
}

/// A content-sized popup holding a `Wide` leaf and a `Square` under it.
struct HeldPopup {
    square: Rect,
}

fn held_popup_ui(ui: &mut Ui, held: &mut HeldPopup) {
    let measures = Rc::new(Cell::new(0));
    let square = &mut held.square;
    let _ = Boxed::new(props(|p| p.mount(Mount::Layer(Overlay::popover()))), &measures).show(ui, |ui, _| {
        Wide { common: Common::new(), w: 120 }.show(ui);
        *square = Square { common: Common::new() }.show(ui).rect;
    });
}

#[test]
fn a_content_sized_popup_is_a_content_column_for_its_held_picture() {
    let mut held = HeldPopup { square: Rect::NOTHING };
    let mut rig = rig();
    let mut step = |rig: &mut Rig<RecordingRenderer>, held: &mut HeldPopup| rig.step_with(&mut |_, cx| root(cx, |ui| held_popup_ui(ui, held)));
    settle(&mut rig, &mut |rig| step(rig, &mut held));
    assert_eq!(held.square.width(), 120.0, "the popup takes the leaf's width and the picture shows across it: {:?}", held.square);
}

/// A container on an edge pane that keeps the placement changes it is told of.
#[derive(craft_ui::contract::Component)]
#[component(container, unregistered)]
struct Paned {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    #[key(skip)]
    changes: Rc<std::cell::RefCell<Vec<PlacementChange>>>,
}

impl Component for Paned {
    const ROLE: craft_ui::contract::Role = craft_ui::contract::Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PanedStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Paned {
    fn placement_changed(&mut self, change: &PlacementChange) {
        self.changes.borrow_mut().push(*change);
    }
}

/// A start-edge pane as wide as a 100-point leaf, then a chip in the rest of the window.
struct Panes {
    pane: Rect,
    after: Rect,
    changes: Rc<std::cell::RefCell<Vec<PlacementChange>>>,
    measures: Rc<Cell<u32>>,
}

fn panes_ui(ui: &mut Ui, panes: &mut Panes) {
    let pane = ShellPane::new("side").edge(PaneEdge::Start);
    let paned = Paned { common: Common::new(), props: props(|p| p.mount(Mount::Pane(pane))), changes: panes.changes.clone() };
    panes.pane = paned.show(ui, |ui, _| Wide { common: Common::new(), w: 100 }.show(ui)).output.rect;
    panes.after = chip(ui, "After", ROW, &panes.measures).rect;
}

#[test]
fn a_pane_edge_drag_saves_its_placement_once() {
    let mut panes = Panes { pane: Rect::NOTHING, after: Rect::NOTHING, changes: Rc::default(), measures: Rc::default() };
    let mut rig = rig();
    let main = rig.main;
    let mut step = |rig: &mut Rig<RecordingRenderer>, panes: &mut Panes| rig.step_with(&mut |_, cx| root(cx, |ui| panes_ui(ui, panes)));
    settle(&mut rig, &mut |rig| step(rig, &mut panes));
    assert_eq!(panes.pane.width(), 100.0, "the pane first takes its content's width: {:?}", panes.pane);
    assert!(panes.after.min.x >= panes.pane.max.x, "the rest of the flow lies past the pane: {:?}", panes.after);

    // The grip lies along the pane's inner (right) edge, inside it.
    let at = Pos2::new(panes.pane.max.x - 1.0, panes.pane.center().y);
    for input in
        [Input::mouse_move(at), Input::mouse_down(at), Input::mouse_move(at + Vec2::new(20.0, 0.0)), Input::mouse_move(at + Vec2::new(40.0, 0.0))]
    {
        rig.inject(main, input);
        step(&mut rig, &mut panes);
    }
    assert_eq!(panes.pane.width(), 140.0, "the pane follows the grip in the frame of the step: {:?}", panes.pane);
    rig.inject(main, Input::mouse_up(at + Vec2::new(40.0, 0.0)));
    settle(&mut rig, &mut |rig| step(rig, &mut panes));
    let changes = panes.changes.borrow().clone();
    let commits: Vec<_> = changes.iter().filter(|c| c.phase == craft_behavior::GesturePhase::Commit).collect();
    assert_eq!(commits.len(), 1, "one commit: {changes:?}");
    assert_eq!(commits[0].rect.width(), 140.0);
    assert!(changes.iter().all(|c| c.kind == PlacementKind::Size(craft_ui::runtime::windows::ResizeEdge::Right)), "{changes:?}");
    assert_eq!(changes.iter().filter(|c| c.phase == craft_behavior::GesturePhase::Begin).count(), 1, "one begin: {changes:?}");

    // The extent stays across later builds, and nothing more is reported.
    rig.driver.runtime.mark_root(main);
    settle(&mut rig, &mut |rig| step(rig, &mut panes));
    assert_eq!(panes.pane.width(), 140.0, "the pane keeps the extent the drag saved: {:?}", panes.pane);
    assert!(panes.after.min.x >= 140.0, "{:?}", panes.after);
    assert_eq!(panes.changes.borrow().len(), changes.len(), "a build reports no placement change");
}
