//! Faces drawn as families of their own, for text set in a document's fonts.
//!
//! One registry per session (the [`FontHost`]'s) serves every window, so the faces held are counted together. A face
//! is drawn alone ([`face`]: a font sample shows what the face itself has) or over the interface fonts
//! ([`over_interface`]: document text such as an input method's composition, whose characters the face lacks come from
//! the interface stack, CJK included).
//!
//! A face drawn alone is read from a font catalog: its load runs as a job on the session's job board, and the face is
//! ready from the frame after it lands: the landing wakes the host, which installs the face in the text engine before
//! that frame runs ([`FontHost::give`]) and writes the face's cell, so the nodes that asked run in it and find it ready. A load whose last asker
//! leaves before it lands is cancelled and the face forgotten, so a face scrolled past is not read. A face over the interface
//! fonts comes from bytes the app holds: its load runs at once and the face is installed and ready in the same run, so
//! document text never draws a frame without it. Installing a face drops nothing the engine has parsed or laid out.
//!
//! Every node that sets text in a face reads its cell, so the face is in use while some node in the tree reads it,
//! drawn or kept (a settled view keeps its reads). The registry keeps faces up to [`MAX_DOCUMENT_FACE_BYTES`] of font
//! files; past it the faces no node reads go, those that lost their last reader longest ago first
//! ([`TextEngine::remove_face`]), and a face in use always stays.
//!
//! A face may name a chain of other document faces ([`FaceBytes::chain`]): a font split over several subsets (Word
//! embeds one subset per run of characters) draws each character from the first subset that has it, before the
//! interface family. A node that reads a face reads its chain's cells too, so the siblings stay while the face does.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_theme::FontSlot;
use craft_ui_rt::{BuildCx, FaceInstall, FontFace, State, Store, TextEngine, Waker, Writes};

use super::{FontHost, FontRef};
use crate::contract::Context;
use crate::contract::rt::JobBoard;
use crate::contract::rt::pending::JobHandle;
use craft_host::queue::Priority;

/// The ceiling on the font file bytes of document faces held in the engine (a collection counted once). When they pass
/// it, the faces no node reads are taken out, those unread longest first, until the held files are at most
/// `1 / LOW_MARK_DIVISOR` of it. The sum passes the ceiling only by faces in use, which always stay.
pub const MAX_DOCUMENT_FACE_BYTES: usize = 512 << 20;

/// Eviction takes the held files down to the budget divided by this, so the engine frees what it parsed once per half a
/// budget of new faces, not for each face that lands past the budget.
const LOW_MARK_DIVISOR: usize = 2;

/// The bytes of a face, its index in a collection, its variation coordinates (axis tag, value) and the faces that
/// draw what it lacks.
#[derive(Clone)]
#[non_exhaustive]
pub struct FaceBytes {
    /// The font file.
    pub bytes: Arc<[u8]>,
    /// The face's index in the file.
    pub index: u32,
    /// The variation coordinates the face is drawn at.
    pub coords: Vec<([u8; 4], f32)>,
    /// The keys of the document faces that draw the characters this face has no glyph for, in order, before the
    /// interface family.
    pub chain: Vec<String>,
}

impl FaceBytes {
    /// Face `index` of the file `bytes`, at its default coordinates, with no chain.
    pub fn new(bytes: Arc<[u8]>, index: u32) -> Self {
        FaceBytes { bytes, index, coords: Vec::new(), chain: Vec::new() }
    }

    /// The face followed by the document faces of `keys`, in order, for the characters it has no glyph for: the
    /// sibling subsets of a font split over several (Word). Each key is a face asked for with `document_face` on its
    /// own; one not registered, unavailable or taken out is skipped, and the face's own key is ignored. Drawing the
    /// face marks its chain drawn too, so a sibling the face relies on stays while the face does.
    #[must_use]
    pub fn chain(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.chain = keys.into_iter().map(Into::into).collect();
        self
    }

    /// The family names of the chain's faces (each drawn alone).
    pub(crate) fn chain_families(&self) -> Vec<String> {
        self.chain.iter().map(|key| Kind::Alone.family_name(key)).collect()
    }

    /// The face drawn at `coords`.
    #[must_use]
    pub fn coords(mut self, coords: impl IntoIterator<Item = ([u8; 4], f32)>) -> Self {
        self.coords = coords.into_iter().collect();
        self
    }

    /// Whether the engines can draw the face: they stop on a face they cannot read.
    pub(crate) fn parses(&self) -> bool {
        skrifa::FontRef::from_index(&self.bytes, self.index).is_ok()
    }

    /// The file's identity: faces of one collection share their bytes.
    fn file(&self) -> usize {
        Arc::as_ptr(&self.bytes).cast::<u8>() as usize
    }
}

/// What drawing a document face finds.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FaceState {
    /// The family that draws the face.
    Ready(FontRef),
    /// The face is being loaded; the asking view runs again in the next pass.
    Loading,
    /// The face could not be loaded.
    Unavailable,
}

/// A face's load, made by the asking view and run on the job board.
type Load = Box<dyn FnOnce() -> Option<FaceBytes> + Send>;

/// Where a face is.
enum Made {
    /// Asked for; its load waits for the asking node's build to end (`load`), then runs (`job`).
    Loading { load: Option<Load>, job: Option<JobHandle> },
    /// Its load ended with the face; the host installs it before the next frame and writes its cell.
    Landed(FaceBytes),
    /// In the engine.
    Installed(FaceBytes),
    /// The load gave nothing, or bytes the engines cannot read.
    Failed,
}

/// Whether nodes read a face.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Readers {
    /// None read it yet.
    None,
    /// Some node in the tree reads it.
    Some,
    /// The last reader left, at this serial.
    Left(u64),
}

struct Kept {
    made: Made,
    /// The face draws over the interface family.
    over_interface: bool,
    /// The cell the nodes that set text in the face read; written when the face is installed or its load fails.
    cell: Option<State<u64>>,
    readers: Readers,
    /// Tells this asking apart from an earlier one of the same key that was cancelled.
    ticket: u64,
}

/// The session's document faces, by family name.
struct Registry {
    faces: HashMap<String, Kept>,
    /// [`MAX_DOCUMENT_FACE_BYTES`], or a trial's own (`testing::document_face_budget`).
    budget: usize,
    /// Counts readers leaving, to order eviction.
    serial: u64,
    /// The last [`Kept::ticket`] handed out.
    tickets: u64,
    /// Wakes the host's loop when a face lands, so the next frame installs it.
    waker: Option<Waker>,
}

impl Default for Registry {
    fn default() -> Self {
        Registry { faces: HashMap::new(), budget: MAX_DOCUMENT_FACE_BYTES, serial: 0, tickets: 0, waker: None }
    }
}

/// The session's document faces, kept by the [`FontHost`].
#[derive(Clone, Default)]
pub(crate) struct DocumentFaces(Arc<Mutex<Registry>>);

impl DocumentFaces {
    /// The registry of a host whose loop `waker` wakes.
    pub(crate) fn new(waker: Option<Waker>) -> Self {
        DocumentFaces(Arc::new(Mutex::new(Registry { waker, ..Registry::default() })))
    }

    fn lock(&self) -> MutexGuard<'_, Registry> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Installs the faces that landed since the last frame and writes their cells in `store`, so the nodes that asked
    /// run in the coming frame; then takes out what passes the budget. The host's frame hook, before the frame runs.
    pub(crate) fn install_landed(&self, engine: &mut dyn TextEngine, store: &mut Store) {
        let cells = {
            let mut registry = self.lock();
            let landed: Vec<String> =
                registry.faces.iter().filter(|(_, kept)| matches!(kept.made, Made::Landed(_))).map(|(name, _)| name.clone()).collect();
            if landed.is_empty() {
                return;
            }
            let mut cells = Vec::new();
            for name in landed {
                let Some(kept) = registry.faces.get_mut(&name) else { continue };
                let Made::Landed(face) = std::mem::replace(&mut kept.made, Made::Failed) else { continue };
                if install(engine, &name, &face, kept.over_interface) {
                    kept.made = Made::Installed(face);
                }
                cells.extend(kept.cell);
            }
            registry.evict(engine);
            cells
        };
        for cell in cells {
            cell.update(store, |installs| *installs += 1);
        }
    }

    /// Reads `name`'s cell (and its chain's) for the node `cx` builds, starting its load if one waits, and watches for
    /// the last reader leaving.
    fn read(&self, cx: &mut BuildCx<'_>, name: &str) {
        let (chain, load, ticket) = {
            let mut registry = self.lock();
            let Some(kept) = registry.faces.get_mut(name) else { return };
            let load = match &mut kept.made {
                Made::Loading { load, .. } => load.take(),
                _ => None,
            };
            let chain = match &kept.made {
                Made::Installed(face) | Made::Landed(face) => face.chain_families(),
                _ => Vec::new(),
            };
            (chain, load, kept.ticket)
        };
        self.read_cell(cx, name);
        for link in chain {
            self.read_cell(cx, &link);
        }
        if let Some(load) = load {
            let board = cx.provided::<JobBoard>().unwrap_or_else(JobBoard::inline);
            let (faces, name) = (self.clone(), name.to_owned());
            let job = board.spawn_with(
                Priority::VISIBLE,
                move |cancel| if cancel.is_cancelled() { None } else { load().filter(FaceBytes::parses) },
                move |result, delivery| {
                    let (failed, waker) = {
                        let mut registry = faces.lock();
                        let waker = registry.waker.clone();
                        let Some(kept) = registry.faces.get_mut(&name).filter(|kept| kept.ticket == ticket) else { return };
                        if !matches!(kept.made, Made::Loading { .. }) {
                            return;
                        }
                        match result.ok().flatten() {
                            Some(face) => {
                                kept.made = Made::Landed(face);
                                (None, waker)
                            }
                            None => {
                                kept.made = Made::Failed;
                                (kept.cell, None)
                            }
                        }
                    };
                    // A failed load is answered now; a face waits for the host's next frame, which installs it.
                    if let Some(cell) = failed {
                        cell.update(delivery.store, |installs| *installs += 1);
                    }
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                },
            );
            if let Some(kept) = self.lock().faces.get_mut(&name).filter(|kept| kept.ticket == ticket)
                && let Made::Loading { job: slot, .. } = &mut kept.made
            {
                *slot = Some(job);
            }
        }
    }

    fn read_cell(&self, cx: &mut BuildCx<'_>, name: &str) {
        let (cell, arm) = {
            let mut registry = self.lock();
            let Some(kept) = registry.faces.get_mut(name) else { return };
            let cell = *kept.cell.get_or_insert_with(|| cx.store_mut().create(0u64));
            let arm = kept.readers != Readers::Some;
            kept.readers = Readers::Some;
            (cell, arm)
        };
        cell.get(cx);
        if arm {
            let (faces, name) = (Arc::downgrade(&self.0), name.to_owned());
            cx.store_mut().on_unread(cell, move || {
                let Some(faces) = faces.upgrade() else { return };
                let mut registry = faces.lock().unwrap_or_else(PoisonError::into_inner);
                if let Some(Kept { made: Made::Loading { job, .. }, .. }) = registry.faces.get(&name) {
                    // Nobody waits for the face any more: its load stops and the next asking starts again.
                    if let Some(job) = job {
                        job.cancel();
                    }
                    registry.faces.remove(&name);
                    return;
                }
                registry.serial += 1;
                let serial = registry.serial;
                if let Some(kept) = registry.faces.get_mut(&name) {
                    kept.readers = Readers::Left(serial);
                }
            });
        }
    }
}

impl Registry {
    /// When the held files pass the budget, takes out the faces no node reads, those unread longest first, down to the
    /// low mark.
    fn evict(&mut self, engine: &mut dyn TextEngine) {
        let mut files: HashMap<usize, (usize, usize)> = HashMap::new();
        for kept in self.faces.values() {
            if let Made::Installed(face) = &kept.made {
                files.entry(face.file()).or_insert((face.bytes.len(), 0)).1 += 1;
            }
        }
        let mut held: usize = files.values().map(|(bytes, _)| bytes).sum();
        if held <= self.budget {
            return;
        }
        let low = self.budget / LOW_MARK_DIVISOR;
        let mut unread: Vec<(u64, String)> = self
            .faces
            .iter()
            .filter_map(|(name, kept)| match (kept.readers, &kept.made) {
                (Readers::Left(serial), Made::Installed(_)) => Some((serial, name.clone())),
                _ => None,
            })
            .collect();
        unread.sort();
        for (_, name) in unread {
            if held <= low {
                break;
            }
            let Some(Kept { made: Made::Installed(face), .. }) = self.faces.remove(&name) else { continue };
            engine.remove_face(&name);
            if let Some((bytes, count)) = files.get_mut(&face.file()) {
                *count -= 1;
                if *count == 0 {
                    held -= *bytes;
                }
            }
        }
    }
}

/// Installs `face` as the family `name`: then its chain of document faces and, `over_interface`, the interface family.
fn install(engine: &mut dyn TextEngine, name: &str, face: &FaceBytes, over_interface: bool) -> bool {
    let mut fallback: Vec<FontFace> = face.chain_families().into_iter().map(|link| FontFace::Family(link.into())).collect();
    if over_interface {
        fallback.push(FontFace::Slot(FontSlot::Ui));
    }
    engine.install_face(FaceInstall { family: name.into(), bytes: Arc::clone(&face.bytes), index: face.index, coords: face.coords.clone(), fallback })
}

/// The session's font host, as the window root keeps it on the window's context (`Context::enter_root`).
#[derive(Clone, Default)]
pub(crate) struct HostSlot(pub(crate) Option<FontHost>);

fn faces_of(ctx: &Context) -> Option<DocumentFaces> {
    ctx.service(|slot: &mut HostSlot| slot.0.as_ref().map(|host| host.documents().clone()))
}

/// Has the node whose build runs read `name`'s cell when its build ends.
fn log_read(faces: &DocumentFaces, name: &str) {
    let (faces, name) = (faces.clone(), name.to_owned());
    crate::contract::context::log_build_read(move |cx| faces.read(cx, &name));
}

/// The face registered under `key` as a family of its own. The load `job()` makes reads it from a font catalog: `job`
/// is called only for a key the registry does not know, and its load runs as a job when the asking node's build ends,
/// once per key while the face is kept; the face is ready from the frame after the job ends. A load that returns
/// `None`, bytes the engines cannot read, or a job that ends without a result marks the key unavailable for the
/// session. The asking node reads the face's cell, so it runs again when the load lands and keeps the face in use.
pub(crate) fn face<L>(ctx: &Context, key: &str, job: impl FnOnce() -> L) -> FaceState
where
    L: FnOnce() -> Option<FaceBytes> + Send + 'static,
{
    let Some(faces) = faces_of(ctx) else { return FaceState::Unavailable };
    let name = Kind::Alone.family_name(key);
    let state = {
        let mut registry = faces.lock();
        match registry.faces.get(&name).map(|kept| &kept.made) {
            Some(Made::Failed) => FaceState::Unavailable,
            Some(Made::Installed(_)) => FaceState::Ready(FontRef::named(&name)),
            Some(Made::Loading { .. } | Made::Landed(_)) => FaceState::Loading,
            None => {
                let load: Load = Box::new(job());
                registry.tickets += 1;
                let ticket = registry.tickets;
                let made = Made::Loading { load: Some(load), job: None };
                registry.faces.insert(name.clone(), Kept { made, over_interface: false, cell: None, readers: Readers::None, ticket });
                FaceState::Loading
            }
        }
    };
    if state != FaceState::Unavailable {
        log_read(&faces, &name);
    }
    state
}

/// The face registered under `key` as a family of its own whose characters the face lacks come from the interface
/// family (CJK included). `load` gives bytes the app holds: it runs at once, the first time the key is drawn while the
/// face is kept, and the face is installed in `engine` and ready in the same run; without an engine at hand it is
/// installed before the next frame. A `load` that returns `None`, or bytes the engines cannot read, marks the key
/// unavailable. Its keys are apart from [`face`]'s: the same key may name a face drawn alone too.
pub(crate) fn over_interface(
    ctx: &Context,
    engine: Option<&mut (dyn TextEngine + '_)>,
    key: &str,
    load: impl FnOnce() -> Option<FaceBytes>,
) -> FaceState {
    let Some(faces) = faces_of(ctx) else { return FaceState::Unavailable };
    let name = Kind::OverInterface.family_name(key);
    let state = {
        let mut registry = faces.lock();
        match registry.faces.get(&name).map(|kept| &kept.made) {
            Some(Made::Failed) => FaceState::Unavailable,
            Some(Made::Installed(_)) => FaceState::Ready(FontRef::named(&name)),
            Some(Made::Landed(_)) => FaceState::Loading,
            Some(Made::Loading { .. }) | None => {
                let made = match (load().filter(FaceBytes::parses), engine) {
                    (None, _) => Made::Failed,
                    (Some(face), Some(engine)) => match install(engine, &name, &face, true) {
                        true => Made::Installed(face),
                        false => Made::Failed,
                    },
                    (Some(face), None) => Made::Landed(face),
                };
                let state = match &made {
                    Made::Failed => FaceState::Unavailable,
                    Made::Installed(_) => FaceState::Ready(FontRef::named(&name)),
                    _ => FaceState::Loading,
                };
                if matches!(made, Made::Landed(_))
                    && let Some(waker) = &registry.waker
                {
                    waker.wake();
                }
                let readers = registry.faces.get(&name).map_or(Readers::None, |kept| kept.readers);
                let cell = registry.faces.get(&name).and_then(|kept| kept.cell);
                registry.tickets += 1;
                let ticket = registry.tickets;
                registry.faces.insert(name.clone(), Kept { made, over_interface: true, cell, readers, ticket });
                state
            }
        }
    };
    if state != FaceState::Unavailable {
        log_read(&faces, &name);
    }
    state
}

/// The bytes of the face registered under `key` to be drawn alone, once it is ready.
pub(crate) fn bytes(ctx: &Context, key: &str) -> Option<FaceBytes> {
    let faces = faces_of(ctx)?;
    let registry = faces.lock();
    match &registry.faces.get(&Kind::Alone.family_name(key))?.made {
        Made::Installed(face) => Some(face.clone()),
        _ => None,
    }
}

/// Sets the budget of the session's document faces to `bytes` in place of [`MAX_DOCUMENT_FACE_BYTES`], so a trial
/// reaches it with a few faces.
#[cfg(feature = "testing")]
pub fn set_budget(host: &FontHost, bytes: usize) {
    host.documents().lock().budget = bytes;
}

/// How a face's family is built.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The face alone.
    Alone,
    /// The face, then the proportional family's chain.
    OverInterface,
}

impl Kind {
    /// The family (and face) name of `key`: one per key and kind.
    pub(crate) fn family_name(self, key: &str) -> String {
        match self {
            Kind::Alone => format!("face:{key}"),
            Kind::OverInterface => format!("face+ui:{key}"),
        }
    }
}
