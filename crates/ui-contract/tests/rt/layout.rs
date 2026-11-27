//! Count trials of containers, scopes and layout on craft-ui-rt: scroll offsets move kept content,
//! `Lazy` builds only rows entering view, a container's header change stays inside it, a measuring
//! run asks kept children for their size, a click in a footer reuses the body's plan, a scroll body
//! is as tall as its content until it outgrows the room, a kept `MemoView` repeats no event, and a column group
//! settles in its first frame.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use craft_behavior::{ColumnKey, Sense};
use craft_ui::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, FooterRegion, Gap, HeaderRegion, Lazy, MeasureCx, MemoKey, MemoProps, MemoView,
    Response, RowAlign, Scroll, Show as _, ShowContainer as _, Sizing, Ui,
};
use craft_ui::layout::{BodyLayout, ColumnGroup};
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
        Boxed { common: Common::new(), props, title: None, footer: false, measures: measures.clone(), footer_rect: Rc::default() }
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
}

fn list_ui(ui: &mut Ui, list: &mut List) {
    list.built.clear();
    let props = props(|p| p.scroll(Scroll::vertical()).sizing(Sizing::Fill));
    let measures = list.measures.clone();
    let built = &mut list.built;
    let lazy = list.lazy;
    let _ = Boxed::new(props, &measures).show(ui, |ui, _| {
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
    let mut sheet = Sheet { body_runs: 0, measures: Rc::default(), done: Rc::default() };
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
    let (mut row, mut other, mut seen) = (Rect::NOTHING, Rect::NOTHING, Vec::new());
    let mut rig = rig();
    let mut step = |rig: &mut Rig<RecordingRenderer>| {
        rig.step_with(&mut |_, cx| {
            root(cx, |ui| {
                let start = ui.cursor().min;
                let shown = MemoView::new("row", RowProps { label: "Row", measures: measures.clone() }, memo_row).show(ui).expect("in view");
                row = Rect::from_min_size(start, Vec2::new(40.0, ROW));
                seen.push((shown.value, shown.fresh, shown.clone().value_if_fresh()));
                other = chip(ui, "Other", ROW, &measures).rect;
            })
        })
    };
    settle(&mut rig, &mut step);
    let main = rig.main;
    for at in [row.center(), other.center()] {
        rig.inject(main, Input::mouse_move(at));
        step(&mut rig);
        rig.inject(main, Input::mouse_down(at));
        step(&mut rig);
        seen.clear();
        rig.inject(main, Input::mouse_up(at));
        step(&mut rig);
        if at == row.center() {
            assert_eq!(seen.last(), Some(&(true, true, Some(true))), "the click runs the view: its event is fresh");
        } else {
            assert_eq!(seen.last(), Some(&(true, false, None)), "a click elsewhere keeps the view: the kept value, no event");
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
