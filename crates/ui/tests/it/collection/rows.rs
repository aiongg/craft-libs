//! The row trials: rows with focusable children, cell selection in a sheet, rows of measured heights, the sheet's
//! grid view, a collection's command scope and a lead that leaves the source.

use super::kit::{Count, Dismissed, List, ROW_H, RowOut};
use crate::contract::kit::PlainStyle;
use craft_behavior::{
    CellAt, CellRange, CellSelection, ColumnKey, ItemSource, PartKey, Selection, SelectionModel as _, Sense, SheetLayout, SpanLayout, keyed,
};
use craft_ui::contract::{
    Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, CommandSet, Commands as _, Common, Component, ContainerProps,
    Cx, Focus as _, MeasureCx, PartsCx, Props as _, Role, SelectMode, ShowCollection as _,
};
use craft_ui::runtime::commands::CommandScope;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, input, pointer, trial};
use egui::{Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::ops::Range;

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const CELL_W: f32 = 50.0;
const TALL: f32 = 3.0;
const CHILD_A: PartKey = PartKey::new("a");
const CHILD_B: PartKey = PartKey::new("b");

/// Commands of the collections of these trials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "rows", scope = "trial.rows")]
#[non_exhaustive]
enum RowsCommand {
    #[command(label = "Bump", default = "Cmd+Alt+B")]
    Bump,
}

const SCOPE: CommandScope = <RowsCommand as CommandSet>::SCOPE;

/// What the trials' rows and collections saw.
#[derive(Default)]
struct Log {
    /// The row and part that held focus in the last pass that had one.
    focus: RefCell<Vec<(usize, &'static str)>>,
    rects: RefCell<Vec<(usize, Rect)>>,
    grid: RefCell<Vec<Range<u32>>>,
    bumps: RefCell<Vec<&'static str>>,
    /// The nudges a row's part took: the row, the direction and whether Alt was held.
    nudges: RefCell<Vec<(usize, Vec2, bool)>>,
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    id: Cell<Option<Id>>,
}

impl Log {
    fn new() -> &'static Log {
        Box::leak(Box::default())
    }

    fn rect(&self, key: usize) -> Rect {
        self.rects.borrow().iter().rev().find(|(row, _)| *row == key).map(|(_, rect)| *rect).expect("the row ran")
    }

    fn leads(&self) -> Vec<usize> {
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::LeadMoved { key, .. } = event { Some(*key) } else { None }).collect()
    }

    fn take(&self, out: CollectionResponse<usize, Dismissed>) {
        self.id.set(Some(out.response.id));
        self.events.borrow_mut().extend(out.events);
    }

    fn id(&self) -> Id {
        self.id.get().expect("the list ran")
    }
}

/// A row that records its rect and focus, with two focusable parts when it has children and a tall measure for
/// the key 1 of an auto-height list.
#[derive(craft_ui::contract::Component)]
struct ProbeRow {
    #[common]
    common: Common,
    key: usize,
    children: bool,
    tall: bool,
    nudging: bool,
    log: &'static Log,
}

impl ProbeRow {
    fn new(key: usize, log: &'static Log) -> Self {
        ProbeRow { common: Common::new(), key, children: false, tall: false, nudging: false, log }
    }

    fn children(mut self) -> Self {
        self.children = true;
        self
    }

    fn nudging(mut self) -> Self {
        self.nudging = true;
        self
    }

    fn tall_when(mut self, tall: bool) -> Self {
        self.tall = tall;
        self
    }
}

impl Component for ProbeRow {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = PlainStyle;
    type Output = RowOut;

    fn style(_: &StyleCx<'_>, _: ProbeRowStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0, if self.tall { ROW_H * TALL } else { ROW_H })
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if self.children {
            let half = Vec2::new(rect.width() / 2.0, rect.height());
            let a = cx.part(CHILD_A, Rect::from_min_size(rect.min, half)).sense(Sense::CLICK).focusable();
            if self.nudging {
                a.nudges();
            }
            cx.part(CHILD_B, Rect::from_min_size(rect.min + Vec2::new(half.x, 0.0), half)).sense(Sense::CLICK).focusable();
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.log.rects.borrow_mut().push((self.key, cx.rect));
        if let Some((direction, mods)) = cx.intents().take_nudge() {
            self.log.nudges.borrow_mut().push((self.key, direction, mods.alt));
        }
        if cx.is_focused() {
            self.log.focus.borrow_mut().push((self.key, cx.active_part().map_or("row", |part| part.key.name())));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone(), event: None }
    }
}

/// A list of `ProbeRow`s over `SpanLayout`, rows of the control height or of the heights they measure.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
struct Probe<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Option<Bind<'a, CellSelection<usize>>>,
    auto: bool,
    name: &'static str,
    log: &'static Log,
}

impl<'a> Probe<'a> {
    fn new(select: SelectMode, log: &'static Log) -> Self {
        Probe {
            common: Common::new(),
            props: ContainerProps::new().sizing(craft_ui::contract::Sizing::Fill),
            items: CollectionProps::new().select(select),
            selection: None,
            auto: false,
            name: "",
            log,
        }
    }

    fn auto(mut self) -> Self {
        self.auto = true;
        self
    }

    fn scoped(mut self, name: &'static str) -> Self {
        self.props = self.props.command_scope(SCOPE);
        self.name = name;
        self
    }
}

impl Component for Probe<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: ProbeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        while cx.take_command::<RowsCommand>().is_some() {
            self.log.bumps.borrow_mut().push(self.name);
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Probe<'_> {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = CellSelection<usize>;
    type Item<'r> = ProbeRow;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, CellSelection<usize>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
        if self.auto { SpanLayout::sized(source, width, ROW_H) } else { SpanLayout::new(source, width, ROW_H) }
    }
}

/// A sheet of `CELL_W` columns over `SheetLayout`, with the first column frozen.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
struct Sheet<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Option<Bind<'a, CellSelection<usize>>>,
}

const COLUMNS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

impl<'a> Sheet<'a> {
    fn new() -> Self {
        Sheet {
            common: Common::new(),
            props: ContainerProps::new().sizing(craft_ui::contract::Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Cells),
            selection: None,
        }
    }

    fn selection(mut self, selection: Bind<'a, CellSelection<usize>>) -> Self {
        self.selection = Some(selection);
        self
    }
}

impl Component for Sheet<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: SheetStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Sheet<'_> {
    type Key = usize;
    type ItemLayout = SheetLayout;
    type Selection = CellSelection<usize>;
    type Item<'r> = ProbeRow;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, CellSelection<usize>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, _: f32, _: &StyleCx<'_>) -> SheetLayout {
        let columns = COLUMNS.iter().map(|key| (ColumnKey::new(key), CELL_W)).collect();
        SheetLayout::new(source, ROW_H, columns).frozen(1)
    }
}

fn press(frames: &mut Frames, key: Key, mods: Modifiers, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![input::key_press(key, mods)], &mut *show);
    frames.frame(vec![craft_ui::testing::focus::key_release(key, mods)], &mut *show);
}

/// A key press and release with the focus record cleared first, so it holds only what the key's frames saw.
fn tap(frames: &mut Frames, log: &Log, key: Key, show: &mut impl FnMut(&mut Ui)) {
    log.focus.borrow_mut().clear();
    press(frames, key, Modifiers::NONE, show);
}

fn focus_list(frames: &mut Frames, log: &Log, show: &mut impl FnMut(&mut Ui)) {
    request(&frames.ctx, FocusTarget::Id(log.id()));
    frames.idle(2, &mut *show);
}

/// 17.5: Forward enters a row's first child and moves on through them, Back returns to the row from the first,
/// Up, Down and Escape leave the children for the row, and the collection stays the one egui focus.
#[trial]
fn forward_enters_a_rows_children_and_arrows_and_escape_leave_them() {
    let (log, source) = (Log::new(), Count(4));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let list = Probe::new(SelectMode::Single, log).id_salt("list");
            log.take(list.show_items(ui, &source, &mut |key, _| ProbeRow::new(*key, log).children()));
        });
    };
    frames.idle(3, &mut show);
    focus_list(&mut frames, log, &mut show);
    let last_focus = || log.focus.borrow().last().copied();

    tap(&mut frames, log, Key::ArrowDown, &mut show);
    assert_eq!(log.leads().last(), Some(&0));
    assert_eq!(last_focus(), None, "the lead row alone holds focus until Forward");
    tap(&mut frames, log, Key::ArrowRight, &mut show);
    assert_eq!(last_focus(), Some((0, "a")), "Forward enters the row's first child");
    assert_eq!(craft_ui::testing::focus::egui_focused(&frames.ctx), Some(log.id()), "the children are members of the collection's arena");
    tap(&mut frames, log, Key::ArrowRight, &mut show);
    assert_eq!(last_focus(), Some((0, "b")));
    tap(&mut frames, log, Key::ArrowLeft, &mut show);
    assert_eq!(last_focus(), Some((0, "a")));
    tap(&mut frames, log, Key::ArrowLeft, &mut show);
    assert_eq!(last_focus(), None, "Back from the first child returns to the row");
    assert_eq!(log.leads().last(), Some(&0));

    tap(&mut frames, log, Key::ArrowRight, &mut show);
    tap(&mut frames, log, Key::ArrowDown, &mut show);
    assert_eq!((last_focus(), log.leads().last().copied()), (None, Some(1)), "Down leaves the children and moves the lead");
    tap(&mut frames, log, Key::ArrowRight, &mut show);
    assert_eq!(last_focus(), Some((1, "a")));
    tap(&mut frames, log, Key::Escape, &mut show);
    assert_eq!((last_focus(), log.leads().last().copied()), (None, Some(1)), "Escape returns to the row and keeps the lead");
}

/// 17.7: the arrows move the lead cell, Shift extends the range from the anchor cell, and the selection reports
/// the rows and the columns, with the active cell at the anchor. The lead stays with the active cell while the range
/// extends, and a plain arrow then steps from the active cell (Excel), here against the sheet's first column.
#[trial]
fn shift_arrows_extend_a_cell_selection_in_a_sheet() {
    let (log, source) = (Log::new(), Count(5));
    let selection = RefCell::new(CellSelection::<usize>::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut held = selection.borrow().clone();
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            log.take(Sheet::new().selection(Bind::bound(&mut held)).id_salt("sheet").show_items(ui, &source, &mut |key, _| ProbeRow::new(*key, log)));
        });
        *selection.borrow_mut() = held;
    };
    frames.idle(3, &mut show);
    focus_list(&mut frames, log, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::ArrowRight, Modifiers::SHIFT, &mut show);
    press(&mut frames, Key::ArrowRight, Modifiers::SHIFT, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::SHIFT, &mut show);
    let held = selection.borrow().clone();
    assert_eq!(held.ranges(), [CellRange::cells(CellAt::new(0, 0), CellAt::new(1, 2))], "the rows and columns between the anchor cell and the head");
    assert_eq!((held.lead(), held.lead_column()), (Some(&0), Some(0)), "the active cell stays at the anchor");
    press(&mut frames, Key::ArrowLeft, Modifiers::NONE, &mut show);
    let held = selection.borrow().clone();
    assert_eq!(
        (held.ranges(), held.lead(), held.lead_column()),
        ([CellRange::cells(CellAt::new(0, 0), CellAt::new(0, 0))].as_slice(), Some(&0), Some(0)),
        "a plain arrow after an extend steps from the active cell, and the lead stays with it"
    );
}

/// 17.3, I1: rows of different measured heights place one under the other and a press finds the row under it.
#[trial]
fn rows_of_different_measured_heights_place_and_hit_correctly() {
    let (log, source) = (Log::new(), Count(6));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let list = Probe::new(SelectMode::Single, log).auto().id_salt("list");
            log.take(list.show_items(ui, &source, &mut |key, _| ProbeRow::new(*key, log).tall_when(*key == 1)));
        });
    };
    frames.idle(4, &mut show);
    assert_eq!(log.rect(1).height(), ROW_H * TALL, "an auto row takes the height it measures");
    assert_eq!((log.rect(0).height(), log.rect(2).min.y), (ROW_H, log.rect(1).max.y), "the rows after it move down");
    let inside = |key: usize, fraction: f32| log.rect(key).min + Vec2::new(5.0, log.rect(key).height() * fraction);
    pointer::click(&mut frames, inside(1, 0.9), &mut show);
    assert_eq!(log.leads().last(), Some(&1), "the lower part of the tall row is the tall row");
    pointer::click(&mut frames, inside(2, 0.5), &mut show);
    assert_eq!(log.leads().last(), Some(&2));
}

/// 17.7: the sheet's grid view follows the viewport: a wider viewport shows more of the scrolling columns, and the
/// frozen column stays its own section.
#[trial]
fn the_sheet_grid_view_follows_the_viewport() {
    let (log, source) = (Log::new(), Count(3));
    let width = Cell::new(160.0);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, Vec2::new(width.get(), VIEW.y)), |ui| {
            let out = Sheet::new().id_salt("sheet").show_items(ui, &source, &mut |key, cx| {
                *log.grid.borrow_mut() =
                    cx.grid().map(|grid| grid.sections.iter().map(|section| section.columns.clone()).collect()).unwrap_or_default();
                ProbeRow::new(*key, log)
            });
            log.take(out);
        });
    };
    frames.idle(3, &mut show);
    let narrow = log.grid.borrow().clone();
    width.set(400.0);
    frames.idle(3, &mut show);
    let wide = log.grid.borrow().clone();
    assert_eq!((narrow[0].clone(), wide[0].clone()), (0..1, 0..1), "the frozen column is its own section");
    assert!(narrow[1].end < wide[1].end, "{narrow:?} then {wide:?}");
    assert_eq!(wide[1], 1..COLUMNS.len() as u32);
}

/// 11.3: a command of the collection's scope reaches the collection that holds focus, not the other one.
#[trial]
fn a_collection_command_scope_routes_a_shortcut_to_the_focused_collection() {
    let (log, source) = (Log::new(), Count(3));
    let ids = [Cell::new(None), Cell::new(None)];
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        for (at, name) in ["a", "b"].into_iter().enumerate() {
            in_region(ui, Rect::from_min_size(ORIGIN + Vec2::new(350.0 * at as f32, 0.0), VIEW), |ui| {
                let out =
                    Probe::new(SelectMode::Single, log).scoped(name).id_salt(name).show_items(ui, &source, &mut |key, _| ProbeRow::new(*key, log));
                ids[at].set(Some(out.response.id));
            });
        }
    };
    frames.idle(3, &mut show);
    request(&frames.ctx, FocusTarget::Id(ids[1].get().expect("the second list ran")));
    frames.idle(2, &mut show);
    frames.frame(vec![input::key_press(Key::B, Modifiers::COMMAND | Modifiers::ALT)], &mut show);
    assert_eq!(*log.bumps.borrow(), ["b"]);
}

/// 17.3 step 9: when the lead's key leaves the source, the lead moves to the key now in its place and a selection
/// that held the old lead moves with it.
#[trial]
fn a_selected_lead_that_leaves_the_source_moves_the_selection_to_the_new_lead() {
    let rows = RefCell::new(vec![(10_usize, "a"), (11, "b"), (12, "c"), (13, "d")]);
    let selection = RefCell::new(Selection::<usize>::default());
    let seen = Cell::new(None);
    let events = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut held = selection.borrow().clone();
        let rows = rows.borrow();
        let source = keyed(&rows, |row| row.0);
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let out = List::new(SelectMode::Single)
                .selection(Bind::bound(&mut held))
                .id_salt("list")
                .show_items(ui, &source, &mut |_, _| super::kit::Row::new());
            seen.set(Some(out.response.id));
            events.borrow_mut().extend(out.events);
        });
        *selection.borrow_mut() = held;
    };
    frames.idle(3, &mut show);
    pointer::click(&mut frames, ORIGIN + Vec2::new(5.0, ROW_H * 2.5), &mut show);
    assert_eq!(selection.borrow().items, vec![12]);
    rows.borrow_mut().remove(2);
    frames.idle(2, &mut show);
    assert_eq!(selection.borrow().items, vec![13], "the selection follows the lead to the key now in its place");
    let leads: Vec<usize> =
        events.borrow().iter().filter_map(|event| if let CollectionEvent::LeadMoved { key, .. } = event { Some(*key) } else { None }).collect();
    assert_eq!(leads.last(), Some(&13));
}

/// A part that declares it takes nudges receives the arrows, Alt included, while it sits in a row; the arrows do not
/// move focus off the part, and the row's other part still gets row-child moves.
#[trial]
fn a_nudging_part_in_a_row_gets_alt_arrows() {
    let (log, source) = (Log::new(), Count(2));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let list = Probe::new(SelectMode::Single, log).id_salt("list");
            log.take(list.show_items(ui, &source, &mut |key, _| ProbeRow::new(*key, log).children().nudging()));
        });
    };
    frames.idle(3, &mut show);
    focus_list(&mut frames, log, &mut show);
    tap(&mut frames, log, Key::ArrowDown, &mut show);
    tap(&mut frames, log, Key::ArrowRight, &mut show);
    assert_eq!(log.focus.borrow().last().copied(), Some((0, "a")), "Forward enters the nudging part");
    press(&mut frames, Key::ArrowRight, Modifiers::ALT, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    assert_eq!(*log.nudges.borrow(), [(0, Vec2::X, true), (0, Vec2::Y, false)]);
    assert_eq!(log.focus.borrow().last().copied(), Some((0, "a")), "the arrows moved nothing");
    assert_eq!(log.leads().last(), Some(&0), "the lead stayed");
}
