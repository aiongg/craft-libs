//! [`StartScreen`]: the home page an app shows with no document open: New and Open, the recent
//! files as a grid or list, pinned folders and templates.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use craft_i18n::Folder;
use craft_theme::{ButtonVariant, ControlSize, TypeRole};
use egui::accesskit::{Action, Role};
use egui::text::{LayoutJob, TextWrapping};
use egui::{AsIdSalt, Id, IdSalt, Key, Modifiers, Rect, ScrollArea, Sense, TextureId, Ui, UiBuilder, pos2, vec2};

use crate::behavior::{GridNav, RowInfo, Selection};
use crate::composites::{EmptyState, SectionHeader};
use crate::controls::{Button, IconButton, Segmented, Select};
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::list::{List, ListEvent, ListRow, ThumbSize};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::primitives::{Content, Corner, Gap, Interactive, Pad, Row, Stack, Surface, SurfaceKind, Text};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// The narrowest and widest a card gets; columns come and go to keep cards within them
/// (FigCraft's home screen).
const CARD_MIN: f32 = 260.0;
const CARD_MAX: f32 = 360.0;

/// A recent file as the start screen lists it. The host keeps the list (craft-platform's recent
/// files in step 4) and passes it every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RecentFile {
    pub path: PathBuf,
    /// The name shown: the file name without its folder.
    pub name: String,
    /// Last opened, in seconds since the Unix epoch, for sorting and the "2 days ago" line.
    pub opened: Option<i64>,
    /// Created, for sorting by Created.
    pub created: Option<i64>,
    /// A thumbnail the host has loaded (`egui::TextureId`), drawn in the grid card.
    pub thumbnail: Option<egui::TextureId>,
}

/// A template card: a named document size or starter file.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub id: String,
    pub name: String,
    /// "1920 × 1080 px".
    pub detail: String,
    pub thumbnail: Option<egui::TextureId>,
}

/// How the recent files show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RecentView {
    /// Cards with thumbnails, as many columns as fit.
    #[default]
    Grid,
    /// One row per file: icon, name, folder, date.
    List,
}

/// The recent files' order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RecentSort {
    /// Most recently opened first.
    #[default]
    Opened,
    Name,
    Created,
}

/// What the user did on the start screen this frame. The host runs it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StartAction {
    /// New… (the host opens its New Document dialog).
    New,
    /// Open… (the host opens its file dialog).
    Open,
    OpenRecent(PathBuf),
    /// "Remove from Recent" on a card's context menu, or its × on hover.
    RemoveRecent(PathBuf),
    /// "Clear Recent Files" on the context menu.
    ClearRecent,
    /// "Show in Folder" on a card's context menu.
    RevealRecent(PathBuf),
    OpenPinned(PathBuf),
    /// "Pin Folder…" (the host asks for a folder).
    PinFolder,
    UnpinFolder(PathBuf),
    UseTemplate(String),
    /// The view or sort changed; the host stores it with its preferences.
    ViewChanged(RecentView, RecentSort),
}

/// The start screen, drawn in the shell's body when no document is open.
///
/// ```no_run
/// use craft_ui::shell::{RecentFile, StartAction, StartScreen};
/// # fn demo(ui: &mut egui::Ui, recent: &[RecentFile]) {
/// for action in StartScreen::new("home").recent(recent).show(ui) {
///     match action {
///         StartAction::New => { /* open the New dialog */ }
///         StartAction::OpenRecent(path) => { /* open it */ }
///         _ => {}
///     }
/// }
/// # }
/// ```
///
/// Layout: a header row with New and Open (New the primary button), then the sections the host
/// fills: Recent (with the Grid/List toggle and the Sort menu), Pinned Folders, Templates. Templates
/// with no template is left out, and Pinned Folders is left out unless the host calls
/// [`pinned`](StartScreen::pinned); with no recent file the Recent section shows an empty state. The page scrolls; the cards wrap at the body's width.
///
/// Keyboard: New and Open are the first Tab stops; the recent cards are one grid
/// ([`crate::behavior`]'s grid navigation): arrows move, Enter opens, the Menu key or Shift+F10
/// opens the card's context menu, Delete removes it from Recent. Cmd+N and Cmd+O are the app's
/// shortcuts and need nothing here.
///
/// AccessKit: the page is a `Group` named [`StartLabels::page`] ("Home"); each card is a button named by the file
/// name, with the folder and date as its description.
#[must_use = "show the start screen with `show`"]
pub struct StartScreen<'a> {
    id_salt: IdSalt,
    recent: &'a [RecentFile],
    pinned: Option<&'a [PathBuf]>,
    templates: &'a [Template],
    view: RecentView,
    sort: RecentSort,
    max_recent: usize,
    now: Option<i64>,
    labels: Option<StartLabels>,
}

impl<'a> StartScreen<'a> {
    /// A start screen with New and Open and nothing else.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            recent: &[],
            pinned: None,
            templates: &[],
            view: RecentView::default(),
            sort: RecentSort::default(),
            max_recent: 12,
            now: None,
            labels: None,
        }
    }

    /// The recent files, most recent first.
    // figcraft: crates/ui-egui/src/home/home_recents.rs:42 (`SHOWN` 12), :48 `View`, :79 `SortKey`;
    // pc: crates/ui-egui/src/canvas/start_screen.rs:99 (`HOME_RECENT` 6).
    pub fn recent(mut self, recent: &'a [RecentFile]) -> Self {
        self.recent = recent;
        self
    }

    /// At most `n` recent files show (12 by default).
    pub fn max_recent(mut self, n: usize) -> Self {
        self.max_recent = n;
        self
    }

    /// The pinned folders, in the host's order. The Pinned Folders section shows once the host
    /// calls this, with its Pin Folder… button even when the list is empty; a host that cannot
    /// pin (a web build) leaves the call out.
    // pdfcraft: crates/ui-egui/src/home/folders_ui.rs:15 (`MAX_PINNED` 8), :51 `PinnedFolders`.
    pub fn pinned(mut self, folders: &'a [PathBuf]) -> Self {
        self.pinned = Some(folders);
        self
    }

    /// The template cards; the section is left out when there are none.
    // figcraft: crates/ui-egui/src/home.rs:23 `Section` (Recents, Online, Templates).
    pub fn templates(mut self, templates: &'a [Template]) -> Self {
        self.templates = templates;
        self
    }

    /// The view and sort the host stored ([`StartAction::ViewChanged`] reports a change).
    pub fn view(mut self, view: RecentView, sort: RecentSort) -> Self {
        self.view = view;
        self.sort = sort;
        self
    }

    /// The time the "2 days ago" lines count from, in seconds since the Unix epoch. The system
    /// clock by default (no date lines on the web, which has no recent paths).
    pub fn now(mut self, seconds: i64) -> Self {
        self.now = Some(seconds);
        self
    }

    /// Translated labels; default the installed words.
    pub fn labels(mut self, labels: StartLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the page in `ui`'s available rect and returns this frame's actions.
    // pc: crates/ui-egui/src/canvas/start_screen.rs:13 `start_screen` (New/Open buttons, recent rows,
    // hover ×, context menu :157-170); figcraft: crates/ui-egui/src/home.rs:54 `Home`; pdfcraft:
    // crates/ui-egui/src/home/folders_ui.rs:199 `pin_folder`, :220 `unpin_folder`.
    pub fn show(self, ui: &mut Ui) -> Vec<StartAction> {
        let labels = self.labels.clone().unwrap_or_else(|| StartLabels::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let now = self.now.or_else(unix_now);
        let numbers = LocaleNumbers::of(ui);
        let age = |then: Option<i64>| match (now, then) {
            (Some(now), Some(then)) => age_text(&labels, now.saturating_sub(then), |n| numbers.count(n)),
            _ => String::new(),
        };
        let mut recent: Vec<RecentFile> = self.recent.iter().take(self.max_recent).cloned().collect();
        sort_recent(&mut recent, self.sort);
        let actions = RefCell::new(Vec::new());
        let view = Cell::new(self.view);
        let sort = Cell::new(self.sort);
        let page = UiBuilder::new().id_salt(id).accessibility_role(Role::Group).accessibility_label(labels.page.clone());
        ui.scope_builder(page, |ui| {
            ScrollArea::vertical().id_salt(id.with("page")).auto_shrink([false, false]).show(ui, |ui| {
                Stack::new(Gap::Section).pad(Pad::Panel).show(ui, |ui| {
                    Row::new(Gap::Inline).show(ui, |ui| {
                        if ui.add(Button::new(labels.new.as_str()).variant(ButtonVariant::Primary)).clicked() {
                            actions.borrow_mut().push(StartAction::New);
                        }
                        if ui.add(Button::new(labels.open.as_str())).clicked() {
                            actions.borrow_mut().push(StartAction::Open);
                        }
                    });
                    SectionHeader::new(id.with("recent"), labels.recent.as_str())
                        .collapsible(false)
                        .controls(|ui| {
                            let (mut v, mut s) = (view.get(), sort.get());
                            Segmented::new(id.with("view"), labels.view.as_str(), &mut v)
                                .icon_item(RecentView::Grid, Icon::LayoutGrid, labels.grid.as_str())
                                .icon_item(RecentView::List, Icon::List, labels.list.as_str())
                                .show(ui);
                            let _ = Select::new(id.with("sort"), labels.sort.as_str(), &mut s)
                                .option(RecentSort::Opened, labels.sort_opened.as_str())
                                .option(RecentSort::Name, labels.sort_name.as_str())
                                .option(RecentSort::Created, labels.sort_created.as_str())
                                .show(ui);
                            view.set(v);
                            sort.set(s);
                        })
                        .show(ui, |ui| {
                            let mut hits = Vec::new();
                            if recent.is_empty() {
                                EmptyState::new(labels.no_recent.as_str()).show(ui);
                            } else if view.get() == RecentView::Grid {
                                let cards: Vec<Card> = recent.iter().map(|f| recent_card(f, &age(f.opened))).collect();
                                hits = card_grid(ui, id.with("recent-grid"), &labels, &cards, true);
                            } else {
                                let rows: Vec<FileRow> = recent.iter().map(|f| recent_row(f, &age(f.opened))).collect();
                                hits = file_list(ui, id.with("recent-list"), &labels, labels.recent.as_str(), &rows, true);
                            }
                            actions.borrow_mut().extend(hits.into_iter().map(|hit| match hit {
                                Hit::Open(i) => StartAction::OpenRecent(recent[i].path.clone()),
                                Hit::Remove(i) => StartAction::RemoveRecent(recent[i].path.clone()),
                                Hit::Reveal(i) => StartAction::RevealRecent(recent[i].path.clone()),
                                Hit::Clear => StartAction::ClearRecent,
                            }));
                        });
                    if let Some(pinned) = self.pinned {
                        SectionHeader::new(id.with("pinned"), labels.pinned.as_str())
                            .collapsible(false)
                            .controls(|ui| {
                                if ui.add(Button::new(labels.pin_folder.as_str()).icon(Icon::NewFolder)).clicked() {
                                    actions.borrow_mut().push(StartAction::PinFolder);
                                }
                            })
                            .show(ui, |ui| {
                                let rows: Vec<FileRow> = pinned.iter().map(|p| pinned_row(p)).collect();
                                let hits = file_list(ui, id.with("pinned-list"), &labels, labels.pinned.as_str(), &rows, false);
                                actions.borrow_mut().extend(hits.into_iter().filter_map(|hit| match hit {
                                    Hit::Open(i) => Some(StartAction::OpenPinned(pinned[i].clone())),
                                    Hit::Remove(i) => Some(StartAction::UnpinFolder(pinned[i].clone())),
                                    _ => None,
                                }));
                            });
                    }
                    if !self.templates.is_empty() {
                        SectionHeader::new(id.with("templates"), labels.templates.as_str()).collapsible(false).show(ui, |ui| {
                            let cards: Vec<Card> = self.templates.iter().map(template_card).collect();
                            let hits = card_grid(ui, id.with("template-grid"), &labels, &cards, false);
                            actions.borrow_mut().extend(hits.into_iter().filter_map(|hit| match hit {
                                Hit::Open(i) => Some(StartAction::UseTemplate(self.templates[i].id.clone())),
                                _ => None,
                            }));
                        });
                    }
                });
            });
        });
        let mut actions = actions.into_inner();
        if (view.get(), sort.get()) != (self.view, self.sort) {
            actions.push(StartAction::ViewChanged(view.get(), sort.get()));
        }
        actions
    }
}

/// What a card or row of a section reported, by position in the section.
enum Hit {
    Open(usize),
    Remove(usize),
    Reveal(usize),
    Clear,
}

/// The commands of a recent file's context menu, and of a pinned folder's (`Remove` is Unpin).
#[derive(Clone, Copy, PartialEq)]
enum Command {
    Remove,
    Clear,
    Reveal,
}

fn menu_entries(labels: &StartLabels, recent: bool) -> Vec<MenuEntry<Command>> {
    if recent {
        vec![
            MenuItem::new(labels.remove_recent.as_str(), Command::Remove).into(),
            MenuItem::new(labels.clear_recent.as_str(), Command::Clear).into(),
            MenuItem::new(labels.reveal.as_str(), Command::Reveal).into(),
        ]
    } else {
        vec![MenuItem::new(labels.unpin.as_str(), Command::Remove).into()]
    }
}

fn hit_of(command: Command, index: usize) -> Hit {
    match command {
        Command::Remove => Hit::Remove(index),
        Command::Clear => Hit::Clear,
        Command::Reveal => Hit::Reveal(index),
    }
}

/// One card of the grid.
struct Card {
    /// Keeps the card's id when the order changes.
    key: String,
    name: String,
    /// The second footer line.
    detail: String,
    /// What a screen reader adds to the name.
    description: String,
    tooltip: String,
    thumbnail: Option<TextureId>,
}

fn folder_of(path: &Path) -> String {
    path.parent().map(|p| p.display().to_string()).unwrap_or_default()
}

fn recent_card(file: &RecentFile, age: &str) -> Card {
    let folder = folder_of(&file.path);
    let description = [folder.as_str(), age].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(", ");
    Card {
        key: file.path.display().to_string(),
        name: file.name.clone(),
        detail: age.to_owned(),
        description,
        tooltip: file.path.display().to_string(),
        thumbnail: file.thumbnail,
    }
}

fn template_card(template: &Template) -> Card {
    Card {
        key: template.id.clone(),
        name: template.name.clone(),
        detail: template.detail.clone(),
        description: template.detail.clone(),
        tooltip: String::new(),
        thumbnail: template.thumbnail,
    }
}

/// Shows `text` on one line, ending in an ellipsis where it does not fit `width`.
fn one_line(ui: &Ui, text: &str, format: &egui::TextFormat, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), format.clone());
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    ui.painter().layout_job(job)
}

/// The cards in as many columns as keep each between the least and greatest width. The cards are
/// one grid for the keyboard: one Tab stop, the arrows, Home and End move between cards.
fn card_grid(ui: &mut Ui, id: Id, labels: &StartLabels, cards: &[Card], recent: bool) -> Vec<Hit> {
    let tokens = Tokens::of(ui);
    let rtl = direction::is_rtl(ui);
    let gap = Gap::Section.points(&tokens);
    let inset = Pad::Control.points(&tokens);
    let name_format = Text::format(&tokens, TypeRole::BodyStrong, Content::Normal);
    let detail_format = Text::format(&tokens, TypeRole::Caption, Content::Muted);
    let line = |format: &egui::TextFormat| format.line_height.unwrap_or(format.font_id.size);
    let width = ui.available_width();
    let columns = GridNav::columns(width, 0.0, gap, CARD_MIN);
    let card_w = ((width - gap * (columns - 1) as f32) / columns as f32).min(CARD_MAX);
    let thumb_w = card_w - 2.0 * inset;
    let thumb_h = (thumb_w * 9.0 / 16.0).round();
    let card_h = inset + thumb_h + inset + line(&name_format) + line(&detail_format) + inset;
    let rows = cards.len().div_ceil(columns);
    let (_, area) = ui.allocate_space(vec2(width, rows as f32 * card_h + rows.saturating_sub(1) as f32 * gap));
    let ids: Vec<Id> = cards.iter().map(|c| id.with(&c.key)).collect();
    let stop = GridNav::roving_stop(ui, &ids, None, true);
    let entries = menu_entries(labels, true);
    let mut hits = Vec::new();
    for (i, card) in cards.iter().enumerate() {
        let (row, column) = (i / columns, i % columns);
        let offset = column as f32 * (card_w + gap);
        let left = if rtl { area.right() - offset - card_w } else { area.left() + offset };
        let rect = Rect::from_min_size(pos2(left, area.top() + row as f32 * (card_h + gap)), vec2(card_w, card_h));
        let mut element = Interactive::with_id(ids[i], Role::Button, card.name.as_str()).surface(SurfaceKind::Raised).corner(Corner::Control);
        if i != stop {
            element = element.sense(Sense::CLICK);
        }
        let response = element.interact_at(ui, rect).finish(ui);
        ui.ctx().accesskit_node_builder(response.id, |node| {
            if !card.description.is_empty() {
                node.set_description(card.description.clone());
            }
            if i != stop {
                node.add_action(Action::Focus);
            }
        });
        if response.gained_focus() {
            ui.scroll_to_rect(rect, None);
        }
        let response = if card.tooltip.is_empty() { response } else { response.on_hover_text(card.tooltip.as_str()) };
        if ui.is_rect_visible(rect) {
            let thumb = Rect::from_min_size(rect.min + vec2(inset, inset), vec2(thumb_w, thumb_h));
            let thumb = if rtl { Rect::from_min_size(pos2(rect.right() - inset - thumb_w, thumb.top()), thumb.size()) } else { thumb };
            Surface::paint(ui.painter(), thumb, SurfaceKind::Sunken, &tokens);
            match card.thumbnail {
                Some(texture) => egui::Image::new(egui::load::SizedTexture::new(texture, thumb.size()))
                    .corner_radius(Corner::Control.radius(&tokens))
                    .paint_at(ui, thumb),
                None => icons::paint(ui, thumb, Icon::File, IconSize::Lg, Content::Faint.color(&tokens).to_egui()),
            }
            let text_width = card_w - 2.0 * inset;
            let text_top = thumb.bottom() + inset;
            let text = |text: &str, format: &egui::TextFormat, top: f32| {
                let galley = one_line(ui, text, format, text_width);
                let x = if rtl { rect.right() - inset - galley.size().x } else { rect.left() + inset };
                ui.painter().galley(pos2(x, top), galley, format.color);
            };
            text(&card.name, &name_format, text_top);
            if !card.detail.is_empty() {
                text(&card.detail, &detail_format, text_top + line(&name_format));
            }
        }
        let menu = recent.then(|| ContextMenu::new(&entries).show(ui, &response));
        if let Some(command) = menu.and_then(|m| m.chosen) {
            hits.push(hit_of(command, i));
        }
        let menu_open = recent && ContextMenu::is_open(ui.ctx(), &response);
        let mut removed = false;
        if recent && (ui.rect_contains_pointer(rect) || response.has_focus()) && !menu_open {
            let side = IconButton::style(&tokens, ControlSize::Sm).side;
            let x = if rtl { rect.left() + inset + side / 2.0 } else { rect.right() - inset - side / 2.0 };
            let corner = pos2(x, rect.top() + inset + side / 2.0);
            let close = IconButton::new(Icon::Close, labels.remove_recent.as_str())
                .size(ControlSize::Sm)
                .id_salt((ids[i], "remove"))
                .focusable(false)
                .show_at(ui, Rect::from_center_size(corner, vec2(side, side)));
            removed = close.clicked();
        }
        let delete = recent && response.has_focus() && !menu_open && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete));
        if removed || delete {
            hits.push(Hit::Remove(i));
        } else if response.clicked() {
            hits.push(Hit::Open(i));
        }
    }
    hits
}

/// One row of a [`file_list`].
struct FileRow {
    path: PathBuf,
    name: String,
    folder: String,
    /// The date at the end of the row.
    age: String,
    description: String,
    thumbnail: Option<TextureId>,
    icon: Icon,
}

fn recent_row(file: &RecentFile, age: &str) -> FileRow {
    let folder = folder_of(&file.path);
    let description = [folder.as_str(), age].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(", ");
    FileRow {
        path: file.path.clone(),
        name: file.name.clone(),
        folder,
        age: age.to_owned(),
        description,
        thumbnail: file.thumbnail,
        icon: Icon::File,
    }
}

fn pinned_row(folder: &Path) -> FileRow {
    let name = folder.file_name().map_or_else(|| folder.display().to_string(), |n| n.to_string_lossy().into_owned());
    FileRow {
        path: folder.to_path_buf(),
        name,
        folder: folder.display().to_string(),
        age: String::new(),
        description: folder.display().to_string(),
        thumbnail: None,
        icon: Icon::Folder,
    }
}

/// The rows as a [`List`] sized to its rows, so the page scrolls rather than the list: Enter or a
/// click opens, the context menu key opens the menu, Delete removes.
fn file_list(ui: &mut Ui, id: Id, labels: &StartLabels, name: &str, rows: &[FileRow], recent: bool) -> Vec<Hit> {
    let tokens = Tokens::of(ui);
    let order: Vec<RowInfo<PathBuf>> = rows.iter().map(|r| RowInfo::leaf(r.path.clone())).collect();
    let height = ListRow::<PathBuf>::style(&tokens).height(ThumbSize::Small, true);
    let selection_id = id.with("selection");
    let mut selection: Selection<PathBuf> = ui.data(|d| d.get_temp(selection_id)).unwrap_or_default();
    let age_width = rows
        .iter()
        .map(|r| {
            ui.painter()
                .layout_no_wrap(r.age.clone(), Text::format(&tokens, TypeRole::Caption, Content::Muted).font_id, egui::Color32::WHITE)
                .size()
                .x
        })
        .fold(0.0, f32::max);
    let output = List::new(id, name, order.as_slice())
        .selection(&mut selection)
        .thumbnails(ThumbSize::Small)
        .detail_lines(true)
        .max_height(height * rows.len() as f32)
        .show(ui, |_, key| {
            let Some(row) = rows.iter().find(|r| &r.path == key) else {
                return ListRow::new(key.clone(), String::new());
            };
            let mut list_row = ListRow::new(key.clone(), row.name.as_str()).description(row.description.as_str()).detail(row.folder.as_str());
            list_row = match row.thumbnail {
                Some(texture) => list_row.thumb(ThumbSize::Small, move |painter, rect| {
                    painter.image(texture, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), egui::Color32::WHITE);
                }),
                None => list_row.icon(row.icon),
            };
            if !row.age.is_empty() {
                let age = row.age.clone();
                list_row = list_row.trailing_item(age_width, move |ui| {
                    ui.add(Text::new(age).role(TypeRole::Caption).content(Content::Muted).truncate());
                });
            }
            list_row
        });
    ui.data_mut(|d| d.insert_temp(selection_id, selection.clone()));
    let position = |key: &PathBuf| rows.iter().position(|r| &r.path == key);
    let mut hits = Vec::new();
    let target_id = id.with("menu-target");
    let mut request = None;
    for event in output.events {
        match event {
            ListEvent::Clicked(key) | ListEvent::Activate(key) => {
                if let Some(i) = position(&key).filter(|i| !hits.iter().any(|h| matches!(h, Hit::Open(j) if j == i))) {
                    hits.push(Hit::Open(i));
                }
            }
            ListEvent::ContextMenu(menu) => {
                if let Some(key) = menu.row {
                    ui.data_mut(|d| d.insert_temp(target_id, key));
                    request = Some(match menu.pointer {
                        Some(pointer) => OpenAt::Pointer(pointer),
                        None => OpenAt::Rect(menu.anchor),
                    });
                }
            }
            _ => {}
        }
    }
    let entries = menu_entries(labels, recent);
    let chosen = ContextMenu::new(&entries).own_gestures(false).open_at(request).show(ui, &output.response).chosen;
    let target: Option<PathBuf> = ui.data(|d| d.get_temp(target_id));
    if let (Some(command), Some(i)) = (chosen, target.as_ref().and_then(position)) {
        hits.push(hit_of(command, i));
    }
    let menu_open = ContextMenu::is_open(ui.ctx(), &output.response);
    if output.response.has_focus()
        && !menu_open
        && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Delete))
        && let Some(i) = selection.lead.as_ref().and_then(position)
    {
        hits.push(Hit::Remove(i));
    }
    hits
}

/// Now, in seconds since the Unix epoch; `None` where the platform has no clock.
fn unix_now() -> Option<i64> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).ok().and_then(|d| i64::try_from(d.as_secs()).ok())
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

/// How long ago `seconds` was: "less than a minute ago", then whole minutes, hours, days, months
/// (30 days) and years.
fn age_text(labels: &StartLabels, seconds: i64, number: impl Fn(u64) -> String) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const MONTH: u64 = 30 * DAY;
    const YEAR: u64 = 365 * DAY;
    let seconds = seconds.max(0).unsigned_abs();
    let (unit, one, other) = match seconds {
        0..MINUTE => return labels.just_now.clone(),
        MINUTE..HOUR => (MINUTE, &labels.minute_one, &labels.minute_other),
        HOUR..DAY => (HOUR, &labels.hour_one, &labels.hour_other),
        DAY..MONTH => (DAY, &labels.day_one, &labels.day_other),
        MONTH..YEAR => (MONTH, &labels.month_one, &labels.month_other),
        _ => (YEAR, &labels.year_one, &labels.year_other),
    };
    let n = seconds / unit;
    craft_i18n::fmt(if n == 1 { one } else { other }, &[("n", &number(n))])
}

/// The recent files in `sort` order (Opened: newest first; Name: by the case- and accent-folded
/// name; Created: newest first, files without a date last).
// figcraft: crates/ui-egui/src/home/home_recents.rs:79 `SortKey`.
pub fn sort_recent(files: &mut [RecentFile], sort: RecentSort) {
    fn newest_first(a: Option<i64>, b: Option<i64>) -> Ordering {
        match (a, b) {
            (Some(a), Some(b)) => b.cmp(&a),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
    match sort {
        RecentSort::Opened => files.sort_by(|a, b| newest_first(a.opened, b.opened)),
        RecentSort::Created => files.sort_by(|a, b| newest_first(a.created, b.created)),
        RecentSort::Name => {
            let folder = Folder::new("");
            files.sort_by_cached_key(|f| (folder.key(&f.name), f.name.clone(), f.path.clone()));
        }
    }
}

crate::words! {
    /// The start screen's labels, English by default.
    #[non_exhaustive]
    pub struct StartLabels {
        page: "start.page" = "Home";
        new: "start.new" = "New…";
        open: "start.open" = "Open…";
        recent: "start.recent" = "Recent";
        no_recent: "start.noRecent" = "Files you open show here.";
        view: "start.view" = "View";
        grid: "start.grid" = "Grid";
        list: "start.list" = "List";
        sort: "start.sort" = "Sort";
        sort_opened: "start.sortOpened" = "Last Opened";
        sort_name: "start.sortName" = "Name";
        sort_created: "start.sortCreated" = "Created";
        remove_recent: "start.removeRecent" = "Remove from Recent";
        clear_recent: "start.clearRecent" = "Clear Recent Files";
        reveal: "start.reveal" = "Show in Folder";
        pinned: "start.pinned" = "Pinned Folders";
        pin_folder: "start.pinFolder" = "Pin Folder…";
        unpin: "start.unpin" = "Unpin";
        templates: "start.templates" = "Templates";
        just_now: "start.justNow" = "less than a minute ago";
        minute_one: "start.minuteOne" = "{n} minute ago";
        minute_other: "start.minuteOther" = "{n} minutes ago";
        hour_one: "start.hourOne" = "{n} hour ago";
        hour_other: "start.hourOther" = "{n} hours ago";
        day_one: "start.dayOne" = "{n} day ago";
        day_other: "start.dayOther" = "{n} days ago";
        month_one: "start.monthOne" = "{n} month ago";
        month_other: "start.monthOther" = "{n} months ago";
        year_one: "start.yearOne" = "{n} year ago";
        year_other: "start.yearOther" = "{n} years ago";
    }
}
