//! Count trials of containers and layout on craft-ui-rt: scroll offsets move kept content, `Lazy`
//! builds only rows entering view, a container's header change stays inside it, and a measuring run
//! asks kept children for their size.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use craft_ui_contract::contract::rt::{
    Component, ComponentKey, Container, Cx, Lazy, MeasureCx, Paint, Response, Ui, show_container, show_leaf, window_root,
};
use craft_ui_contract::contract::{ContainerProps, Gap, HeaderRegion, IdSource, MemoKey, RowAlign, Scroll, Sizing};
use craft_ui_contract::layout::BodyLayout;
use craft_ui_contract::primitives::PaintShape;
use craft_ui_contract::style::{StyleColor, UiTokens, rgba_color};
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Mods, Pos2, Rect, Sense, StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;

/// A leaf `h` high that counts its `measure` calls and reports clicks.
struct Chip {
    label: String,
    h: f32,
    measures: Rc<Cell<u32>>,
}

impl ComponentKey for Chip {
    fn key(&self, key: &mut MemoKey) {
        key.feed_hash(&(self.label.as_str(), self.h.to_bits()));
    }
}

impl Component for Chip {
    const ROLE: accesskit::Role = accesskit::Role::Button;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        self.measures.set(self.measures.get() + 1);
        Vec2::new(40.0, self.h)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn paint(&mut self, paint: &Paint<'_>, _: &Cx<'_, Self>) {
        let bounds = paint.bounds();
        let ink = paint.tokens.color.text.faint;
        paint.draw().fill(&PaintShape::Rect(bounds, UiTokens::corner(0.0)), StyleColor(rgba_color(ink)));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn chip(ui: &mut Ui<'_, '_>, label: &str, h: f32, measures: &Rc<Cell<u32>>) -> Response {
    show_leaf(Chip { label: label.into(), h, measures: measures.clone() }, ui)
}

/// A container with the given props and a header whose text sets its height: ten points a line.
struct Boxed {
    props: ContainerProps,
    title: Option<String>,
    measures: Rc<Cell<u32>>,
}

impl ComponentKey for Boxed {
    fn key(&self, key: &mut MemoKey) {
        key.feed_hash(&self.title);
    }
}

impl Component for Boxed {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn id_source(&self) -> IdSource {
        IdSource::Auto
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Boxed {
    fn props(&self) -> &ContainerProps {
        &self.props
    }

    fn header(&mut self, ui: &mut Ui<'_, '_>) {
        if let Some(title) = &self.title {
            let lines = title.lines().count() as f32;
            chip(ui, title, 10.0 * lines, &self.measures);
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

fn settle(rig: &mut Rig<RecordingRenderer>, step: &mut dyn FnMut(&mut Rig<RecordingRenderer>) -> StepReport) {
    for _ in 0..6 {
        if !step(rig).ran {
            return;
        }
    }
    panic!("the rig did not settle");
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

fn list_ui(ui: &mut Ui<'_, '_>, list: &mut List) {
    list.built.clear();
    let props = props(|p| p.scroll(Scroll::vertical()).sizing(Sizing::Fill));
    let measures = list.measures.clone();
    show_container(Boxed { props, title: None, measures: measures.clone() }, ui, |ui| {
        for i in 0..ROWS {
            if list.lazy {
                if Lazy::new(i, ROW).show(ui, |ui| chip(ui, &format!("row {i}"), ROW, &measures)).is_some() {
                    list.built.insert(i);
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
    rig.step_with(&mut |_, cx| window_root(cx, |ui| list_ui(ui, list)))
}

#[test]
fn a_wheel_step_over_a_600_row_lazy_list_builds_only_rows_entering_view() {
    let mut list = List { lazy: true, ..List::default() };
    let mut rig = rig();
    settle(&mut rig, &mut |rig| rig.step_with(&mut |_, cx| window_root(cx, |ui| list_ui(ui, &mut list))));
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
    settle(&mut rig, &mut |rig| rig.step_with(&mut |_, cx| window_root(cx, |ui| list_ui(ui, &mut list))));
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

impl Page {
    fn new() -> Page {
        Page { title: "Title".into(), edit: Rect::NOTHING, after: Rect::NOTHING, body_runs: 0, measures: Rc::default() }
    }
}

fn page_ui(ui: &mut Ui<'_, '_>, page: &mut Page) {
    let measures = page.measures.clone();
    let edit = chip(ui, "Edit", ROW, &measures);
    page.edit = edit.rect;
    if edit.clicked() {
        page.title.push_str("\nmore");
    }
    let props = props(|p| p.header(HeaderRegion::new()));
    show_container(Boxed { props, title: Some(page.title.clone()), measures: measures.clone() }, ui, |ui| {
        page.body_runs += 1;
        chip(ui, "one", ROW, &measures);
        chip(ui, "two", ROW, &measures);
    });
    page.after = chip(ui, "After", ROW, &measures).rect;
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

#[test]
fn a_header_text_change_lays_out_the_header_and_its_body_and_no_sibling_outside() {
    let mut page = Page::new();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, page: &mut Page| rig.step_with(&mut |_, cx| window_root(cx, |ui| page_ui(ui, page)));
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

impl Bar {
    fn new() -> Bar {
        Bar { other: Rect::NOTHING, first: Rect::NOTHING, body_runs: 0, measures: Rc::default() }
    }
}

fn bar_ui(ui: &mut Ui<'_, '_>, bar: &mut Bar) {
    let measures = bar.measures.clone();
    bar.other = chip(ui, "Other", ROW, &measures).rect;
    let props = props(|p| p.layout(BodyLayout::row().align(RowAlign::Center)).sizing(Sizing::FillWidth));
    show_container(Boxed { props, title: None, measures: measures.clone() }, ui, |ui| {
        bar.body_runs += 1;
        bar.first = chip(ui, "a", ROW, &measures).rect;
        chip(ui, "b", ROW, &measures);
        chip(ui, "c", ROW, &measures);
    });
}

#[test]
fn an_unchanged_container_in_a_rerun_scope_costs_a_key_check() {
    let mut bar = Bar::new();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, bar: &mut Bar| rig.step_with(&mut |_, cx| window_root(cx, |ui| bar_ui(ui, bar)));
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
    let mut bar = Bar::new();
    let mut rig = rig();
    let step = |rig: &mut Rig<RecordingRenderer>, bar: &mut Bar| rig.step_with(&mut |_, cx| window_root(cx, |ui| bar_ui(ui, bar)));
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
