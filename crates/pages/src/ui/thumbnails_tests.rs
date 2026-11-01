//! The thumbnails panel in a harness.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::behavior::{DropPosition, Move, Selection, SelectionPick};
use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::edit::EditEvent;
use craft_ui::list::CaptionPlace;
use craft_ui::testing::events;
use craft_ui::theme::{Base, install};
use egui::accesskit::{AriaCurrent, Role};
use egui::{Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};
use kurbo::Size;

use super::thumbnails::*;
use super::view::PageSource;

const LETTER: Size = Size::new(612.0, 792.0);

const CMD: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

struct Pages {
    count: usize,
    fake: FakeThumbSource<usize>,
}

impl ThumbSource<usize> for Pages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.fake.revision(key, request)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        self.fake.render(key, request)
    }
}

impl PageSource for Pages {
    fn page_count(&self) -> usize {
        self.count
    }

    fn page_size(&self, _page: usize) -> Size {
        LETTER
    }
}

struct App {
    pages: Pages,
    picked: Selection<usize>,
    current: usize,
    events: Vec<ThumbnailEvent>,
    editable: bool,
    zoom: Option<f32>,
    caption_place: CaptionPlace,
    sections: Vec<ThumbnailSection>,
    marks: Option<fn(usize) -> ThumbnailMarks>,
    new_page_row: bool,
    drop_target: Option<Move<usize>>,
}

impl App {
    fn new(count: usize) -> Self {
        App {
            pages: Pages { count, fake: FakeThumbSource::new().aspect(612, 792) },
            picked: Selection::new(),
            current: 0,
            events: Vec::new(),
            editable: true,
            zoom: None,
            caption_place: CaptionPlace::Below,
            sections: Vec::new(),
            marks: None,
            new_page_row: false,
            drop_target: None,
        }
    }

    fn take(&mut self) -> Vec<ThumbnailEvent> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut egui::Ui, app: &mut App) {
    let App { pages, picked, current, events, editable, zoom, caption_place, sections, marks, new_page_row, drop_target } = app;
    let mut panel = PageThumbnails::new("pages", &*pages, picked, *current)
        .editable(*editable)
        .caption_place(*caption_place)
        .sections(sections)
        .new_page_row(*new_page_row);
    if let Some(zoom) = zoom.as_mut() {
        panel = panel.size(zoom);
    }
    if let Some(marks) = marks.as_ref() {
        panel = panel.marks(marks);
    }
    let out = panel.show(ui);
    events.extend(out.events);
    *drop_target = out.drop_target;
}

fn open(app: App) -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([700.0, 800.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn tile<'h>(harness: &'h Harness<'_, App>, page: usize) -> Node<'h> {
    let label = format!("Page {}", page + 1);
    harness.get_by(move |n| {
        n.role() == Role::ListBoxOption
            && n.label().is_some_and(|l| l.starts_with(&label) && l[label.len()..].chars().next().is_none_or(|c| c == ','))
    })
}

fn centre(harness: &Harness<'_, App>, page: usize) -> Pos2 {
    tile(harness, page).rect().center()
}

fn click_with(harness: &mut Harness<'_, App>, page: usize, button: PointerButton, modifiers: Modifiers) {
    let pos = centre(harness, page);
    play(harness, events::click(pos, button, 1, modifiers));
    harness.run_steps(8);
}

fn click(harness: &mut Harness<'_, App>, page: usize) {
    click_with(harness, page, PointerButton::Primary, Modifiers::NONE);
}

fn key(harness: &mut Harness<'_, App>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn play(harness: &mut Harness<'_, App>, frames: Vec<events::Frame>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
}

fn picked(harness: &Harness<'_, App>) -> Vec<usize> {
    let mut items = harness.state().picked.items.clone();
    items.sort_unstable();
    items
}

fn reorders(events: &[ThumbnailEvent]) -> Vec<&EditEvent<Move<usize>>> {
    events.iter().filter_map(|e| if let ThumbnailEvent::Reorder(r) = e { Some(r) } else { None }).collect()
}

// A click reports Picked then Go and resets the picked set; Cmd/Ctrl-click toggles a page; Shift-click extends from the last plain click; neither reports Go.
#[test]
fn clicks_go_and_modifier_clicks_pick() {
    let mut harness = open(App::new(9));
    click(&mut harness, 2);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(change), ThumbnailEvent::Go(2)] if change.pick == SelectionPick::Replace), "{events:?}");
    assert_eq!(picked(&harness), [2]);

    click_with(&mut harness, 4, PointerButton::Primary, CMD);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(change)] if change.pick == SelectionPick::Toggle), "{events:?}");
    assert_eq!(picked(&harness), [2, 4]);

    click(&mut harness, 1);
    harness.state_mut().take();
    click_with(&mut harness, 3, PointerButton::Primary, Modifiers::SHIFT);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(change)] if change.pick == SelectionPick::Range), "{events:?}");
    assert_eq!(picked(&harness), [1, 2, 3]);
}

// Dragging page 6 of picked 6–8 to the gap before page 2 reports Reorder Begin then Commit with Move { items: [5, 6, 7], target: 1, position: Before }; dropping where they are reports Cancel; Esc reports Cancel; moved_page agrees.
#[test]
fn dragging_picked_pages_moves_them_together() {
    let mut harness = open(App::new(9));
    click(&mut harness, 5);
    click_with(&mut harness, 7, PointerButton::Primary, Modifiers::SHIFT);
    harness.state_mut().take();

    let drag = |harness: &Harness<'_, App>, grab: usize, onto: usize| {
        let to = centre(harness, onto) - vec2(30.0, 0.0);
        events::drag(centre(harness, grab), to, Modifiers::NONE)
    };
    let frames = drag(&harness, 6, 1);
    play(&mut harness, frames);
    harness.run_steps(4);
    let events = harness.state_mut().take();
    let mv = Move { items: vec![5, 6, 7], target: 1, position: DropPosition::Before, copy: false };
    let seen = reorders(&events);
    assert!(matches!(seen.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(seen.last(), Some(EditEvent::Commit { value, .. }) if *value == mv), "{events:?}");
    let after: Vec<usize> = (0..9).map(|p| moved_page(&mv, p, 9)).collect();
    assert_eq!(after, [0, 4, 5, 6, 7, 1, 2, 3, 8]);

    // Dropped where they are.
    let frames = drag(&harness, 6, 5);
    play(&mut harness, frames);
    harness.run_steps(4);
    let events = harness.state_mut().take();
    let seen = reorders(&events);
    assert!(seen.iter().any(|e| matches!(e, EditEvent::Cancel { .. })), "{events:?}");
    assert!(!seen.iter().any(|e| matches!(e, EditEvent::Commit { .. })), "{events:?}");

    // Esc halfway.
    let mut frames = drag(&harness, 6, 1);
    let release = frames.pop().expect("release");
    frames.extend(events::key_press(Key::Escape, Modifiers::NONE));
    frames.push(release);
    play(&mut harness, frames);
    harness.run_steps(4);
    let events = harness.state_mut().take();
    let seen = reorders(&events);
    assert!(seen.iter().any(|e| matches!(e, EditEvent::Cancel { .. })), "{events:?}");
    assert!(!seen.iter().any(|e| matches!(e, EditEvent::Commit { .. })), "{events:?}");
}

// Delete with the panel focused deletes the picked pages; with every page picked it does nothing; read-only it does nothing.
#[test]
fn delete_needs_the_panel_and_keeps_a_page() {
    let mut harness = open(App::new(5));
    click(&mut harness, 1);
    click_with(&mut harness, 2, PointerButton::Primary, CMD);
    harness.state_mut().take();
    key(&mut harness, Modifiers::NONE, Key::Delete);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Delete(pages)] if *pages == [1, 2]), "{events:?}");

    key(&mut harness, CMD, Key::A);
    harness.state_mut().take();
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert!(harness.state_mut().take().is_empty(), "a document keeps one page");

    // Nothing picked: the current page.
    let mut harness = open(App::new(5));
    key(&mut harness, Modifiers::NONE, Key::Tab);
    key(&mut harness, Modifiers::NONE, Key::Delete);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Delete(pages)] if *pages == [0]), "{events:?}");

    let mut app = App::new(5);
    app.editable = false;
    let mut harness = open(app);
    click(&mut harness, 1);
    harness.state_mut().take();
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert!(harness.state_mut().take().is_empty(), "read-only");
}

// The slider's smaller button steps THUMBNAIL_ZOOM by 0.25 and stops at 0.5; a slider drag writes the zoom and Esc restores it; Cmd/Ctrl+minus steps it too; thumbnails resize.
#[test]
fn size_row_steps_and_clamps_the_zoom() {
    let mut app = App::new(6);
    app.zoom = Some(1.0);
    let mut harness = open(app);
    let width = |harness: &Harness<'_, App>| tile(harness, 0).rect().width();
    let full = width(&harness);

    harness.get_by_label("Smaller Thumbnails").click();
    harness.run_steps(4);
    assert_eq!(harness.state().zoom, Some(0.75));
    assert!(width(&harness) < full, "thumbnails resize");
    harness.get_by_label("Smaller Thumbnails").click();
    harness.run_steps(4);
    assert_eq!(harness.state().zoom, Some(0.5));
    assert!(harness.get_by_label("Smaller Thumbnails").accesskit_node().is_disabled(), "the least size");

    click(&mut harness, 0);
    key(&mut harness, CMD, Key::Plus);
    assert_eq!(harness.state().zoom, Some(0.75), "Cmd+plus steps up");
    key(&mut harness, CMD, Key::Minus);
    assert_eq!(harness.state().zoom, Some(0.5), "Cmd+minus steps down");

    // A slider drag writes the zoom; Esc puts the start back.
    let slider = harness.get_by_role(Role::Slider).rect();
    let mut frames = events::drag(slider.left_center() + vec2(2.0, 0.0), slider.right_center() - vec2(2.0, 0.0), Modifiers::NONE);
    frames.pop();
    play(&mut harness, frames);
    let dragged = harness.state().zoom.expect("zoom");
    assert!(dragged > 1.0, "the drag wrote {dragged}");
    play(&mut harness, events::key_press(Key::Escape, Modifiers::NONE));
    harness.run_steps(4);
    assert_eq!(harness.state().zoom, Some(0.5), "Esc restores the start");
}

// Down moves the lead and reports Go; Shift+Down extends the picked set; Esc clears it; a secondary click on an unpicked page reports Picked then ContextMenu.
#[test]
fn keys_and_context_menu_follow_the_list_rules() {
    let mut harness = open(App::new(9));
    click(&mut harness, 0);
    harness.state_mut().take();
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(_), ThumbnailEvent::Go(1)]), "{events:?}");

    key(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(change)] if change.pick == SelectionPick::Range), "{events:?}");
    assert_eq!(picked(&harness), [1, 2]);

    key(&mut harness, Modifiers::NONE, Key::Escape);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(change)] if change.pick == SelectionPick::Clear), "{events:?}");
    assert!(picked(&harness).is_empty());

    click(&mut harness, 3);
    harness.state_mut().take();
    click_with(&mut harness, 6, PointerButton::Secondary, Modifiers::NONE);
    let events = harness.state_mut().take();
    assert!(matches!(&events[..], [ThumbnailEvent::Picked(_), ThumbnailEvent::ContextMenu(request)] if request.row == Some(6)), "{events:?}");
    assert_eq!(picked(&harness), [6]);
}

// The current page carries the grid's current ring and is aria_current; with CaptionPlace::Start, sections and marks, a hidden slide's option names "Hidden", a section chevron reports SectionOpen and the New Slide row reports NewPage.
#[test]
fn current_page_sections_and_slide_marks_map_to_the_grid() {
    let mut app = App::new(6);
    app.current = 2;
    app.caption_place = CaptionPlace::Start;
    app.new_page_row = true;
    app.sections = vec![
        ThumbnailSection { name: "Intro".to_owned(), first: 0, count: 3, open: true },
        ThumbnailSection { name: "Body".to_owned(), first: 3, count: 3, open: true },
    ];
    app.marks = Some(|page| ThumbnailMarks { hidden: page == 1, animated: page == 4 });
    let mut harness = open(app);
    assert_eq!(tile(&harness, 2).accesskit_node().aria_current(), Some(AriaCurrent::Page));
    assert_eq!(tile(&harness, 0).accesskit_node().aria_current(), None);
    assert_eq!(tile(&harness, 1).accesskit_node().label().as_deref(), Some("Page 2, Hidden"));
    assert_eq!(tile(&harness, 4).accesskit_node().label().as_deref(), Some("Page 5, Has animations"));

    harness.get_by_label("Collapse Intro").click();
    harness.run_steps(4);
    let events = harness.state_mut().take();
    assert!(events.iter().any(|e| matches!(e, ThumbnailEvent::SectionOpen { section: 0, open: false })), "{events:?}");
    harness.get_by_label("New Slide").click();
    harness.run_steps(4);
    let events = harness.state_mut().take();
    assert!(events.iter().any(|e| matches!(e, ThumbnailEvent::NewPage)), "{events:?}");
}
