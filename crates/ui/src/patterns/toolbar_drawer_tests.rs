//! Edit Toolbar's drawer.

use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::rail::{LastUsed, RailTool, ToolRail};
use super::toolbar_drawer::{DrawerGroup, DropAt, ToolbarDrawer, ToolbarEdit, apply_toolbar_edit};
use super::{ToolSlot, ToolbarDrawerOutput};
use crate::icons::Icon;
use crate::theme::{Base, install};

type Tool = &'static str;

fn tool(id: Tool, icon: Icon, label: &str) -> RailTool<Tool> {
    RailTool::new(id, icon, label)
}

fn defaults() -> Vec<ToolSlot<Tool>> {
    vec![
        ToolSlot::new(0, vec![tool("move", Icon::Move, "Move")]),
        ToolSlot::new(1, vec![tool("brush", Icon::Brush, "Brush"), tool("pencil", Icon::Pencil, "Pencil")]),
        ToolSlot::new(2, vec![tool("stamp", Icon::Stamp, "Stamp")]),
    ]
}

fn groups() -> Vec<DrawerGroup<Tool>> {
    vec![
        DrawerGroup { label: "Select".to_owned(), tools: vec![tool("move", Icon::Move, "Move")] },
        DrawerGroup {
            label: "Paint".to_owned(),
            tools: vec![
                tool("brush", Icon::Brush, "Brush"),
                tool("pencil", Icon::Pencil, "Pencil"),
                tool("stamp", Icon::Stamp, "Stamp"),
                tool("eraser", Icon::Eraser, "Eraser"),
            ],
        },
    ]
}

struct App {
    slots: Vec<ToolSlot<Tool>>,
    groups: Vec<DrawerGroup<Tool>>,
    last: LastUsed<Tool>,
    open: bool,
    edits: Vec<ToolbarEdit<Tool>>,
    closed: usize,
    selected: Vec<Tool>,
    time: f64,
}

impl App {
    fn new() -> Self {
        Self {
            slots: defaults(),
            groups: groups(),
            last: LastUsed::default(),
            open: false,
            edits: Vec::new(),
            closed: 0,
            selected: Vec::new(),
            time: 0.0,
        }
    }

    fn frame(&mut self, ui: &mut Ui) {
        let out = ToolRail::new("tools").edit_toolbar("Edit Toolbar…").editing(self.open).show(ui, &self.slots, &"move", &mut self.last, |_, _| {});
        egui::CentralPanel::default().show(ui, |ui| ui.label("Document"));
        if out.edit_toolbar {
            self.open = !self.open;
        }
        if self.open {
            let result: ToolbarDrawerOutput<Tool> = ToolbarDrawer::new("edit").show(ui, out.rect, &self.groups, &self.slots);
            for edit in result.edits {
                self.slots = apply_toolbar_edit(&self.slots, &edit, &defaults());
                self.edits.push(edit);
            }
            self.selected.extend(result.select);
            if result.closed {
                self.open = false;
                self.closed += 1;
            }
        }
    }
}

fn harness(app: App) -> Harness<'static, App> {
    let builder = HarnessBuilder::default().with_size(vec2(800.0, 700.0)).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()));
    let mut h = builder.build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.input_mut().focused = true;
    h.run_steps(3);
    h
}

fn step(h: &mut Harness<'_, App>, events: Vec<Event>) {
    h.state_mut().time += 0.016;
    let time = h.state().time;
    h.input_mut().time = Some(time);
    h.input_mut().events.extend(events);
    h.run_steps(1);
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn click(h: &mut Harness<'_, App>, at: Pos2) {
    step(h, vec![Event::PointerMoved(at), button(at, true)]);
    step(h, vec![button(at, false)]);
    step(h, vec![]);
}

/// Presses at `from`, moves in steps to `to` and releases there.
fn drag(h: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    step(h, vec![Event::PointerMoved(from), button(from, true)]);
    for k in 1..=4 {
        step(h, vec![Event::PointerMoved(from + (to - from) * (k as f32 / 4.0))]);
    }
    step(h, vec![button(to, false)]);
    step(h, vec![]);
}

fn key(h: &mut Harness<'_, App>, key: Key) {
    let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    step(h, vec![event(true)]);
    step(h, vec![event(false)]);
    step(h, vec![]);
}

/// The centre of the node named `label` that is furthest left (the rail's button) or right (the drawer's row).
#[allow(clippy::expect_used)]
fn centre(h: &Harness<'_, App>, label: &str, rail: bool) -> Pos2 {
    let mut found: Vec<Pos2> = h.query_all_by_label(label).map(|n| n.rect().center()).collect();
    found.sort_by(|a, b| a.x.total_cmp(&b.x));
    let at = if rail { found.first() } else { found.last() };
    *at.expect("a node with that label")
}

fn open_drawer(h: &mut Harness<'_, App>) {
    let to = centre(h, "Edit Toolbar…", true);
    click(h, to);
    assert!(h.state().open);
    h.run_steps(2);
}

#[allow(clippy::expect_used)]
fn rail_cell(h: &Harness<'_, App>, label: &str) -> Rect {
    let at = centre(h, label, true);
    Rect::from_center_size(at, vec2(32.0, 32.0))
}

#[test]
fn dragging_a_drawer_row_onto_the_rail_adds_the_tool() {
    let mut h = harness(App::new());
    open_drawer(&mut h);
    let eraser = centre(&h, "Eraser", false);

    // Onto the top edge of the Brush button: a new slot before slot index 1.
    let brush = rail_cell(&h, "Brush");
    drag(&mut h, eraser, pos2(brush.center().x, brush.top() + 1.0));
    assert!(
        matches!(h.state().edits.as_slice(), [ToolbarEdit::Add { tool, at: DropAt::NewSlot { before: 1 } }] if tool.id == "eraser"),
        "{:?}",
        h.state().edits
    );
    let ids: Vec<Vec<Tool>> = h.state().slots.iter().map(|s| s.tools.iter().map(|t| t.id).collect()).collect();
    assert_eq!(ids, [vec!["move"], vec!["eraser"], vec!["brush", "pencil"], vec!["stamp"]]);

    // The eraser is in the toolbar now; its row is dimmed and cannot be picked up again.
    h.run_steps(2);
    let before = h.state().edits.len();
    let eraser_row = centre(&h, "Eraser", false);
    let stamp = rail_cell(&h, "Stamp");
    drag(&mut h, eraser_row, stamp.center());
    assert_eq!(h.state().edits.len(), before);

    // Onto the middle of a button: into that slot's group, last.
    let mut h = harness(App::new());
    open_drawer(&mut h);
    let eraser = centre(&h, "Eraser", false);
    let stamp = rail_cell(&h, "Stamp");
    drag(&mut h, eraser, stamp.center());
    assert!(matches!(h.state().edits.as_slice(), [ToolbarEdit::Add { at: DropAt::Slot { slot: 2, index: 1 }, .. }]), "{:?}", h.state().edits);
    assert_eq!(h.state().slots[2].tools.iter().map(|t| t.id).collect::<Vec<_>>(), ["stamp", "eraser"]);
}

#[test]
fn dragging_a_rail_tool_into_the_drawer_removes_it() {
    let mut h = harness(App::new());
    open_drawer(&mut h);
    let stamp = rail_cell(&h, "Stamp").center();
    let eraser_row = centre(&h, "Eraser", false);
    drag(&mut h, stamp, eraser_row);
    assert!(matches!(h.state().edits.as_slice(), [ToolbarEdit::Remove { tool: "stamp" }]), "{:?}", h.state().edits);
    let ids: Vec<Vec<Tool>> = h.state().slots.iter().map(|s| s.tools.iter().map(|t| t.id).collect()).collect();
    assert_eq!(ids, [vec!["move"], vec!["brush", "pencil"]], "the slot the stamp left empty goes");

    // Restore Defaults brings it back.
    h.run_steps(2);
    let restore = centre(&h, "Restore Defaults", false);
    click(&mut h, restore);
    assert_eq!(h.state().slots, defaults());
}

#[test]
fn drawer_closes_on_escape_and_returns_focus() {
    let mut h = harness(App::new());
    open_drawer(&mut h);
    assert_eq!(h.state().closed, 0);
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert_eq!(h.state().closed, 1);
    assert!(!h.state().open);
    let focused = h.query_all_by_label("Edit Toolbar…").any(|n| n.accesskit_node().is_focused());
    assert!(focused, "focus returns to the Edit Toolbar button");
}

#[test]
fn space_picks_a_row_up_arrows_choose_its_place_and_enter_drops_it() {
    let mut h = harness(App::new());
    open_drawer(&mut h);
    h.get_by_label("Eraser").focus();
    h.run_steps(2);
    key(&mut h, Key::Space);
    key(&mut h, Key::ArrowUp);
    key(&mut h, Key::ArrowUp);
    key(&mut h, Key::Enter);
    // The place starts after the last slot; two steps up (past the last slot's group) is before it.
    assert!(
        matches!(h.state().edits.as_slice(), [ToolbarEdit::Add { tool, at: DropAt::NewSlot { before: 2 } }] if tool.id == "eraser"),
        "{:?}",
        h.state().edits
    );
}
