//! Screen tests of the footer and the panel menu (L7): `footer::show_footer` shown alone, and
//! `panel_menu` with `run_command`.

#![allow(clippy::expect_used, reason = "tests")]

use std::cell::RefCell;

use craft_theme::LabelColor;
use craft_ui::behavior::ListDrag;
use craft_ui::list::ListRow;
use craft_ui::localized::Labels as _;
use craft_ui::menu::{MenuEntry, MenuItem, OpenAt};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::HasPopup;
use egui::{Context, Event, Id, IdSalt, Modifiers, PointerButton, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::LayersPanel;
use super::context::{PanelContext, RowSink};
use super::events::LayersEvent;
use super::footer::{LayersFooterAction, LayersFooterButton, accepts_footer_drop, show_footer};
use super::labels::LayersLabels;
use super::menu::{CLIP_THUMBNAILS, COLLAPSE_ALL, LayersMenuAction, MenuRequest, PANEL_OPTIONS, SHOW_FILTERS, note_drawn, show_row_menus};
use super::style::LayersStyle;
use crate::ui::mock::FakeLayers;
use crate::ui::options::{LayersOptions, LayersSettings};
use crate::ui::thumbs::ThumbContents;
use crate::ui::tree::{LayerRowKey, LayerTree as _};

struct App {
    tree: FakeLayers,
    events: Vec<LayersEvent<u32>>,
}

fn footer() -> Harness<'static, App> {
    let app = App { tree: FakeLayers::sample(), events: Vec::new() };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, app: &mut App| {
                let tokens = Tokens::of(ui);
                let options = LayersOptions::default();
                let labels = LayersLabels::of(ui);
                let cx = PanelContext {
                    ctx: ui.ctx().clone(),
                    id: Id::unique("layers"),
                    tree: &app.tree,
                    selection: app.tree.selection(),
                    options: &options,
                    thumbs: None,
                    labels: &labels,
                    style: LayersStyle::of(&tokens),
                    row_style: ListRow::<u32>::style(&tokens),
                    lock_column: false,
                    visibility_key: None,
                    trailing: None,
                    sink: RefCell::new(RowSink::default()),
                };
                app.events.extend(show_footer(ui, &cx, Id::unique("list")));
            },
            app,
        );
    harness.run();
    harness
}

const DELETE: &str = "Delete Layer";
const NEW_LAYER: &str = "Create a new layer";
const NEW_GROUP: &str = "Create a new group";
const ADJUSTMENT: &str = "Create new fill or adjustment layer";
const MASK: &str = "Add Layer Mask";
const STYLE: &str = "Add a layer style";
const LINK: &str = "Link layers";

fn click(harness: &mut Harness<'static, App>, label: &str, modifiers: Modifiers) -> Option<LayersFooterAction> {
    harness.state_mut().events.clear();
    harness.get_by_label(label).click_modifiers(modifiers);
    harness.run();
    harness.state().events.iter().find_map(|event| if let LayersEvent::Footer(action) = event { Some(action.clone()) } else { None })
}

// Link, fx, Mask, Adjustment, Group, New, Delete from the reading start.
#[test]
fn footer_order_is_link_fx_mask_adjustment_group_new_delete() {
    let harness = footer();
    let xs: Vec<f32> =
        [LINK, STYLE, MASK, ADJUSTMENT, NEW_GROUP, NEW_LAYER, DELETE].iter().map(|l| harness.get_by_label(l).rect().center().x).collect();
    assert!(xs.windows(2).all(|pair| pair[0] < pair[1]), "left to right: {xs:?}");
}

// New Layer: Alt asks, Cmd below; New Group: Shift or two selected from the selection, Alt asks,
// Cmd below; Delete: Alt doesn't confirm; Mask: Alt inverts.
#[test]
fn footer_modifiers_report_ask_below_and_from_selection() {
    let mut h = footer();
    let layer = |ask, below| Some(LayersFooterAction::NewLayer { ask, below });
    assert_eq!(click(&mut h, NEW_LAYER, Modifiers::NONE), layer(false, false));
    assert_eq!(click(&mut h, NEW_LAYER, Modifiers::ALT), layer(true, false));
    assert_eq!(click(&mut h, NEW_LAYER, Modifiers::COMMAND), layer(false, true));

    let group = |from_selection, ask, below| Some(LayersFooterAction::NewGroup { from_selection, ask, below });
    assert_eq!(click(&mut h, NEW_GROUP, Modifiers::NONE), group(false, false, false));
    assert_eq!(click(&mut h, NEW_GROUP, Modifiers::SHIFT), group(true, false, false));
    assert_eq!(click(&mut h, NEW_GROUP, Modifiers::ALT), group(false, true, false));
    assert_eq!(click(&mut h, NEW_GROUP, Modifiers::COMMAND), group(false, false, true));
    h.state_mut().tree.selection.layers = vec![2, 4];
    h.state_mut().tree.selection.active = Some(4);
    assert_eq!(click(&mut h, NEW_GROUP, Modifiers::NONE), group(true, false, false), "two selected layers");
    h.state_mut().tree.selection.active = None;
    assert_eq!(click(&mut h, NEW_GROUP, Modifiers::SHIFT), group(false, false, false), "no active layer: an empty group");

    let delete = |confirm| Some(LayersFooterAction::Delete { confirm });
    h.state_mut().tree.selection.active = Some(4);
    assert_eq!(click(&mut h, DELETE, Modifiers::NONE), delete(true));
    assert_eq!(click(&mut h, DELETE, Modifiers::ALT), delete(false));
    assert_eq!(click(&mut h, MASK, Modifiers::NONE), Some(LayersFooterAction::AddMask { invert: false }));
    assert_eq!(click(&mut h, MASK, Modifiers::ALT), Some(LayersFooterAction::AddMask { invert: true }));
    assert_eq!(click(&mut h, LINK, Modifiers::NONE), Some(LayersFooterAction::Link));
}

// The adjustment menu opens on the press and a release over an item reports `Adjustment(command)`.
#[test]
fn adjustment_menu_opens_on_press_and_release_chooses() {
    let mut h = footer();
    let button = h.get_by_label(ADJUSTMENT).rect().center();
    h.event(Event::PointerMoved(button));
    h.event(Event::PointerButton { pos: button, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run();
    let levels = h.get_by_label("Levels").rect().center();
    assert!(h.state().events.is_empty(), "the press only opens the menu");
    h.event(Event::PointerMoved(levels));
    h.run();
    h.event(Event::PointerButton { pos: levels, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
    assert_eq!(h.state().events, [LayersEvent::Footer(LayersFooterAction::Adjustment(String::from("levels")))]);

    h.state_mut().events.clear();
    let style = h.get_by_label(STYLE).rect().center();
    h.event(Event::PointerMoved(style));
    h.event(Event::PointerButton { pos: style, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run();
    let stroke = h.get_by_label("Stroke").rect().center();
    h.event(Event::PointerMoved(stroke));
    h.run();
    h.event(Event::PointerButton { pos: stroke, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
    assert_eq!(h.state().events, [LayersEvent::Footer(LayersFooterAction::Style(String::from("stroke")))]);
}

// Show Filters, Clip Thumbnails, Panel Options… and Collapse All Groups: the entries and what
// `run_command` does with each.
#[test]
fn panel_menu_entries_and_commands_change_the_settings() {
    let ctx = Context::default();
    let mut settings = LayersSettings::default();
    let rows = |entries: &[MenuEntry<String>]| -> Vec<(String, Option<String>, bool, Option<bool>)> {
        entries
            .iter()
            .filter_map(MenuEntry::item)
            .map(|item| (item.label().to_owned(), item.command().cloned(), item.is_enabled(), item.item_kind().checked()))
            .collect()
    };
    let entries = LayersPanel::<FakeLayers>::panel_menu(&ctx, "layers", &settings);
    let top = rows(&entries);
    assert_eq!(
        top.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
        ["Filter Options", "Panel Options…", "Collapse All Groups", "Clip Thumbnails to Layer Bounds"]
    );
    let show = entries[0].item().and_then(|item| item.children()).map(rows).expect("Filter Options is a submenu");
    assert_eq!(show, [(String::from("Show"), Some(SHOW_FILTERS.to_owned()), true, Some(true))]);
    assert!(!top[2].2, "Collapse All Groups waits for the first pass");
    assert_eq!(top[3].3, Some(false));

    note_drawn(&ctx, IdSalt::new("layers"));
    assert!(rows(&LayersPanel::<FakeLayers>::panel_menu(&ctx, "layers", &settings))[2].2);
    assert!(!rows(&LayersPanel::<FakeLayers>::panel_menu(&ctx, "other", &settings))[2].2, "the mark belongs to its panel");

    let run = |settings: &mut LayersSettings, command: &str| LayersPanel::<FakeLayers>::run_command(&ctx, "layers", settings, &command.to_owned());
    assert_eq!(run(&mut settings, SHOW_FILTERS), Some(LayersMenuAction::SettingsChanged));
    assert!(!settings.show_filters);
    assert_eq!(run(&mut settings, CLIP_THUMBNAILS), Some(LayersMenuAction::SettingsChanged));
    assert_eq!(settings.options.thumbnail_contents, ThumbContents::LayerBounds);
    assert_eq!(rows(&LayersPanel::<FakeLayers>::panel_menu(&ctx, "layers", &settings))[3].3, Some(true));
    assert_eq!(run(&mut settings, CLIP_THUMBNAILS), Some(LayersMenuAction::SettingsChanged));
    assert_eq!(settings.options.thumbnail_contents, ThumbContents::EntireDocument);
    assert_eq!(run(&mut settings, PANEL_OPTIONS), Some(LayersMenuAction::OpenOptions));
    assert_eq!(run(&mut settings, COLLAPSE_ALL), Some(LayersMenuAction::CollapseAll));
    assert_eq!(run(&mut settings, "somebody.else"), None);
}

// The footer buttons are named by their words (Delete and Mask by the app's labels), the menu
// buttons have `HasPopup`.
#[test]
fn footer_and_toggle_cells_are_named() {
    let h = footer();
    for label in [LINK, STYLE, MASK, ADJUSTMENT, NEW_GROUP, NEW_LAYER, DELETE] {
        let popup = h.get_by_label(label).accesskit_node().has_popup();
        let menu = label == STYLE || label == ADJUSTMENT;
        assert_eq!(popup, menu.then_some(HasPopup::Menu), "{label}");
    }
}

// Delete takes every kind of row and a mask; New Layer and New Group take layer rows only.
#[test]
fn footer_buttons_accept_the_drags_the_spec_names() {
    let list = Id::unique("list");
    let drag = |items: Vec<LayerRowKey<u32>>| ListDrag { list, grabbed: items[0].clone(), items, label: String::new() };
    let layers = drag(vec![LayerRowKey::Layer(1), LayerRowKey::Layer(2)]);
    let effects = drag(vec![LayerRowKey::Effects(1, Some(0))]);
    let mask = drag(vec![LayerRowKey::Mask(1, crate::ui::tree::LayerPart::LayerMask)]);
    let accepts = |button, drag: &ListDrag<LayerRowKey<u32>>| accepts_footer_drop(button, drag, list);
    for drag in [&layers, &effects, &mask] {
        assert!(accepts(LayersFooterButton::Delete, drag));
    }
    for button in [LayersFooterButton::NewLayer, LayersFooterButton::NewGroup] {
        assert!(accepts(button, &layers));
        assert!(!accepts(button, &effects));
        assert!(!accepts(button, &mask));
    }
}

// A drag from another panel's list (a second document's Layers panel) is taken by no button.
#[test]
fn footer_buttons_refuse_a_drag_from_another_list() {
    let drag = ListDrag { list: Id::unique("other"), grabbed: LayerRowKey::Layer(1u32), items: vec![LayerRowKey::Layer(1)], label: String::new() };
    for button in [LayersFooterButton::Delete, LayersFooterButton::NewLayer, LayersFooterButton::NewGroup] {
        assert!(!accepts_footer_drop(button, &drag, Id::unique("list")), "{button:?}");
    }
}

// The eye menu lists None and the eleven colours with the current one checked, stays open over
// later passes, and a pick reports `LabelColor` for every row it was opened for; the row menu
// reports the host's command with its target.
#[test]
fn row_menus_stay_open_and_report_their_pick() {
    struct Menus {
        tree: FakeLayers,
        request: Option<MenuRequest<u32>>,
        events: Vec<LayersEvent<u32>>,
    }
    let app = Menus { tree: FakeLayers::sample(), request: None, events: Vec::new() };
    let mut h =
        HarnessBuilder::default().with_size(vec2(400.0, 400.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, app: &mut Menus| {
                let tokens = Tokens::of(ui);
                let options = LayersOptions::default();
                let labels = LayersLabels::of(ui);
                let cx = PanelContext {
                    ctx: ui.ctx().clone(),
                    id: Id::unique("layers"),
                    tree: &app.tree,
                    selection: app.tree.selection(),
                    options: &options,
                    thumbs: None,
                    labels: &labels,
                    style: LayersStyle::of(&tokens),
                    row_style: ListRow::<u32>::style(&tokens),
                    lock_column: false,
                    visibility_key: None,
                    trailing: None,
                    sink: RefCell::new(RowSink::default()),
                };
                let list = ui.allocate_response(vec2(100.0, 50.0), egui::Sense::click());
                let host = |target: &super::events::MenuTarget<u32>| -> Vec<MenuEntry<String>> {
                    vec![MenuItem::new(format!("Rename {}", target.row.layer()), String::from("rename")).into()]
                };
                let request = app.request.take();
                app.events.extend(show_row_menus(ui, &cx, &list, Some(&host), request));
            },
            app,
        );
    h.run();
    h.state_mut().request = Some(MenuRequest::Eye { rows: vec![2, 3], current: Some(LabelColor::Blue), at: OpenAt::Pointer(egui::pos2(20.0, 20.0)) });
    h.run();
    h.run();
    for name in ["None", "Red", "Gray"] {
        h.get_by_label(name);
    }
    h.get_by_label("Blue").click();
    h.run();
    assert_eq!(h.state().events, [LayersEvent::LabelColor { rows: vec![2, 3], color: Some(LabelColor::Blue) }]);
    assert!(h.query_by_label("Red").is_none(), "a pick closes the menu");

    h.state_mut().events.clear();
    let target = super::events::MenuTarget { row: LayerRowKey::Layer(4), part: None, on_selection: false };
    h.state_mut().request = Some(MenuRequest::Row { target: target.clone(), at: OpenAt::Pointer(egui::pos2(20.0, 20.0)) });
    h.run();
    h.run();
    h.get_by_label("Rename 4").click();
    h.run();
    assert_eq!(h.state().events, [LayersEvent::Menu { target, command: String::from("rename") }]);
}
