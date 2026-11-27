//! Count trials of a collection on craft-ui-rt (docs/contract-rt.md section 15.5): the item fn is
//! called for the band rows only, a hover over a row swaps that row's paint variant, a selection
//! runs the list and the rows whose facts moved, and a wheel step inside the band runs nothing.

use std::cell::Cell;

use craft_behavior::{DisplayOrder, ItemSource, Selection, SpanLayout};
use craft_ui::contract::{
    Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput, NoValue, Outcome, PaintUi,
    Response, Role, SelectMode, ShowCollection as _, Sizing, Ui,
};
use craft_ui::primitives::{CornerRadius, PaintShape};
use craft_ui::runtime::windows::{WindowEnv, window_root_with};
use craft_ui::style::StyleCx;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Pos2, Rect, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

const ROW_H: f32 = 20.0;
const VIEW: Vec2 = Vec2::new(300.0, 300.0);

/// `n` rows keyed by their position.
struct Count(usize);

impl DisplayOrder<usize> for Count {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }
}

impl ItemSource<usize> for Count {}

#[derive(Clone, Debug, PartialEq)]
enum Never {}

struct RowOut {
    response: Response,
}

impl Outcome for RowOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<Never> for RowOut {
    fn into_event(self) -> Option<Never> {
        None
    }
}

/// A row whose paint reads its own `Hovered` fact and fills its rect while hovered.
#[derive(craft_ui::contract::Component)]
struct Row {
    #[common]
    common: Common,
}

impl Component for Row {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = PlainStyle;
    type Output = RowOut;

    fn style(_: &StyleCx<'_>, _: RowStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(10.0, ROW_H))
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        if cx.response.hovered() {
            let draw = ui.draw();
            draw.fill(&PaintShape::Rect(cx.rect, CornerRadius::ZERO), draw.role(|colors| colors.text.faint));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone() }
    }
}

#[derive(craft_ui::contract::Component)]
#[component(collection)]
struct List<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Option<Bind<'a, Selection<usize>>>,
}

impl Component for List<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Never>;

    fn style(_: &StyleCx<'_>, _: ListStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for List<'_> {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = Row;
    type ItemEvent = Never;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<usize>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, ROW_H)
    }
}

struct App {
    rows: usize,
    selection: Selection<usize>,
    /// Item fn calls in the last step.
    calls: Cell<usize>,
    rect: Rect,
    /// The list takes its width from its widest row, with a rows stamp so its kept measure serves.
    content_width: bool,
}

fn app_ui(ui: &mut Ui, app: &mut App) {
    app.calls.set(0);
    let calls = &app.calls;
    let list = List {
        common: Common::new(),
        props: ContainerProps::new().sizing(if app.content_width { Sizing::Content } else { Sizing::Fill }),
        items: if app.content_width {
            CollectionProps::new().select(SelectMode::Single).rows_stamp(1)
        } else {
            CollectionProps::new().select(SelectMode::Single)
        },
        selection: Some(Bind::bound(&mut app.selection)),
    };
    let out = list.show_items(ui, &Count(app.rows), &mut |_, _| {
        calls.set(calls.get() + 1);
        Row { common: Common::new() }
    });
    app.rect = out.response.rect;
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    rig.step_with(&mut |_, cx| window_root_with(cx, WindowEnv::default(), |ui| app_ui(ui, app)))
}

fn rig(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(VIEW, 1.0, RecordingRenderer::default());
    step(&mut rig, app);
    for _ in 0..4 {
        if !step(&mut rig, app).ran {
            return rig;
        }
    }
    panic!("the rig did not settle");
}

fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: craft_ui_platform_headless::Input) -> StepReport {
    let main = rig.main;
    rig.inject(main, input);
    step(rig, app)
}

/// The window position of row `at`'s centre while the list is scrolled to its top.
fn row_center(app: &App, at: usize) -> Pos2 {
    Pos2::new(app.rect.center().x, app.rect.min.y + ROW_H * (at as f32 + 0.5))
}

fn app(rows: usize) -> App {
    App { rows, selection: Selection::default(), calls: Cell::new(0), rect: Rect::NOTHING, content_width: false }
}

#[test]
fn six_hundred_rows_build_only_the_band() {
    let mut app = app(600);
    let mut rig = Rig::inline(VIEW, 1.0, RecordingRenderer::default());
    let first = step(&mut rig, &mut app);
    let in_view = (VIEW.y / ROW_H) as usize;
    assert!(app.calls.get() >= in_view, "every row in view is built: {}", app.calls.get());
    assert!(app.calls.get() <= 3 * in_view, "the item fn runs for the band only (the view and its overscan), not 600 rows: {}", app.calls.get());
    assert!(first.frame.runs as usize <= app.calls.get() + 2, "a run per band row, the list and the root: {first:?}");
}

#[test]
fn hover_over_a_row_paints_one_row_and_runs_nothing() {
    let mut app = app(600);
    let mut rig = rig(&mut app);
    let entered = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(row_center(&app, 3)));
    assert_eq!(entered.frame.painted, 1, "only the hovered row repaints: {entered:?}");
    assert_eq!((entered.frame.runs, entered.frame.scopes, entered.frame.built), (0, 0, 0), "a hover is a kept variant: {entered:?}");
}

#[test]
fn select_runs_the_list_and_the_rows_whose_facts_moved() {
    let mut app = app(600);
    let mut rig = rig(&mut app);
    let at = row_center(&app, 3);
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_down(at));
    let up = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_up(at));
    assert_eq!(app.selection.lead(), Some(&3), "the click selects row 3");
    assert_eq!(up.frame.runs, 2, "the list and row 3 run; the other band rows are kept: {up:?}");
    let other = row_center(&app, 5);
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(other));
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_down(other));
    let moved = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_up(other));
    assert_eq!(app.selection.lead(), Some(&5));
    assert_eq!(moved.frame.runs, 3, "the list, row 3 (deselected) and row 5 (selected) run: {moved:?}");
}

#[test]
fn a_wheel_step_inside_the_band_runs_nothing() {
    let mut app = app(600);
    let mut rig = rig(&mut app);
    let main = rig.main;
    let at = row_center(&app, 3);
    rig.driver.platform.wheel(main, at, Vec2::new(0.0, -ROW_H));
    app.calls.set(0);
    let report = step(&mut rig, &mut app);
    assert_eq!((report.frame.runs, report.frame.scopes, report.frame.built), (0, 0, 0), "the rows view moves its content in layout: {report:?}");
    assert_eq!(app.calls.get(), 0, "no item fn call: the scope did not build");
}

#[test]
fn a_select_keeps_the_largest_row_measure() {
    let mut app = App { content_width: true, ..app(40) };
    let mut rig = rig(&mut app);
    let built = craft_ui::contract::take_widest_runs();
    assert!(!built.is_empty(), "a content-sized list measures its largest row once: {built:?}");
    let at = row_center(&app, 3);
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_down(at));
    let _ = craft_ui::contract::take_row_builds();
    let up = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_up(at));
    assert_eq!(app.selection.lead(), Some(&3), "the click selects row 3");
    assert!(up.frame.runs >= 1, "the list runs on the select: {up:?}");
    assert!(craft_ui::contract::take_widest_runs().is_empty(), "the rows, room and stamp hold: the kept measure serves");
    assert!(craft_ui::contract::take_row_builds().is_empty(), "no row is built to be measured again");
}
