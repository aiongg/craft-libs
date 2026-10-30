//! [`GradientPalette`] in a headless egui context: picks, keys, the list form, AccessKit and
//! right-to-left order.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{ColorStop, GradientGroup, GradientItem, GradientPalette, GradientRef, GradientStops, PaletteLayout};
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::theme::{Base, install};

fn stops(shade: f32) -> GradientStops {
    GradientStops::new(vec![ColorStop::new(0.0, Srgba::rgb(shade, 0.0, 0.0)), ColorStop::new(1.0, Srgba::rgb(0.0, 0.0, 1.0))])
}

fn groups() -> Vec<GradientGroup> {
    let items = |names: &[&str]| names.iter().enumerate().map(|(i, n)| GradientItem::new(*n, stops(i as f32 / 8.0))).collect();
    vec![GradientGroup::new("Basics", items(&["Ash", "Brick", "Clay", "Dune"])), GradientGroup::new("Cool", items(&["Ice"]))]
}

struct Rig {
    ctx: Context,
    time: f64,
    layout: PaletteLayout,
    selected: Option<GradientRef>,
    width: f32,
    groups: Vec<GradientGroup>,
    recent: Vec<GradientStops>,
    events: Vec<EditEvent<GradientRef>>,
    root: Id,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(layout: PaletteLayout, direction: Direction) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        direction::apply(&ctx, direction);
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            layout,
            selected: None,
            // Three 38 pt slots to a row.
            width: 120.0,
            groups: groups(),
            recent: vec![stops(0.9), stops(0.8)],
            events: Vec::new(),
            root: Id::NULL,
            nodes: Vec::new(),
        };
        for _ in 0..3 {
            rig.frame(vec![]);
        }
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let mut shown = None;
        let mut root = Id::NULL;
        let output = self.ctx.clone().run_ui(input, |ui| {
            root = ui.make_persistent_id(egui::IdSalt::new("palette"));
            ui.set_width(self.width);
            shown = Some(
                GradientPalette::new("palette", "Presets", &self.groups)
                    .recent(&self.recent, "Recent")
                    .layout(self.layout)
                    .max_height(400.0)
                    .show(ui),
            );
        });
        self.root = root;
        self.events.extend(shown.and_then(|s| s.event));
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn cell(&self, i: usize) -> Id {
        self.root.with(("preset", i))
    }

    fn rect(&self, i: usize) -> Rect {
        self.ctx.read_response(self.cell(i)).map(|r| r.rect).unwrap_or(Rect::NOTHING)
    }

    fn focused(&self) -> Option<usize> {
        (0..8).find(|&i| self.ctx.memory(|m| m.has_focus(self.cell(i))))
    }

    fn focus(&mut self, i: usize) {
        self.ctx.memory_mut(|m| m.request_focus(self.cell(i)));
        self.frame(vec![]);
    }

    /// Press and release in one frame, a second after the last click.
    fn click(&mut self, i: usize) {
        let at = self.rect(i).center();
        let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.time += 1.0;
        self.frame(vec![Event::PointerMoved(at), button(true), button(false)]);
        self.frame(vec![]);
    }

    fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
        self.frame(vec![]);
    }

    /// The active row of the list form.
    fn active(&self) -> Option<usize> {
        self.ctx.data(|d| d.get_temp::<usize>(self.root.with("active")))
    }

    fn bounds(&self, name: &str) -> Rect {
        let b = self.nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some(name)).and_then(|n| n.bounds()).expect("a node with bounds");
        Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
    }

    /// Strokes of the selection ring's halo and accent widths drawn outside the cells.
    fn ring_strokes(&mut self) -> usize {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), ..Default::default() };
        self.time += 1.0 / 60.0;
        let selected = self.selected;
        let output = self.ctx.clone().run_ui(input, |ui| {
            ui.set_width(self.width);
            let _ = GradientPalette::new("palette", "Presets", &self.groups).layout(self.layout).selected(selected).show(ui);
        });
        let n = output.shapes.iter().filter(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.stroke_kind == egui::StrokeKind::Outside)).count();
        output.drop_without_applying_deltas();
        n
    }

    fn node(&self, role: Role, name: &str) -> &Node {
        self.nodes.iter().map(|(_, n)| n).find(|n| n.role() == role && n.label() == Some(name)).unwrap_or_else(|| panic!("no {role:?} named {name}"))
    }
}

#[test]
fn a_click_is_one_set_of_the_preset_clicked_and_nothing_else_changes() {
    let mut rig = Rig::new(PaletteLayout::Grid, Direction::Ltr);
    // Entries: Recent 1, Recent 2, then Basics (Ash..Dune), then Cool (Ice).
    rig.click(4);
    rig.click(0);
    rig.click(6);
    let picks: Vec<_> = rig.events.iter().map(|e| (matches!(e, EditEvent::Set { .. }), e.value().copied())).collect();
    assert_eq!(
        picks,
        [
            (true, Some(GradientRef::Item { group: 0, item: 2 })),
            (true, Some(GradientRef::Recent(0))),
            (true, Some(GradientRef::Item { group: 1, item: 0 })),
        ]
    );
}

#[test]
fn enter_picks_the_focused_cell_and_the_arrows_move_without_picking() {
    let mut rig = Rig::new(PaletteLayout::Grid, Direction::Ltr);
    // Rows: [0 1] [2 3 4] [5] [6]; each group starts a row.
    rig.focus(1);
    rig.key(Key::ArrowDown);
    assert_eq!(rig.focused(), Some(3), "down keeps the column across the group break");
    rig.key(Key::ArrowDown);
    assert_eq!(rig.focused(), Some(5));
    rig.key(Key::ArrowDown);
    assert_eq!(rig.focused(), Some(6));
    rig.key(Key::ArrowUp);
    rig.key(Key::ArrowUp);
    assert_eq!(rig.focused(), Some(3), "up returns to the column the move started in");
    rig.key(Key::ArrowRight);
    rig.key(Key::ArrowRight);
    assert_eq!(rig.focused(), Some(5), "right runs on to the next row");
    rig.key(Key::Home);
    assert_eq!(rig.focused(), Some(0));
    rig.key(Key::End);
    assert_eq!(rig.focused(), Some(6));
    assert!(rig.events.is_empty());
    rig.key(Key::Enter);
    assert_eq!(rig.events.iter().filter_map(|e| e.value().copied()).collect::<Vec<_>>(), [GradientRef::Item { group: 1, item: 0 }]);
}

#[test]
fn the_list_form_shows_names_and_up_and_down_move_while_left_and_right_do_nothing() {
    let mut rig = Rig::new(PaletteLayout::List, Direction::Ltr);
    let list = rig.node(Role::ListBox, "Presets");
    assert_eq!(list.size_of_set(), Some(7));
    for (i, name) in ["Recent 1", "Recent 2", "Ash", "Brick", "Clay", "Dune", "Ice"].into_iter().enumerate() {
        assert_eq!(rig.node(Role::ListBoxOption, name).position_in_set(), Some(i));
    }
    // One row per preset, in order down the list.
    let (ash, brick) = (rig.bounds("Ash"), rig.bounds("Brick"));
    assert!(ash.top() < brick.top() && ash.left() == brick.left());
    assert!(ash.width() > 100.0, "a row spans the width");
    rig.ctx.memory_mut(|m| m.request_focus(rig.root));
    rig.frame(vec![]);
    assert_eq!(rig.active(), Some(0));
    rig.key(Key::ArrowDown);
    rig.key(Key::ArrowDown);
    assert_eq!(rig.active(), Some(2));
    rig.key(Key::ArrowRight);
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.active(), Some(2));
    rig.key(Key::ArrowUp);
    assert_eq!(rig.active(), Some(1));
    rig.key(Key::Enter);
    assert_eq!(rig.events.iter().filter_map(|e| e.value().copied()).collect::<Vec<_>>(), [GradientRef::Recent(1)]);
}

#[test]
fn accesskit_names_the_group_and_each_cell_and_there_is_one_tab_stop() {
    let rig = Rig::new(PaletteLayout::Grid, Direction::Ltr);
    assert_eq!(rig.node(Role::Group, "Presets").size_of_set(), Some(7));
    for (i, name) in ["Recent 1", "Recent 2", "Ash", "Brick", "Clay", "Dune", "Ice"].into_iter().enumerate() {
        assert_eq!(rig.node(Role::Button, name).position_in_set(), Some(i));
    }
    // Only the first cell, the resting stop, is reached by Tab; the others take a screen reader's Focus.
    let focusable: Vec<bool> = (0..7).map(|i| rig.ctx.read_response(rig.cell(i)).is_some_and(|r| r.sense.is_focusable())).collect();
    assert_eq!(focusable, [true, false, false, false, false, false, false]);
}

#[test]
fn right_to_left_runs_the_cells_and_the_arrows_from_the_right_and_leaves_the_gradients() {
    let mut rig = Rig::new(PaletteLayout::Grid, Direction::Rtl);
    assert!(rig.rect(0).center().x > rig.rect(1).center().x, "the first cell is at the start, on the right");
    assert!(rig.rect(2).center().x > rig.rect(3).center().x);
    rig.focus(2);
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.focused(), Some(3), "Left is forward right to left");
    rig.key(Key::ArrowRight);
    assert_eq!(rig.focused(), Some(2));
}

#[test]
fn only_a_selected_cell_has_the_accent_ring_and_its_halo() {
    let mut rig = Rig::new(PaletteLayout::Grid, Direction::Ltr);
    assert_eq!(rig.ring_strokes(), 0, "no ring by default");
    rig.selected = Some(GradientRef::Item { group: 0, item: 1 });
    assert_eq!(rig.ring_strokes(), 2, "the ring and the halo outside it");
}
