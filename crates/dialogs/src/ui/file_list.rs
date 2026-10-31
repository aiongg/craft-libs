//! [`FileListPicker`]: a list of chosen files (or one folder) with Browse, Remove and the other
//! list buttons, inside a dialog that loads several files.

use std::path::{Path, PathBuf};

use craft_host::{Host, PickId, PickKind, PickRequest, Picked};
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::controls::{Button, Select};
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::Labels;
use craft_ui::primitives::{Content, Gap, Surface, SurfaceKind, Text};
use craft_ui::theme::{Tokens, TokensOf};
use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Key, Modifiers, ScrollArea, Ui, UiBuilder};

craft_ui::words! {
    /// The picker's words, English by default.
    #[non_exhaustive]
    pub struct FileListLabels {
        use_label: "fileList.use" = "Use:";
        files: "fileList.files" = "Files";
        folder: "fileList.folder" = "Folder";
        browse: "fileList.browse" = "Browse…";
        remove: "fileList.remove" = "Remove";
        add_open_files: "fileList.addOpenFiles" = "Add Open Files";
        sort_by_name: "fileList.sortByName" = "Sort By Name";
        add_files: "fileList.addFiles" = "Add the files to load";
        choose_folder: "fileList.chooseFolder" = "Choose a folder to load";
    }
}

/// Files picked one by one, or one folder whose files the app lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FileListMode {
    #[default]
    Files,
    Folder,
}

/// The picker's value; the dialog keeps it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileList {
    pub mode: FileListMode,
    /// The chosen files in Files mode, in the user's order, each once.
    pub files: Vec<PathBuf>,
    /// The folder in Folder mode.
    pub folder: Option<PathBuf>,
    /// The selected row in Files mode.
    pub selected: Option<usize>,
}

impl FileList {
    /// Adds paths not in the list yet, in order (Browse, Add Open Files, a drop).
    // pc: crates/ui-egui/src/dialogs/stack_dialog.rs:66 add
    pub fn add(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        for path in paths {
            if !self.files.contains(&path) {
                self.files.push(path);
            }
        }
    }

    /// Whether OK may run: Files mode needs a file, Folder mode a folder.
    // pc: crates/ui-egui/src/dialogs/stack_dialog.rs:86 can_confirm
    pub fn is_valid(&self) -> bool {
        match self.mode {
            FileListMode::Files => !self.files.is_empty(),
            FileListMode::Folder => self.folder.as_ref().is_some_and(|folder| !folder.as_os_str().is_empty()),
        }
    }

    /// Why OK can't run, for the dialog's message: "Add the files to load" or "Choose a folder
    /// to load"; `None` when [`is_valid`](Self::is_valid).
    // pc: crates/ui-egui/src/dialogs/stack_dialog.rs:213 confirm
    pub fn problem<'l>(&self, labels: &'l FileListLabels) -> Option<&'l str> {
        if self.is_valid() {
            return None;
        }
        Some(match self.mode {
            FileListMode::Files => labels.add_files.as_str(),
            FileListMode::Folder => labels.choose_folder.as_str(),
        })
    }
}

/// The second line of a file's row.
type InfoLine<'a> = &'a dyn Fn(&Path) -> Option<String>;

/// What the picker reported this pass.
#[derive(Debug, Default)]
pub struct FileListResponse {
    /// The list, mode or folder changed.
    pub changed: bool,
    /// Browse queued a picker; the answer fills the list on a later pass.
    pub browsing: Option<PickId>,
}

/// Photoshop's Load Files into Stack file list, after PhotoCraft's port of it.
///
/// The user sees "Use:" with Files and Folder (when [`folder_mode`](Self::folder_mode) is on), the
/// list box (300 × 160; one row per file showing its name, the full path as tooltip, and the
/// app's info line under the name when given, VectorCraft's Place list), and the buttons beside
/// it: Browse… (an open-many picker in Files mode, a folder picker in Folder mode, both through
/// `host.files()`), Remove (with a row selected), Add Open Files (when the app passed open
/// documents with a path) and Sort By Name (more than one file; sorts by name ignoring case).
/// In Folder mode the list shows the folder's files the app passes and has no selection.
///
/// Keys: the list is one Tab stop; arrows move the selection; Delete and Backspace remove the
/// selected file (PdfCraft's Combine list); Alt+↑ and Alt+↓ move it. Dropped files are added by
/// the dialog with [`FileList::add`].
///
/// ```ignore
/// let r = FileListPicker::new("stack-files", &mut self.list).folder_mode(true).open_files(&open_paths).show(ui);
/// ok_enabled = self.list.is_valid();
/// ```
pub struct FileListPicker<'a> {
    id_salt: IdSalt,
    list: &'a mut FileList,
    request: PickRequest,
    folder_mode: bool,
    open_files: &'a [PathBuf],
    folder_files: &'a [PathBuf],
    info: Option<InfoLine<'a>>,
}

impl<'a> FileListPicker<'a> {
    /// The picker over `list`; Browse asks with `PickRequest::open_many("stack")` until
    /// [`request`](Self::request) replaces it.
    pub fn new(id_salt: impl AsIdSalt, list: &'a mut FileList) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            list,
            request: PickRequest::open_many("stack"),
            folder_mode: false,
            open_files: &[],
            folder_files: &[],
            info: None,
        }
    }

    /// Browse's picker in Files mode (purpose, filters); Folder mode asks for a folder with the
    /// same purpose.
    pub fn request(mut self, request: PickRequest) -> Self {
        self.request = request;
        self
    }

    /// Offers Folder mode.
    pub fn folder_mode(mut self, on: bool) -> Self {
        self.folder_mode = on;
        self
    }

    /// The open documents' paths, for Add Open Files.
    pub fn open_files(mut self, paths: &'a [PathBuf]) -> Self {
        self.open_files = paths;
        self
    }

    /// The files the app found in the chosen folder, shown in Folder mode.
    pub fn folder_files(mut self, paths: &'a [PathBuf]) -> Self {
        self.folder_files = paths;
        self
    }

    /// A second line per file ("1200 × 800 px, 300 ppi, RGB"); `None` for no line.
    pub fn info(mut self, info: InfoLine<'a>) -> Self {
        self.info = Some(info);
        self
    }

    /// Takes its own pick answers from `Host::of(ui).files()`; returns any other answer.
    // pc: crates/ui-egui/src/dialogs/stack_dialog.rs:114 body, :92 browse, :81 open_files;
    // vectorcraft: crates/ui-egui/src/dialogs/place.rs (info lines); pdfcraft: crates/ui-egui/src/workflows/combine_ui/view.rs:470 keyboard
    pub fn show(self, ui: &mut Ui) -> FileListResponse {
        let Self { id_salt, list, request, folder_mode, open_files, folder_files, info } = self;
        let host = Host::of(ui);
        let labels = FileListLabels::of(ui);
        let tokens = Tokens::of(ui);
        let ticket_key = ui.scope_id().with((id_salt, "ticket"));
        let enabled = ui.is_enabled();
        let mut response = FileListResponse::default();

        // The answer of this picker's Browse fills the list before it is drawn.
        if let Some(ticket) = ui.data(|d| d.get_temp::<PickId>(ticket_key))
            && let Some(answer) = host.files().take_pick()
        {
            if answer.id == ticket {
                ui.data_mut(|d| d.remove::<PickId>(ticket_key));
                let paths: Vec<PathBuf> = match answer.picked {
                    Picked::Paths(paths) => paths,
                    Picked::Contents(files) => files.into_iter().map(|(name, _)| PathBuf::from(name)).collect(),
                    Picked::Cancelled => Vec::new(),
                };
                if answer.request.kind == PickKind::Folder {
                    if let Some(folder) = paths.into_iter().next() {
                        list.folder = Some(folder);
                        response.changed = true;
                    }
                } else if !paths.is_empty() {
                    let before = list.files.len();
                    list.add(paths);
                    response.changed |= list.files.len() != before;
                }
            } else {
                host.files().return_pick(answer);
            }
        }
        if list.selected.is_some_and(|at| at >= list.files.len()) {
            list.selected = None;
        }
        let folder = list.mode == FileListMode::Folder;

        if folder_mode {
            ui.horizontal(|ui| {
                ui.add(Text::new(labels.use_label.as_str()));
                let mut mode = list.mode;
                let _ = Select::new((id_salt, "use"), labels.use_label.as_str(), &mut mode)
                    .option(FileListMode::Files, labels.files.as_str())
                    .option(FileListMode::Folder, labels.folder.as_str())
                    .show(ui);
                if mode != list.mode {
                    list.mode = mode;
                    list.selected = None;
                    response.changed = true;
                }
            });
        }

        let mut browse = false;
        let mut remove = false;
        let mut add_open = false;
        let mut sort = false;
        let gap = Gap::Stack.points(&tokens);
        let list_size = egui::Vec2::from(tokens.component.file_list.size);
        let button_min = tokens.component.file_list.button_min;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
                ui.allocate_ui(list_size, |ui| {
                    ui.set_min_size(list_size);
                    ui.set_max_width(list_size.x);
                    if folder {
                        folder_rows(ui, &labels, list.folder.as_deref(), folder_files, info, list_size.y);
                    } else {
                        let moved = file_rows(ui, id_salt, &labels, list, info, list_size.y);
                        remove = moved.remove;
                        response.changed |= moved.changed;
                    }
                });
            });

            let can_remove = !folder && list.selected.is_some();
            let can_sort = !folder && list.files.len() > 1;
            let can_add_open = !folder && !open_files.is_empty();
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = Gap::Row.points(&tokens);
                let button = |ui: &mut Ui, label: &str, on: bool| {
                    ui.add(Button::new(label).id_salt((id_salt, label)).min_width(button_min).enabled(on && enabled)).clicked()
                };
                browse = button(ui, &labels.browse, true);
                remove |= button(ui, &labels.remove, can_remove);
                add_open = button(ui, &labels.add_open_files, can_add_open);
                sort = button(ui, &labels.sort_by_name, can_sort);
            });
        });

        if remove && let Some(at) = list.selected.filter(|&at| at < list.files.len()) {
            list.files.remove(at);
            list.selected = None;
            response.changed = true;
        }
        if add_open {
            let before = list.files.len();
            list.add(open_files.iter().cloned());
            response.changed |= list.files.len() != before;
        }
        if sort {
            list.files.sort_by_key(|path| file_name(path).to_lowercase());
            list.selected = None;
            response.changed = true;
        }
        if browse {
            let request = if folder { PickRequest::folder(request.purpose.clone()) } else { request };
            let id = host.files().pick(request);
            ui.data_mut(|d| d.insert_temp(ticket_key, id));
            response.browsing = Some(id);
        }
        response
    }
}

/// What the Files list reported besides its selection.
struct RowEdits {
    remove: bool,
    changed: bool,
}

/// The Files list: one Tab stop, arrows select, Delete and Backspace remove the selected file,
/// Alt+↑ and Alt+↓ move it.
fn file_rows(ui: &mut Ui, id_salt: IdSalt, labels: &FileListLabels, list: &mut FileList, info: Option<InfoLine<'_>>, height: f32) -> RowEdits {
    let mut edits = RowEdits { remove: false, changed: false };
    let rows: Vec<RowInfo<usize>> = (0..list.files.len()).map(RowInfo::leaf).collect();
    let mut selection = list.selected.map_or_else(Selection::new, Selection::only);
    let files = &list.files;
    let output = List::new((id_salt, "files"), labels.files.as_str(), rows.as_slice())
        .selection(&mut selection)
        .mode(SelectionMode::Single)
        .detail_lines(info.is_some())
        .max_height(height)
        .show(ui, |_, &at| file_row(at, &files[at], info));
    for event in &output.events {
        if let ListEvent::Selection(change) = event {
            list.selected = change.selection.items.first().copied();
        }
    }
    if output.response.has_focus()
        && ui.is_enabled()
        && let Some(at) = list.selected
    {
        let (delete, up, down) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Delete) || i.consume_key(Modifiers::NONE, Key::Backspace),
                i.consume_key(Modifiers::ALT, Key::ArrowUp),
                i.consume_key(Modifiers::ALT, Key::ArrowDown),
            )
        });
        if delete {
            edits.remove = true;
        } else if up && at > 0 {
            list.files.swap(at, at - 1);
            list.selected = Some(at - 1);
            edits.changed = true;
        } else if down && at + 1 < list.files.len() {
            list.files.swap(at, at + 1);
            list.selected = Some(at + 1);
            edits.changed = true;
        }
    }
    edits
}

/// One file's row: its name, the path for a screen reader, the app's info line under the name.
fn file_row<'a>(at: usize, path: &Path, info: Option<InfoLine<'_>>) -> ListRow<'a, usize> {
    let name = file_name(path);
    let line = info.and_then(|info| info(path));
    let accessible = match &line {
        Some(line) => format!("{name}, {line}"),
        None => name.clone(),
    };
    let row = ListRow::new(at, name).accessible_name(accessible).description(path.to_string_lossy()).tooltip(path.to_string_lossy());
    match line {
        Some(line) => row.detail(line),
        None => row,
    }
}

/// The Folder list: the folder's path, then the files the app found there. No selection.
fn folder_rows(ui: &mut Ui, labels: &FileListLabels, folder: Option<&Path>, files: &[PathBuf], info: Option<InfoLine<'_>>, height: f32) {
    let builder = UiBuilder::new().accessibility_role(Role::List).accessibility_label(labels.files.clone());
    ui.scope_builder(builder, |ui| {
        ScrollArea::vertical().id_salt("folder-files").max_height(height).auto_shrink([false, false]).show(ui, |ui| {
            if let Some(folder) = folder {
                ui.add(Text::new(folder.to_string_lossy()).content(Content::Muted).truncate());
            }
            for (at, path) in files.iter().enumerate() {
                let _ = file_row(at, path, info).role(Role::ListItem).show(ui);
            }
        });
    });
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.to_string_lossy(), |name| name.to_string_lossy()).into_owned()
}
