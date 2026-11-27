//! Count trials of a collection on craft-ui-rt (docs/contract-rt.md section 15.5): the item fn is
//! called for the band rows only, a hover over a row swaps that row's paint variant, a selection
//! runs the list and the rows whose facts moved, and a wheel step inside the band runs nothing.

use std::cell::Cell;

use craft_behavior::{DisplayOrder, ItemSource, Selection, SpanLayout};
use craft_ui::contract::{
    Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, HoverLead, ItemOutput, NoValue, Outcome,
    PaintUi, Response, Role, SelectMode, ShowCollection as _, Sizing, Ui,
};
use craft_ui::primitives::PaintShape;
use craft_ui::runtime::windows::{WindowEnv, window_root_with};
use craft_ui::style::{StyleCx, UiTokens};
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
            draw.fill(&PaintShape::Rect(cx.rect, UiTokens::corner(0.0)), draw.role(|colors| colors.text.faint));
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
    /// What the pointer moving over the rows changes.
    hover: HoverLead,
    /// Props a trial adds (reorder, middle-button auto-scroll).
    tune: fn(&mut CollectionProps),
    /// The last response's drop preview, visible rows and auto-scroll latch.
    pending: Option<craft_ui::contract::PendingDrop<usize>>,
    visible: std::ops::Range<usize>,
    auto_scrolling: bool,
}

fn app_ui(ui: &mut Ui, app: &mut App) {
    app.calls.set(0);
    let calls = &app.calls;
    let mut items = if app.content_width {
        CollectionProps::new().select(SelectMode::Single).rows_stamp(1)
    } else {
        CollectionProps::new().select(SelectMode::Single).hover_lead(app.hover)
    };
    (app.tune)(&mut items);
    let list = List {
        common: Common::new(),
        props: ContainerProps::new().sizing(if app.content_width { Sizing::Content } else { Sizing::Fill }),
        items,
        selection: Some(Bind::bound(&mut app.selection)),
    };
    let out = list.show_items(ui, &Count(app.rows), &mut |_, _| {
        calls.set(calls.get() + 1);
        Row { common: Common::new() }
    });
    app.rect = out.response.rect;
    app.pending = out.pending_drop;
    app.visible = out.visible;
    app.auto_scrolling = out.auto_scrolling;
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
    App {
        rows,
        selection: Selection::default(),
        calls: Cell::new(0),
        rect: Rect::NOTHING,
        content_width: false,
        hover: HoverLead::Off,
        tune: |_| {},
        pending: None,
        visible: 0..0,
        auto_scrolling: false,
    }
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
    // The pointer enters the list first: its overlay scroll bar shows while the list is hovered.
    let (into, at) = (row_center(&app, 2), row_center(&app, 3));
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(into));
    let entered = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    assert_eq!(entered.frame.painted, 2, "only the row the pointer left and the hovered row repaint: {entered:?}");
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
    assert_eq!(app.selection.lead, Some(3), "the click selects row 3");
    assert_eq!(up.frame.runs, 2, "the list and row 3 run; the other band rows are kept: {up:?}");
    let other = row_center(&app, 5);
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(other));
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_down(other));
    let moved = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_up(other));
    assert_eq!(app.selection.lead, Some(5));
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
    // Clear of the scroll bar, which covers most of the narrow list's width.
    let at = Pos2::new(app.rect.min.x + 1.0, row_center(&app, 3).y);
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_down(at));
    let _ = craft_ui::contract::take_row_builds();
    let up = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_up(at));
    assert_eq!(app.selection.lead, Some(3), "the click selects row 3");
    assert!(up.frame.runs >= 1, "the list runs on the select: {up:?}");
    assert!(craft_ui::contract::take_widest_runs().is_empty(), "the rows, room and stamp hold: the kept measure serves");
    assert!(craft_ui::contract::take_row_builds().is_empty(), "no row is built to be measured again");
}

#[test]
fn hover_lead_follows_the_pointer_with_one_list_run_per_move() {
    let mut app = App { hover: HoverLead::LeadAndSelection, ..app(600) };
    let mut rig = rig(&mut app);
    let at = row_center(&app, 3);
    let entered = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    assert_eq!(app.selection.lead, Some(3), "the lead and the pick follow the pointer onto row 3");
    assert_eq!(entered.frame.runs, 2, "the list runs once and row 3 takes its new facts: {entered:?}");
    let at = row_center(&app, 3) + Vec2::new(4.0, 2.0);
    let inside = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    assert_eq!(app.selection.lead, Some(3), "a move inside the lead row keeps the lead");
    assert_eq!(inside.frame.runs, 1, "the hover point moved: the list alone runs, no row: {inside:?}");
    let at = row_center(&app, 4);
    let next = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    assert_eq!(app.selection.lead, Some(4), "the lead follows onto row 4");
    assert_eq!(next.frame.runs, 3, "the list once, row 3 and row 4 whose facts moved: {next:?}");
}

#[test]
fn a_list_without_hover_lead_runs_nothing_on_a_move() {
    let mut app = app(600);
    let mut rig = rig(&mut app);
    let at = row_center(&app, 3);
    input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    let at = row_center(&app, 3) + Vec2::new(4.0, 2.0);
    let moved = input(&mut rig, &mut app, craft_ui_platform_headless::Input::mouse_move(at));
    assert_eq!(moved.frame.runs, 0, "only a list with hover lead reads the hover point: {moved:?}");
}

/// A drag's edge scroll and the middle-button auto-scroll glide the rows view (`rt::scroll::glide`): ui-rt pays the
/// glide out in layout. A row drag follows the moving rows with one list run per glide frame; with no drag, a glide
/// frame runs nothing.
mod glide {
    use super::*;
    use craft_ui::contract::{MiddleButton, PendingDrop, ReorderRule};
    use craft_ui_platform_headless::{Contact, Input};
    use craft_ui_rt::PointerPhase;

    const FRAME: f64 = 1.0 / 60.0;

    /// One frame later with no input.
    fn frame(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
        rig.driver.platform.advance(FRAME);
        app.calls.set(0);
        step(rig, app)
    }

    #[test]
    fn a_reorder_drag_at_the_edge_glides_and_its_drop_spot_follows_with_one_list_run_per_frame() {
        let mut app = App { tune: |items| items.reorder = Some(ReorderRule::new()), ..app(600) };
        let mut rig = rig(&mut app);
        let grab = row_center(&app, 2);
        input(&mut rig, &mut app, Input::mouse_move(grab));
        input(&mut rig, &mut app, Input::mouse_down(grab));
        input(&mut rig, &mut app, Input::mouse_move(grab + Vec2::new(0.0, 12.0)));
        // Into the bottom edge band of the narrow list: the run notes the rate and asks for a glide.
        let edge = Pos2::new(grab.x, app.rect.max.y - 2.0);
        input(&mut rig, &mut app, Input::mouse_move(edge));
        let first = app.pending.clone();
        assert!(matches!(first, Some(PendingDrop::Reorder(_))), "the drag over the rows previews a reorder: {first:?}");
        // Two points inside the edge the glide is slow (production's edge rate): enough frames to move the rows past
        // the middle of the row under the pointer.
        let mut runs = Vec::new();
        for _ in 0..20 {
            let report = frame(&mut rig, &mut app);
            runs.push((report.frame.runs, app.calls.get() > 0));
        }
        assert!(runs.iter().all(|&(_, list_ran)| list_ran), "each glide frame runs the list once to follow the offset: {runs:?}");
        assert_ne!(app.pending, first, "the drop spot follows the rows the glide moved under the pointer");
        input(&mut rig, &mut app, Input::mouse_up(edge));
        frame(&mut rig, &mut app);
        let after = frame(&mut rig, &mut app);
        assert_eq!((after.frame.runs, app.calls.get()), (0, 0), "with no drag the glide is stopped and nothing runs: {after:?}");
    }

    #[test]
    fn the_middle_button_latch_scrolls_with_no_run_per_frame_and_escape_ends_it() {
        let mut app = App { tune: |items| items.middle_button = MiddleButton::AutoScroll, ..app(600) };
        let mut rig = rig(&mut app);
        let main = rig.main;
        let anchor = row_center(&app, 3);
        let middle = Contact { button: craft_behavior::PointerButton::Middle, ..Contact::MOUSE };
        input(&mut rig, &mut app, Input::mouse_move(anchor));
        input(&mut rig, &mut app, Input::pointer(middle, PointerPhase::Down, anchor));
        input(&mut rig, &mut app, Input::pointer(middle, PointerPhase::Up, anchor));
        assert!(app.auto_scrolling, "a middle click on the rows latches the auto-scroll");
        // Below the anchor past the dead zone: the rows view glides toward the end.
        input(&mut rig, &mut app, Input::mouse_move(anchor + Vec2::new(0.0, 80.0)));
        let quiet: Vec<(u32, bool)> = (0..6)
            .map(|_| {
                let report = frame(&mut rig, &mut app);
                (report.frame.runs, app.calls.get() > 0)
            })
            .collect();
        // Only a glide that leaves the band the last build covered builds the list (new rows come into the band).
        assert!(quiet.iter().filter(|&&(_, list_ran)| list_ran).count() <= 1, "a glide frame runs no list: {quiet:?}");
        // The list builds again once the glide carries the view out of the band it built: the rows it shows move on.
        for _ in 0..30 {
            if app.visible.start > 0 {
                break;
            }
            frame(&mut rig, &mut app);
        }
        input(&mut rig, &mut app, Input::mouse_move(anchor + Vec2::new(1.0, 80.0)));
        assert!(app.visible.start > 0, "the latch scrolled the rows: {:?}", app.visible);
        rig.driver.platform.key_press(main, craft_ui_rt::Key::Escape, craft_ui_rt::Mods::NONE);
        step(&mut rig, &mut app);
        assert!(!app.auto_scrolling, "Escape ends the latch though the list has no key focus");
        let at = app.visible.clone();
        for _ in 0..6 {
            frame(&mut rig, &mut app);
        }
        input(&mut rig, &mut app, Input::mouse_move(anchor + Vec2::new(2.0, 80.0)));
        assert_eq!(app.visible, at, "after Escape the rows stay where the latch left them");
    }
}
