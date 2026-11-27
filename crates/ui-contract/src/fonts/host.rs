//! The hand-over of font stacks to the text engine: what changes the faces (an install, a fetched face, on-demand
//! script coverage, the OS interface face resolving, the installed fonts changing) offers the stack the engine should
//! set text in next; the host gives it to the engine once per frame, before the frame runs. The host also keeps the
//! session's document faces and the sources an installed stack shares across its reinstalls (the served faces'
//! loader).

use std::sync::{Arc, Mutex, PoisonError};

use craft_ui_rt::{FontStack, Store, TextEngine, Waker};

use super::document::DocumentFaces;
use super::served::loader::Loader;

/// The stack offered and not yet given, shared with the threads that offer one (a fetch, the system font index).
/// The host provides it above every window root and calls [`FontHost::give`] in its frame hook.
#[derive(Clone)]
pub struct FontHost {
    shared: Arc<Shared>,
}

struct Shared {
    pending: Mutex<Option<FontStack>>,
    /// The stack offered last, offered again by [`FontHost::refresh`].
    last: Mutex<Option<FontStack>>,
    waker: Option<Waker>,
    /// A stack installed before the OS face resolved, to install again with it, unless a newer stack is installed
    /// first.
    waiting: Mutex<Option<super::Stack>>,
    loader: Mutex<Option<Arc<Loader>>>,
    on_demand: Mutex<Option<Arc<super::on_demand::OnDemandSource>>>,
    documents: DocumentFaces,
}

impl FontHost {
    /// A hand-over that wakes the loop through `waker` when a stack is offered; `None` where the caller steps the loop
    /// itself (a test harness).
    pub fn new(waker: Option<Waker>) -> FontHost {
        let documents = DocumentFaces::new(waker.clone());
        FontHost {
            shared: Arc::new(Shared {
                pending: Mutex::default(),
                last: Mutex::default(),
                waker,
                waiting: Mutex::default(),
                loader: Mutex::default(),
                on_demand: Mutex::default(),
                documents,
            }),
        }
    }

    /// Offers `stack` in place of any stack offered and not yet given, and wakes the loop. Callable from any thread.
    pub fn offer(&self, stack: FontStack) {
        *lock(&self.shared.last) = Some(stack.clone());
        *lock(&self.shared.pending) = Some(stack);
        self.wake();
    }

    /// Offers the stack offered last again: a source found faces it had none for (the installed fonts' index arrived
    /// or changed, a fetch failed and its retry time came), so the engine forgets the characters it asked them for and
    /// asks again. Callable from any thread.
    pub fn refresh(&self) {
        let last = lock(&self.shared.last).clone();
        if let Some(stack) = last {
            *lock(&self.shared.pending) = Some(stack);
            self.wake();
        }
    }

    /// Gives `engine` the stack offered last, if one is waiting, and the document faces that landed since the last
    /// frame; returns whether a stack was given. With a stack the engine's stack generation moves, and the frame that
    /// follows lays out again what laid text out. Each face given writes its cell in `store`, so the nodes that asked
    /// for it run in that frame and find it ready. Call it before each frame runs.
    pub fn give(&self, engine: &mut dyn TextEngine, store: &mut Store) -> bool {
        let pending = lock(&self.shared.pending).take();
        let given = match pending {
            Some(stack) => {
                engine.set_stack(&stack);
                true
            }
            None => false,
        };
        self.shared.documents.install_landed(engine, store);
        given
    }

    /// Wakes the loop. Callable from any thread.
    pub(crate) fn wake(&self) {
        if let Some(waker) = &self.shared.waker {
            waker.wake();
        }
    }

    /// Keeps `stack` to install again once the OS face resolves, in place of one kept before; `None` drops it.
    pub(crate) fn set_waiting(&self, stack: Option<super::Stack>) {
        *lock(&self.shared.waiting) = stack;
    }

    /// Installs the stack kept for the OS face, with the face, once it has resolved. Callable from any thread.
    pub(crate) fn install_waiting(&self) {
        let Some(face) = super::system::ui_face::resolved() else { return };
        let waiting = lock(&self.shared.waiting).take();
        if let Some(mut stack) = waiting {
            stack.provide_system_ui(face);
            stack.install(self);
        }
    }

    /// Whether two handles are the same host.
    pub(crate) fn same(&self, other: &FontHost) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }

    /// The served faces' loader, made by `make` the first time.
    pub(crate) fn loader(&self, make: impl FnOnce() -> Arc<Loader>) -> Arc<Loader> {
        Arc::clone(lock(&self.shared.loader).get_or_insert_with(make))
    }

    /// The served faces' loader, if one was made.
    pub(crate) fn existing_loader(&self) -> Option<Arc<Loader>> {
        lock(&self.shared.loader).clone()
    }

    /// Keeps the on-demand loader the installed stack asks, in place of an earlier one.
    pub(crate) fn set_on_demand(&self, source: Arc<super::on_demand::OnDemandSource>) {
        *lock(&self.shared.on_demand) = Some(source);
    }

    /// The faces the installed stack's on-demand loader registered so far (font name, file, face index), for the font
    /// list in Preferences and the tests.
    pub fn on_demand_registered(&self) -> Vec<(String, std::path::PathBuf, u32)> {
        lock(&self.shared.on_demand).as_ref().map(|source| source.registered()).unwrap_or_default()
    }

    /// Whether the UI language's served faces are still being fetched, for an app that waits for them before drawing
    /// its interface. False without a fetcher, and once every fetch has finished, failed ones included.
    pub fn loading(&self) -> bool {
        self.existing_loader().is_some_and(|loader| loader.loading())
    }

    /// The session's document faces.
    pub(crate) fn documents(&self) -> &DocumentFaces {
        &self.shared.documents
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
