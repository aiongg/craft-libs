//! `ReorderableSections` through egui_kittest (task B2).

use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Ui, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, Node};

use super::{ReorderableSections, SectionButton, SectionEvent, SectionInfo, SectionKey, SectionLayout};
use crate::icons::Icon;
use crate::menu::MenuEntry;
use crate::theme::{Base, install};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Side {
    Albums,
    Local,
    ByDate,
    Folders,
    Keywords,
}

use Side::{Albums, ByDate, Folders, Keywords, Local};

impl SectionKey for Side {
    const ALL: &'static [Self] = &[Albums, Local, ByDate, Folders, Keywords];

    fn id(self) -> &'static str {
        match self {
            Albums => "albums",
            Local => "local",
            ByDate => "byDate",
            Folders => "folders",
            Keywords => "keywords",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|s| s.id() == id)
    }
}

struct App {
    layout: SectionLayout<Side>,
    sections: Vec<SectionInfo<Side>>,
    rows: usize,
    scroll_to: Option<f32>,
    log: Vec<SectionEvent<Side>>,
}

impl App {
    fn new() -> Self {
        let titles = [(Albums, "Albums"), (Local, "Local"), (ByDate, "By Date"), (Folders, "Folders"), (Keywords, "Keywords")];
        Self {
            layout: SectionLayout::default(),
            sections: titles.iter().map(|(s, t)| SectionInfo::new(*s, *t).empty_hint(format!("No {t} yet"))).collect(),
            rows: 2,
            scroll_to: None,
            log: Vec::new(),
        }
    }

    fn frame(&mut self, ui: &mut Ui) {
        let rows = self.rows;
        let mut area = egui::ScrollArea::vertical().id_salt("scroll");
        if let Some(offset) = self.scroll_to.take() {
            area = area.vertical_scroll_offset(offset);
        }
        let out = area.show(ui, |ui| {
            ReorderableSections::new("left", &self.layout).label("Sidebar").show(ui, &self.sections, |ui, section| {
                for row in 0..rows {
                    ui.label(format!("{section:?} row {row}"));
                }
            })
        });
        for event in out.inner.events {
            self.layout.apply(&event);
            self.log.push(event);
        }
    }
}

fn harness(app: App, height: f32) -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(vec2(240.0, height))
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.input_mut().focused = true;
    h.run_steps(2);
    h
}

fn header<'a>(h: &'a Harness<'_, App>, title: &'a str) -> Node<'a> {
    h.get_by(|n| n.role() == Role::DisclosureTriangle && n.label().as_deref() == Some(title))
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn send(h: &mut Harness<'_, App>, events: Vec<Event>) {
    h.input_mut().events.extend(events);
    h.run_steps(1);
}

fn press_and_move(h: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    send(h, vec![Event::PointerMoved(from), button(from, true)]);
    send(h, vec![Event::PointerMoved(from + vec2(0.0, 6.0))]);
    send(h, vec![Event::PointerMoved(to)]);
}

fn moves(app: &App) -> Vec<SectionEvent<Side>> {
    app.log.iter().filter(|e| matches!(e, SectionEvent::Moved { .. })).copied().collect()
}

#[test]
fn dragging_a_header_reports_where_it_goes_and_escape_leaves_it() {
    let mut h = harness(App::new(), 400.0);
    let from = header(&h, "Albums").rect().center();
    let below_local = pos2(from.x, header(&h, "By Date").rect().top() - 2.0);

    press_and_move(&mut h, from, below_local);
    send(&mut h, vec![button(below_local, false)]);
    h.run_steps(2);
    assert_eq!(moves(h.state()), [SectionEvent::Moved { section: Albums, before: Some(ByDate) }]);
    assert!(!h.state().log.iter().any(|e| matches!(e, SectionEvent::Folded { .. })), "a drag does not fold");

    let mut h = harness(App::new(), 400.0);
    press_and_move(&mut h, from, below_local);
    send(&mut h, vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    send(&mut h, vec![button(below_local, false)]);
    h.run_steps(2);
    assert!(h.state().log.is_empty(), "{:?}", h.state().log);
}

#[test]
fn the_pinned_header_folds_and_brings_its_section_into_view() {
    let mut app = App::new();
    app.rows = 40;
    app.scroll_to = Some(300.0);
    let mut h = harness(app, 300.0);
    h.run_steps(2);
    let albums: Vec<f32> =
        h.query_all_by(|n| n.role() == Role::DisclosureTriangle && n.label().as_deref() == Some("Albums")).map(|n| n.rect().top()).collect();
    assert_eq!(albums.len(), 2, "the section's own header and the pinned copy: {albums:?}");
    let pinned = albums.iter().copied().fold(f32::MIN, f32::max);
    h.input_mut().events.push(Event::PointerMoved(pos2(100.0, pinned + 8.0)));
    h.run_steps(1);
    send(&mut h, vec![button(pos2(100.0, pinned + 8.0), true)]);
    send(&mut h, vec![button(pos2(100.0, pinned + 8.0), false)]);
    h.run_steps(3);
    assert_eq!(h.state().log, [SectionEvent::Folded { section: Albums, folded: true }]);
    let top = header(&h, "Albums").rect().top();
    assert!((0.0..60.0).contains(&top), "the folded section's header is at the top of the view: {top}");
}

#[test]
fn a_press_on_the_header_button_never_folds_or_drags() {
    let mut app = App::new();
    app.sections[0] = std::mem::replace(&mut app.sections[0], SectionInfo::new(Albums, "Albums")).button(SectionButton::new(
        "albumNew",
        Icon::Plus,
        "Create Album",
    ));
    let mut h = harness(app, 400.0);
    let at = h.get_by_label("Create Album").rect().center();
    press_and_move(&mut h, at, at + vec2(2.0, 30.0));
    send(&mut h, vec![Event::PointerMoved(at), button(at, false)]);
    h.run_steps(2);
    assert!(!h.state().log.iter().any(|e| matches!(e, SectionEvent::Folded { .. } | SectionEvent::Moved { .. })), "{:?}", h.state().log);
    h.get_by_label("Create Album").click();
    h.run_steps(2);
    assert!(h.state().log.contains(&SectionEvent::Button { section: Albums, id: "albumNew" }), "{:?}", h.state().log);
    assert!(!h.state().layout.is_folded(Albums));
}

#[test]
fn cmd_bracket_moves_the_focused_header() {
    let mut h = harness(App::new(), 400.0);
    h.key_press(Key::Tab);
    h.run_steps(1);
    assert!(header(&h, "Albums").is_focused());
    h.key_press_modifiers(Modifiers::COMMAND, Key::CloseBracket);
    h.run_steps(3);
    assert_eq!(moves(h.state()), [SectionEvent::Moved { section: Albums, before: Some(ByDate) }]);
    assert_eq!(h.state().layout.states()[1].section, Albums);
    assert!(h.query_by_label("Albums moved to position 2 of 5").is_some(), "the new place is announced");
}

#[test]
fn an_unavailable_section_keeps_its_place() {
    let mut app = App::new();
    app.sections[1].available = false;
    let mut h = harness(app, 400.0);
    assert!(h.query_by_label("Local").is_none(), "an unavailable section is not drawn");

    header(&h, "By Date").focus();
    h.run_steps(1);
    h.key_press_modifiers(Modifiers::COMMAND, Key::CloseBracket);
    h.run_steps(2);
    let order: Vec<Side> = h.state().layout.states().iter().map(|s| s.section).collect();
    assert_eq!(order, [Albums, Local, Folders, ByDate, Keywords], "the neighbours moved; Local stayed second");
}

#[test]
fn an_empty_section_shows_its_hint() {
    let mut app = App::new();
    app.rows = 0;
    let h = harness(app, 400.0);
    for hint in ["No Albums yet", "No Keywords yet"] {
        assert!(h.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(hint)).is_some(), "{hint}");
    }
}

#[test]
fn a_move_is_announced_in_the_pass_that_reports_it() {
    let mut h = harness(App::new(), 400.0);
    header(&h, "Albums").focus();
    h.run_steps(1);
    let key = Event::Key { key: Key::CloseBracket, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND };
    send(&mut h, vec![key]);
    assert_eq!(moves(h.state()), [SectionEvent::Moved { section: Albums, before: Some(ByDate) }]);
    assert!(h.query_by_label("Albums moved to position 2 of 5").is_some(), "announced with the move, not a pass later");
}

#[test]
fn the_menu_entries_carry_their_events() {
    let app = App::new();
    let labels = super::SectionsLabels::default();
    let entries = ReorderableSections::menu(&app.layout, &app.sections, Some(Albums), &labels);
    let events: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item(item) => item.command().copied(),
            MenuEntry::Separator => None,
        })
        .collect();
    assert_eq!(events.first(), Some(&SectionEvent::Hidden(Albums)));
    assert_eq!(events.last(), Some(&SectionEvent::Reset));
    let from_title_block = ReorderableSections::menu(&app.layout, &app.sections, None, &labels);
    assert_eq!(from_title_block.len(), entries.len() - 4, "no Hide, Move Up, Move Down or their separator");
}
