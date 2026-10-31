//! [`LutPicker`] (F04.04): the 3D LUT choice of a Color Lookup adjustment, as PhotoCraft's: Load
//! 3D LUT…, None, the built-in looks ([`craft_cms::lutfile::BUILTIN`]), the files of the app's
//! LUT folder, and the loaded LUT.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use egui::{AsIdSalt, Context, Id, IdSalt, Modifiers, Ui};

use craft_ui::controls::Select;
use craft_ui::edit::{EditEvent, EditResponse, GestureId};
use craft_ui::localized::{Labels as _, Word, Words};

craft_ui::words! {
    /// The LUT picker's rows, English by default. The built-in looks' names are [`LUT_WORDS`].
    #[non_exhaustive]
    pub struct LutLabels {
        label: "lut.label" = "3D LUT File";
        load: "lut.load" = "Load 3D LUT…";
        none: "lut.none" = "None";
        /// The loaded LUT's row; `{name}` its title, `{size}` its grid size ("Film (33³)").
        custom: "lut.custom" = "{name} ({size}³)";
    }
}

/// The built-in looks' names as words, keyed `lut.builtin.<id>` with
/// [`BUILTIN`](craft_cms::lutfile::BUILTIN)'s English, in its order.
pub const LUT_WORDS: &[craft_ui::localized::Word] = &[
    craft_ui::localized::Word::keyed("lut.builtin.warm", "Warm Filter"),
    craft_ui::localized::Word::keyed("lut.builtin.cool", "Cool Filter"),
    craft_ui::localized::Word::keyed("lut.builtin.tealOrange", "Teal & Orange"),
    craft_ui::localized::Word::keyed("lut.builtin.bleachBypass", "Bleach Bypass"),
    craft_ui::localized::Word::keyed("lut.builtin.fadedFilm", "Faded Film"),
    craft_ui::localized::Word::keyed("lut.builtin.dayForNight", "Day for Night"),
    craft_ui::localized::Word::keyed("lut.builtin.monoContrast", "Mono High Contrast"),
    craft_ui::localized::Word::keyed("lut.builtin.crossProcess", "Cross Process"),
];

/// The chosen LUT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LutChoice {
    None,
    /// A built-in look by id ([`craft_cms::lutfile::builtin`]).
    Builtin(&'static str),
    /// A file of the LUT folder; the app reads and embeds it.
    Folder(PathBuf),
    /// The LUT the layer holds, loaded before: its title and grid size.
    Loaded {
        name: String,
        size: usize,
    },
}

/// What [`LutPicker::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the choice"]
pub struct LutPickerResponse {
    /// A row was chosen ([`Set`](craft_ui::edit::EditEvent::Set)); while the wheel or the arrow
    /// keys step through the built-ins it is a gesture the host previews (Begin, Update, then
    /// Commit when the stepping stops).
    pub edit: EditResponse<LutChoice>,
    /// Load 3D LUT… was chosen: the app opens a `.cube`, `.3dl` or `.look` file (its file
    /// dialogs are craft-dialogs', which this crate does not depend on) and parses it with
    /// [`craft_cms::lutfile::parse`]; the choice does not change.
    pub load_requested: bool,
}

/// A [`Select`](craft_ui::controls::Select) of LUTs, 170 points wide in PhotoCraft.
///
/// - Rows: Load 3D LUT…, None, the eight built-in looks, the LUT folder's files by name (sorted),
///   and the loaded LUT ("name (33³)") when the layer holds one.
/// - The folder is scanned on a worker thread at most every 2 seconds (a network share can be
///   slow); until a scan ends the list shows the last one. Another folder drops the old list at
///   once. Web has no folder.
/// - The wheel and the arrow keys over the closed control step through None and the built-ins
///   only, as a previewed gesture (a folder file would be read from disk each step).
pub struct LutPicker<'a> {
    id_salt: IdSalt,
    value: &'a LutChoice,
    folder: Option<PathBuf>,
    enabled: bool,
}

impl<'a> LutPicker<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a LutChoice) -> LutPicker<'a> {
        LutPicker { id_salt: IdSalt::new(id_salt), value, folder: None, enabled: true }
    }

    /// The app's LUT folder (Preferences › File Handling › LUT Folder).
    pub fn folder(mut self, folder: Option<PathBuf>) -> LutPicker<'a> {
        self.folder = folder;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> LutPicker<'a> {
        self.enabled = enabled;
        self
    }

    // pc: crates/ui-egui/src/panels/adjust_ui.rs:175 `color_lookup_editor` (rows at :191, wheel
    // cycling at :200), :113 `folder_luts` (2 s rescan), :137 `start_folder_scan`, :96 `FOLDER_LUT`,
    // :85 `browse_color_lookup` (the app's side).
    pub fn show(self, ui: &mut Ui) -> LutPickerResponse {
        let Self { id_salt, value, folder, enabled } = self;
        let labels = LutLabels::of(ui);
        let words = Words::of(ui);
        let ctx = ui.ctx().clone();
        let scan_id = Id::unique(("craft_color::lut_folder", id_salt));
        let files = folder_files(&ctx, scan_id, folder.as_deref());

        let mut pick = Pick::Choice(value.clone());
        let mut select = Select::new(id_salt, labels.label.clone(), &mut pick)
            .wheel_steps(true)
            .enabled(enabled)
            .option(Pick::Load, labels.load.clone())
            .option(Pick::Choice(LutChoice::None), labels.none.clone());
        for (id, _) in craft_cms::lutfile::BUILTIN {
            select = select.option(Pick::Choice(LutChoice::Builtin(id)), builtin_name(&words, id));
        }
        let mut listed = false;
        for (name, path) in &files {
            listed |= matches!(value, LutChoice::Folder(current) if current == path);
            select = select.option(Pick::Choice(LutChoice::Folder(path.clone())), name.clone());
        }
        // The file in use while a scan has not found it yet (or the folder changed under it).
        if let LutChoice::Folder(path) = value
            && !listed
        {
            let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
            select = select.option(Pick::Choice(value.clone()), name);
        }
        if let LutChoice::Loaded { name, size } = value {
            let text = craft_i18n::fmt(&labels.custom, &[("name", name), ("size", &size.to_string())]);
            select = select.option(Pick::Choice(value.clone()), text);
        }
        let shown = select.show(ui);
        let response = shown.response;

        let step_id = response.id.with("lut-step");
        let now = ui.input(|i| i.time);
        let mut load_requested = false;
        let mut event = None;
        match shown.event {
            // The list is closed: the wheel or an arrow key stepped. Only None and the built-ins
            // preview; a folder file would be read from disk at each step.
            Some(EditEvent::Set { value: Pick::Choice(step @ (LutChoice::None | LutChoice::Builtin(_))), modifiers, .. }) => {
                event = Some(step_event(&ctx, step_id, step, value, modifiers, now));
            }
            Some(EditEvent::Commit { value: Pick::Load, changed: true, .. }) => load_requested = true,
            Some(EditEvent::Commit { value: Pick::Choice(choice), changed: true, modifiers, .. }) => {
                event = Some(EditEvent::set(choice, modifiers));
            }
            _ => {}
        }
        if event.is_none() {
            event = finish_steps(&ctx, step_id, value, now);
        }
        LutPickerResponse { edit: EditResponse::new(response, event), load_requested }
    }
}

/// How long the wheel or the keys rest before the stepping gesture commits.
const STEP_REST: f64 = 0.75;

/// A gesture of wheel or key steps over the closed select.
#[derive(Clone, Copy)]
struct Stepping {
    id: GestureId,
    last_step: f64,
}

#[derive(Clone)]
struct StepStart(LutChoice);

fn step_event(ctx: &Context, key: Id, step: LutChoice, current: &LutChoice, modifiers: Modifiers, now: f64) -> EditEvent<LutChoice> {
    let running = ctx.data(|d| d.get_temp::<Stepping>(key));
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
    match running {
        Some(running) => {
            ctx.data_mut(|d| d.insert_temp(key, Stepping { last_step: now, ..running }));
            EditEvent::update(running.id, step, modifiers)
        }
        None => {
            let id = GestureId::next();
            ctx.data_mut(|d| {
                d.insert_temp(key, Stepping { id, last_step: now });
                d.insert_temp(key.with("start"), StepStart(current.clone()));
            });
            EditEvent::begin_with(id, step)
        }
    }
}

/// Commits the stepping gesture once the steps have rested; `current` is the host's previewed value.
fn finish_steps(ctx: &Context, key: Id, current: &LutChoice, now: f64) -> Option<EditEvent<LutChoice>> {
    let running = ctx.data(|d| d.get_temp::<Stepping>(key))?;
    if now - running.last_step < STEP_REST {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        return None;
    }
    let start = ctx.data(|d| d.get_temp::<StepStart>(key.with("start")))?.0;
    ctx.data_mut(|d| d.remove::<Stepping>(key));
    Some(EditEvent::commit(running.id, current.clone(), *current != start, Modifiers::NONE))
}

/// A row of the list: Load 3D LUT… or a choice.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Pick {
    Load,
    Choice(LutChoice),
}

fn builtin_name(words: &Words, id: &str) -> String {
    let key = format!("lut.builtin.{id}");
    LUT_WORDS.iter().find(|word| word.id == key).map_or_else(|| id.to_owned(), |word: &Word| words.get(*word).into_owned())
}

type FolderFiles = Vec<(String, PathBuf)>;

/// The LUT folder scan kept between frames: the latest finished list of `folder`, and the scan in
/// flight, if any.
#[derive(Clone, Default)]
struct FolderScan {
    folder: Option<PathBuf>,
    started: Option<f64>,
    files: FolderFiles,
    pending: Option<Arc<Mutex<Option<FolderFiles>>>>,
}

/// How often the LUT folder is scanned again.
const RESCAN: f64 = 2.0;

/// The folder's LUT files: scanned on a worker thread at most every [`RESCAN`] seconds, since a
/// network share can be slow. Until a scan ends this is the last one; another folder drops it.
fn folder_files(ctx: &Context, key: Id, folder: Option<&Path>) -> FolderFiles {
    let now = ctx.input(|i| i.time);
    let mut scan = ctx.data(|d| d.get_temp::<FolderScan>(key)).unwrap_or_default();
    if let Some(done) = scan.pending.as_ref().and_then(|p| p.lock().unwrap_or_else(PoisonError::into_inner).take()) {
        scan.files = done;
        scan.pending = None;
    }
    if scan.folder.as_deref() != folder {
        scan = FolderScan { folder: folder.map(Path::to_path_buf), ..FolderScan::default() };
    }
    if let Some(folder) = folder
        && scan.pending.is_none()
        && scan.started.is_none_or(|at| now - at >= RESCAN)
    {
        scan.started = Some(now);
        scan.pending = Some(start_scan(folder.to_path_buf(), ctx.clone()));
    }
    let files = scan.files.clone();
    ctx.data_mut(|d| d.insert_temp(key, scan));
    files
}

fn start_scan(folder: PathBuf, ctx: Context) -> Arc<Mutex<Option<FolderFiles>>> {
    let result = Arc::new(Mutex::new(None));
    let shared = Arc::clone(&result);
    let work = move || {
        *shared.lock().unwrap_or_else(PoisonError::into_inner) = Some(list_folder(&folder));
        ctx.request_repaint();
    };
    #[cfg(not(target_arch = "wasm32"))]
    if std::thread::Builder::new().name("lut-folder-scan".into()).spawn(work).is_err() {
        // No thread: list nothing this time; the next scan retries.
        *result.lock().unwrap_or_else(PoisonError::into_inner) = Some(Vec::new());
    }
    // The web has no file system to list.
    #[cfg(target_arch = "wasm32")]
    drop(work);
    result
}

/// Most files [`list_folder`] lists.
const MAX_FILES: usize = 500;
/// Largest file [`list_folder`] lists: a 129³ `.cube` is about 60 MB.
const MAX_BYTES: u64 = 64 << 20;

/// The `.cube`, `.3dl` and `.look` files directly in `dir` as (file name, path), sorted by name
/// without regard to case. An unreadable folder lists nothing; hidden files, subfolders and files
/// too large to be a LUT are skipped, and a symlink to a LUT file is listed.
#[cfg(not(target_arch = "wasm32"))]
fn list_folder(dir: &Path) -> FolderFiles {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: FolderFiles = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
            if name.starts_with('.') || !matches!(extension.as_str(), "cube" | "3dl" | "look") {
                return None;
            }
            let path = entry.path();
            let meta = std::fs::metadata(&path).ok()?;
            (meta.is_file() && meta.len() <= MAX_BYTES).then_some((name, path))
        })
        .collect();
    out.sort_by_key(|(name, _)| name.to_lowercase());
    out.truncate(MAX_FILES);
    out
}

#[cfg(target_arch = "wasm32")]
fn list_folder(_: &Path) -> FolderFiles {
    Vec::new()
}
