//! The swatch palette's table mode in a headless egui context: selection by index, the open
//! request, the cell size, keys and AccessKit.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::SwatchPalette;
use crate::theme::{Base, Tokens, TokensOf as _, install};

/// A host with a table of six entries, indexes 1 and 3 the same colour, that applies the selection
/// it is told of and records the open requests.
struct Rig {
    ctx: Context,
    time: f64,
    selected: Option<usize>,
    opened: Vec<usize>,
    columns: Option<usize>,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self { ctx, time: 0.0, selected: None, opened: Vec::new(), columns: None, nodes: Vec::new() };
        rig.settle();
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(500.0, 400.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let (mut picked, mut open) = (None, None);
        let output = ctx.run_ui(input, |ui| {
            let duplicate = Srgba::rgb(0.2, 0.4, 0.8);
            let colors =
                [Srgba::rgb(1.0, 0.0, 0.0), duplicate, Srgba::rgb(0.0, 1.0, 0.0), duplicate, Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0)];
            let entries = colors.into_iter().enumerate().map(|(i, c)| (format!("Index {i}"), c));
            let mut table = SwatchPalette::table("table", "Color Table", entries).selected(self.selected);
            if let Some(columns) = self.columns {
                table = table.columns(columns);
            }
            let shown = table.show(ui);
            ui.text_edit_singleline(&mut String::new());
            (picked, open) = (shown.selected, shown.open);
        });
        if picked.is_some() {
            self.selected = picked;
        }
        self.opened.extend(open);
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.frame(vec![]);
        }
    }

    fn key(&mut self, key: Key) {
        let modifiers = Modifiers::NONE;
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers }]);
        self.settle();
    }

    fn press(&mut self, at: Pos2, pressed: bool) {
        self.frame(vec![
            Event::PointerMoved(at),
            Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE },
        ]);
    }

    fn click(&mut self, at: Pos2) {
        self.press(at, true);
        self.press(at, false);
    }

    fn bounds(&self, name: &str) -> Rect {
        let node = self.nodes.iter().map(|(_, n)| n).find(|n| n.role() == Role::ListBoxOption && n.label() == Some(name)).expect("cell");
        let b = node.bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }
}

#[test]
fn a_click_selects_the_clicked_index_among_duplicate_colours() {
    let mut rig = Rig::new();
    let at = rig.bounds("Index 3").center();
    rig.click(at);
    assert_eq!(rig.selected, Some(3));
    rig.settle();
    let selected: Vec<_> = rig
        .nodes
        .iter()
        .map(|(_, n)| n)
        .filter(|n| n.role() == Role::ListBoxOption && n.is_selected() == Some(true))
        .map(|n| n.label().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(selected, ["Index 3"], "the cell of the same colour at index 1 is not selected");
    assert!(rig.opened.is_empty(), "a click edits nothing");
}

#[test]
fn a_double_click_and_enter_ask_to_open_the_index_and_space_selects_the_focused_one() {
    let mut rig = Rig::new();
    let at = rig.bounds("Index 2").center();
    rig.click(at);
    rig.click(at);
    assert_eq!(rig.opened, [2], "the second click of a double-click opens");
    rig.settle();

    rig.key(Key::Tab);
    rig.key(Key::ArrowRight);
    assert_eq!(rig.selected, Some(2), "the arrows select nothing");
    rig.key(Key::Space);
    assert_eq!(rig.selected, Some(3), "Space selects the focused cell");
    rig.key(Key::ArrowRight);
    rig.key(Key::Enter);
    assert_eq!(rig.opened, [2, 4], "Enter opens the focused index");
    assert_eq!(rig.selected, Some(3), "Enter selects nothing");
}

#[test]
fn the_cells_are_the_tokens_size_with_no_gap_and_wrap_at_the_columns() {
    let mut rig = Rig::new();
    let mut cell = 0.0;
    rig.ctx.run_ui(RawInput::default(), |ui| cell = Tokens::of(ui).component.swatch_palette.table_cell).drop_without_applying_deltas();
    assert!(cell > 0.0);
    let (a, b) = (rig.bounds("Index 0"), rig.bounds("Index 1"));
    assert_eq!(a.size(), vec2(cell, cell));
    assert_eq!(a.max.x, b.min.x, "neighbours touch");
    rig.columns = Some(4);
    rig.settle();
    let (first, fifth) = (rig.bounds("Index 0"), rig.bounds("Index 4"));
    assert_eq!(rig.bounds("Index 3").min.y, first.min.y, "four to a row");
    assert_eq!((fifth.min.x, fifth.min.y), (first.min.x, first.max.y), "the fifth starts the next row");
}

#[test]
fn the_table_is_one_tab_stop_and_a_listbox_of_options() {
    let mut rig = Rig::new();
    rig.selected = Some(4);
    rig.settle();
    let group = rig.nodes.iter().map(|(_, n)| n).find(|n| n.role() == Role::ListBox && n.label() == Some("Color Table")).expect("list box");
    assert_eq!(group.size_of_set(), Some(6));
    rig.key(Key::Tab);
    let focused = rig.ctx.memory(|m| m.focused()).expect("focus").accesskit_id();
    let name = rig.nodes.iter().find(|(id, _)| *id == focused).and_then(|(_, n)| n.label().map(str::to_owned));
    assert_eq!(name.as_deref(), Some("Index 4"), "Tab enters on the selected cell");
    rig.key(Key::Tab);
    let focused = rig.ctx.memory(|m| m.focused()).expect("focus").accesskit_id();
    let name = rig.nodes.iter().find(|(id, _)| *id == focused).and_then(|(_, n)| n.label().map(str::to_owned));
    assert!(!name.is_some_and(|n| n.starts_with("Index")), "the next Tab leaves the table");
}

/// Enter on a focused cell opens the entry and nothing else: the dialog around the table reads
/// the key after the body, so a press left in the input would also answer OK.
#[test]
fn enter_on_a_cell_in_a_dialog_opens_the_entry_without_answering_ok() {
    use crate::dialog::{Dialog, DialogState, Footer};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    #[derive(Default)]
    struct App {
        state: DialogState,
        opened: Vec<usize>,
        outcomes: Vec<crate::dialog::Outcome>,
    }
    let mut harness =
        Harness::<App>::builder().with_size(vec2(640.0, 480.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let entries = (0..4).map(|i| (format!("Index {i}"), Srgba::rgb(i as f32 / 4.0, 0.2, 0.5)));
                let shown = Dialog::new("table", "Colors").footer(Footer::standard()).show(&ui.ctx().clone(), &mut app.state, &mut (), |ui, ()| {
                    let table = SwatchPalette::table("table", "Color Table", entries).show(ui);
                    app.opened.extend(table.open);
                });
                app.outcomes.extend(shown.outcome);
            },
            App::default(),
        );
    harness.run();
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Index 1")).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().opened, [1], "Enter opens the focused entry");
    assert!(harness.state().outcomes.is_empty(), "the dialog gets no Enter: {:?}", harness.state().outcomes);
}
