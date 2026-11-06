//! Behaviour tests of the Channels panel over fake rows (XT, after CH1 and CH2).

#![allow(clippy::expect_used, reason = "tests")]

use craft_theme::Srgba;
use craft_ui::behavior::DropPosition;
use craft_ui::canvas::SelectionOperation;
use craft_ui::canvas::textures::{FakeThumbSource, Priority, Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::edit::EditEvent;
use craft_ui::list::ThumbSize;
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, TextureId, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::labels::{chord_text, shortcut_numbers};
use super::{ChannelKind, ChannelRow, ChannelsEvent, ChannelsFooterButton, ChannelsPanel, ColorIndicates};

struct App {
    rows: Vec<ChannelRow<u32>>,
    events: Vec<ChannelsEvent<u32>>,
    source: FakeThumbSource<u32>,
    /// The texture each row's thumbnail showed in the last pass.
    textures: Vec<(u32, Option<TextureId>)>,
    /// Every text painted in the last pass, with whether its font is italic.
    texts: Vec<(String, bool)>,
    /// A row to open the rename field of in the next pass.
    start_rename: Option<u32>,
}

const COMPOSITE: u32 = 0;
const ALPHA_1: u32 = 4;
const ALPHA_2: u32 = 5;
const SPOT: u32 = 6;
const QUICK_MASK: u32 = 7;

fn sample_rows() -> Vec<ChannelRow<u32>> {
    let indicates = ColorIndicates::MaskedAreas;
    let mut rows = vec![
        ChannelRow::new(COMPOSITE, ChannelKind::Composite, "RGB"),
        ChannelRow::new(1, ChannelKind::Color, "Red"),
        ChannelRow::new(2, ChannelKind::Color, "Green"),
        ChannelRow::new(3, ChannelKind::Color, "Blue"),
        ChannelRow::new(ALPHA_1, ChannelKind::Alpha { indicates }, "Alpha 1"),
        ChannelRow::new(ALPHA_2, ChannelKind::Alpha { indicates }, "Alpha 2"),
        ChannelRow::new(SPOT, ChannelKind::Spot { ink: Srgba::rgb(0.8, 0.0, 0.2) }, "Spot 1"),
        ChannelRow::new(QUICK_MASK, ChannelKind::QuickMask { indicates }, "Quick Mask"),
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

fn panel(rows: Vec<ChannelRow<u32>>) -> Harness<'static, App> {
    panel_sized(rows, None)
}

fn panel_sized(rows: Vec<ChannelRow<u32>>, size: Option<ThumbSize>) -> Harness<'static, App> {
    let app = App { rows, events: Vec::new(), source: FakeThumbSource::new(), textures: Vec::new(), texts: Vec::new(), start_rename: None };
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(320.0, 400.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            move |ui, app: &mut App| {
                let thumbs = Thumbnails::<u32>::in_context(ui.ctx(), "thumbs", ThumbnailOptions::default());
                let mut panel = ChannelsPanel::new("channels", &app.rows).thumbnails(&thumbs, &app.source);
                if let Some(size) = size {
                    panel = panel.thumbnail_size(size);
                }
                if let Some(key) = app.start_rename.take() {
                    panel = panel.start_rename(key);
                }
                let out = panel.show(ui);
                for event in &out.events {
                    if let ChannelsEvent::Target(key) = event {
                        target(&mut app.rows, *key);
                    }
                }
                app.events.extend(out.events);
                app.texts = painted_texts(ui.ctx());
                let side = ChannelsPanel::<u32>::style(&Tokens::of(ui)).thumb;
                let request = ThumbRequest::for_cell(side, ui.pixels_per_point());
                app.textures = app
                    .rows
                    .iter()
                    .map(|row| {
                        (row.key, thumbs.state(ui.ctx(), &app.source, &row.key, &request, Priority::VISIBLE).shown().map(|texture| texture.id))
                    })
                    .collect();
            },
            app,
        );
    harness.run();
    harness
}

fn painted_texts(ctx: &egui::Context) -> Vec<(String, bool)> {
    let layers: Vec<_> = ctx.memory(|memory| memory.layer_ids().collect());
    let mut texts = Vec::new();
    ctx.graphics(|graphics| {
        for list in layers.iter().filter_map(|layer| graphics.get(*layer)) {
            for clipped in list.all_entries() {
                if let egui::Shape::Text(text) = &clipped.shape {
                    texts.push((text.galley.job.text.clone(), text.galley.job.sections.iter().any(|section| section.format.italics)));
                }
            }
        }
    });
    texts
}

fn row_rect(h: &Harness<'_, App>, name: &str) -> Rect {
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(name)).rect()
}

/// A pointer position that is found when the helper runs, so a call can borrow the harness mutably.
#[derive(Clone, Copy)]
enum At<'a> {
    Name(&'a str),
    Thumb(&'a str),
    Eye(&'a str),
    Footer(&'a str),
    /// `dy` points from the row's bottom edge (negative: inside the row).
    Bottom(&'a str, f32),
    Top(&'a str, f32),
}

impl At<'_> {
    fn pos(self, h: &Harness<'_, App>) -> Pos2 {
        match self {
            At::Name(name) => name_of(h, name),
            At::Thumb(name) => thumb_of(h, name),
            At::Eye(name) => h.get_by_label(&format!("Visibility {name}")).rect().center(),
            At::Footer(label) => h.get_by_role_and_label(Role::Button, label).rect().center(),
            At::Bottom(name, dy) => Pos2::new(name_of(h, name).x, row_rect(h, name).bottom() + dy),
            At::Top(name, dy) => Pos2::new(name_of(h, name).x, row_rect(h, name).top() + dy),
        }
    }
}

/// The thumbnail's centre: right of the eye cell, inside the row.
fn thumb_of(h: &Harness<'_, App>, name: &str) -> Pos2 {
    let row = row_rect(h, name);
    let eye = h.get_by_label(&format!("Visibility {name}")).rect();
    Pos2::new(eye.right() + 4.0 + 14.0, row.center().y)
}

/// A point on the row's name, past the thumbnail.
fn name_of(h: &Harness<'_, App>, name: &str) -> Pos2 {
    let row = row_rect(h, name);
    Pos2::new(row.center().x, row.center().y)
}

fn press(h: &mut Harness<'_, App>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event_modifiers(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }, modifiers);
    h.run();
}

fn click(h: &mut Harness<'_, App>, at: At<'_>, modifiers: Modifiers) {
    let pos = at.pos(h);
    h.event(Event::PointerMoved(pos));
    h.run();
    h.event_modifiers(Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers }, modifiers);
    h.event_modifiers(Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers }, modifiers);
    h.run();
}

fn double_click(h: &mut Harness<'_, App>, at: At<'_>) {
    let pos = at.pos(h);
    h.event(Event::PointerMoved(pos));
    h.run();
    for pressed in [true, false, true, false] {
        h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    h.run();
}

/// A pointer drag from `from` to `to` in a few moves; `release` false leaves the button down.
fn drag(h: &mut Harness<'_, App>, from: At<'_>, to: At<'_>, release: bool) {
    let (from, to) = (from.pos(h), to.pos(h));
    h.event(Event::PointerMoved(from));
    h.run();
    press(h, from, true, Modifiers::NONE);
    for step in 1..=6 {
        h.event(Event::PointerMoved(from + (to - from) * (step as f32 / 6.0)));
        h.run();
    }
    if release {
        press(h, to, false, Modifiers::NONE);
        h.run_steps(2);
    }
}

fn take(h: &mut Harness<'_, App>) -> Vec<ChannelsEvent<u32>> {
    std::mem::take(&mut h.state_mut().events)
}

fn key(h: &mut Harness<'_, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

// A click on a row reports `Target`; Shift-click too.
#[test]
fn channels_click_targets_row() {
    let mut h = panel(sample_rows());
    click(&mut h, At::Name("Alpha 1"), Modifiers::NONE);
    assert_eq!(take(&mut h), [ChannelsEvent::Target(ALPHA_1)]);
    click(&mut h, At::Name("Alpha 2"), Modifiers::SHIFT);
    assert_eq!(take(&mut h), [ChannelsEvent::Target(ALPHA_2)]);
}

// Cmd/Ctrl-click reports `LoadSelection`: new, Shift add, Alt subtract, Shift+Alt intersect; no
// `Target`.
#[test]
fn channels_command_click_loads_selection() {
    let mut h = panel(sample_rows());
    let cases = [
        (Modifiers::COMMAND, SelectionOperation::New),
        (Modifiers::COMMAND | Modifiers::SHIFT, SelectionOperation::Add),
        (Modifiers::COMMAND | Modifiers::ALT, SelectionOperation::Subtract),
        (Modifiers::COMMAND | Modifiers::SHIFT | Modifiers::ALT, SelectionOperation::Intersect),
    ];
    for (modifiers, operation) in cases {
        click(&mut h, At::Name("Alpha 1"), modifiers);
        assert_eq!(take(&mut h), [ChannelsEvent::LoadSelection { key: ALPHA_1, operation }], "{modifiers:?}");
    }
}

// A sweep down the eyes is one `Visibility` gesture from `Begin` to `Commit`; Esc cancels it.
#[test]
fn channels_eye_sweep_is_one_gesture() {
    let mut h = panel(sample_rows());
    let sweep = |h: &mut Harness<'_, App>, release: bool| {
        let (from, to) = (At::Eye("Red").pos(h), At::Eye("Blue").pos(h));
        h.event(Event::PointerMoved(from));
        h.run();
        press(h, from, true, Modifiers::NONE);
        for step in 1..=6 {
            h.event(Event::PointerMoved(from + (to - from) * (step as f32 / 6.0)));
            h.run();
        }
        if release {
            press(h, to, false, Modifiers::NONE);
        }
    };
    sweep(&mut h, true);
    let events = take(&mut h);
    let gesture: Vec<_> = events.iter().filter_map(|event| if let ChannelsEvent::Visibility(edit) = event { Some(edit) } else { None }).collect();
    assert!(matches!(gesture.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(gesture.last(), Some(EditEvent::Commit { .. })), "{events:?}");
    assert_eq!(gesture.iter().filter(|edit| matches!(edit, EditEvent::Begin { .. })).count(), 1);
    assert_eq!(gesture.iter().filter(|edit| matches!(edit, EditEvent::Commit { .. })).count(), 1);
    assert!(events.iter().all(|event| matches!(event, ChannelsEvent::Visibility(_))), "the eye does not target: {events:?}");

    sweep(&mut h, false);
    key(&mut h, Key::Escape, Modifiers::NONE);
    let events = take(&mut h);
    assert!(matches!(events.last(), Some(ChannelsEvent::Visibility(EditEvent::Cancel { .. }))), "{events:?}");
    assert!(!events.iter().any(|event| matches!(event, ChannelsEvent::Visibility(EditEvent::Commit { .. }))));
}

// A double click on an alpha row's thumbnail reports `Options`; on the composite it reports
// nothing.
#[test]
fn channels_double_click_thumbnail_opens_options() {
    let mut h = panel(sample_rows());
    double_click(&mut h, At::Thumb("Alpha 1"));
    assert!(take(&mut h).contains(&ChannelsEvent::Options(ALPHA_1)));
    double_click(&mut h, At::Thumb("RGB"));
    assert!(!take(&mut h).iter().any(|event| matches!(event, ChannelsEvent::Options(_))));
}

// A double click on an alpha row's name opens the rename; composite and colour rows don't rename.
#[test]
fn channels_double_click_name_renames_alpha_only() {
    let mut h = panel(sample_rows());
    let editing = |h: &Harness<'_, App>| h.query_by_role(Role::TextInput).is_some();
    for name in ["RGB", "Red"] {
        double_click(&mut h, At::Name(name));
        assert!(!editing(&h), "{name} renamed");
        key(&mut h, Key::Escape, Modifiers::NONE);
    }
    take(&mut h);
    double_click(&mut h, At::Name("Alpha 1"));
    assert!(editing(&h));
    h.get_by_role(Role::TextInput).focus();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
    h.get_by_role(Role::TextInput).type_text("Sketch");
    h.run();
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(take(&mut h).contains(&ChannelsEvent::Rename { key: ALPHA_1, text: "Sketch".to_owned() }));
}

// An alpha row dragged between alpha rows or above Quick Mask reports `Reorder`; over colour and
// mask rows no marker shows and nothing is reported.
#[test]
fn channels_alpha_drag_reorders_and_refuses_other_rows() {
    let reorder = |events: &[ChannelsEvent<u32>]| {
        events.iter().filter_map(|event| if let ChannelsEvent::Reorder(edit) = event { Some(edit.clone()) } else { None }).collect::<Vec<_>>()
    };
    let mut h = panel(sample_rows());
    drag(&mut h, At::Name("Alpha 1"), At::Bottom("Alpha 2", -4.0), true);
    let moves = reorder(&take(&mut h));
    let Some(EditEvent::Commit { value, .. }) = moves.last() else { panic!("expected a committed move, got {moves:?}") };
    assert_eq!((value.items.clone(), value.target, value.position), (vec![ALPHA_1], ALPHA_2, DropPosition::After));

    drag(&mut h, At::Name("Alpha 2"), At::Top("Quick Mask", 4.0), true);
    let moves = reorder(&take(&mut h));
    let Some(EditEvent::Commit { value, .. }) = moves.last() else { panic!("expected a committed move, got {moves:?}") };
    assert_eq!((value.target, value.position), (QUICK_MASK, DropPosition::Before));

    for (label, to) in [("Red", At::Name("Red")), ("RGB", At::Name("RGB")), ("below Quick Mask", At::Bottom("Quick Mask", -4.0))] {
        drag(&mut h, At::Name("Alpha 1"), to, true);
        let moves = reorder(&take(&mut h));
        assert!(!moves.iter().any(|edit| matches!(edit, EditEvent::Commit { .. } | EditEvent::Set { .. })), "{label}: {moves:?}");
    }
}

// Rows dropped on New, Delete and Load report `FooterDrop` with the key; a colour row on Delete
// reports nothing.
#[test]
fn channels_drop_on_footer_reports_button_and_key() {
    let mut h = panel(sample_rows());
    target_alpha(&mut h);
    let drops =
        |events: Vec<ChannelsEvent<u32>>| events.into_iter().filter(|event| matches!(event, ChannelsEvent::FooterDrop { .. })).collect::<Vec<_>>();
    for (label, button) in [
        ("Create new channel", ChannelsFooterButton::New),
        ("Delete current channel", ChannelsFooterButton::Delete),
        ("Load channel as selection", ChannelsFooterButton::LoadSelection),
    ] {
        drag(&mut h, At::Name("Alpha 1"), At::Footer(label), true);
        assert_eq!(drops(take(&mut h)), [ChannelsEvent::FooterDrop { button, rows: vec![ALPHA_1] }], "{label}");
    }
    drag(&mut h, At::Name("Alpha 1"), At::Footer("Save selection as channel"), true);
    assert!(drops(take(&mut h)).is_empty(), "Save takes no drops");
    drag(&mut h, At::Name("Red"), At::Footer("Delete current channel"), true);
    assert!(drops(take(&mut h)).is_empty(), "a colour row is not deleted");
    drag(&mut h, At::Name("Red"), At::Footer("Create new channel"), true);
    assert_eq!(drops(take(&mut h)), [ChannelsEvent::FooterDrop { button: ChannelsFooterButton::New, rows: vec![1] }]);
}

/// Targets Alpha 2, so the footer's Delete is enabled.
fn target_alpha(h: &mut Harness<'_, App>) {
    target(&mut h.state_mut().rows, ALPHA_2);
    h.run();
}

// A disabled Delete (the composite is targeted) reports no click.
#[test]
fn channels_disabled_delete_reports_no_click() {
    let mut h = panel(sample_rows());
    click(&mut h, At::Footer("Delete current channel"), Modifiers::NONE);
    assert!(!take(&mut h).iter().any(|event| matches!(event, ChannelsEvent::Footer { .. })));
}

// While the composite is targeted Delete is disabled for a click, but a dragged alpha channel
// still deletes: Delete reads enabled during the hover and reports `FooterDrop`. A colour channel
// is refused: Delete stays disabled and reports nothing.
#[test]
fn channels_disabled_delete_takes_no_drop() {
    let delete_enabled = |h: &Harness<'_, App>| !h.get_by_role_and_label(Role::Button, "Delete current channel").accesskit_node().is_disabled();
    let mut h = panel(sample_rows());
    assert!(!delete_enabled(&h));
    drag(&mut h, At::Name("Alpha 1"), At::Footer("Delete current channel"), false);
    assert!(delete_enabled(&h), "an accepted drag over Delete enables it");
    let release = At::Footer("Delete current channel").pos(&h);
    press(&mut h, release, false, Modifiers::NONE);
    h.run_steps(2);
    let events = take(&mut h);
    assert!(events.contains(&ChannelsEvent::FooterDrop { button: ChannelsFooterButton::Delete, rows: vec![ALPHA_1] }), "{events:?}");
    assert!(!delete_enabled(&h), "enabled only during the hover");

    drag(&mut h, At::Name("Red"), At::Footer("Delete current channel"), false);
    assert!(!delete_enabled(&h), "a refused drag leaves Delete disabled");
    let release = At::Footer("Delete current channel").pos(&h);
    press(&mut h, release, false, Modifiers::NONE);
    h.run_steps(2);
    assert!(!take(&mut h).iter().any(|event| matches!(event, ChannelsEvent::FooterDrop { .. })));
}

// With the composite targeted the list selects RGB, Red, Green and Blue; a drag of Green carries
// all four and a drop acts on Green, the row grabbed.
#[test]
fn channels_footer_drop_acts_on_the_grabbed_row_while_the_composite_is_targeted() {
    let mut h = panel(sample_rows());
    drag(&mut h, At::Name("Green"), At::Footer("Create new channel"), true);
    let drops: Vec<_> = take(&mut h).into_iter().filter(|event| matches!(event, ChannelsEvent::FooterDrop { .. })).collect();
    assert_eq!(drops, [ChannelsEvent::FooterDrop { button: ChannelsFooterButton::New, rows: vec![2] }]);
}

// Rename Channel… from a row's context menu opens that row's rename field.
#[test]
fn rename_channel_from_the_row_menu_starts_the_rename() {
    let mut h = panel(sample_rows());
    let pos = At::Name("Alpha 1").pos(&h);
    h.event(Event::PointerMoved(pos));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
    }
    h.run_steps(3);
    h.get_by_label("Rename Channel…").click();
    h.run_steps(3);
    assert!(h.query_by_role(Role::TextInput).is_some(), "the rename field is open");
}

// Rename Channel… from the docking host's panel menu opens the targeted row's rename field.
#[test]
fn rename_channel_from_the_panel_menu_starts_the_rename() {
    let mut rows = sample_rows();
    target(&mut rows, ALPHA_2);
    let mut h = panel(rows);
    assert!(h.query_by_role(Role::TextInput).is_none());
    let rows = h.state().rows.clone();
    let event = ChannelsPanel::run_command(&h.ctx, "channels", &rows, &"channels.rename".to_owned());
    assert_eq!(event, None);
    h.run_steps(3);
    assert!(h.query_by_role(Role::TextInput).is_some(), "the rename field is open");
}

// `ChannelsPanel::start_rename` opens the row's field in the pass it is given.
#[test]
fn start_rename_opens_the_rename_field() {
    let mut h = panel(sample_rows());
    h.state_mut().start_rename = Some(SPOT);
    h.run_steps(2);
    assert!(h.query_by_role(Role::TextInput).is_some());
}

// The `.thumbnail_size` override sets the thumbnails, so the rows' height: larger for Large,
// smaller with the thumbnails hidden.
#[test]
fn channels_thumbnail_size_override_sets_the_row_height() {
    let height = |size| row_rect(&panel_sized(sample_rows(), size), "Alpha 1").height();
    let default = height(None);
    assert!(height(Some(ThumbSize::Large)) > default, "Large");
    assert!(height(Some(ThumbSize::None)) < default, "None");
}

// The thumbnail side is `size.thumb.md`: 24 at Compact and 34 at Comfortable; `.thumbnail_size` overrides it
// (the `.thumbnail_size` override is checked with the rows).
#[test]
fn channels_thumbnail_side_follows_the_size_preset() {
    use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, builtins, resolve};
    use craft_ui::theme::{FontFamilies, UiTokens};

    let side = |size| {
        let appearance = Appearance { mode: Mode::Light, contrast: Contrast::Standard, size, text_size: TextSize::Small, reduced_motion: false };
        let theme = builtins().iter().find(|t| t.id.as_str() == "craft.studio").cloned().unwrap();
        let families = FontFamilies {
            ui: egui::FontFamily::Proportional,
            ui_semibold: egui::FontFamily::Proportional,
            mono: egui::FontFamily::Monospace,
            optical_size: false,
        };
        ChannelsPanel::<u32>::style(&UiTokens::new(resolve(&theme, appearance, None).tokens, appearance, Mode::Light, families)).thumb
    };
    assert_eq!((side(Size::Compact), side(Size::Comfortable)), (24.0, 34.0));
}

// The chords read Cmd+2 for the composite, Cmd+3.. for colours, then alphas while at most 9; Quick
// Mask and the layer mask have none.
#[test]
fn channels_shortcut_labels_follow_row_order() {
    let indicates = ColorIndicates::MaskedAreas;
    let mut rows = sample_rows();
    rows.insert(4, ChannelRow::new(20, ChannelKind::LayerMask { enabled: true }, "Layer 1 Mask"));
    for key in 30..33 {
        rows.push(ChannelRow::new(key, ChannelKind::Alpha { indicates }, format!("Alpha {key}")));
    }
    assert_eq!(
        shortcut_numbers(&rows),
        [Some(2), Some(3), Some(4), Some(5), None, Some(6), Some(7), Some(8), None, Some(9), None, None],
        "the layer mask and Quick Mask take no number; numbers stop after 9"
    );

    let h = panel(sample_rows());
    let chords = h.query_all_by(|n| n.role() == Role::Label && n.label().is_none()).count();
    assert_eq!(chords, 7, "one chord per row but Quick Mask");
    let (two, three) = (chord_text(&h.ctx, 2), chord_text(&h.ctx, 3));
    assert!(two.ends_with('2') && three.ends_with('3'), "{two:?} {three:?}");
}

// Cmd/Ctrl+Enter on the lead row reports `LoadSelection`.
#[test]
fn channels_keyboard_loads_selection() {
    let mut h = panel(sample_rows());
    click(&mut h, At::Name("Alpha 2"), Modifiers::NONE);
    take(&mut h);
    key(&mut h, Key::Enter, Modifiers::COMMAND);
    assert_eq!(take(&mut h), [ChannelsEvent::LoadSelection { key: ALPHA_2, operation: SelectionOperation::New }]);
    key(&mut h, Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(take(&mut h), [ChannelsEvent::LoadSelection { key: ALPHA_2, operation: SelectionOperation::Add }]);
}

// Spot, Quick Mask and layer-mask rows draw their names italic.
#[test]
fn channels_layer_mask_row_is_italic() {
    let mut rows = sample_rows();
    rows.insert(4, ChannelRow::new(20, ChannelKind::LayerMask { enabled: true }, "Layer 1 Mask"));
    let h = panel(rows);
    let styled = h.state().texts.clone();
    let italic = |name: &str| styled.iter().find(|(text, _)| text == name).map(|(_, italic)| *italic);
    for name in ["Spot 1", "Quick Mask", "Layer 1 Mask"] {
        assert_eq!(italic(name), Some(true), "{name}");
    }
    for name in ["RGB", "Red", "Alpha 1"] {
        assert_eq!(italic(name), Some(false), "{name}");
    }
}

// Adding and removing channels keeps the other rows' textures (same `TextureId`).
#[test]
fn channels_rows_update_in_place() {
    let mut h = panel(sample_rows());
    let before = h.state().textures.clone();
    assert!(before.iter().all(|(_, texture)| texture.is_some()), "every row has a picture: {before:?}");
    let indicates = ColorIndicates::MaskedAreas;
    let rows = &mut h.state_mut().rows;
    rows.remove(4);
    rows.insert(5, ChannelRow::new(40, ChannelKind::Alpha { indicates }, "Alpha 3"));
    h.run();
    let after = h.state().textures.clone();
    for (key, texture) in &before {
        if *key != ALPHA_1 {
            assert_eq!(after.iter().find(|(k, _)| k == key).map(|(_, t)| *t), Some(*texture), "row {key}");
        }
    }
    assert!(after.iter().any(|(key, texture)| *key == 40 && texture.is_some()));
}

/// A source that records the keys the panel asked pictures for.
struct Asked {
    inner: FakeThumbSource<u32>,
    keys: std::sync::Mutex<Vec<u32>>,
}

impl ThumbSource<u32> for Asked {
    fn revision(&self, key: &u32, request: &ThumbRequest) -> Revision {
        self.keys.lock().expect("lock").push(*key);
        self.inner.revision(key, request)
    }

    fn render(&self, key: &u32, request: &ThumbRequest) -> ThumbRender {
        self.inner.render(key, request)
    }
}

// A short panel draws the first rows; it also asks for the one row beyond the last drawn and no
// further.
#[test]
fn channels_rows_ask_one_row_beyond_the_drawn_ones() {
    let rows = sample_rows();
    let source = Asked { inner: FakeThumbSource::new(), keys: std::sync::Mutex::new(Vec::new()) };
    let mut h =
        HarnessBuilder::default().with_size(vec2(320.0, 150.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|ui| {
            let thumbs = Thumbnails::<u32>::in_context(ui.ctx(), "thumbs", ThumbnailOptions::default());
            let _ = ChannelsPanel::new("channels", &rows).thumbnails(&thumbs, &source).show(ui);
        });
    h.run();
    let drawn = h.query_all_by_role(Role::ListBoxOption).count();
    assert!((2..rows.len() - 2).contains(&drawn), "the panel draws some of the rows: {drawn}");
    let asked = source.keys.lock().expect("lock").clone();
    assert!(asked.contains(&(drawn as u32)), "one beyond: {asked:?}");
    assert!(!asked.contains(&(drawn as u32 + 1)), "not two beyond: {asked:?}");
}
