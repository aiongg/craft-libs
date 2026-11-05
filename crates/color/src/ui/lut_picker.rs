//! [`LutPicker`] (F04.04): the 3D LUT choice of a Color Lookup adjustment, as PhotoCraft's: Load
//! 3D LUT…, None, the built-in looks ([`craft_cms::lutfile::BUILTIN`]), the files of the app's
//! LUT folder, and the loaded LUT. Load 3D LUT… picks the file through the host's
//! [`Files`](craft_host::Files) and reads and parses it as a [`Jobs`](craft_host::Jobs) job.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use egui::{AsIdSalt, Context, Id, IdSalt, Modifiers, Ui};

use craft_cms::lutfile::{LutError, LutFile};
use craft_host::{Host, PickId, PickRequest, Picked};
use craft_ui::controls::Select;
use craft_ui::edit::{EditEvent, EditResponse, GestureId};
use craft_ui::localized::{Labels as _, Word, Words};

craft_ui::words! {
    /// The LUT picker's rows, English by default. The built-in looks' names are [`LUT_WORDS`].
    #[non_exhaustive]
    pub struct LutPickerLabels {
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
    /// No LUT.
    None,
    /// A built-in look by id ([`craft_cms::lutfile::builtin`]).
    Builtin(&'static str),
    /// A file of the LUT folder; the app reads and embeds it.
    Folder(PathBuf),
    /// The LUT the layer holds, loaded before: its title and grid size.
    Loaded {
        /// The LUT's title.
        name: String,
        /// The grid size: points per side.
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
    /// The LUT file Load 3D LUT… picked, once it is read and parsed, or why it could not be: the
    /// app embeds the LUT and sets the choice to [`LutChoice::Loaded`]. The choice itself does not
    /// change until the app does. A cancelled picker reports nothing.
    pub loaded: Option<Result<LutFile, LutError>>,
}

/// A [`Select`](craft_ui::controls::Select) of LUTs, 170 points wide in PhotoCraft.
///
/// - Rows: Load 3D LUT…, None, the eight built-in looks, the LUT folder's files by name (sorted),
///   and the loaded LUT ("name (33³)") when the layer holds one.
/// - Load 3D LUT… asks the host's file service for a `.cube`, `.3dl` or `.look` file (purpose
///   `"lut"`) and answers in [`LutPickerResponse::loaded`] on the pass the file is parsed. Without
///   an installed [`Host`] it does nothing.
/// - The folder is scanned as a host job at most every 2 seconds (a network share can be
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
    /// A picker showing `value`; `id_salt` keys its state.
    pub fn new(id_salt: impl AsIdSalt, value: &'a LutChoice) -> LutPicker<'a> {
        LutPicker { id_salt: IdSalt::new(id_salt), value, folder: None, enabled: true }
    }

    /// The app's LUT folder (Preferences › File Handling › LUT Folder).
    pub fn folder(mut self, folder: Option<PathBuf>) -> LutPicker<'a> {
        self.folder = folder;
        self
    }

    /// Turns input on or off; on by default.
    pub fn enabled(mut self, enabled: bool) -> LutPicker<'a> {
        self.enabled = enabled;
        self
    }

    // pc: crates/ui-egui/src/panels/adjust_ui.rs:175 `color_lookup_editor` (rows at :191, wheel
    // cycling at :200), :113 `folder_luts` (2 s rescan), :137 `start_folder_scan`, :96 `FOLDER_LUT`,
    // :85 `browse_color_lookup` (the app's side).
    /// Draws the picker and returns the edit and any LUT file the user loaded.
    pub fn show(self, ui: &mut Ui) -> LutPickerResponse {
        let Self { id_salt, value, folder, enabled } = self;
        let labels = LutPickerLabels::of(ui);
        let words = Words::of(ui);
        let ctx = ui.ctx().clone();
        let host = Host::try_of(ui);
        let scan_id = Id::unique(("craft_color::lut_folder", id_salt));
        let files = folder_files(&ctx, host.as_deref(), scan_id, folder.as_deref());
        let load_id = Id::unique(("craft_color::lut_load", id_salt));
        let loaded = poll_load(&ctx, host.as_ref(), load_id);

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
        let mut event = None;
        match shown.event {
            // The list is closed: the wheel or an arrow key stepped. Only None and the built-ins
            // preview; a folder file would be read from disk at each step.
            Some(EditEvent::Set { value: Pick::Choice(step @ (LutChoice::None | LutChoice::Builtin(_))), modifiers, .. }) => {
                event = Some(step_event(&ctx, step_id, step, value, modifiers, now));
            }
            Some(EditEvent::Commit { value: Pick::Load, changed: true, .. }) => {
                if let Some(host) = &host {
                    start_load(&ctx, host, load_id, labels.load.trim_end_matches('…'));
                }
            }
            Some(EditEvent::Commit { value: Pick::Choice(choice), changed: true, modifiers, .. }) => {
                event = Some(EditEvent::set(choice, modifiers));
            }
            _ => {}
        }
        if event.is_none() {
            event = finish_steps(&ctx, step_id, value, now);
        }
        LutPickerResponse { edit: EditResponse::new(response, event), loaded }
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
fn folder_files(ctx: &Context, host: Option<&Host>, key: Id, folder: Option<&Path>) -> FolderFiles {
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
        scan.pending = Some(start_scan(host, folder.to_path_buf(), ctx.clone()));
    }
    let files = scan.files.clone();
    ctx.data_mut(|d| d.insert_temp(key, scan));
    files
}

fn start_scan(host: Option<&Host>, folder: PathBuf, ctx: Context) -> Arc<Mutex<Option<FolderFiles>>> {
    let result = Arc::new(Mutex::new(None));
    let shared = Arc::clone(&result);
    spawn(host, "lut-folder-scan", move || {
        *shared.lock().unwrap_or_else(PoisonError::into_inner) = Some(list_folder(&folder));
        ctx.request_repaint();
    });
    result
}

/// Runs `work` as a job of `host`; without a host, on a thread of its own (inline on the web).
fn spawn(host: Option<&Host>, name: &str, work: impl FnOnce() + Send + 'static) {
    match host {
        Some(host) => host.jobs().spawn(name, Box::new(work)),
        #[cfg(not(target_arch = "wasm32"))]
        None => craft_host::Jobs::spawn(&craft_host::ThreadJobs, name, Box::new(work)),
        #[cfg(target_arch = "wasm32")]
        None => craft_host::Jobs::spawn(&craft_host::InlineJobs, name, Box::new(work)),
    }
}

/// Where Load 3D LUT… is: the picker open, or the chosen file being read.
#[derive(Clone)]
enum Load {
    Picking(PickId),
    Reading(Arc<Mutex<Option<Result<LutFile, LutError>>>>),
}

/// The purpose of the LUT file picker: its folder is remembered under it.
const PICK_PURPOSE: &str = "lut";

/// Asks for the LUT file; a pick still open is dropped.
fn start_load(ctx: &Context, host: &Host, key: Id, title: &str) {
    if let Some(Load::Picking(old)) = ctx.data(|d| d.get_temp::<Load>(key)) {
        host.files().forget(old);
    }
    let request = PickRequest::open(PICK_PURPOSE).title(title).filter("3D LUT", &["cube", "3dl", "look"]);
    let id = host.files().pick(request);
    ctx.data_mut(|d| d.insert_temp(key, Load::Picking(id)));
}

/// Moves Load 3D LUT… along: takes the picker's answer and starts reading the file as a job, and
/// returns the LUT of a finished read.
fn poll_load(ctx: &Context, host: Option<&Arc<Host>>, key: Id) -> Option<Result<LutFile, LutError>> {
    let host = host?;
    match ctx.data(|d| d.get_temp::<Load>(key))? {
        Load::Picking(id) => {
            let picked = host.files().take_answer(id)?;
            ctx.data_mut(|d| d.remove::<Load>(key));
            let slot = Arc::new(Mutex::new(None));
            let (shared, wake, reader) = (Arc::clone(&slot), ctx.clone(), Arc::clone(host));
            let job = match picked {
                Picked::Cancelled => return None,
                Picked::Paths(paths) => {
                    let path = paths.into_iter().next()?;
                    Box::new(move || {
                        let result = reader
                            .files()
                            .read(&path)
                            .map_err(|e| LutError(e.to_string()))
                            .and_then(|bytes| craft_cms::lutfile::parse(&file_name(&path), &bytes));
                        finish(&shared, &wake, result);
                    }) as Box<dyn FnOnce() + Send>
                }
                Picked::Contents(contents) => {
                    let (name, bytes) = contents.into_iter().next()?;
                    Box::new(move || finish(&shared, &wake, craft_cms::lutfile::parse(&name, &bytes)))
                }
            };
            host.jobs().spawn("lut-load", job);
            ctx.data_mut(|d| d.insert_temp(key, Load::Reading(slot)));
            None
        }
        Load::Reading(slot) => {
            let done = slot.lock().unwrap_or_else(PoisonError::into_inner).take()?;
            ctx.data_mut(|d| d.remove::<Load>(key));
            Some(done)
        }
    }
}

fn finish(slot: &Mutex<Option<Result<LutFile, LutError>>>, ctx: &Context, result: Result<LutFile, LutError>) {
    *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(result);
    ctx.request_repaint();
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
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
