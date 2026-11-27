//! Count trials of the driver's own reads and outputs: the tracked hover point (`CxCore::hover_point`)
//! reruns only the node that reads it; a paint slot fill repaints the slot's node and runs nothing.

use craft_behavior::IconId;
use craft_ui_contract::contract::*;
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_contract::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Pos2, Rect, Sense, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

use crate::skeleton::IconButton;

/// A pad that reads where the pointer is over it, as a collection's hover lead does.
#[derive(Component)]
struct HoverPad {
    #[common]
    common: Common,
}

impl Component for HoverPad {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HoverPadStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(120.0, 60.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let _ = cx.hover_point();
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct App {
    pad: Rect,
    button: Rect,
}

impl Default for App {
    fn default() -> App {
        App { pad: Rect::NOTHING, button: Rect::NOTHING }
    }
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    rig.step_with(&mut |_, cx| {
        window_root(cx, |ui| {
            app.button = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
            app.pad = Show::show(HoverPad { common: Common::new() }, ui).rect;
        })
    })
}

fn settled(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    for _ in 0..5 {
        if !step(&mut rig, app).ran {
            return rig;
        }
    }
    panic!("the rig did not settle");
}

fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
    let main = rig.main;
    rig.inject(main, input);
    step(rig, app)
}

#[test]
fn a_pointer_move_reruns_only_the_node_that_reads_its_hover_point() {
    let mut app = App::default();
    let mut rig = settled(&mut app);
    let entered = {
        let event = Input::mouse_move(app.pad.center());
        input(&mut rig, &mut app, event)
    };
    assert_eq!(entered.frame.runs, 1, "entering the pad runs the pad alone: {entered:?}");
    let within = {
        let event = Input::mouse_move(app.pad.center() + Vec2::new(5.0, 3.0));
        input(&mut rig, &mut app, event)
    };
    assert_eq!(within.frame.runs, 1, "a move over the pad runs the pad again, and nothing else: {within:?}");
    let left = {
        let event = Input::mouse_move(app.button.center());
        input(&mut rig, &mut app, event)
    };
    assert_eq!(left.frame.runs, 1, "leaving the pad runs it once; the button only repaints: {left:?}");
    let over_button = {
        let event = Input::mouse_move(app.button.center() + Vec2::new(1.0, 1.0));
        input(&mut rig, &mut app, event)
    };
    assert_eq!(over_button.frame.runs, 0, "a move away from the pad runs nothing: {over_button:?}");
    let away = input(&mut rig, &mut app, Input::mouse_move(Pos2::new(390.0, 290.0)));
    assert_eq!(away.frame.runs, 0);
}

const MARK: craft_theme::Rgba = craft_theme::Rgba([200, 30, 40, 255]);

thread_local! {
    /// The slot `SlotHost` reserved in its last paint, and its rect.
    static SLOT: std::cell::Cell<Option<(craft_ui_contract::primitives::PaintSlot, Rect)>> = const { std::cell::Cell::new(None) };
}

/// A pad that reserves one paint slot, as the canvas reserves its picture slot.
#[derive(Component)]
struct SlotHost {
    #[common]
    common: Common,
}

impl Component for SlotHost {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SlotHostStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(120.0, 60.0)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let slot = ui.reserve_slot(cx.rect);
        SLOT.with(|kept| kept.set(Some((slot, cx.rect))));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A pad that fills `SlotHost`'s slot from its update whenever the pointer moves over it, as a transform
/// preview fills the canvas picture slot.
#[derive(Component)]
struct SlotFiller {
    #[common]
    common: Common,
}

impl Component for SlotFiller {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SlotFillerStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(120.0, 60.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.hover_point().is_some()
            && let Some((slot, rect)) = SLOT.with(std::cell::Cell::get)
        {
            slot.fill(vec![craft_ui_rt::Item::Rect { rect: rect.shrink(4.0), radii: craft_ui_rt::CornerRadii::ZERO, fill: MARK, stroke: None }]);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct SlotApp {
    filler: Rect,
}

impl Default for SlotApp {
    fn default() -> SlotApp {
        SlotApp { filler: Rect::NOTHING }
    }
}

fn slot_step(rig: &mut Rig<RecordingRenderer>, app: &mut SlotApp) -> StepReport {
    rig.step_with(&mut |_, cx| {
        window_root(cx, |ui| {
            let _ = Show::show(SlotHost { common: Common::new() }, ui);
            app.filler = Show::show(SlotFiller { common: Common::new() }, ui).rect;
        })
    })
}

/// Whether the last frame drew the slot's fill.
fn shows_mark(rig: &Rig<RecordingRenderer>) -> bool {
    let Some(frame) = rig.driver.renderer.last() else { return false };
    frame.draws.iter().any(|draw| draw.list.items.iter().any(|item| matches!(item, craft_ui_rt::Item::Rect { fill, .. } if *fill == MARK)))
}

#[test]
fn a_slot_fill_repaints_the_slot_node_and_runs_no_build_of_it() {
    let mut app = SlotApp::default();
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let mut settled = false;
    for _ in 0..5 {
        if !slot_step(&mut rig, &mut app).ran {
            settled = true;
            break;
        }
    }
    assert!(settled, "the rig did not settle");
    assert!(SLOT.with(std::cell::Cell::get).is_some(), "the host reserved its slot");
    assert!(!shows_mark(&rig), "an unfilled slot paints nothing");

    let main = rig.main;
    rig.inject(main, Input::mouse_move(app.filler.center()));
    let filled = slot_step(&mut rig, &mut app);
    assert_eq!(filled.frame.runs, 1, "the fill runs the filler alone; the host keeps its run: {filled:?}");
    assert!(shows_mark(&rig), "the host repaints with the slot's items");

    rig.inject(main, Input::mouse_move(app.filler.center() + Vec2::new(4.0, 2.0)));
    let again = slot_step(&mut rig, &mut app);
    assert_eq!(again.frame.runs, 1, "a second fill again runs only the filler: {again:?}");
    assert!(shows_mark(&rig), "the slot holds the new items in place of the old");
}

/// A composite that measures one child through `MeasureCx::child` and shows it from its paint, and reads where the
/// pointer is over it, so a pointer move runs it.
#[derive(Component)]
struct Composite {
    #[common]
    common: Common,
}

impl Component for Composite {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: CompositeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let mut child = IconButton::new(IconId::new("plus"), "Inner");
        cx.child("inner", &mut child, craft_behavior::Proposal::Ideal) + Vec2::new(80.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let _ = cx.hover_point();
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rect = Rect::from_min_size(cx.rect.min, Vec2::splat(24.0));
        let _ = ui.child("inner", rect, IconButton::new(IconId::new("plus"), "Inner"));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_rerun_composite_measures_its_unchanged_child_from_the_size_cache() {
    let mut pad = Rect::NOTHING;
    let step = |rig: &mut Rig<RecordingRenderer>, pad: &mut Rect| {
        rig.step_with(&mut |_, cx| window_root(cx, |ui| *pad = Show::show(Composite { common: Common::new() }, ui).rect))
    };
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let mut settled = false;
    for _ in 0..5 {
        if !step(&mut rig, &mut pad).ran {
            settled = true;
            break;
        }
    }
    assert!(settled, "the rig did not settle");
    let main = rig.main;
    rig.inject(main, Input::mouse_move(pad.center() + Vec2::new(30.0, 0.0)));
    let entered = step(&mut rig, &mut pad);
    assert_eq!(entered.frame.runs, 1, "the move runs the composite alone: {entered:?}");
    assert_eq!(entered.frame.measures, 1, "the composite measures; its child answers from its size cache: {entered:?}");
}

/// A press inside a collection nested in another reaches the outer one as a descendant press.
mod nested {
    use std::cell::Cell;

    use craft_behavior::{DisplayOrder, IconId, ItemSource, Selection, SpanLayout};
    use craft_ui_contract::contract::{
        Bind, BindMut, BodyCx, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput, NoValue, Outcome,
        Response, Role, SelectMode, ShowCollection as _, Ui,
    };
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_platform_headless::Input;
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_rt::{Pos2, Rect, StepReport, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use craft_ui_test_kit::rt::Rig;

    use crate::skeleton::IconButton;

    const ROW_H: f32 = 20.0;

    thread_local! {
        /// Real runs of the outer collection (its `update`).
        static OUTER_RUNS: Cell<u32> = const { Cell::new(0) };
        static INNER_RECT: Cell<Rect> = const { Cell::new(Rect::NOTHING) };
    }

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

    #[derive(Component)]
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

        fn finish(self, cx: Cx<'_, Self>) -> RowOut {
            RowOut { response: cx.response.clone() }
        }
    }

    /// The nested list: five selectable rows.
    #[derive(Component)]
    #[component(collection)]
    struct Inner<'a> {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        items: CollectionProps,
        selection: Option<Bind<'a, Selection<usize>>>,
    }

    impl Component for Inner<'_> {
        const ROLE: Role = Role::LIST;
        type Style = PlainStyle;
        type Output = CollectionResponse<usize, Never>;

        fn style(_: &StyleCx<'_>, _: InnerStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
            cx.collection_response()
        }
    }

    impl Collection for Inner<'_> {
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

    /// The outer list: no rows of its own; its empty state shows the inner list.
    #[derive(Component)]
    #[component(collection)]
    struct Outer<'a> {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        items: CollectionProps,
        #[key(skip)]
        inner: Option<&'a mut Selection<usize>>,
    }

    impl Component for Outer<'_> {
        const ROLE: Role = Role::LIST;
        type Style = PlainStyle;
        type Output = CollectionResponse<usize, Never>;

        fn style(_: &StyleCx<'_>, _: OuterStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn update(&mut self, _: &mut Cx<'_, Self>) {
            OUTER_RUNS.with(|runs| runs.set(runs.get() + 1));
        }

        fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
            cx.collection_response()
        }
    }

    impl Collection for Outer<'_> {
        type Key = usize;
        type ItemLayout = SpanLayout;
        type Selection = Selection<usize>;
        type Item<'r> = Row;
        type ItemEvent = Never;

        fn items(&self) -> &CollectionProps {
            &self.items
        }

        fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
            SpanLayout::new(source, width, ROW_H)
        }

        fn empty(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
            let inner = Inner {
                common: Common::new(),
                props: ContainerProps::new(),
                items: CollectionProps::new().select(SelectMode::Single),
                selection: self.inner.as_deref_mut().map(Bind::bound),
            };
            let out = inner.show_items(ui, &Count(5), &mut |_, _| Row { common: Common::new() });
            INNER_RECT.with(|rect| rect.set(out.response.rect));
        }
    }

    struct App {
        selection: Selection<usize>,
        button: Rect,
    }

    impl Default for App {
        fn default() -> App {
            App { selection: Default::default(), button: Rect::NOTHING }
        }
    }

    fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                app.button = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
                let outer =
                    Outer { common: Common::new(), props: ContainerProps::new(), items: CollectionProps::new(), inner: Some(&mut app.selection) };
                let _ = outer.show_items(ui, &Count(0), &mut |_, _| Row { common: Common::new() });
            })
        })
    }

    fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
        let main = rig.main;
        rig.inject(main, input);
        step(rig, app)
    }

    #[test]
    fn a_press_in_a_nested_collection_reaches_the_outer_one() {
        let mut app = App::default();
        let mut rig = Rig::inline(Vec2::new(300.0, 300.0), 1.0, RecordingRenderer::default());
        let mut settled = false;
        for _ in 0..5 {
            if !step(&mut rig, &mut app).ran {
                settled = true;
                break;
            }
        }
        assert!(settled, "the rig did not settle");
        let inner = INNER_RECT.with(Cell::get);
        let row = Pos2::new(inner.center().x, inner.min.y + ROW_H * 1.5);

        {
            let event = Input::mouse_move(app.button.center());
            input(&mut rig, &mut app, event)
        };
        let before = OUTER_RUNS.with(Cell::get);
        {
            let event = Input::mouse_down(app.button.center());
            input(&mut rig, &mut app, event)
        };
        {
            let event = Input::mouse_up(app.button.center());
            input(&mut rig, &mut app, event)
        };
        assert_eq!(OUTER_RUNS.with(Cell::get), before, "a press outside the outer collection does not run it");

        input(&mut rig, &mut app, Input::mouse_move(row));
        let before = OUTER_RUNS.with(Cell::get);
        input(&mut rig, &mut app, Input::mouse_down(row));
        assert!(OUTER_RUNS.with(Cell::get) > before, "the inner collection's press reaches the outer one");
        input(&mut rig, &mut app, Input::mouse_up(row));
        assert_eq!(app.selection.lead, Some(1), "the inner collection still takes the click");
    }
}

/// A members collection (`Collection::MEMBERS`): each row runs as a part of the collection's node.
mod members {
    use std::cell::RefCell;

    use craft_behavior::{DisplayOrder, IconId, ItemSource, Selection, SpanLayout};
    use craft_ui_contract::contract::{
        Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput, NoValue, Outcome, PaintUi,
        Response, Role, SelectMode, ShowCollection as _,
    };
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_platform_headless::Input;
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_rt::{Pos2, Rect, StepReport, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use craft_ui_test_kit::rt::Rig;

    use crate::skeleton::IconButton;

    const ROW_H: f32 = 20.0;
    const ROWS: usize = 3;

    thread_local! {
        /// Each member hook as it runs: the hook and the member's key.
        static HOOKS: RefCell<Vec<(&'static str, usize)>> = const { RefCell::new(Vec::new()) };
    }

    fn take_hooks() -> Vec<(&'static str, usize)> {
        HOOKS.with(|hooks| std::mem::take(&mut *hooks.borrow_mut()))
    }

    struct Count;

    impl DisplayOrder<usize> for Count {
        fn len(&self) -> usize {
            ROWS
        }

        fn key(&self, at: usize) -> usize {
            at
        }

        fn index_of(&self, key: &usize) -> Option<usize> {
            (*key < ROWS).then_some(*key)
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

    #[derive(Component)]
    struct Member {
        #[common]
        common: Common,
        key: usize,
    }

    impl Component for Member {
        const ROLE: Role = Role::TAB;
        type Style = PlainStyle;
        type Output = RowOut;

        fn style(_: &StyleCx<'_>, _: MemberStyleKey) -> PlainStyle {
            PlainStyle::min(Vec2::new(10.0, ROW_H))
        }

        fn update(&mut self, _: &mut Cx<'_, Self>) {
            HOOKS.with(|hooks| hooks.borrow_mut().push(("update", self.key)));
        }

        fn paint(&mut self, _: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
            HOOKS.with(|hooks| hooks.borrow_mut().push(("paint", self.key)));
        }

        fn finish(self, cx: Cx<'_, Self>) -> RowOut {
            HOOKS.with(|hooks| hooks.borrow_mut().push(("finish", self.key)));
            RowOut { response: cx.response.clone() }
        }
    }

    #[derive(Component)]
    #[component(collection)]
    struct Strip<'a> {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        items: CollectionProps,
        selection: Option<Bind<'a, Selection<usize>>>,
    }

    impl Component for Strip<'_> {
        const ROLE: Role = Role::TAB_LIST;
        type Style = PlainStyle;
        type Output = CollectionResponse<usize, Never>;

        fn style(_: &StyleCx<'_>, _: StripStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
            cx.collection_response()
        }
    }

    impl Collection for Strip<'_> {
        type Key = usize;
        type ItemLayout = SpanLayout;
        type Selection = Selection<usize>;
        type Item<'r> = Member;
        type ItemEvent = Never;
        const MEMBERS: bool = true;

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
        selection: Selection<usize>,
        strip: Rect,
        button: Rect,
    }

    impl Default for App {
        fn default() -> App {
            App { selection: Default::default(), strip: Rect::NOTHING, button: Rect::NOTHING }
        }
    }

    fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                app.button = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
                let strip = Strip {
                    common: Common::new(),
                    props: ContainerProps::new(),
                    items: CollectionProps::new().select(SelectMode::Single),
                    selection: Some(Bind::bound(&mut app.selection)),
                };
                app.strip = strip.show_items(ui, &Count, &mut |key, _| Member { common: Common::new(), key: *key }).response.rect;
            })
        })
    }

    fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
        let main = rig.main;
        rig.inject(main, input);
        step(rig, app)
    }

    fn row(app: &App, at: usize) -> Pos2 {
        Pos2::new(app.strip.center().x, app.strip.min.y + ROW_H * (at as f32 + 0.5))
    }

    #[test]
    fn a_collection_run_runs_each_member_once_in_hook_order_and_a_hover_runs_none() {
        let mut app = App::default();
        let mut rig = Rig::inline(Vec2::new(300.0, 300.0), 1.0, RecordingRenderer::default());
        let mut settled = false;
        for _ in 0..5 {
            if !step(&mut rig, &mut app).ran {
                settled = true;
                break;
            }
        }
        assert!(settled, "the rig did not settle");

        {
            let event = Input::mouse_move(row(&app, 0));
            input(&mut rig, &mut app, event)
        };
        take_hooks();
        let report = {
            let event = Input::mouse_move(row(&app, 1));
            input(&mut rig, &mut app, event)
        };
        assert!(take_hooks().is_empty(), "a hover moving between member rows runs no member: {report:?}");

        {
            let event = Input::mouse_down(row(&app, 1));
            input(&mut rig, &mut app, event)
        };
        take_hooks();
        {
            let event = Input::mouse_up(row(&app, 1));
            input(&mut rig, &mut app, event)
        };
        assert_eq!(app.selection.lead, Some(1), "the click on a member row selects it");
        let hooks = take_hooks();
        assert!(!hooks.is_empty(), "the selection change runs the collection and its members");
        for key in 0..ROWS {
            let mine: Vec<&str> = hooks.iter().filter(|(_, k)| *k == key).map(|(hook, _)| *hook).collect();
            let runs = mine.iter().filter(|hook| **hook == "update").count();
            assert!(runs >= 1, "member {key} ran: {mine:?}");
            // One collection run per change: each run gives the member one update, its paint looks and one finish.
            assert_eq!(mine.iter().filter(|hook| **hook == "finish").count(), runs, "member {key}: one finish per update: {mine:?}");
            let first_update = mine.iter().position(|hook| *hook == "update").unwrap();
            let first_paint = mine.iter().position(|hook| *hook == "paint").unwrap();
            let first_finish = mine.iter().position(|hook| *hook == "finish").unwrap();
            assert!(first_update < first_paint && first_paint < first_finish, "member {key} hook order: {mine:?}");
        }
        let updates = hooks.iter().filter(|(hook, _)| *hook == "update").count();
        assert_eq!(updates % ROWS, 0, "every member runs in every collection run: {hooks:?}");
        assert!(updates <= 2 * ROWS, "the click runs the collection at most once more than the release itself: {hooks:?}");
    }
}

/// Focus asked for by `Id` on a node that is no Tab stop (a collection row holding stops) lands on its first stop.
mod focus_into {
    use craft_behavior::IconId;
    use craft_ui_contract::contract::{Id, Props as _};
    use craft_ui_contract::runtime::focus::FocusTarget;
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_rt::Vec2;
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_test_kit::rt::Rig;

    use crate::skeleton::{IconButton, Stack};

    #[derive(Default)]
    struct App {
        ask: bool,
        asked: bool,
        first: bool,
        second: bool,
    }

    fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                let _ = IconButton::new(IconId::new("minus"), "Outside").focusable().show(ui);
                let row = Id::new("focus-into-row");
                Stack::new().id(row).show(ui, |ui| {
                    app.first = IconButton::new(IconId::new("plus"), "First").focusable().show(ui).has_focus();
                    app.second = IconButton::new(IconId::new("cross"), "Second").focusable().show(ui).has_focus();
                });
                if std::mem::take(&mut app.ask) {
                    app.asked = ui.request_focus(FocusTarget::Id(row));
                }
            })
        });
    }

    #[test]
    fn id_focus_on_a_row_that_is_no_stop_lands_on_its_first_stop() {
        let mut app = App::default();
        let mut rig = Rig::inline(Vec2::new(300.0, 200.0), 1.0, RecordingRenderer::default());
        for _ in 0..3 {
            step(&mut rig, &mut app);
        }
        assert!(!app.first && !app.second);
        // The app's state changed outside the build: the host marks the window's root.
        app.ask = true;
        let main = rig.main;
        rig.driver.runtime.mark_root(main);
        step(&mut rig, &mut app);
        assert!(app.asked, "the row's node answers to its id");
        for _ in 0..3 {
            step(&mut rig, &mut app);
        }
        assert!(app.first, "focus lands on the row's first stop");
        assert!(!app.second);
    }
}

/// Context requests reach the region around the requester (`Cx::context_requests`) through the descendant watch.
mod context_requests {
    use std::cell::{Cell, RefCell};

    use craft_behavior::{DisplayOrder, ItemSource, SpanLayout};
    use craft_ui_contract::contract::{
        Collection, CollectionProps, CollectionResponse, Common, Component, Container, ContainerProps, Cx, HoverLead, Id, ItemOutput, NoValue,
        Outcome, Response, Role, ShowCollection as _, ShowContainer,
    };
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_platform_headless::{Contact, Input};
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_rt::{PointerButton, PointerPhase, Pos2, Rect, Sense, StepReport, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use craft_ui_test_kit::rt::Rig;

    const ROW_H: f32 = 20.0;

    thread_local! {
        /// Runs of the region (its `update`).
        static REGION_RUNS: Cell<u32> = const { Cell::new(0) };
        /// The requests the region's `finish` read, every run.
        static SEEN: RefCell<Vec<(Id, Rect)>> = const { RefCell::new(Vec::new()) };
        static LIST_RECT: Cell<Rect> = const { Cell::new(Rect::NOTHING) };
    }

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

    /// A row that takes clicks, so a secondary click on it is its own context request.
    #[derive(Component)]
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

        fn sense(&self) -> Sense {
            Sense::HOVER | Sense::CLICK
        }

        fn finish(self, cx: Cx<'_, Self>) -> RowOut {
            RowOut { response: cx.response.clone() }
        }
    }

    #[derive(Component)]
    #[component(collection)]
    struct List {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
        items: CollectionProps,
    }

    impl Component for List {
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

    impl Collection for List {
        type Key = usize;
        type ItemLayout = SpanLayout;
        type Selection = craft_behavior::Selection<usize>;
        type Item<'r> = Row;
        type ItemEvent = Never;

        fn items(&self) -> &CollectionProps {
            &self.items
        }

        fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
            SpanLayout::new(source, width, ROW_H)
        }
    }

    /// The region around the list: it reads the context requests made under it, as a ribbon's region does.
    #[derive(Component)]
    #[component(container, unregistered)]
    struct Region {
        #[common]
        common: Common,
        #[container]
        props: ContainerProps,
    }

    impl Component for Region {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: RegionStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn update(&mut self, _: &mut Cx<'_, Self>) {
            REGION_RUNS.with(|runs| runs.set(runs.get() + 1));
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            let requests = cx.context_requests();
            SEEN.with(|seen| seen.borrow_mut().extend(requests));
            cx.response.clone()
        }
    }

    impl Container for Region {}

    fn step(rig: &mut Rig<RecordingRenderer>) -> StepReport {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                let region = Region { common: Common::new(), props: ContainerProps::new() };
                let _ = ShowContainer::show(region, ui, |ui, _| {
                    let items = CollectionProps::new().hover_lead(HoverLead::Lead);
                    let list = List { common: Common::new(), props: ContainerProps::new(), items };
                    let out = list.show_items(ui, &Count(4), &mut |_, _| Row { common: Common::new() });
                    LIST_RECT.with(|rect| rect.set(out.response.rect));
                });
            })
        })
    }

    fn input(rig: &mut Rig<RecordingRenderer>, input: Input) -> StepReport {
        let main = rig.main;
        rig.inject(main, input);
        step(rig)
    }

    fn secondary(phase: PointerPhase, pos: Pos2) -> Input {
        Input::pointer(Contact { button: PointerButton::Secondary, ..Contact::MOUSE }, phase, pos)
    }

    #[test]
    fn a_context_request_on_a_row_reaches_the_region_around_the_collection_with_one_run_of_it() {
        let mut rig = Rig::inline(Vec2::new(300.0, 300.0), 1.0, RecordingRenderer::default());
        let settled = (0..5).any(|_| !step(&mut rig).ran);
        assert!(settled, "the rig did not settle");
        // A second build reads the requests from then on: the first run made the region watch.
        let list = LIST_RECT.with(Cell::get);
        let row = Pos2::new(list.center().x, list.min.y + ROW_H * 1.5);
        input(&mut rig, Input::mouse_move(row));
        assert!(SEEN.with(|seen| seen.borrow().is_empty()), "no request without a click");

        let before = REGION_RUNS.with(Cell::get);
        input(&mut rig, secondary(PointerPhase::Down, row));
        assert_eq!(REGION_RUNS.with(Cell::get), before, "a secondary press makes no request and runs no region");
        input(&mut rig, secondary(PointerPhase::Up, row));
        assert_eq!(REGION_RUNS.with(Cell::get), before + 1, "the request runs the region once");
        let seen = SEEN.with(|seen| seen.borrow().clone());
        assert_eq!(seen.len(), 1, "the region reads the row's request: {seen:?}");
        assert!(seen[0].1.contains(row), "at the row's rect: {seen:?}");

        let before = REGION_RUNS.with(Cell::get);
        input(&mut rig, Input::mouse_move(row + Vec2::new(4.0, 0.0)));
        assert_eq!(REGION_RUNS.with(Cell::get), before, "a frame without requests runs no region");
    }

    /// The collection follows the pointer with its hover lead (`Cx::hover_point`): a move inside it runs it alone.
    #[test]
    fn a_hover_move_inside_a_collection_reruns_only_that_collection() {
        let mut rig = Rig::inline(Vec2::new(300.0, 300.0), 1.0, RecordingRenderer::default());
        assert!((0..5).any(|_| !step(&mut rig).ran), "the rig did not settle");
        let list = LIST_RECT.with(Cell::get);
        let row = Pos2::new(list.center().x, list.min.y + ROW_H * 0.5);
        input(&mut rig, Input::mouse_move(row));
        let regions = REGION_RUNS.with(Cell::get);
        let within = input(&mut rig, Input::mouse_move(row + Vec2::new(5.0, 2.0)));
        assert_eq!(within.frame.runs, 1, "a move within a row runs the collection alone: {within:?}");
        let next = input(&mut rig, Input::mouse_move(row + Vec2::new(0.0, ROW_H)));
        assert!(next.frame.runs <= 3, "a move to the next row runs the collection and the two rows whose lead moved: {next:?}");
        assert_eq!(REGION_RUNS.with(Cell::get), regions, "the region around it never runs");
    }
}

/// A run-time drop target (`Pointer::drop_target`) shows where a drop would land over its node while a drag is over it.
mod drop_mark {
    use craft_ui_contract::contract::*;
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_platform_headless::Input;
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_rt::{Pos2, Rect, Sense, StepReport, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use craft_ui_test_kit::rt::Rig;

    /// Drags a `u32`.
    #[derive(Component)]
    struct Source {
        #[common]
        common: Common,
    }

    impl Component for Source {
        const ROLE: Role = Role::BUTTON;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: SourceStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::splat(24.0)
        }

        fn sense(&self) -> Sense {
            Sense::HOVER | Sense::CLICK | Sense::DRAG
        }

        fn drag_source(&self) -> Option<DragSource> {
            Some(DragSource::new(1_u32, DragPreview::None))
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    /// Takes a `u32` anywhere over it; a drop would land on a line across its top.
    struct TopLine(Rect);

    impl DropTarget<u32> for TopLine {
        type Drop = ();

        fn accept(&self, _: &u32, _: Pos2, _: DropOperation) -> Option<()> {
            Some(())
        }

        fn mark(&self, _: &()) -> DropMark {
            DropMark::line(Pos2::new(self.0.min.x, self.0.min.y + 10.0), Pos2::new(self.0.max.x, self.0.min.y + 10.0))
        }
    }

    #[derive(Component)]
    struct Target {
        #[common]
        common: Common,
    }

    impl Component for Target {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: TargetStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(120.0, 60.0)
        }

        fn update(&mut self, cx: &mut Cx<'_, Self>) {
            let rect = cx.rect;
            let _ = cx.drop_target::<u32, _>(&TopLine(rect));
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    struct App {
        source: Rect,
        target: Rect,
    }

    impl Default for App {
        fn default() -> App {
            App { source: Rect::NOTHING, target: Rect::NOTHING }
        }
    }

    fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
        rig.step_with(&mut |_, cx| {
            window_root(cx, |ui| {
                app.source = Show::show(Source { common: Common::new() }, ui).rect;
                app.target = Show::show(Target { common: Common::new() }, ui).rect;
            })
        })
    }

    fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
        let main = rig.main;
        rig.inject(main, input);
        step(rig, app)
    }

    /// Whether the last frame paints a band as wide as the target's top line (the mark's band and its halo).
    fn shows_line(rig: &Rig<RecordingRenderer>, target: Rect) -> bool {
        let Some(frame) = rig.driver.renderer.last() else { return false };
        frame.draws.iter().any(|draw| {
            draw.list.items.iter().any(
                |item| matches!(item, craft_ui_rt::Item::Rect { rect, .. } if rect.height() <= 10.0 && (rect.width() - target.width()).abs() <= 10.0),
            )
        })
    }

    #[test]
    fn a_drop_mark_paints_over_a_run_time_target_while_a_drag_is_over_it() {
        let mut app = App::default();
        let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
        assert!((0..5).any(|_| !step(&mut rig, &mut app).ran), "the rig did not settle");
        assert!(!shows_line(&rig, app.target), "no mark without a drag");

        let from = app.source.center();
        input(&mut rig, &mut app, Input::mouse_move(from));
        input(&mut rig, &mut app, Input::mouse_down(from));
        for step in 1..=4 {
            input(&mut rig, &mut app, Input::mouse_move(from + Vec2::new(0.0, 6.0 * step as f32)));
        }
        let over = app.target.center();
        input(&mut rig, &mut app, Input::mouse_move(over));
        input(&mut rig, &mut app, Input::mouse_move(over + Vec2::new(3.0, 0.0)));
        assert!(rig.driver.runtime.drag().is_some(), "the drag runs");
        assert!(shows_line(&rig, app.target), "the target paints the mark of the drop it would take");

        input(&mut rig, &mut app, Input::mouse_move(Pos2::new(390.0, 290.0)));
        assert!(!shows_line(&rig, app.target), "the mark goes when the drag leaves the target");
        input(&mut rig, &mut app, Input::mouse_up(Pos2::new(390.0, 290.0)));
    }
}

/// A part's tooltip (`PartBuilder::tooltip`) shows under its part while the pointer rests on it, and on no other part.
mod part_tips {
    use craft_behavior::PartKey;
    use craft_ui_contract::contract::tooltip::DELAY;
    use craft_ui_contract::contract::*;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_contract::testing::Harness;
    use craft_ui_platform_headless::Input;
    use craft_ui_rt::{Rect, Sense, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;

    const LEFT: PartKey = PartKey::new("left");
    const RIGHT: PartKey = PartKey::new("right");

    /// Two halves; only the left one has a tooltip.
    #[derive(Component)]
    struct Strip {
        #[common]
        common: Common,
    }

    impl Component for Strip {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: StripStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(200.0, 40.0)
        }

        fn sense(&self) -> Sense {
            Sense::HOVER
        }

        fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
            let (left, right) = rect.split_left_right_at_fraction(0.5);
            cx.part(LEFT, left).sense(Sense::HOVER | Sense::CLICK).tooltip("Left Part");
            cx.part(RIGHT, right).sense(Sense::HOVER | Sense::CLICK);
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    #[test]
    fn a_part_tip_shows_under_its_part_only() {
        let mut h = Harness::new_ui_state(|ui, rect: &mut Rect| *rect = Show::show(Strip { common: Common::new() }, ui).rect, Rect::NOTHING);
        h.run();
        let (left, right) = h.state().split_left_right_at_fraction(0.5);
        let tip = |h: &Harness<Rect>| h.layers().into_iter().find(|layer| layer.tip);

        h.send(Input::mouse_move(left.center()));
        h.advance(DELAY + 0.05);
        h.run();
        let shown = tip(&h).expect("the left part's tooltip shows after the delay");
        assert!(shown.rect.min.y >= left.max.y - 0.5, "under the part: {:?} for {left:?}", shown.rect);
        assert!(shown.rect.min.x < right.min.x, "at the part's start: {:?} for {left:?}", shown.rect);
        assert!(h.query_by_label("Left Part").is_some(), "with the part's text");

        h.send(Input::mouse_move(right.center()));
        h.advance(DELAY + 0.05);
        h.run();
        assert!(tip(&h).is_none(), "the right part has no tooltip, and the component none of its own");

        h.send(Input::mouse_move(left.center()));
        h.run();
        assert!(tip(&h).is_none(), "back on the left part, the tooltip waits for the delay again");
        h.advance(DELAY + 0.05);
        h.run();
        assert!(tip(&h).is_some());
    }
}

/// OS file drags while a modal layer is shown reach only a target that takes files under a modal
/// (`DropTarget::under_modal`), and a two-finger tap reaches the node under the fingers once.
mod modal_files_and_taps {
    use std::cell::RefCell;

    use craft_ui_contract::contract::*;
    use craft_ui_contract::runtime::drag::OsFiles;
    use craft_ui_contract::runtime::windows::window_root;
    use craft_ui_contract::style::StyleCx;
    use craft_ui_platform_headless::{Contact, Input};
    use craft_ui_rt::record::RecordingRenderer;
    use craft_ui_rt::{BuildCx, Constraints, Element, Layer, LayerSpec, LayoutCx, Placement, PointerPhase, Pos2, Rect, Sense, StepReport, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use craft_ui_test_kit::rt::Rig;

    thread_local! {
        /// The targets that took a drop, by name.
        static DROPPED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        /// The multi-finger taps each pad took: its name and the finger count.
        static TAPS: RefCell<Vec<(&'static str, u8)>> = const { RefCell::new(Vec::new()) };
    }

    /// Takes OS files anywhere over its rect; under a modal only when `under_modal`.
    struct Files {
        rect: Rect,
        under_modal: bool,
    }

    impl DropTarget<OsFiles> for Files {
        type Drop = ();

        fn accept(&self, _: &OsFiles, _: Pos2, _: DropOperation) -> Option<()> {
            Some(())
        }

        fn mark(&self, _: &()) -> DropMark {
            DropMark::line(self.rect.left_top(), self.rect.right_top())
        }

        fn under_modal(&self) -> bool {
            self.under_modal
        }
    }

    /// A pad that takes OS files (under a modal when `under_modal`) and multi-finger taps.
    #[derive(Component)]
    struct Pad {
        #[common]
        common: Common,
        name: &'static str,
        under_modal: bool,
    }

    impl Component for Pad {
        const ROLE: Role = Role::GROUP;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: PadStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::new(120.0, 60.0)
        }

        fn sense(&self) -> Sense {
            Sense::HOVER | Sense::CLICK
        }

        fn update(&mut self, cx: &mut Cx<'_, Self>) {
            let rect = cx.rect;
            if cx.drop_target::<OsFiles, _>(&Files { rect, under_modal: self.under_modal }).is_some() {
                DROPPED.with(|d| d.borrow_mut().push(self.name));
            }
            if let Some(tap) = cx.take_multi_tap() {
                TAPS.with(|t| t.borrow_mut().push((self.name, tap.fingers)));
            }
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    /// A blank sheet on the modal layer, away from the pads.
    struct Sheet;

    impl Element for Sheet {
        type Output = ();
        type Local = ();

        fn props_key(&self) -> u64 {
            0
        }

        fn init(&self, _: &mut BuildCx<'_>) {}

        fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
            c.constrain(Vec2::new(40.0, 40.0))
        }
    }

    struct App {
        modal: bool,
        flagged: Rect,
        plain: Rect,
    }

    fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
        rig.step_with(&mut |_, cx| {
            if app.modal {
                cx.layer(Layer::Modal, "sheet", LayerSpec::new(Placement::At(Pos2::new(350.0, 250.0))), Sheet);
            }
            window_root(cx, |ui| {
                app.flagged = Show::show(Pad { common: Common::new(), name: "flagged", under_modal: true }, ui).rect;
                app.plain = Show::show(Pad { common: Common::new(), name: "plain", under_modal: false }, ui).rect;
            })
        })
    }

    fn settled(app: &mut App) -> Rig<RecordingRenderer> {
        let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
        assert!((0..5).any(|_| !step(&mut rig, app).ran), "the rig did not settle");
        rig
    }

    fn drop_files(rig: &mut Rig<RecordingRenderer>, app: &mut App, at: Pos2) {
        let main = rig.main;
        rig.driver.platform.file_drop(main, vec!["photo.png".into()], Some(at));
        step(rig, app);
        step(rig, app);
    }

    #[test]
    fn an_os_file_drag_under_a_modal_reaches_only_a_target_that_takes_files_under_a_modal() {
        let mut app = App { modal: true, flagged: Rect::NOTHING, plain: Rect::NOTHING };
        let mut rig = settled(&mut app);
        let (flagged, plain) = (app.flagged.center(), app.plain.center());
        drop_files(&mut rig, &mut app, plain);
        assert_eq!(DROPPED.with(|d| d.borrow().clone()), Vec::<&str>::new(), "the modal refuses the files over the plain target");
        drop_files(&mut rig, &mut app, flagged);
        assert_eq!(DROPPED.with(|d| d.borrow().clone()), vec!["flagged"], "the flagged target takes them under the modal");
        // The app's state changed outside the build: the host marks the window's root.
        app.modal = false;
        let main = rig.main;
        rig.driver.runtime.mark_root(main);
        step(&mut rig, &mut app);
        drop_files(&mut rig, &mut app, plain);
        assert_eq!(DROPPED.with(|d| d.borrow().clone()), vec!["flagged", "plain"], "without the modal the plain target takes them");
    }

    #[test]
    fn a_two_finger_tap_reaches_the_pad_under_the_fingers_once_and_no_other() {
        let mut app = App { modal: false, flagged: Rect::NOTHING, plain: Rect::NOTHING };
        let mut rig = settled(&mut app);
        let main = rig.main;
        let at = app.plain.center();
        let fingers = [(Contact::touch(1), at - Vec2::new(6.0, 0.0)), (Contact::touch(2), at + Vec2::new(6.0, 0.0))];
        for (contact, pos) in fingers {
            rig.inject(main, Input::pointer(contact, PointerPhase::Down, pos));
        }
        step(&mut rig, &mut app);
        for (contact, pos) in fingers {
            rig.inject(main, Input::pointer(contact, PointerPhase::Up, pos));
        }
        step(&mut rig, &mut app);
        step(&mut rig, &mut app);
        assert_eq!(TAPS.with(|t| t.borrow().clone()), vec![("plain", 2)], "one tap of two fingers, on the pad under them");
    }
}
