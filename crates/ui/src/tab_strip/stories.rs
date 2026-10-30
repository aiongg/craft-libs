//! Stories of [`TabStrip`]: a document strip and panel groups.

use crate::localized::Labels as _;
use std::sync::Arc;

use craft_theme::ControlSize;
use egui::mutex::Mutex;
use egui::{Align2, DragAndDrop, Pos2, Rect, Ui, vec2};

use super::layout::TabKind;
use super::strip::{TabEvent, TabMenuRequest, TabStrip};
use super::tab::{Tab, TabStripText};
use crate::behavior::{DropPosition, Move};
use crate::controls::IconButton;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt, PressMenu};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{Tokens, TokensOf as _, role_format};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct DocumentsArgs {
    #[arg(range = 160.0..=1200.0)]
    width: f64,
    large: bool,
    tear_off: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PanelsArgs {
    #[arg(range = 120.0..=480.0)]
    width: f64,
    locked: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DropsArgs {
    #[arg(range = 160.0..=1200.0)]
    width: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TextArgs {
    #[arg(range = 160.0..=1200.0)]
    width: f64,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/tab-strip/documents",
            Level::Composites,
            "TabStrip",
            "Documents",
            DocumentsArgs { width: 640.0, large: false, tear_off: true },
            show_documents
        ),
        typed_story!("composites/tab-strip/panels", Level::Composites, "TabStrip", "Panels", PanelsArgs { width: 260.0, locked: false }, show_panels),
        typed_story!("composites/tab-strip/drops", Level::Composites, "TabStrip", "Drops", DropsArgs { width: 480.0 }, show_drops),
        typed_story!("composites/tab-strip/text", Level::Composites, "TabStrip", "Text", TextArgs { width: 640.0 }, show_text),
    ]
}

/// Text tabs as in a New Document dialog's categories: labels only, the selected one in the text
/// colour and the others dimmed.
fn show_text(ui: &mut Ui, args: &mut TextArgs) {
    let width = (args.width as f32).min(ui.available_width());
    let shared = shared(ui, "tab-strip-text", || 0_u32);
    let mut selected = shared.lock();
    let names = ["Recent", "Saved", "Photo", "Print", "Art & Illustration", "Web", "Mobile", "Film & Video"];
    let tabs: Vec<Tab<u32>> = names.iter().zip(0..).map(|(name, key)| Tab::new(key, *name)).collect();
    let out = ui.allocate_ui(vec2(width, 0.0), |ui| TabStrip::new("text", "Categories", TabKind::Text).selected(&*selected).show(ui, &tabs)).inner;
    for event in out.events {
        if let TabEvent::Activate(key) = event {
            *selected = key;
        }
    }
}

/// The state shared between frames of a story.
fn shared<T: Send + 'static>(ui: &Ui, salt: &str, new: fn() -> T) -> Arc<Mutex<T>> {
    let id = ui.scope_id().with(salt);
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(new()))).clone())
}

/// Where a host opens its context menu for a request.
fn open_at<K>(request: &TabMenuRequest<K>) -> OpenAt {
    request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer)
}

/// A panel body under a strip, in the active tab's colour, with `text` in its middle.
fn body(ui: &mut Ui, width: f32, height: f32, text: &str) -> Rect {
    let tokens = Tokens::of(ui);
    let style = Tab::<u8>::style(&tokens);
    let (_, rect) = ui.allocate_space(vec2(width, height));
    ui.painter().rect_filled(rect, 0.0, style.active);
    let font = role_format(&tokens, craft_theme::TypeRole::Body, style.text).font_id;
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, font, style.text);
    rect
}

struct Doc {
    key: u32,
    name: String,
    detail: String,
    modified: bool,
    progress: Option<f32>,
}

struct Docs {
    docs: Vec<Doc>,
    selected: u32,
    /// The tab the open context menu acts on; `None` for the empty strip.
    menu_for: Option<u32>,
    last: String,
}

impl Docs {
    fn new() -> Self {
        let names = [
            ("Untitled-1", "100% (RGB/8)", false, None),
            ("Harbour at dusk.psd", "50% (Layer 1, RGB/8)", true, None),
            ("Portrait retouch, final version for the printer.tif", "33.3% (Background, CMYK/16)", false, None),
            ("Logo.svg", "200% (RGB/8)", false, None),
            ("Opening.raw", "", false, Some(0.35)),
            ("Saving.psd", "25% (Curves 1, RGB/16)", true, Some(0.8)),
            ("Mountains panorama, stitched from twelve frames.psb", "12.5% (RGB/16)", true, None),
            ("Scan 0042.tif", "66.7% (Gray/8)", false, None),
            ("Banner.png", "100% (Layer 3, RGB/8)", false, None),
            ("Cover art.psd", "50% (Type, CMYK/8)", true, None),
            ("Texture.exr", "100% (RGB/32)", false, None),
            ("Sketch.jpg", "75% (RGB/8)", false, None),
            ("Wedding album page 17.psd", "25% (Group 2, RGB/8)", true, None),
            ("Icon set.psd", "800% (Icons, RGB/8)", false, None),
        ];
        let docs = names
            .into_iter()
            .zip(1..)
            .map(|((name, detail, modified, progress), key)| Doc { key, name: name.to_owned(), detail: detail.to_owned(), modified, progress })
            .collect();
        Self { docs, selected: 2, menu_for: None, last: "Click, middle-click, double-click or right-click a tab".to_owned() }
    }

    fn name(&self, key: u32) -> &str {
        self.docs.iter().find(|d| d.key == key).map_or("", |d| d.name.as_str())
    }

    /// Closes the documents `close` says, and shows a neighbour when the shown one closes.
    fn close(&mut self, close: impl Fn(u32) -> bool) {
        let at = self.docs.iter().position(|d| d.key == self.selected).unwrap_or(0);
        self.docs.retain(|d| !close(d.key));
        if !self.docs.iter().any(|d| d.key == self.selected)
            && let Some(doc) = self.docs.get(at.min(self.docs.len().saturating_sub(1)))
        {
            self.selected = doc.key;
        }
    }
}

#[derive(Clone, Copy)]
enum DocCommand {
    Close,
    CloseOthers,
    CloseAll,
}

/// Moves the item with key `mv.items[0]` next to the one with key `mv.target`, as a host applies
/// a [`Move`] to its order.
fn apply_move<T>(items: &mut Vec<T>, key: impl Fn(&T) -> u32, mv: &Move<u32>) {
    let Some(from) = items.iter().position(|item| mv.items.contains(&key(item))) else { return };
    let moved = items.remove(from);
    let Some(target) = items.iter().position(|item| key(item) == mv.target) else {
        items.insert(from.min(items.len()), moved);
        return;
    };
    let at = if mv.position == DropPosition::After { target + 1 } else { target };
    items.insert(at, moved);
}

/// Fourteen documents: modified marks, one opening and one saving with progress, long titles, the
/// overflow menu of every document, the ×, middle click to close and a context menu with Close,
/// Close Others and Close All. Drag a tab to reorder it, or press Cmd+] and Cmd+[ (Ctrl off
/// macOS, Shift to the ends) on the focused tab; with `tear_off`, a drag past the band shows a log
/// line and a release there reports the tab leaving. The line under the strip shows the last
/// event.
fn show_documents(ui: &mut Ui, args: &mut DocumentsArgs) {
    let (width, large, tear_off) = ((args.width as f32).min(ui.available_width()), args.large, args.tear_off);
    let shared = shared(ui, "tab-strip-documents", Docs::new);
    let mut docs = shared.lock();
    let tabs: Vec<Tab<u32>> = docs
        .docs
        .iter()
        .map(|d| Tab::new(d.key, d.name.clone()).detail(d.detail.clone()).modified(d.modified).closable(true).progress(d.progress))
        .collect();
    let text = TabStripText { more: "More documents".to_owned(), ..TabStripText::default() };
    let out = ui
        .allocate_ui(vec2(width, 0.0), |ui| {
            TabStrip::new("documents", "Documents", TabKind::Document)
                .selected(&docs.selected)
                .large(large)
                .tear_off(tear_off)
                .text(text)
                .show(ui, &tabs)
        })
        .inner;
    if let Some(out_of) = &out.dragging_out {
        docs.last = format!("{} is outside the strip at {}", docs.name(out_of.tab), point(out_of.pointer));
    } else if let Some(mv) = &out.drop_target {
        docs.last = format!("Drop {} {} {}", docs.name(mv.items[0]), position(mv.position), docs.name(mv.target));
    }
    let mut open = None;
    for event in out.events {
        match event {
            TabEvent::Activate(key) => {
                docs.selected = key;
                docs.last = format!("Show {}", docs.name(key));
            }
            TabEvent::Close(key) => {
                docs.last = format!("Close {}", docs.name(key));
                docs.close(|k| k == key);
            }
            TabEvent::DoubleClick(key) => {
                docs.last = key.map_or_else(|| "Double click on the strip".to_owned(), |key| format!("Double click on {}", docs.name(key)));
            }
            TabEvent::ContextMenu(request) => {
                docs.menu_for = request.tab;
                open = Some(open_at(&request));
            }
            TabEvent::Reorder(EditEvent::Commit { value: mv, .. } | EditEvent::Set { value: mv, .. }) => {
                docs.last = format!("Moved {} {} {}", docs.name(mv.items[0]), position(mv.position), docs.name(mv.target));
                apply_move(&mut docs.docs, |d| d.key, &mv);
            }
            TabEvent::Reorder(EditEvent::Cancel { reason, .. }) => docs.last = format!("Drag cancelled: {reason:?}"),
            TabEvent::DroppedOut { tab, pointer } => docs.last = format!("{} dropped outside the strip at {}", docs.name(tab), point(pointer)),
            _ => {}
        }
    }
    let on_tab = docs.menu_for.is_some();
    let entries: Vec<MenuEntry<DocCommand>> = vec![
        MenuItem::new("Close", DocCommand::Close).enabled(on_tab).into(),
        MenuItem::new("Close Others", DocCommand::CloseOthers).enabled(on_tab && docs.docs.len() > 1).into(),
        MenuItem::new("Close All", DocCommand::CloseAll).enabled(!docs.docs.is_empty()).into(),
    ];
    let menu = ContextMenu::new(&entries).own_gestures(false).open_at(open).show(ui, &out.response);
    if let Some(command) = menu.chosen {
        let target = docs.menu_for;
        match command {
            DocCommand::Close => docs.close(|k| Some(k) == target),
            DocCommand::CloseOthers => docs.close(|k| Some(k) != target),
            DocCommand::CloseAll => docs.close(|_| true),
        }
    }
    let shown = docs.name(docs.selected).to_owned();
    body(ui, width, 120.0, &shown);
    ui.label(docs.last.as_str());
}

fn position(position: DropPosition) -> &'static str {
    match position {
        DropPosition::Before => "before",
        DropPosition::After => "after",
        DropPosition::Into => "into",
    }
}

fn point(pos: Pos2) -> String {
    format!("({:.0}, {:.0})", pos.x, pos.y)
}

struct Group {
    name: &'static str,
    panels: Vec<(u32, &'static str)>,
    selected: u32,
    collapsed: bool,
}

struct Panels {
    groups: Vec<Group>,
    /// The group and panel the open context menu acts on.
    menu_for: Option<(usize, Option<u32>)>,
    last: String,
}

impl Panels {
    fn new() -> Self {
        let group = |name, panels: &[&'static str], selected| Group {
            name,
            panels: panels.iter().copied().zip(1..).map(|(name, key)| (key, name)).collect(),
            selected,
            collapsed: false,
        };
        Self {
            groups: vec![
                group("Color", &["Color", "Swatches", "Gradients", "Patterns"], 1),
                group("Layers", &["Layers", "Channels", "Paths", "History", "Actions", "Properties"], 4),
                group("Adjustments", &["Adjustments", "Libraries"], 1),
            ],
            menu_for: None,
            last: "Double-click a strip to collapse its group; drag the empty strip to move it".to_owned(),
        }
    }
}

#[derive(Clone, Copy)]
enum PanelCommand {
    Close,
    CloseGroup,
}

/// Three panel groups: tabs shrink and then overflow into a menu of the hidden panels, a collapse
/// button at the reading start and a panel menu at the end, double click to collapse, a group
/// drag from the empty strip, and a context menu. `locked` stops tab drags, so a drag on a tab
/// moves the group.
fn show_panels(ui: &mut Ui, args: &mut PanelsArgs) {
    let (width, locked) = ((args.width as f32).min(ui.available_width()), args.locked);
    let shared = shared(ui, "tab-strip-panels", Panels::new);
    let mut state = shared.lock();
    let button = Tab::<u32>::style(&Tokens::of(ui)).height;
    let mut command = None;
    for at in 0..state.groups.len() {
        let group = &state.groups[at];
        let tabs: Vec<Tab<u32>> = group.panels.iter().map(|&(key, name)| Tab::new(key, name).closable(true)).collect();
        let (name, selected, collapsed) = (group.name, group.selected, group.collapsed);
        let mut toggle = false;
        let mut chosen = None;
        let text = TabStripText { more: "More panels".to_owned(), ..TabStripText::default() };
        let out = ui
            .allocate_ui(vec2(width, 0.0), |ui| {
                TabStrip::new(("panels", at), name, TabKind::Panel)
                    .selected(&selected)
                    .reorderable(!locked)
                    .text(text)
                    .leading(button, |ui| {
                        let icon = if collapsed { Icon::ChevronRight } else { Icon::ChevronDown };
                        let label = if collapsed { "Expand group" } else { "Collapse group" };
                        toggle = ui.add(IconButton::new(icon, label).size(ControlSize::Sm)).clicked();
                    })
                    .trailing(button, |ui| {
                        let entries: Vec<MenuEntry<PanelCommand>> = vec![
                            MenuItem::new("Close", PanelCommand::Close).into(),
                            MenuItem::new("Close Tab Group", PanelCommand::CloseGroup).into(),
                        ];
                        let response = ui
                            .add(IconButton::new(Icon::More, crate::composites::panel_header::PanelHeaderLabels::of(ui).menu).size(ControlSize::Sm));
                        chosen = PressMenu::new(&entries).show(ui, &response).chosen;
                    })
                    .show(ui, &tabs)
            })
            .inner;
        if let Some(chosen) = chosen {
            command = Some((at, Some(selected), chosen));
        }
        let mut open = None;
        let mut last = None;
        let mut collapse = toggle;
        for event in out.events {
            match event {
                TabEvent::Activate(key) => state.groups[at].selected = key,
                TabEvent::Reorder(EditEvent::Commit { value: mv, .. } | EditEvent::Set { value: mv, .. }) => {
                    apply_move(&mut state.groups[at].panels, |&(key, _)| key, &mv);
                }
                TabEvent::DoubleClick(_) => collapse = true,
                TabEvent::ContextMenu(request) => {
                    state.menu_for = Some((at, request.tab));
                    open = Some(open_at(&request));
                }
                _ => {}
            }
        }
        if collapse {
            let group = &mut state.groups[at];
            group.collapsed = !group.collapsed;
            last = Some(format!("{} the {name} group", if group.collapsed { "Collapsed" } else { "Expanded" }));
        }
        if out.background.dragged() {
            last = Some(format!("Moving the {name} group"));
        } else if out.background.drag_stopped() {
            last = Some(format!("Moved the {name} group"));
        }
        if let Some(last) = last {
            state.last = last;
        }
        let menu_tab = state.menu_for.filter(|(group, _)| *group == at).and_then(|(_, tab)| tab);
        let entries: Vec<MenuEntry<PanelCommand>> = vec![
            MenuItem::new("Close", PanelCommand::Close).enabled(menu_tab.is_some()).into(),
            MenuItem::new("Close Tab Group", PanelCommand::CloseGroup).into(),
        ];
        if let Some(chosen) = ContextMenu::new(&entries).own_gestures(false).open_at(open).show(ui, &out.response).chosen {
            command = Some((at, menu_tab, chosen));
        }
        let group = &state.groups[at];
        if !group.collapsed {
            let shown = group.panels.iter().find(|(key, _)| *key == group.selected).map_or("", |(_, name)| name);
            body(ui, width, 80.0, shown);
        }
        ui.add_space(8.0);
    }
    if let Some((at, tab, chosen)) = command {
        apply_panel_command(&mut state, at, tab, chosen);
    }
    ui.label(state.last.as_str());
}

fn apply_panel_command(state: &mut Panels, at: usize, tab: Option<u32>, command: PanelCommand) {
    let Some(group) = state.groups.get_mut(at) else { return };
    match command {
        PanelCommand::Close => {
            group.panels.retain(|(key, _)| Some(*key) != tab);
            if let Some(&(key, _)) = group.panels.first().filter(|_| Some(group.selected) == tab) {
                group.selected = key;
            }
            state.last = format!("Closed a panel of the {} group", group.name);
        }
        PanelCommand::CloseGroup => {
            state.last = format!("Closed the {} group", group.name);
            state.groups.remove(at);
        }
    }
}

/// A file dragged from the button over a document strip: the strip draws its drop line at
/// `slot_at` between tabs, and outlines the tab under the pointer's middle part (a drop onto that
/// document).
#[derive(Clone)]
struct FileDrag;

struct Drops {
    names: Vec<(u32, &'static str)>,
    selected: u32,
    slot: Option<usize>,
    over: Option<u32>,
    last: String,
}

fn show_drops(ui: &mut Ui, args: &mut DropsArgs) {
    let width = (args.width as f32).min(ui.available_width());
    let shared = shared(ui, "tab-strip-drops", || Drops {
        names: ["Cover", "Chapter 1", "Chapter 2", "Notes"].iter().copied().zip(1..).map(|(name, key)| (key, name)).collect(),
        selected: 1,
        slot: None,
        over: None,
        last: "Drag the file over the strip".to_owned(),
    });
    let mut drops = shared.lock();
    ui.dnd_drag_source(ui.make_persistent_id("file"), FileDrag, |ui| {
        ui.label("scan.tif (drag onto the strip)");
    });
    let tabs: Vec<Tab<u32>> = drops.names.iter().map(|&(key, name)| Tab::new(key, name)).collect();
    let out = ui
        .allocate_ui(vec2(width, 0.0), |ui| {
            TabStrip::new("drops", "Documents", TabKind::Document)
                .selected(&drops.selected)
                .insertion_marker(drops.slot)
                .drop_highlight(drops.over)
                .show(ui, &tabs)
        })
        .inner;
    for event in &out.events {
        if let TabEvent::Activate(key) = event {
            drops.selected = *key;
        }
    }
    let pointer = ui.input(|i| i.pointer.latest_pos()).filter(|pos| out.response.rect.contains(*pos));
    let dragging = DragAndDrop::has_payload_of_type::<FileDrag>(ui.ctx());
    let (slot, over) = match pointer.filter(|_| dragging) {
        Some(pos) => {
            let over =
                out.tab_at(pos).filter(|key| out.tabs.iter().any(|(k, rect)| k == key && rect.shrink2(vec2(rect.width() / 4.0, 0.0)).contains(pos)));
            (over.is_none().then(|| out.slot_at(pos)), over)
        }
        None => (None, None),
    };
    if dragging && ui.input(|i| i.pointer.any_released()) && pointer.is_some() {
        drops.last = match (slot, over) {
            (_, Some(key)) => format!("Dropped the file onto {}", drops.names.iter().find(|(k, _)| *k == key).map_or("", |(_, n)| n)),
            (Some(slot), None) => format!("Dropped the file as a new document at {slot}"),
            _ => String::new(),
        };
    }
    drops.slot = slot;
    drops.over = over;
    ui.label(drops.last.as_str());
}
