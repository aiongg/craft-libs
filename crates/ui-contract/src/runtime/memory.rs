//! Ids and memory: `Remember`, the memory stage and saved state. A component's shown and press slots live in its
//! node (`contract::rt::Memory`), so they go with the node; session and saved slots live in the runtime's memory store
//! ([`RuntimeMemory`], one per runtime, held by every window's `Context`) by `Id`, and saved slots reach the host's
//! settings.

use crate::contract::Context;
use crate::contract::rt::memory::Memory;
use crate::primitives::Id;
use crate::runtime::env::Env;
use craft_behavior::{CancelReason, GestureId};
use craft_host::Host;
use craft_settings::Remembered;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

/// A value kept in memory. `Clone` is required because session and saved slots are copied into a stage and back;
/// large caches hold `Arc`s.
pub trait Slot: Clone + Default + Send + Sync + 'static {}
impl<T: Clone + Default + Send + Sync + 'static> Slot for T {}

/// A slot that also lives in saved state.
pub trait Savable: Slot + Serialize + DeserializeOwned {}
impl<T: Slot + Serialize + DeserializeOwned> Savable for T {}

/// The key of a saved slot, such as `"craft_dialogs.export.place"`. The crate prefix is checked, and the key
/// is never derived from an `Id`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, crate::contract::MemoProps)]
pub struct SaveKey(Cow<'static, str>);

impl SaveKey {
    /// A key `crate_prefix.name`; evaluated in a const context it rejects a malformed name at compile time. The
    /// prefix is lowercase ASCII letters, digits and underscores.
    pub const fn new(key: &'static str) -> Self {
        check(key);
        SaveKey(Cow::Borrowed(key))
    }

    /// A key `crate_prefix.name` built at run time, for slots kept per item, such as a window for each open
    /// document. The format is `new`'s; a malformed key panics.
    pub fn named(key: impl Into<Arc<str>>) -> Self {
        let key: Arc<str> = key.into();
        check(&key);
        SaveKey(Cow::Owned(String::from(&*key)))
    }

    /// The key's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The id the key's slots are stored under: one derivation, whatever component writes them.
    fn id(&self) -> Id {
        global("craft_ui.saved", self)
    }
}

/// Panics unless `key` is `crate_prefix.name`.
const fn check(key: &str) {
    let bytes = key.as_bytes();
    let mut prefix = 0;
    while prefix < bytes.len() && bytes[prefix] != b'.' {
        let byte = bytes[prefix];
        assert!(
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_',
            "a SaveKey's crate prefix is lowercase ASCII, digits and underscores"
        );
        prefix += 1;
    }
    assert!(prefix > 0 && prefix + 1 < bytes.len(), "a SaveKey is `crate_prefix.name`");
}

/// Implemented by `Cx`, `BodyCx` and `ItemCx` over the pipeline's memory stage, each keyed by its own base id.
pub trait Remember {
    /// `base.with(part)`: child and relation ids.
    fn key(&self, part: impl Hash + Debug) -> Id;
    /// The shown slot of this component.
    fn memory<T: Slot>(&mut self) -> &mut T;
    /// A shown slot keyed `sub(base, ("mem", part))`.
    fn memory_at<T: Slot>(&mut self, part: impl Hash + Debug) -> &mut T;
    /// The shown slot, read once: it moves out, so it follows the shown lifetime's eviction.
    fn take<T: Slot>(&mut self) -> Option<T>;
    /// The press slot: it goes when its press ends.
    fn press_slot<T: Slot>(&mut self) -> &mut Option<T>;
    /// The session slot: it goes only through `forget`.
    fn keep<T: Slot>(&mut self) -> &mut T;
    /// The saved slot of `key`: it goes only through `forget_saved`. With a host in the frame it is loaded from the
    /// host's settings on first use and stored there when a pass changes it.
    fn saved<T: Savable>(&mut self, key: &SaveKey) -> &mut T;
    /// A read-only copy of the shown slot, for paint.
    fn recall<T: Slot>(&self) -> T;
    /// True in the run that created the component's node: its first showing, or its first after it left the tree.
    fn appeared(&self) -> bool;
}

/// A press slot in a node's memory: one type apart from the shown slot of the same `T` under the same id.
#[derive(Clone, Default)]
struct PressSlot<T>(Option<T>);

/// The session and saved slots of the runtime, by type and `Id`, and its edit facts: what outlives the nodes that
/// use it.
#[derive(Default)]
pub(crate) struct HostMemory {
    session: HashMap<(TypeId, Id), Box<dyn Any>>,
    saved: HashMap<(TypeId, Id), Box<dyn Any>>,
    pub(crate) edits: Edits,
    pub(crate) gestures: crate::runtime::edit::LiveGestures,
}

/// The runtime's edit facts: the latest cancel of every running edit and the gesture ids handed out.
#[derive(Clone, Copy, Default)]
pub(crate) struct Edits {
    /// Grows with each `Ui::cancel_edits`; 0 before the first.
    pub(crate) cancel_serial: u64,
    pub(crate) cancel_reason: Option<CancelReason>,
    /// The newest gesture id handed out: ids grow in the order gestures begin, across components and windows.
    pub(crate) newest: GestureId,
    /// How many times a component reported its gesture running after its apply step (`Ui::watch_gestures`).
    pub(crate) running: u64,
}

/// The runtime's memory store: session and saved slots by type and `Id`. The host makes one per runtime and provides
/// it above every window root, so a panel moved to another window keeps its state; each window's `Context` holds it.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct RuntimeMemory(Rc<RefCell<HostMemory>>);

impl RuntimeMemory {
    /// An empty store.
    pub fn new() -> RuntimeMemory {
        RuntimeMemory::default()
    }

    /// Runs `f` on the store. `f` must not reach the store again.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut HostMemory) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }
}

/// How a staged saved slot reaches the host's settings: the serialized form it had when staged, and the typed
/// store and form functions, monomorphised where `T: Savable` is known.
struct Persist {
    key: SaveKey,
    host: Arc<Host>,
    before: Option<toml::Value>,
    form: fn(&dyn Any) -> Option<toml::Value>,
    store: fn(&dyn Any, &Host, &SaveKey),
}

fn saved_form<T: Savable>(value: &dyn Any) -> Option<toml::Value> {
    value.downcast_ref::<T>().and_then(|value| toml::Value::try_from(value).ok())
}

fn store_saved<T: Savable>(value: &dyn Any, host: &Host, key: &SaveKey) {
    if let Some(value) = value.downcast_ref::<T>() {
        Remembered::<T>::new(key.as_str()).store(host, value);
    }
}

/// Removes the stored value of `key`, whatever its type: `Remembered::forget` reads no value.
fn forget_stored(host: &Host, key: &SaveKey) {
    Remembered::<toml::Value>::new(key.as_str()).forget(host);
}

/// One staged session or saved slot.
struct Staged {
    ty: TypeId,
    id: Id,
    saved: bool,
    value: Box<dyn Any>,
    persist: Option<Persist>,
}

/// What a component's hooks remember through `Remember`: its node's memory, with the window's session and saved
/// slots copied in on first use and back by [`MemoryStage::finish`]. It appears in the sealed `StageAccess`.
#[doc(hidden)]
pub struct MemoryStage {
    /// The window's context: set when the stage is made for a run, or by [`MemoryStage::within`] for a node's kept stage.
    ctx: Option<Context>,
    /// The component whose node the shown and press slots belong to.
    owner: Id,
    memory: Memory,
    staged: Vec<Staged>,
    /// A measuring run: slots are read and changed in the stage, and nothing is written back.
    peek_only: bool,
    /// The node was created by this run.
    appeared: bool,
    /// A face's stage: a `Remember` write is a debug assertion, because a face has no state of its own.
    pub(crate) face: bool,
    /// The node's motions (`Animate`), which go with the node.
    pub(crate) motion: crate::runtime::motion::Motions,
}

/// A node's stage before its first run: [`MemoryStage::within`] gives it the window's context and its owner.
impl Default for MemoryStage {
    fn default() -> Self {
        MemoryStage {
            ctx: None,
            owner: Id::NULL,
            memory: Memory::default(),
            staged: Vec::new(),
            peek_only: false,
            appeared: false,
            face: false,
            motion: Default::default(),
        }
    }
}

impl MemoryStage {
    /// The window's context.
    #[doc(hidden)]
    pub fn ctx(&self) -> &Context {
        self.ctx.as_ref().expect("a memory stage is used inside a run, which gives it the window's context (MemoryStage::within)")
    }

    /// The same stage for a run of `owner`'s node in the window of `ctx`: a node's kept stage gets both at the start of
    /// every run.
    pub(crate) fn within(mut self, ctx: &Context, owner: Id) -> Self {
        self.ctx = Some(ctx.clone());
        self.owner = owner;
        self
    }

    /// A measuring run's stage over `base`, with no node behind it: nothing it changes is written back.
    pub(crate) fn open_peek(ctx: &Context, base: Id) -> Self {
        MemoryStage::open(ctx, base).peek_only()
    }

    /// Notes that the stage's component has a gesture running after its apply step, for `Ui::watch_gestures`. A
    /// measuring run's stage notes nothing.
    pub(crate) fn note_gesture_running(&self) {
        if !self.peek_only {
            self.ctx().memory().with(|memory| memory.edits.running += 1);
        }
    }

    /// The serial and reason of the latest cancel of every running edit (`Ui::cancel_edits`); serial 0 before the
    /// first. A gesture that began before the serial it last saw ends with the reason.
    pub(crate) fn cancel_all(&self) -> (u64, Option<CancelReason>) {
        self.ctx().memory().with(|memory| (memory.edits.cancel_serial, memory.edits.cancel_reason))
    }

    /// A stage over the node memory `memory` of the component `owner`; [`MemoryStage::finish`] hands it back.
    pub(crate) fn on_node(ctx: &Context, owner: Id, memory: Memory) -> Self {
        MemoryStage { memory, ..MemoryStage::default() }.within(ctx, owner)
    }

    /// A stage with no node behind it, for code that keeps session or saved slots only: its shown slots last for the
    /// stage alone.
    pub fn open(ctx: &Context, base: Id) -> Self {
        MemoryStage::on_node(ctx, base, Memory::default())
    }

    /// The same stage for a measuring run: nothing it changes is written back.
    pub(crate) fn peek_only(mut self) -> Self {
        self.peek_only = true;
        self
    }

    /// The same stage for the run that created its node (`appeared` answers true).
    pub(crate) fn appearing(mut self, appeared: bool) -> Self {
        self.appeared = appeared;
        self
    }

    /// Whether this stage is a measuring run's.
    pub fn is_peek(&self) -> bool {
        self.peek_only
    }

    /// The component the stage's node memory belongs to.
    pub(crate) fn owner(&self) -> Id {
        self.owner
    }

    /// The shown slot of type `T` under `id`, in the node's memory.
    pub fn shown<T: Slot>(&mut self, id: Id) -> &mut T {
        self.memory.shown::<T>(id)
    }

    /// The press slot of type `T` under `id`: the press machine clears it when its press ends.
    pub(crate) fn press<T: Slot>(&mut self, id: Id) -> &mut Option<T> {
        &mut self.memory.shown::<PressSlot<T>>(id).0
    }

    /// The shown slot of type `T` under `id`, moved out.
    pub(crate) fn take<T: Slot>(&mut self, id: Id) -> Option<T> {
        self.memory.take::<T>(id)
    }

    /// A copy of the shown slot of type `T` under `id`, or its default.
    pub fn recall<T: Slot>(&self, id: Id) -> T {
        self.memory.recall::<T>(id)
    }

    /// A copy of the press slot of type `T` under `id`.
    pub(crate) fn recall_press<T: Slot>(&self, id: Id) -> Option<T> {
        self.memory.recall::<PressSlot<T>>(id).0
    }

    fn staged<T: Slot>(&mut self, id: Id, saved: bool, load: impl FnOnce(&Context) -> (T, Option<Persist>)) -> &mut T {
        let ty = TypeId::of::<T>();
        let at = match self.staged.iter().position(|s| s.ty == ty && s.id == id && s.saved == saved) {
            Some(at) => at,
            None => {
                let (value, persist) = load(self.ctx());
                self.staged.push(Staged { ty, id, saved, value: Box::new(value), persist });
                self.staged.len() - 1
            }
        };
        self.staged[at].value.downcast_mut::<T>().expect("a staged slot holds the type it was staged with")
    }

    /// The session slot of type `T` under `id`: runtime-wide, it goes only through `forget`.
    pub(crate) fn session<T: Slot>(&mut self, id: Id) -> &mut T {
        self.staged(id, false, |ctx| (peek_tier::<T>(ctx, id, false), None))
    }

    /// The saved slot of `key`: loaded from the host's settings on first use, stored there by `finish` when its
    /// serialized form changed.
    pub fn saved<T: Savable>(&mut self, key: &SaveKey) -> &mut T {
        let id = key.id();
        let key = key.clone();
        self.staged(id, true, move |ctx| {
            let host = Env::host_of(ctx);
            let value: T = load_saved(ctx, &key, host.as_deref());
            let persist = host.map(|host| Persist { before: saved_form::<T>(&value), key, host, form: saved_form::<T>, store: store_saved::<T> });
            (value, persist)
        })
    }

    /// Writes the staged session and saved slots back to the runtime's store and the host (unless this is a measuring
    /// run's stage) and empties the stage of them; the node's memory stays. A node's kept stage is flushed at the end of
    /// each run, so the next run (in this window or another) reads the store again.
    pub(crate) fn flush(&mut self) {
        let staged = std::mem::take(&mut self.staged);
        if self.peek_only || staged.is_empty() {
            return;
        }
        // The host's settings store takes its own locks: values persist before the window's memory is borrowed.
        for staged in &staged {
            let Some(p) = &staged.persist else { continue };
            let after = (p.form)(&*staged.value);
            if after == p.before {
                continue;
            }
            match after {
                Some(_) => (p.store)(&*staged.value, &p.host, &p.key),
                None => forget_stored(&p.host, &p.key),
            }
        }
        // Replaced values drop outside the borrow: a slot's drop may reach the runtime's memory.
        let replaced: Vec<Box<dyn Any>> = self.ctx().memory().with(|host| {
            staged
                .into_iter()
                .filter_map(|staged| {
                    let map = if staged.saved { &mut host.saved } else { &mut host.session };
                    map.insert((staged.ty, staged.id), staged.value)
                })
                .collect()
        });
        drop(replaced);
    }

    /// [`MemoryStage::flush`], then hands back the node's memory.
    pub(crate) fn finish(mut self) -> Memory {
        self.flush();
        std::mem::take(&mut self.memory)
    }

    /// Writes the stage back, for a stage with no node: its shown slots go with it.
    pub fn write_back(self) {
        let _ = self.finish();
    }

    /// A copy of the session slot of type `T` under `id`, or its default.
    pub(crate) fn peek_session<T: Slot>(ctx: &Context, id: Id) -> T {
        peek_tier::<T>(ctx, id, false)
    }

    /// A copy of the saved value under `key`, loaded from the host on first use.
    pub fn peek_saved<T: Savable>(ctx: &Context, key: &SaveKey) -> T {
        let mut stage = MemoryStage::open(ctx, key.id());
        let value = stage.saved::<T>(key).clone();
        stage.write_back();
        value
    }
}

/// A copy of the window's session (`saved` false) or saved slot of type `T` under `id`, or its default.
fn peek_tier<T: Slot>(ctx: &Context, id: Id, saved: bool) -> T {
    ctx.memory().with(|host| {
        let map = if saved { &host.saved } else { &host.session };
        map.get(&(TypeId::of::<T>(), id)).and_then(|value| value.downcast_ref::<T>()).cloned().unwrap_or_default()
    })
}

/// The saved value of `key`: the window's copy, else the host's stored value (a value that no longer fits `T`, or
/// none, loads as `T::default()`). Without a host the tier is session only.
fn load_saved<T: Savable>(ctx: &Context, key: &SaveKey, host: Option<&Host>) -> T {
    let kept = ctx.memory().with(|memory| memory.saved.get(&(TypeId::of::<T>(), key.id())).and_then(|value| value.downcast_ref::<T>()).cloned());
    match (kept, host) {
        (Some(value), _) => value,
        (None, Some(host)) => Remembered::<T>::new(key.as_str()).recall(host).unwrap_or_default(),
        (None, None) => T::default(),
    }
}

/// Removes the session slots under `id` (all types). The only way a session slot goes.
pub(crate) fn forget(ctx: &Context, id: Id) {
    let removed: Vec<Box<dyn Any>> = ctx.memory().with(|host| {
        let keys: Vec<(TypeId, Id)> = host.session.keys().filter(|key| key.1 == id).copied().collect();
        keys.into_iter().filter_map(|key| host.session.remove(&key)).collect()
    });
    // The slots drop outside the borrow: a slot's drop may reach the window's memory.
    drop(removed);
}

/// Removes the saved slots of `key` and the value stored under it. The only way a saved slot goes.
pub(crate) fn forget_saved(ctx: &Context, key: &SaveKey) {
    let id = key.id();
    let removed: Vec<Box<dyn Any>> = ctx.memory().with(|host| {
        let keys: Vec<(TypeId, Id)> = host.saved.keys().filter(|k| k.1 == id).copied().collect();
        keys.into_iter().filter_map(|k| host.saved.remove(&k)).collect()
    });
    drop(removed);
    if let Some(host) = Env::host_of(ctx) {
        forget_stored(&host, key);
    }
}

/// How long a remembered value lives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lifetime {
    /// The value goes through the host's saved state (the `SaveKey` tier).
    #[default]
    Saved,
    /// The value lasts until the app quits.
    Run,
}

/// The run-only value under `key`, in the session tier. It never reaches the host. It gives the default until a value
/// is set.
pub fn run_value<T: Slot>(ctx: &Context, key: &SaveKey) -> T {
    MemoryStage::peek_session(ctx, global("craft_ui.run", key))
}

/// Sets the run-only value under `key`; see [`run_value`].
pub fn set_run_value<T: Slot>(ctx: &Context, key: &SaveKey, value: T) {
    with_session::<T, ()>(ctx, global("craft_ui.run", key), |slot| *slot = value);
}

/// The saved value under `key`, outside a dialog: an app reads one saved value through the same store the dialogs'
/// remembered fields use.
pub fn saved_value<T: Savable>(ctx: &Context, key: &SaveKey) -> T {
    MemoryStage::peek_saved(ctx, key)
}

/// Writes `value` as the saved value under `key`, outside a dialog. It is stored through the host when its
/// serialized form differs from the saved one.
pub fn set_saved_value<T: Savable>(ctx: &Context, key: &SaveKey, value: T) {
    let mut stage = MemoryStage::open(ctx, key.id());
    *stage.saved::<T>(key) = value;
    stage.write_back();
}

/// Runs `f` on the session slot of `id` for a service or pattern that has no component context: the slot is read,
/// changed and written back in one call, and it goes only through `forget`.
#[doc(hidden)]
pub fn with_session<T: Slot, R>(ctx: &Context, id: Id, f: impl FnOnce(&mut T) -> R) -> R {
    let mut stage = MemoryStage::open(ctx, id);
    let out = f(stage.session::<T>(id));
    stage.write_back();
    out
}

/// An id no other `source` shares: the id of something with a stable key and no component of its own.
#[doc(hidden)]
pub fn unique(source: impl std::hash::Hash + std::fmt::Debug) -> Id {
    Id::new(source)
}
pub(crate) mod sealed {
    /// Keeps `StageAccess` to the contexts of this crate.
    pub trait Sealed {}
}

/// The `Remember` calls are written once over this accessor. Sealed: only the pipeline's contexts implement it.
#[doc(hidden)]
pub trait StageAccess: sealed::Sealed {
    /// The open stage.
    fn stage(&self) -> &MemoryStage;
    /// The open stage, mutably.
    fn stage_mut(&mut self) -> &mut MemoryStage;
    /// The id this context keys its memory by: the component id for `Cx`, `sub(id, "body")` for a body, the
    /// item id for an item, the popup id for a popup body.
    fn base(&self) -> Id;
}

impl<X: StageAccess> Remember for X {
    fn key(&self, part: impl Hash + Debug) -> Id {
        self.base().with(part)
    }

    fn memory<T: Slot>(&mut self) -> &mut T {
        face_guard(self.stage());
        let id = self.base();
        self.stage_mut().shown(id)
    }

    fn memory_at<T: Slot>(&mut self, part: impl Hash + Debug) -> &mut T {
        face_guard(self.stage());
        let id = sub(self.base(), ("mem", part));
        self.stage_mut().shown(id)
    }

    fn take<T: Slot>(&mut self) -> Option<T> {
        face_guard(self.stage());
        let id = self.base();
        self.stage_mut().take(id)
    }

    fn press_slot<T: Slot>(&mut self) -> &mut Option<T> {
        face_guard(self.stage());
        let id = self.base();
        self.stage_mut().press(id)
    }

    fn keep<T: Slot>(&mut self) -> &mut T {
        face_guard(self.stage());
        let id = self.base();
        self.stage_mut().session(id)
    }

    fn saved<T: Savable>(&mut self, key: &SaveKey) -> &mut T {
        face_guard(self.stage());
        self.stage_mut().saved(key)
    }

    fn recall<T: Slot>(&self) -> T {
        self.stage().recall(self.base())
    }

    fn appeared(&self) -> bool {
        self.stage().appeared
    }
}

fn face_guard(stage: &MemoryStage) {
    debug_assert!(!stage.face, "a face writes no memory: the composite owns the state a face shows");
}

/// The tag type of runtime sub-ids. Its `Hash` writes a private `TypeId` first, so no `cx.key(k)`, child id or
/// app-chosen key can equal a runtime sub-id.
#[derive(Debug)]
struct Sub<T>(T);

impl<T: Hash> Hash for Sub<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        TypeId::of::<Sub<()>>().hash(state);
        self.0.hash(state);
    }
}

/// A salt for an id made from `key`, a value that is only `Hash`: the ids egui makes also ask for `Debug`, which an
/// app's key type need not have. Equal keys give equal salts within a run, so the id stays the same from pass to pass.
pub fn key_salt(key: &impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// The runtime sub-id of `base` for `tag`: the one derivation of every id the runtime makes from a component
/// id (body, children, header, footer, layer, rows, items, popups, the gesture, `memory_at` slots, animation
/// slots, edge and transform widgets, text and access sub-nodes).
#[doc(hidden)]
pub fn sub(base: Id, tag: impl Hash + Debug) -> Id {
    base.with(Sub(tag))
}

/// The id of a runtime singleton named `name`, optionally under a `scope` such as a window.
#[doc(hidden)]
pub fn global(name: &'static str, scope: impl Hash + Debug) -> Id {
    unique(name).with(scope)
}
