//! Fetching served faces: the UI language's at install, others when the engine asks the stack's sources for a character
//! no installed face has. The fetching itself is a [`FontFetcher`]: the browser's on the web, a fake one in tests.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use craft_ui_rt::{FaceSource, FallbackAsk, FontBytes, StackFace};

use crate::fonts::{FontHost, Stack};

/// Seconds before a failed file is fetched again; each further failure doubles it, up to
/// [`RETRY_MAX`].
const RETRY_FIRST: f64 = 2.0;
const RETRY_MAX: f64 = 600.0;
/// The most doublings of the retry delay; the delay is capped long before.
const MAX_DOUBLINGS: u32 = 16;

/// Called with a fetched file's bytes, or why it failed.
pub type FetchDone = Box<dyn FnOnce(Result<Vec<u8>, String>) + Send>;
/// Called when a timer fires.
pub type FetchWake = Box<dyn FnOnce() + Send>;

/// Fetches files and keeps time for the loader of served faces.
pub trait FontFetcher: Send + Sync {
    /// Start fetching `url`, checked against `integrity` when not empty, and call `done` with its
    /// bytes or why it failed.
    fn fetch(&self, url: &'static str, integrity: &'static str, done: FetchDone);
    /// Seconds since some fixed time.
    fn now(&self) -> f64;
    /// Call `wake` once, `seconds` from now.
    fn after(&self, seconds: f64, wake: FetchWake);
}

/// Install `stack`, add the files fetched so far, and fetch the UI language's faces.
pub(in crate::fonts) fn install(host: &FontHost, mut stack: Stack, fetcher: Arc<dyn FontFetcher>) {
    let loader = Loader::of(host, fetcher);
    let fetch = {
        let mut state = loader.lock();
        let now = loader.fetcher.now();
        for (&(url, integrity), file) in &state.files {
            stack.provide_fetched(url, integrity, file);
        }
        state.ui_pending.clear();
        let mut fetch: Vec<Key> = Vec::new();
        for face in stack.ui_downloads() {
            let key = (face.url, face.integrity);
            if state.files.contains_key(&key) || !state.due(key, now) {
                continue;
            }
            state.ui_pending.insert(key);
            if !fetch.contains(&key) {
                fetch.push(key);
            }
        }
        state.stack = Some(stack.clone());
        fetch
    };
    stack.offer_faces(host, Some(Arc::clone(&loader) as Arc<dyn FaceSource>));
    for key in fetch {
        loader.fetch(key);
    }
}

/// The host's fetched files and the stack they go into. Also the source asked for characters no face
/// has, which starts fetching the face that draws them.
pub(crate) struct Loader {
    host: FontHost,
    me: Weak<Loader>,
    fetcher: Arc<dyn FontFetcher>,
    state: Mutex<State>,
}

/// A file as one face asks for it: its URL and the integrity the bytes were checked against.
/// Bytes fetched for one integrity (or none) are never handed to a face that requires another.
type Key = (&'static str, &'static str);

#[derive(Default)]
struct State {
    /// The installed stack, without the files' data.
    stack: Option<Stack>,
    /// Fetched files; the browser's HTTP cache keeps them across page loads.
    files: BTreeMap<Key, FontBytes>,
    in_flight: BTreeSet<Key>,
    failed: BTreeMap<Key, Failure>,
    /// The UI language's files not fetched yet.
    ui_pending: BTreeSet<Key>,
}

struct Failure {
    attempts: u32,
    retry_at: f64,
}

impl State {
    /// Whether `key` may be fetched: it never failed, or its retry time has come.
    fn due(&self, key: Key, now: f64) -> bool {
        self.failed.get(&key).is_none_or(|failure| now >= failure.retry_at)
    }
}

impl Loader {
    fn of(host: &FontHost, fetcher: Arc<dyn FontFetcher>) -> Arc<Self> {
        // The loader holds a host handle, and the host holds the loader: both live for the session.
        host.loader(|| Arc::new_cyclic(|me| Self { host: host.clone(), me: me.clone(), fetcher, state: Mutex::default() }))
    }

    /// Whether the UI language's files are still being fetched.
    pub(crate) fn loading(&self) -> bool {
        !self.lock().ui_pending.is_empty()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn fetch(&self, key: Key) {
        if !self.lock().in_flight.insert(key) {
            return;
        }
        let Some(me) = self.me.upgrade() else { return };
        let (url, integrity) = key;
        self.fetcher.fetch(url, integrity, Box::new(move |result| me.arrived(key, result)));
    }

    fn arrived(&self, key: Key, result: Result<Vec<u8>, String>) {
        let (stack, retry) = {
            let mut state = self.lock();
            state.in_flight.remove(&key);
            state.ui_pending.remove(&key);
            match result {
                Ok(bytes) => {
                    state.failed.remove(&key);
                    let file: FontBytes = Arc::new(bytes);
                    state.files.insert(key, Arc::clone(&file));
                    let stack = state.stack.as_mut().map(|stack| {
                        stack.provide_fetched(key.0, key.1, &file);
                        stack.clone()
                    });
                    (stack, None)
                }
                Err(_) => {
                    let attempts = state.failed.get(&key).map_or(1, |failure| failure.attempts + 1);
                    let delay = (RETRY_FIRST * 2f64.powi(i32::try_from(attempts.saturating_sub(1).min(MAX_DOUBLINGS)).unwrap_or(0))).min(RETRY_MAX);
                    state.failed.insert(key, Failure { attempts, retry_at: self.fetcher.now() + delay });
                    (None, Some(delay))
                }
            }
        };
        // The stack with the file that arrived draws characters the engine found no face for; it
        // moves the engine's stack generation once, which forgets the misses. After a failure, the
        // characters go to the next face that draws them now and to this one again at its retry time.
        match (stack, self.me.upgrade()) {
            (Some(stack), Some(me)) => stack.offer_faces(&self.host, Some(me as Arc<dyn FaceSource>)),
            _ => self.host.refresh(),
        }
        if let Some(delay) = retry {
            let me = self.me.clone();
            self.fetcher.after(
                delay,
                Box::new(move || {
                    if let Some(me) = me.upgrade() {
                        me.host.refresh();
                    }
                }),
            );
        }
    }
}

impl FaceSource for Loader {
    /// Start fetching the face that draws the character and find nothing for now: the face is
    /// installed when it arrives, and the engine asks again. Only faces whose files map the
    /// character are candidates, the first one the family asks; a failed one leaves the character
    /// to the next until its retry time. A character no served face maps fetches nothing.
    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        let now = self.fetcher.now();
        let state = self.lock();
        let mut next = None;
        for face in state.stack.as_ref()?.downloads(ask.family, ask.cluster) {
            let key = (face.url, face.integrity);
            if state.in_flight.contains(&key) || state.files.contains_key(&key) {
                // It draws the character once it arrives, or already does.
                return None;
            }
            if state.due(key, now) {
                next = Some(key);
                break;
            }
        }
        drop(state);
        self.fetch(next?);
        None
    }
}
