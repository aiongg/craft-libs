//! Command palette tests: keys, focus, choosing, closing, recents and AccessKit, with a raw egui
//! context driven frame by frame.

use egui::accesskit::{self, NodeId, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};

use super::command_palette::focus_requests;
use super::*;
use crate::menu::MenuItem;
use crate::theme::{Base, install};

struct Palette {
    ctx: egui::Context,
    entries: Vec<PaletteEntry<&'static str>>,
    open: bool,
    events: Vec<Event>,
    time: f64,
    tree: Vec<(NodeId, accesskit::Node)>,
    chosen: Vec<&'static str>,
    recents: Option<Vec<&'static str>>,
    /// The Enter press was still in the input after the palette ran.
    enter_after: bool,
}

fn entries() -> Vec<PaletteEntry<&'static str>> {
    let mut e = PaletteEntry::from_menu(&[
        MenuItem::submenu(
            "Image",
            vec![
                MenuItem::submenu(
                    "Adjustments",
                    vec![
                        MenuItem::new("Levels…", "levels").into(),
                        MenuItem::new("Curves…", "curves").into(),
                        MenuItem::new("Locked", "locked").enabled(false).into(),
                    ],
                )
                .into(),
            ],
        ),
        MenuItem::submenu("Edit", vec![MenuItem::new("Undo", "undo").into(), MenuItem::new("Redo", "redo").into()]),
    ]);
    e.push(PaletteEntry::new("brush", "Brush").kind("Tool"));
    e
}

impl Palette {
    fn new() -> Self {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx.enable_accesskit();
        ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
        let mut p = Self {
            ctx,
            entries: entries(),
            open: true,
            events: Vec::new(),
            time: 0.0,
            tree: Vec::new(),
            chosen: Vec::new(),
            recents: None,
            enter_after: false,
        };
        p.run(4);
        p
    }

    fn frame(&mut self) {
        self.time += 1.0 / 60.0;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
            time: Some(self.time),
            predicted_dt: 1.0 / 60.0,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let mut chosen = None;
        let (entries, mut open) = (&self.entries, self.open);
        let mut recents = self.recents.take();
        let mut enter_after = false;
        let mut full = self.ctx.run_ui(raw, |ui| {
            let mut palette = CommandPalette::new("test", entries);
            if let Some(list) = recents.as_mut() {
                palette = palette.recents(list);
            }
            chosen = palette.show(ui.ctx(), &mut open).map(|e| *e.command());
            enter_after = ui.input(|i| i.key_pressed(Key::Enter));
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
        }
        self.open = open;
        self.enter_after = enter_after;
        self.recents = recents;
        self.chosen.extend(chosen);
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    fn key(&mut self, key: Key) {
        for pressed in [true, false] {
            self.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
        }
        self.run(3);
    }

    fn typed(&mut self, text: &str) {
        for c in text.chars() {
            self.events.push(Event::Text(c.to_string()));
            self.run(2);
        }
    }

    fn node_by_label(&self, label: &str) -> Option<&accesskit::Node> {
        self.tree.iter().map(|(_, n)| n).find(|n| n.label() == Some(label))
    }

    /// The options from the top down.
    fn options(&self) -> Vec<(NodeId, &accesskit::Node)> {
        let mut options: Vec<_> = self.tree.iter().filter(|(_, n)| n.role() == Role::ListBoxOption).map(|(id, n)| (*id, n)).collect();
        options.sort_by(|a, b| a.1.bounds().map(|r| r.y0).partial_cmp(&b.1.bounds().map(|r| r.y0)).unwrap_or(std::cmp::Ordering::Equal));
        options
    }

    fn labels(&self) -> Vec<String> {
        self.options().iter().filter_map(|(_, n)| n.label().map(str::to_owned)).collect()
    }

    fn active(&self) -> Option<String> {
        self.options().iter().find(|(_, n)| n.is_selected() == Some(true)).and_then(|(_, n)| n.label().map(str::to_owned))
    }

    fn click_at(&mut self, p: Pos2) {
        self.time += 1.0;
        self.events.push(Event::PointerMoved(p));
        self.frame();
        for pressed in [true, false] {
            self.events.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        }
        self.run(3);
    }

    fn centre_of(&self, label: &str) -> Pos2 {
        let b = self.node_by_label(label).and_then(|n| n.bounds()).unwrap_or_else(|| panic!("no {label:?}"));
        pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    }
}

#[test]
fn an_empty_query_lists_the_first_entries_and_a_typed_one_ranks_the_matches() {
    let mut p = Palette::new();
    assert_eq!(p.labels(), ["Levels…", "Curves…", "Locked", "Undo", "Redo", "Brush"]);
    p.typed("re");
    assert_eq!(p.labels(), ["Redo", "Curves…"], "a label start, then a subsequence");
    p.typed("do");
    assert_eq!(p.labels(), ["Redo"]);
    assert_eq!(p.active().as_deref(), Some("Redo"), "the first match is active");
}

#[test]
fn up_and_down_wrap_and_enter_runs_the_active_row_and_is_taken() {
    let mut p = Palette::new();
    assert_eq!(p.active().as_deref(), Some("Levels…"));
    p.key(Key::ArrowUp);
    assert_eq!(p.active().as_deref(), Some("Brush"), "Up from the first wraps to the last");
    p.key(Key::ArrowDown);
    assert_eq!(p.active().as_deref(), Some("Levels…"), "Down from the last wraps to the first");
    p.key(Key::ArrowDown);
    p.key(Key::ArrowDown);
    assert_eq!(p.active().as_deref(), Some("Undo"), "the disabled row is skipped");
    p.events.push(Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    p.frame();
    assert_eq!(p.chosen, ["undo"]);
    assert!(!p.open, "choosing closes the palette");
    assert!(!p.enter_after, "the Enter press does not reach a dialog the command opens");
}

#[test]
fn a_disabled_entry_is_listed_but_does_not_run() {
    let mut p = Palette::new();
    p.typed("locked");
    assert_eq!(p.labels(), ["Locked"]);
    assert_eq!(p.active(), None, "nothing enabled to be active");
    p.key(Key::Enter);
    let at = p.centre_of("Locked");
    p.click_at(at);
    assert!(p.chosen.is_empty() && p.open);
}

#[test]
fn escape_clears_the_text_then_closes_and_a_click_outside_closes() {
    let mut p = Palette::new();
    p.typed("re");
    p.key(Key::Escape);
    assert!(p.open && p.labels().len() == 6, "the first Escape clears the text: {:?}", p.labels());
    p.key(Key::Escape);
    assert!(!p.open, "the second closes");
    p.open = true;
    p.run(4);
    p.click_at(pos2(10.0, 560.0));
    assert!(!p.open && p.chosen.is_empty(), "a click on the scrim closes");
}

#[test]
fn a_click_on_a_row_runs_it_and_the_pointer_moves_the_active_row_only_when_it_moves() {
    let mut p = Palette::new();
    let at = p.centre_of("Redo");
    p.events.push(Event::PointerMoved(pos2(400.0, 5.0)));
    p.run(2);
    p.events.push(Event::PointerMoved(at));
    p.run(3);
    assert_eq!(p.active().as_deref(), Some("Redo"), "a moving pointer makes its row active");
    p.key(Key::ArrowUp);
    assert_eq!(p.active().as_deref(), Some("Undo"));
    p.run(5);
    assert_eq!(p.active().as_deref(), Some("Undo"), "a still pointer over a row leaves the keys' choice");
    p.click_at(at);
    assert_eq!(p.chosen, ["redo"]);
    assert!(!p.open);
}

#[test]
fn the_field_is_given_the_focus_once_and_keeps_it() {
    let mut p = Palette::new();
    p.run(10);
    assert!(p.ctx.text_edit_focused(), "the field has the focus");
    assert_eq!(focus_requests(&p.ctx, "test"), 1, "asked once, not each pass");
    p.typed("u");
    assert_eq!(focus_requests(&p.ctx, "test"), 1);
}

#[test]
fn an_empty_query_lists_the_commands_run_newest_first_without_repeats() {
    let mut p = Palette::new();
    p.recents = Some(Vec::new());
    for query in ["undo", "brush", "undo"] {
        p.typed(query);
        p.key(Key::Enter);
        assert!(!p.open, "{query}");
        p.open = true;
        p.run(4);
    }
    assert_eq!(p.recents.as_deref(), Some(&["undo", "brush"][..]));
    assert_eq!(p.labels(), ["Undo", "Brush"], "the empty query lists the recents");
}

#[test]
fn accesskit_sees_a_dialog_with_a_search_input_controlling_a_list_and_a_result_count() {
    let mut p = Palette::new();
    p.typed("re");
    let (list_id, _) = p.tree.iter().find(|(_, n)| n.role() == Role::ListBox).expect("a list box");
    let (field_id, field) = p.tree.iter().find(|(_, n)| n.role() == Role::SearchInput).expect("a search input");
    assert!(field.controls().contains(list_id), "the field controls the list");
    let active = p.options().into_iter().find(|(_, n)| n.is_selected() == Some(true)).map(|(id, _)| id);
    assert_eq!(field.active_descendant(), active, "the active option is the active descendant");
    assert!(p.tree.iter().any(|(_, n)| n.role() == Role::Dialog && n.label() == Some("Command palette")), "a dialog named by the labels");
    let count = p.node_by_label("2 results").expect("the count is announced");
    assert_eq!((count.role(), count.live()), (Role::Status, Some(accesskit::Live::Polite)));
    let _ = field_id;
}
