//! `ChannelsPanel` over sample rows: targeting, loading, options, rename, reorder, the footer and the commands.

use craft_behavior::EditEvent;
use craft_imaging::ui::channels::{ChannelKind, ChannelRow, ChannelsEvent, ChannelsFooterButton, ChannelsPanel, ColorIndicates};
use craft_theme::Srgba;
use craft_ui::contract::{Role, SweepStep};
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const COMPOSITE: u32 = 0;
const ALPHA_1: u32 = 4;
const ALPHA_2: u32 = 5;
const QUICK_MASK: u32 = 7;

/// A step that starts a drag.
const DRAG_STEP: Vec2 = Vec2::new(0.0, 8.0);

struct App {
    rows: Vec<ChannelRow<u32>>,
    events: Vec<ChannelsEvent<u32>>,
    /// A row to open the rename field of in the next pass, as the app does for its Rename Channel command.
    rename: Option<u32>,
}

type Shared = Rc<RefCell<App>>;

fn sample_rows() -> Vec<ChannelRow<u32>> {
    let indicates = ColorIndicates::MaskedAreas;
    let mut rows = vec![
        ChannelRow::new(COMPOSITE, ChannelKind::Composite, "RGB"),
        ChannelRow::new(1, ChannelKind::Color, "Red"),
        ChannelRow::new(2, ChannelKind::Color, "Green"),
        ChannelRow::new(3, ChannelKind::Color, "Blue"),
        ChannelRow::new(ALPHA_1, ChannelKind::Alpha(indicates), "Alpha 1"),
        ChannelRow::new(ALPHA_2, ChannelKind::Alpha(indicates), "Alpha 2"),
        ChannelRow::new(6, ChannelKind::Spot(Srgba::rgb(0.8, 0.0, 0.2)), "Spot 1"),
        ChannelRow::new(QUICK_MASK, ChannelKind::QuickMask(indicates), "Quick Mask"),
    ];
    target(&mut rows, COMPOSITE);
    rows
}

/// What the app does for `Target(key)`: the composite targets every colour channel too.
fn target(rows: &mut [ChannelRow<u32>], key: u32) {
    let composite = rows.iter().any(|row| row.key == key && row.kind == ChannelKind::Composite);
    for row in rows {
        row.targeted = row.key == key || (composite && row.kind == ChannelKind::Color);
    }
}

fn rig(rows: Vec<ChannelRow<u32>>) -> (Fixture, Shared) {
    let app = Rc::new(RefCell::new(App { rows, events: Vec::new(), rename: None }));
    let shared = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shared.borrow_mut();
        let rows = app.rows.clone();
        let mut panel = ChannelsPanel::new(&rows);
        if let Some(key) = app.rename.take() {
            panel = panel.start_rename(key);
        }
        let out = panel.show(ui);
        for event in &out.events {
            if let ChannelsEvent::Target(key) = event {
                target(&mut app.rows, *key);
            }
        }
        app.events.extend(out.events);
    });
    fixture.room(Vec2::new(320.0, 400.0));
    fixture.settle();
    (fixture, app)
}

fn take(app: &Shared) -> Vec<ChannelsEvent<u32>> {
    std::mem::take(&mut app.borrow_mut().events)
}

fn row(fixture: &Fixture, name: &str) -> Target {
    fixture.by_label(name).unwrap_or_else(|| panic!("no row named {name}"))
}

fn double_click(fixture: &mut Fixture, pos: Pos2) {
    fixture.frame(vec![pointer::moved(pos)]);
    for _ in 0..2 {
        fixture.frame(vec![pointer::press(pos)]);
        fixture.frame(vec![pointer::release(pos)]);
    }
    fixture.idle(2);
}

fn drag(fixture: &mut Fixture, from: Pos2, to: Pos2) {
    fixture.idle(6);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    fixture.frame(vec![pointer::moved(from + DRAG_STEP)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::release(to)]);
    fixture.idle(2);
}

fn editing(fixture: &Fixture) -> bool {
    !fixture.by_role(Role::TEXT_INPUT).is_empty()
}

fn drops(events: Vec<ChannelsEvent<u32>>) -> Vec<(ChannelsFooterButton, Vec<u32>)> {
    events
        .into_iter()
        .filter_map(|event| if let ChannelsEvent::FooterDrop { button, rows, .. } = event { Some((button, rows)) } else { None })
        .collect()
}

fn renames(events: Vec<ChannelsEvent<u32>>) -> Vec<(u32, String)> {
    events.into_iter().filter_map(|event| if let ChannelsEvent::Rename { key, text, .. } = event { Some((key, text)) } else { None }).collect()
}

/// A click targets the row; Shift-click reports the clicked row as the new target.
#[trial]
fn a_click_targets_the_row() {
    let (mut fixture, app) = rig(sample_rows());
    fixture.click(&row(&fixture, "Alpha 1"));
    assert_eq!(take(&app), [ChannelsEvent::Target(ALPHA_1)]);
    fixture.hold(Modifiers::SHIFT);
    fixture.click(&row(&fixture, "Alpha 2"));
    assert_eq!(take(&app), [ChannelsEvent::Target(ALPHA_2)]);
}

/// A sweep down the eyes flips each eye it crosses once, ignoring Alt; Escape restores them with the value from before.
#[trial]
fn an_eye_sweep_flips_each_eye_and_escape_restores_them() {
    let (mut fixture, app) = rig(sample_rows());
    let (from, to) = (fixture.by_label("Visibility Red").expect("eye").center(), fixture.by_label("Visibility Blue").expect("eye").center());
    fixture.hold(Modifiers::ALT);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    fixture.frame(vec![pointer::moved(from + DRAG_STEP)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.idle(2);
    let flips: Vec<_> = take(&app)
        .into_iter()
        .filter_map(|event| if let ChannelsEvent::Visibility(SweepStep::Flip(cell)) = event { Some(cell.key) } else { None })
        .collect();
    assert_eq!(flips, [1, 2, 3], "one flip per eye crossed");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    let restored: Vec<_> = take(&app)
        .into_iter()
        .filter_map(|event| if let ChannelsEvent::Visibility(SweepStep::Restore(cell)) = event { Some(cell.key) } else { None })
        .collect();
    assert_eq!(restored, [1, 2, 3], "every flipped eye is restored");
}

/// A double click on an alpha row's name opens the rename field and Enter commits it; composite and colour rows do not
/// rename.
#[trial]
fn a_double_click_on_the_name_renames_an_alpha_channel_only() {
    let (mut fixture, app) = rig(sample_rows());
    for name in ["RGB", "Red"] {
        let at = row(&fixture, name).center();
        double_click(&mut fixture, at);
        assert!(!editing(&fixture), "{name} renamed");
        fixture.key(Key::Escape, Modifiers::NONE);
    }
    take(&app);
    let at = row(&fixture, "Alpha 1").center();
    double_click(&mut fixture, at);
    assert!(editing(&fixture));
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text("Sketch");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(renames(take(&app)), [(ALPHA_1, String::from("Sketch"))]);
}

/// An alpha row dragged below another or above Quick Mask reports a move; over colour rows nothing is reported.
#[trial]
fn an_alpha_drag_reorders_and_refuses_other_rows() {
    let (mut fixture, app) = rig(sample_rows());
    let moves = |events: Vec<ChannelsEvent<u32>>| -> Vec<EditEvent<craft_ui::contract::ReorderMove<u32>>> {
        events.into_iter().filter_map(|event| if let ChannelsEvent::Reorder(edit) = event { Some(edit) } else { None }).collect()
    };
    let below = |fixture: &Fixture, name: &str| {
        let rect = row(fixture, name).rect;
        Pos2::new(rect.center().x, rect.bottom() - 3.0)
    };
    let from = row(&fixture, "Alpha 1").center();
    let to = below(&fixture, "Alpha 2");
    drag(&mut fixture, from, to);
    let made = moves(take(&app));
    // A drop reports its move as one `Set` (craft-ui contract/items/drag.rs `land`, `CollectionEvent::Reorder`).
    let [EditEvent::Set { after, .. }] = made.as_slice() else { panic!("expected one set of the move, got {made:?}") };
    assert_eq!((after.keys.clone(), after.to), (vec![ALPHA_1], 5));
    for name in ["Red", "RGB"] {
        let from = row(&fixture, "Alpha 1").center();
        let to = row(&fixture, name).center();
        drag(&mut fixture, from, to);
        let made = moves(take(&app));
        assert!(!made.iter().any(|edit| matches!(edit, EditEvent::Commit { .. } | EditEvent::Set { .. })), "{name}: {made:?}");
    }
}

/// Rows dropped on New, Delete and Load report `FooterDrop` with the grabbed key; Save takes no drops; a colour row
/// is not deleted.
#[trial]
fn a_drop_on_the_footer_reports_the_button_and_the_key() {
    let (mut fixture, app) = rig(sample_rows());
    target(&mut app.borrow_mut().rows, ALPHA_2);
    fixture.idle(2);
    for (label, button) in [
        ("Create new channel", ChannelsFooterButton::New),
        ("Delete current channel", ChannelsFooterButton::Delete),
        ("Load channel as selection", ChannelsFooterButton::LoadSelection),
    ] {
        let (from, to) = (row(&fixture, "Alpha 1").center(), row(&fixture, label).center());
        drag(&mut fixture, from, to);
        assert_eq!(drops(take(&app)), [(button, vec![ALPHA_1])], "{label}");
    }
    let (from, to) = (row(&fixture, "Alpha 1").center(), row(&fixture, "Save selection as channel").center());
    drag(&mut fixture, from, to);
    assert!(drops(take(&app)).is_empty(), "Save takes no drops");
    let (from, to) = (row(&fixture, "Red").center(), row(&fixture, "Delete current channel").center());
    drag(&mut fixture, from, to);
    assert!(drops(take(&app)).is_empty(), "a colour row is not deleted");
}

/// Rename Channel… in a row's context menu opens that row's rename field.
#[trial]
fn rename_from_the_row_menu_starts_the_rename() {
    let (mut fixture, _) = rig(sample_rows());
    let pos = row(&fixture, "Alpha 1").center();
    fixture.frame(vec![pointer::moved(pos)]);
    fixture.frame(vec![pointer::secondary(pos, true)]);
    fixture.frame(vec![pointer::secondary(pos, false)]);
    fixture.idle(3);
    let item = row(&fixture, "Rename Channel…");
    fixture.click(&item);
    fixture.idle(3);
    assert!(editing(&fixture), "the rename field is open");
}
