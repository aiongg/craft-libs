#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`FileListPicker`]: a list of chosen files (or one folder) with Browse, Remove and the other
//! list buttons, inside a dialog that loads several files.

use std::path::{Path, PathBuf};

use craft_host::{PickId, PickRequest};
use egui::{AsIdSalt, IdSalt, Ui};

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
        todo!()
    }

    /// Whether OK may run: Files mode needs a file, Folder mode a folder.
    // pc: crates/ui-egui/src/dialogs/stack_dialog.rs:86 can_confirm
    pub fn is_valid(&self) -> bool {
        todo!()
    }

    /// Why OK can't run, for the dialog's message: "Add the files to load" or "Choose a folder
    /// to load"; `None` when [`is_valid`](Self::is_valid).
    // pc: crates/ui-egui/src/dialogs/stack_dialog.rs:213 confirm
    pub fn problem<'l>(&self, labels: &'l FileListLabels) -> Option<&'l str> {
        todo!()
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
        todo!()
    }
}
