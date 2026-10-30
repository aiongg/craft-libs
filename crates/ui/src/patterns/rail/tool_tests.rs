//! `ToolRail` tests: the pure slot rules, then the rail driven by input.

use egui::accesskit::{HasPopup, Orientation, Role, Toggled};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::tool::stories;
use super::tool::{GridMove, grid_step};
use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::panes::Edge;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

type Tool = &'static str;

fn slots() -> Vec<ToolSlot<Tool>> {
    vec![
        ToolSlot::new(0, vec![RailTool::new("move", Icon::Move, "Move").shortcut("V")]),
        ToolSlot::new(
            1,
            vec![RailTool::new("brush", Icon::Brush, "Brush").shortcut("B"), RailTool::new("pencil", Icon::Pencil, "Pencil").shortcut("B")],
        ),
        ToolSlot::new(2, vec![RailTool::new("stamp", Icon::Stamp, "Stamp")]),
        ToolSlot::new(3, vec![RailTool::new("eraser", Icon::Eraser, "Eraser"), RailTool::new("gum", Icon::EraserMagic, "Gum")]),
    ]
}

fn ids(tools: &[&RailTool<Tool>]) -> Vec<Tool> {
    tools.iter().map(|tool| tool.id).collect()
}

#[test]
fn hidden_tools_leave_their_slot_and_an_empty_slot_is_dropped_with_indices_kept() {
    let slots = slots();
    let all = visible_slots(&slots, &[]);
    assert_eq!(all.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 1, 2, 3]);

    let some = visible_slots(&slots, &["pencil", "stamp"]);
    assert_eq!(some.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 1, 3], "the stamp slot is empty");
    assert_eq!(ids(&some[1].1), ["brush"], "the brush slot keeps its visible tool");

    let none = visible_slots(&slots, &["move", "brush", "pencil", "stamp", "eraser", "gum"]);
    assert!(none.is_empty());
}

#[test]
fn a_slot_shows_the_current_tool_then_the_stored_one_then_the_first() {
    let slots = slots();
    let visible = visible_slots(&slots, &[]);
    let brush = &visible[1].1;
    let mut last = LastUsed::default();
    let shown = |current: Tool, last: &LastUsed<Tool>| shown_tool(&current, 1, brush, last).map(|tool| tool.id);
    assert_eq!(shown("pencil", &last), Some("pencil"), "the current tool of the slot");
    assert_eq!(shown("move", &last), Some("brush"), "nothing stored: the first");
    last.set(1, "pencil");
    assert_eq!(shown("move", &last), Some("pencil"), "the stored tool");
    assert_eq!(shown("brush", &last), Some("brush"), "the current tool wins over the stored one");

    let hidden = visible_slots(&slots, &["pencil"]);
    assert_eq!(shown_tool(&"move", 1, &hidden[1].1, &last).map(|tool| tool.id), Some("brush"), "the stored tool is hidden");
    assert_eq!(shown_tool(&"move", 1, &[], &last), None);
}

#[test]
fn last_used_keeps_one_tool_per_slot() {
    let mut last = LastUsed::default();
    assert_eq!(last.get(3), None);
    last.set(3, "gum");
    last.set(1, "pencil");
    last.set(3, "eraser");
    assert_eq!((last.get(1), last.get(3)), (Some(&"pencil"), Some(&"eraser")));
}

#[test]
fn the_grid_moves_by_rows_and_columns_and_skips_disabled_buttons() {
    let on = [true; 5];
    assert_eq!(grid_step(0, &on, 2, GridMove::Down), Some(2));
    assert_eq!(grid_step(3, &on, 2, GridMove::Up), Some(1));
    assert_eq!(grid_step(1, &on, 2, GridMove::Down), Some(3));
    assert_eq!(grid_step(3, &on, 2, GridMove::Down), None, "no button below in a partial last row");
    assert_eq!(grid_step(0, &on, 2, GridMove::Up), None);
    assert_eq!(grid_step(1, &on, 2, GridMove::Next), Some(2), "reading order wraps to the next row");
    assert_eq!(grid_step(0, &on, 2, GridMove::Previous), None);
    assert_eq!(grid_step(1, &on, 1, GridMove::Down), Some(2));
    let off = [true, false, false, true, true];
    assert_eq!(grid_step(0, &off, 1, GridMove::Down), Some(3), "disabled buttons are skipped");
    assert_eq!(grid_step(3, &off, 1, GridMove::Up), Some(0));
    assert_eq!(grid_step(0, &off, 2, GridMove::Next), Some(3));
}

/// The host side: the tool, what the rail reported, and the footer.
struct App {
    slots: Vec<ToolSlot<Tool>>,
    current: Tool,
    last: LastUsed<Tool>,
    hidden: Vec<Tool>,
    columns: usize,
    edge: Edge,
    chrome: bool,
    selects: Vec<Tool>,
    doubles: Vec<Tool>,
    toggles: usize,
    edits: usize,
    time: f64,
    out: Option<ToolRailOutput<Tool>>,
    footer: Option<RailFooter>,
}

impl App {
    fn new() -> Self {
        Self {
            slots: slots(),
            current: "move",
            last: LastUsed::default(),
            hidden: Vec::new(),
            columns: 1,
            edge: Edge::Start,
            chrome: true,
            selects: Vec::new(),
            doubles: Vec::new(),
            toggles: 0,
            edits: 0,
            time: 0.0,
            out: None,
            footer: None,
        }
    }

    fn frame(&mut self, ui: &mut Ui) {
        let mut footer = None;
        let mut rail = ToolRail::new("tools").edge(self.edge).columns(self.columns).hidden(&self.hidden).label("Tools");
        if self.chrome {
            rail = rail.column_toggle("Toolbar").edit_toolbar("Edit Toolbar…");
        }
        let out = rail.show(ui, &self.slots, &self.current, &mut self.last, |ui, info| {
            footer = Some(info);
            let _ = ui.button("Foot");
        });
        egui::CentralPanel::default().show(ui, |ui| ui.label("Document"));
        if let Some(tool) = out.select {
            self.current = tool;
            self.selects.push(tool);
        }
        self.doubles.extend(out.double_clicked);
        self.toggles += usize::from(out.toggle_columns);
        self.edits += usize::from(out.edit_toolbar);
        self.footer = footer;
        self.out = Some(out);
    }

    #[allow(clippy::unwrap_used)]
    fn out(&self) -> &ToolRailOutput<Tool> {
        self.out.as_ref().unwrap()
    }
}

fn harness_in(dir: Direction, app: App, height: f32) -> Harness<'static, App> {
    let builder = HarnessBuilder::default().with_size(vec2(800.0, height)).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    });
    let mut h = builder.build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.input_mut().focused = true;
    h.run_steps(3);
    h
}

fn harness(app: App) -> Harness<'static, App> {
    harness_in(Direction::Ltr, app, 700.0)
}

fn button(at: Pos2, b: PointerButton, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: b, pressed, modifiers: Modifiers::NONE }
}

/// Runs one frame 16 ms after the last, with `events`.
fn step(h: &mut Harness<'_, App>, events: Vec<Event>) {
    h.state_mut().time += 0.016;
    let time = h.state().time;
    h.input_mut().time = Some(time);
    h.input_mut().events.extend(events);
    h.run_steps(1);
}

fn click_with(h: &mut Harness<'_, App>, at: Pos2, b: PointerButton) {
    step(h, vec![Event::PointerMoved(at), button(at, b, true)]);
    step(h, vec![button(at, b, false)]);
    step(h, vec![]);
}

fn click(h: &mut Harness<'_, App>, at: Pos2) {
    click_with(h, at, PointerButton::Primary);
}

fn click_on(h: &mut Harness<'_, App>, label: &str) {
    let to = at(h, label);
    click(h, to);
}

fn click_on_with(h: &mut Harness<'_, App>, label: &str, b: PointerButton) {
    let to = at(h, label);
    click_with(h, to, b);
}

fn key(h: &mut Harness<'_, App>, key: Key) {
    let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    step(h, vec![event(true)]);
    step(h, vec![event(false)]);
    step(h, vec![]);
}

fn at(h: &Harness<'_, App>, label: &str) -> Pos2 {
    rect(h, label).center()
}

#[allow(clippy::expect_used)]
fn rect(h: &Harness<'_, App>, label: &str) -> Rect {
    h.query_by_label(label).expect("a button").rect()
}

fn focused(h: &Harness<'_, App>) -> Option<String> {
    ["Move", "Brush", "Pencil", "Stamp", "Eraser", "Gum", "Edit Toolbar…", "Toolbar"]
        .into_iter()
        .find(|label| h.query_all_by_label(label).any(|n| n.accesskit_node().is_focused()))
        .map(str::to_owned)
}

#[test]
fn a_click_selects_the_shown_tool_and_a_flyout_row_selects_that_tool() {
    let mut h = harness(App::new());
    click_on(&mut h, "Brush");
    assert_eq!(h.state().selects, ["brush"]);
    assert_eq!(h.state().out().select, None, "one pass only");

    click_on_with(&mut h, "Brush", PointerButton::Secondary);
    assert_eq!(h.state().out().flyout, Some(1));
    click_on(&mut h, "Pencil");
    assert_eq!(h.state().selects, ["brush", "pencil"]);
    assert_eq!(h.state().out().flyout, None);
    assert!(h.query_by_label("Pencil").is_some(), "the slot now shows the pencil");
    assert!(h.query_by_label("Brush").is_none());

    click_on(&mut h, "Move");
    assert_eq!(h.state().current, "move");
    assert!(h.query_by_label("Pencil").is_some(), "the slot keeps the last used tool");
}

#[test]
fn a_double_click_reports_the_shown_tool() {
    let mut h = harness(App::new());
    click_on(&mut h, "Stamp");
    click_on(&mut h, "Stamp");
    assert_eq!(h.state().doubles, ["stamp"]);
}

#[test]
fn the_slot_with_the_current_tool_is_selected_and_hidden_tools_are_not_listed() {
    let mut app = App::new();
    app.current = "gum";
    app.hidden = vec!["stamp", "pencil"];
    let mut h = harness(app);
    assert_eq!(h.get_by_label("Gum").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(h.get_by_label("Move").accesskit_node().toggled(), Some(Toggled::False));
    assert!(h.query_by_label("Stamp").is_none(), "a slot without visible tools is dropped");
    // The brush slot has one tool left, so it is a plain button.
    assert_eq!(h.get_by_label("Brush").accesskit_node().has_popup(), None);
    click_on_with(&mut h, "Brush", PointerButton::Secondary);
    assert_eq!(h.state().out().flyout, None);
}

#[test]
#[allow(clippy::expect_used)]
fn two_columns_widen_the_rail_by_a_button_and_a_gap_and_fill_rows_of_two() {
    let tokens = {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut metrics = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let tokens = Tokens::of(ui);
            metrics = Some((
                ToolRail::<Tool>::metrics(&tokens, 1),
                ToolRail::<Tool>::metrics(&tokens, 2),
                tokens.component.rail.button,
                tokens.component.rail.gap,
            ));
        })
        .drop_without_applying_deltas();
        metrics.expect("metrics")
    };
    let (one, two, button, gap) = tokens;
    assert_eq!(two.width - one.width, button + gap);

    let single = harness(App::new());
    assert_eq!(single.state().out().rect.width(), one.width);
    let (move1, brush1) = (rect(&single, "Move"), rect(&single, "Brush"));
    assert_eq!(move1.left(), brush1.left(), "one column");
    assert!(brush1.top() > move1.top());

    let mut app = App::new();
    app.columns = 2;
    let h = harness(app);
    assert_eq!(h.state().out().rect.width(), two.width);
    let (a, b, c) = (rect(&h, "Move"), rect(&h, "Brush"), rect(&h, "Stamp"));
    assert_eq!(a.top(), b.top(), "the second slot is in the first row");
    assert_eq!(b.left() - a.left(), button + gap);
    assert_eq!((c.left(), c.top() > a.top()), (a.left(), true), "the third starts the next row");
    assert_eq!(h.state().footer, Some(RailFooter { columns: 2 }));
}

#[test]
fn in_a_tall_window_the_footer_lies_right_under_the_last_button() {
    let h = harness_in(Direction::Ltr, App::new(), 900.0);
    let (edit, foot) = (rect(&h, "Edit Toolbar…"), rect(&h, "Foot"));
    let gap = {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut gap = 0.0;
        ctx.run_ui(egui::RawInput::default(), |ui| gap = Tokens::of(ui).component.rail.row_gap).drop_without_applying_deltas();
        gap
    };
    assert_eq!(foot.top() - edit.bottom(), gap, "{edit:?} {foot:?}");
    assert!(foot.bottom() < 900.0 - 100.0, "not pinned to the window bottom");
}

#[test]
fn in_a_short_window_the_slots_scroll_and_the_footer_stays_in_view() {
    let mut h = harness_in(Direction::Ltr, App::new(), 220.0);
    let foot = rect(&h, "Foot");
    assert!(foot.bottom() <= 220.0 && foot.bottom() > 220.0 - 20.0 - 8.0, "the footer is fully visible at the bottom: {foot:?}");
    assert!(rect(&h, "Eraser").bottom() > foot.top(), "the last slot starts out of view");

    let over = at(&h, "Move");
    h.input_mut().events.extend([
        Event::PointerMoved(over),
        Event::MouseWheel {
            source: egui::MouseWheelSource::Unknown,
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -800.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.run_steps(3);
    assert!(rect(&h, "Edit Toolbar…").bottom() <= rect(&h, "Foot").top(), "the Edit Toolbar button is reachable above the footer");
    assert_eq!(rect(&h, "Foot"), foot);
}

#[test]
fn the_column_toggle_and_edit_toolbar_report_and_are_named_by_the_host() {
    let mut h = harness(App::new());
    assert_eq!(h.get_by_label("Toolbar").accesskit_node().role(), Role::Button);
    click_on(&mut h, "Toolbar");
    assert_eq!(h.state().toggles, 1);
    assert!(h.state().selects.is_empty());
    click_on(&mut h, "Edit Toolbar…");
    assert_eq!((h.state().edits, h.state().toggles), (1, 1));
    assert!(rect(&h, "Edit Toolbar…").top() > rect(&h, "Eraser").top(), "after the last slot");

    let mut app = App::new();
    app.chrome = false;
    let h = harness(app);
    assert!(h.query_by_label("Toolbar").is_none() && h.query_by_label("Edit Toolbar…").is_none());
}

#[test]
fn the_arrow_keys_move_by_the_grid() {
    let mut h = harness(App::new());
    h.get_by_label("Move").focus();
    h.run_steps(2);
    assert_eq!(focused(&h).as_deref(), Some("Move"));
    key(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("Brush"));
    key(&mut h, Key::ArrowRight);
    assert_eq!(focused(&h).as_deref(), Some("Brush"), "no horizontal move in one column");
    key(&mut h, Key::ArrowUp);
    assert_eq!(focused(&h).as_deref(), Some("Move"));
    key(&mut h, Key::ArrowUp);
    assert_eq!(focused(&h).as_deref(), Some("Move"), "stops at the top");

    let mut app = App::new();
    app.columns = 2;
    let mut h = harness(app);
    h.get_by_label("Move").focus();
    h.run_steps(2);
    key(&mut h, Key::ArrowRight);
    assert_eq!(focused(&h).as_deref(), Some("Brush"));
    key(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("Eraser"));
    key(&mut h, Key::ArrowLeft);
    assert_eq!(focused(&h).as_deref(), Some("Stamp"));
    key(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h).as_deref(), Some("Edit Toolbar…"));
}

#[test]
fn in_a_mirrored_interface_the_rail_sits_at_the_other_edge_and_left_moves_forward() {
    for (dir, edge, left) in [
        (Direction::Ltr, Edge::Start, true),
        (Direction::Rtl, Edge::Start, false),
        (Direction::Ltr, Edge::End, false),
        (Direction::Rtl, Edge::End, true),
    ] {
        let mut app = App::new();
        app.edge = edge;
        app.columns = 2;
        let mut h = harness_in(dir, app, 700.0);
        let rail = h.state().out().rect;
        let case = format!("{dir:?} {edge:?}: {rail:?}");
        assert_eq!(rail.left() == 8.0, left, "{case}");
        let (a, b) = (rect(&h, "Move"), rect(&h, "Brush"));
        assert_eq!(b.left() > a.left(), dir == Direction::Ltr, "the first column is at the reading start: {case}");
        h.get_by_label("Move").focus();
        h.run_steps(2);
        key(&mut h, if dir == Direction::Ltr { Key::ArrowRight } else { Key::ArrowLeft });
        assert_eq!(focused(&h).as_deref(), Some("Brush"), "{case}");
    }
}

#[test]
fn while_a_flyout_is_open_the_rail_leaves_the_arrow_keys_to_its_menu() {
    let mut h = harness(App::new());
    h.get_by_label("Brush").focus();
    h.run_steps(2);
    key(&mut h, Key::Menu);
    assert_eq!(h.state().out().flyout, Some(1));
    key(&mut h, Key::ArrowDown);
    assert_eq!(h.state().out().flyout, Some(1), "the menu stays open");
    assert_eq!(focused(&h).as_deref(), Some("Pencil"), "the menu moved its own row; the rail did not move the focus");
}

#[test]
fn accesskit_names_the_toolbar_the_slots_and_their_shortcuts() {
    let h = harness(App::new());
    let toolbar = h.get_by_role(Role::Toolbar).accesskit_node();
    assert_eq!(toolbar.label().as_deref(), Some("Tools"));
    assert_eq!(toolbar.orientation(), Some(Orientation::Vertical));
    let brush = h.get_by_label("Brush").accesskit_node();
    assert_eq!(brush.role(), Role::Button);
    assert_eq!(brush.has_popup(), Some(HasPopup::Menu));
    assert_eq!(brush.data().is_expanded(), Some(false));
    assert_eq!(brush.description().as_deref(), Some("B"));
    assert_eq!(h.get_by_label("Move").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(h.get_by_label("Stamp").accesskit_node().has_popup(), None, "a single tool opens nothing");
}

#[test]
fn the_story_names_every_control_in_both_directions() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for story in stories() {
            let mut args = (story.args)();
            let show = story.show;
            let builder = HarnessBuilder::default().with_size(vec2(800.0, 700.0)).with_context_setup(move |ctx: &Context| {
                install(ctx, &Base::suite_default());
                direction::apply(ctx, dir);
            });
            let mut harness = builder.build_ui_state(move |ui, _: &mut App| show(ui, &mut args), App::new());
            harness.run();
            a11y::assert_labelled(&harness);
        }
    }
}
