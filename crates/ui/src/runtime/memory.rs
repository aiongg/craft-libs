//! Ids and memory: `Remember`, the memory stage, `Facts`, eviction and saved state.

use crate::runtime::egui::data::Service;
use crate::runtime::egui::memory::{self as store, Removed, take_raw};
use crate::runtime::egui::pass;
use crate::runtime::frame::PerViewport;
use egui::util::IdTypeMap;
use egui::{Context, Id, ViewportId};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A value kept in memory. `Clone` is required because egui's temp store clones on insert; large caches hold `Arc`s.
pub trait Slot: Clone + Default + Send + Sync + 'static {}
impl<T: Clone + Default + Send + Sync + 'static> Slot for T {}

/// A slot that also lives in saved state.
pub trait Savable: Slot + Serialize + DeserializeOwned {}
impl<T: Slot + Serialize + DeserializeOwned> Savable for T {}

/// The key of a saved slot, such as `"craft_dialogs.export.place"`. The crate prefix is checked, and the key
/// is never derived from an `Id`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SaveKey(Cow<'static, str>);

impl SaveKey {
    /// A key; evaluated in a const context it rejects a name without a crate prefix at compile time.
    pub const fn new(key: &'static str) -> Self {
        let bytes = key.as_bytes();
        let mut i = 0;
        let mut prefixed = false;
        while i < bytes.len() {
            if bytes[i] == b'.' {
                prefixed = true;
            }
            i += 1;
        }
        assert!(prefixed, "a SaveKey needs a crate prefix");
        SaveKey(Cow::Borrowed(key))
    }

    /// The key's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
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
    /// The saved slot of `key`: it goes only through `forget_saved`.
    fn saved<T: Savable>(&mut self, key: &SaveKey) -> &mut T;
    /// A read-only copy of the shown slot, for paint.
    fn recall<T: Slot>(&self) -> T;
    /// True on the first pass a shown stage exists for this id.
    fn appeared(&self) -> bool;
}

mod tier {
    pub(super) const SHOWN: u8 = 0;
    pub(super) const PRESS: u8 = 1;
    pub(super) const SESSION: u8 = 2;
    pub(super) const SAVED: u8 = 3;
    pub(super) const ONESHOT: u8 = 4;
}
use tier::{ONESHOT, PRESS, SAVED, SESSION, SHOWN};

/// One lifetime of one type under one id, so a type can sit in two lifetimes.
#[derive(Clone, Default)]
struct Wrapped<const TIER: u8, T>(T);

/// Moves one typed slot out of the map; the caller drops the value outside every lock.
type Remover = fn(&mut IdTypeMap, Id) -> Option<Removed>;

fn remove_slot<const TIER: u8, T: Slot>(data: &mut IdTypeMap, id: Id) -> Option<Removed> {
    take_raw::<Wrapped<TIER, T>>(data, id)
}

struct Entry {
    value: Box<dyn Any + Send + Sync>,
    write_back: fn(Box<dyn Any + Send + Sync>, &Context, Id),
    remove: Remover,
    tier: u8,
    ty: TypeId,
    /// A press entry whose press ended is dropped at write-back instead of kept.
    ended: Option<fn(&(dyn Any + Send + Sync)) -> bool>,
}

/// The last registration of an owner: the component id registered at step 3 whose pipeline opened a stage.
struct OwnerRecord {
    viewport: ViewportId,
    pass: u64,
    at: f64,
}

/// One entry the runtime keeps for a component, and the owner that is tested for its eviction.
struct SlotRecord {
    owner: Id,
    remove: Remover,
}

#[derive(Default)]
struct Registry {
    owners: HashMap<Id, OwnerRecord>,
    /// Keyed by `(slot id, tier, type)`.
    slots: HashMap<(Id, u8, TypeId), SlotRecord>,
    /// Evicted slots, moved out inside the end hook; the first pipeline run of the next pass drops them.
    graveyard: Vec<Removed>,
}

/// The ids claimed as the base of an open stage, per viewport and its pass number.
#[derive(Default)]
struct BaseClaims(HashMap<ViewportId, (u64, HashSet<Id>)>);

/// Marks the bases whose stage was written back, for `appeared`.
#[derive(Clone)]
struct Seen;

/// Eviction, run by the end hook. Only an owner's registration is tested: an owner not registered for 3
/// passes of the viewport that last showed it and 0.5 s loses its shown and one-shot entries (its press
/// entries went when their press ended). Session and saved entries go only through `forget` and
/// `forget_saved`. A viewport egui no longer reports counts as 3 passes elapsed.
#[doc(hidden)]
pub fn evict(ctx: &Context) {
    let now = pass::now(ctx);
    let live = pass::live_viewports(ctx);
    let registry = Service::<Registry>::of(ctx);
    registry.with(|reg| {
        let due = |r: &OwnerRecord| now - r.at >= 0.5 && (!live.contains(&r.viewport) || pass::pass_nr(ctx, r.viewport) >= r.pass + 3);
        let dead: HashSet<Id> = reg.owners.iter().filter(|(_, r)| due(r)).map(|(id, _)| *id).collect();
        if dead.is_empty() {
            return;
        }
        reg.owners.retain(|id, _| !dead.contains(id));
        let gone: Vec<(Id, u8, TypeId)> =
            reg.slots.iter().filter(|(k, r)| dead.contains(&r.owner) && matches!(k.1, SHOWN | ONESHOT | PRESS)).map(|(k, _)| *k).collect();
        let mut moved: Vec<Removed> = Vec::new();
        for key in gone {
            if let Some(rec) = reg.slots.remove(&key) {
                moved.extend(ctx.data_mut(|data| (rec.remove)(data, key.0)));
            }
        }
        reg.graveyard.extend(moved);
    });
}

/// Removes the session slots under `id` (all types). The only way a session slot goes.
pub fn forget(ctx: &Context, id: Id) {
    forget_tier(ctx, id, SESSION);
}

/// Removes the saved slots under `id`. The only way a saved slot goes.
pub fn forget_saved(ctx: &Context, id: Id) {
    forget_tier(ctx, id, SAVED);
}

fn forget_tier(ctx: &Context, id: Id, tier: u8) {
    let moved = Service::<Registry>::of(ctx).with(|reg| {
        let keys: Vec<(Id, u8, TypeId)> = reg.slots.keys().filter(|k| k.0 == id && k.1 == tier).copied().collect();
        let mut moved: Vec<Removed> = Vec::new();
        for key in keys {
            if let Some(rec) = reg.slots.remove(&key) {
                moved.extend(ctx.data_mut(|data| (rec.remove)(data, key.0)));
            }
        }
        moved
    });
    drop(moved);
}

/// Registers a slot the motion service wrote straight into context data, so it goes with its owner's shown entries.
#[doc(hidden)]
pub fn adopt<T: Any + Clone + Send + Sync>(ctx: &Context, owner: Id, id: Id) {
    fn remove<T: Any + Send + Sync>(data: &mut IdTypeMap, id: Id) -> Option<Removed> {
        take_raw::<T>(data, id)
    }
    Service::<Registry>::of(ctx).with(|reg| {
        reg.slots.insert((id, SHOWN, TypeId::of::<T>()), SlotRecord { owner, remove: remove::<T> });
    });
}

/// Drops the evicted slots, outside every lock and hook, so a slot's `Drop` may touch the `Context`.
#[doc(hidden)]
pub fn dispose_evicted(ctx: &Context) {
    let queue = Service::<Registry>::of(ctx).with(|reg| std::mem::take(&mut reg.graveyard));
    drop(queue);
}

fn write_back<const TIER: u8, T: Slot>(value: Box<dyn Any + Send + Sync>, ctx: &Context, id: Id) {
    if let Ok(wrapped) = value.downcast::<Wrapped<TIER, T>>() {
        store::put(ctx, id, *wrapped);
    }
}

fn press_ended<T: Slot>(value: &(dyn Any + Send + Sync)) -> bool {
    value.downcast_ref::<Wrapped<PRESS, Option<T>>>().is_some_and(|wrapped| wrapped.0.is_none())
}

/// Slots move into the stage on first use and back after `finish` returns.
#[doc(hidden)]
pub struct MemoryStage {
    ctx: Context,
    /// The component id registered at step 3 whose pipeline opened this stage; eviction tests only it.
    owner: Id,
    entries: HashMap<(u8, TypeId, Id), Entry>,
    /// Base ids whose `appeared()` was asked this pass; they are marked seen at write-back.
    touched: RefCell<Vec<Id>>,
    /// Bases this stage claimed (the component's own, its body's, its items').
    claimed: RefCell<Vec<Id>>,
    /// A sizing pass or `measure_in`: reads clone from context data, nothing moves, nothing is written back.
    peek_only: bool,
    /// A face's stage: a `Remember` write is a debug assertion, because a face has no state of its own.
    pub face: bool,
}

impl MemoryStage {
    /// A stage whose entries belong to `base`.
    pub fn open(ctx: &Context, base: Id) -> Self {
        Self::open_owned(ctx, base, base)
    }

    /// A stage whose entries belong to `owner`: a popup body's stage belongs to the component that shows it.
    pub fn open_owned(ctx: &Context, base: Id, owner: Id) -> Self {
        let stage = Self::new(ctx, base, owner, false);
        stage.claim(base);
        stage
    }

    /// The stage of a sizing pass: it opens no lifetime stage, so a container's first appearance in a new
    /// layer cannot lose a kept value.
    pub fn open_peek(ctx: &Context, base: Id) -> Self {
        Self::new(ctx, base, base, true)
    }

    fn new(ctx: &Context, base: Id, owner: Id, peek_only: bool) -> Self {
        MemoryStage {
            ctx: ctx.clone(),
            owner,
            entries: HashMap::new(),
            touched: RefCell::new(vec![base]),
            claimed: RefCell::new(Vec::new()),
            peek_only,
            face: false,
        }
    }

    /// Debug assertion: no id is the base of two open stages in one pass of its viewport.
    pub fn claim(&self, base: Id) {
        if self.peek_only || self.claimed.borrow().contains(&base) {
            return;
        }
        self.claimed.borrow_mut().push(base);
        let viewport = pass::current_viewport(&self.ctx);
        let pass_nr = pass::pass_nr(&self.ctx, viewport);
        let duplicate = Service::<BaseClaims>::of(&self.ctx).with(|claims| {
            let slot = claims.0.entry(viewport).or_insert_with(|| (pass_nr, HashSet::new()));
            if slot.0 != pass_nr {
                *slot = (pass_nr, HashSet::new());
            }
            !slot.1.insert(base)
        });
        debug_assert!(!duplicate, "an id is the base of two open stages in one pass");
    }

    /// Read-only access for `measure_in`: opens no stage.
    pub fn peek<T: Slot>(ctx: &Context, id: Id) -> T {
        store::peek_clone::<Wrapped<SHOWN, T>>(ctx, id).map(|wrapped| wrapped.0).unwrap_or_default()
    }

    pub fn owner(&self) -> Id {
        self.owner
    }

    /// True for the first pass a base id is shown; per base id, so a body and its component differ.
    pub fn appeared_at(&self, base: Id) -> bool {
        let mut touched = self.touched.borrow_mut();
        if !touched.contains(&base) {
            touched.push(base);
        }
        !store::contains::<Seen>(&self.ctx, base)
    }

    #[allow(clippy::unreachable, reason = "the entry key holds the lifetime and the type, so the downcast cannot fail")]
    fn slot<const TIER: u8, T: Slot>(&mut self, id: Id) -> &mut T {
        let ctx = &self.ctx;
        let peek = self.peek_only;
        let entry = self.entries.entry((TIER, TypeId::of::<T>(), id)).or_insert_with(|| {
            let loaded: Wrapped<TIER, T> = if peek {
                store::peek_clone::<Wrapped<TIER, T>>(ctx, id).unwrap_or_default()
            } else {
                store::take::<Wrapped<TIER, T>>(ctx, id).unwrap_or_default()
            };
            Entry {
                value: Box::new(loaded),
                write_back: write_back::<TIER, T>,
                remove: remove_slot::<TIER, T>,
                tier: TIER,
                ty: TypeId::of::<T>(),
                ended: None,
            }
        });
        match entry.value.downcast_mut::<Wrapped<TIER, T>>() {
            Some(wrapped) => &mut wrapped.0,
            None => unreachable!("an entry's value has the type its key names"),
        }
    }

    pub fn shown<T: Slot>(&mut self, id: Id) -> &mut T {
        self.slot::<SHOWN, T>(id)
    }

    pub fn session<T: Slot>(&mut self, id: Id) -> &mut T {
        self.slot::<SESSION, T>(id)
    }

    pub fn press<T: Slot>(&mut self, id: Id) -> &mut Option<T> {
        self.slot::<PRESS, Option<T>>(id);
        if let Some(entry) = self.entries.get_mut(&(PRESS, TypeId::of::<Option<T>>(), id)) {
            entry.ended = Some(press_ended::<T>);
        }
        self.slot::<PRESS, Option<T>>(id)
    }

    pub fn saved<T: Slot>(&mut self, id: Id) -> &mut T {
        self.slot::<SAVED, T>(id)
    }

    pub fn take<T: Slot>(&mut self, id: Id) -> Option<T> {
        self.slot::<ONESHOT, Option<T>>(id).take()
    }

    pub fn recall<T: Slot>(&self, id: Id) -> T {
        let open = self.entries.get(&(SHOWN, TypeId::of::<T>(), id)).and_then(|entry| entry.value.downcast_ref::<Wrapped<SHOWN, T>>());
        match open {
            Some(wrapped) => wrapped.0.clone(),
            None => store::peek_clone::<Wrapped<SHOWN, T>>(&self.ctx, id).map(|wrapped| wrapped.0).unwrap_or_default(),
        }
    }

    /// Step 17, after `finish` returned: every slot goes back to context data and the owner's registration is
    /// recorded for eviction.
    pub fn write_back(self) {
        if self.peek_only {
            return;
        }
        let viewport = pass::current_viewport(&self.ctx);
        let (pass_nr, now) = (pass::pass_nr(&self.ctx, viewport), pass::now(&self.ctx));
        let owner = self.owner;
        let touched = self.touched.into_inner();
        let mut ended = Vec::new();
        Service::<Registry>::of(&self.ctx).with(|reg| {
            reg.owners.insert(owner, OwnerRecord { viewport, pass: pass_nr, at: now });
            for base in &touched {
                reg.slots.insert((*base, SHOWN, TypeId::of::<Seen>()), SlotRecord { owner, remove: take_raw::<Seen> });
            }
            for ((_, _, id), entry) in &self.entries {
                let key = (*id, entry.tier, entry.ty);
                if entry.ended.is_some_and(|is_ended| is_ended(&*entry.value)) {
                    reg.slots.remove(&key);
                    ended.push(key);
                } else {
                    reg.slots.insert(key, SlotRecord { owner, remove: entry.remove });
                }
            }
        });
        for ((tier, ty, id), entry) in self.entries {
            // An ended press leaves no entry behind; its value drops here, under no lock.
            if ended.contains(&(id, tier, ty)) {
                continue;
            }
            (entry.write_back)(entry.value, &self.ctx, id);
        }
        for base in touched {
            store::put(&self.ctx, base, Seen);
        }
    }
}

/// The `Remember` calls are written once over this accessor.
#[doc(hidden)]
pub trait StageAccess {
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
        self.stage_mut().saved(Id::unique(key.as_str()))
    }

    fn recall<T: Slot>(&self) -> T {
        self.stage().recall(self.base())
    }

    fn appeared(&self) -> bool {
        self.stage().appeared_at(self.base())
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

/// The runtime sub-id of `base` for `tag`: the one derivation of every id the runtime makes from a component
/// id (body, children, header, footer, layer, rows, items, popups, the gesture, `memory_at` slots, animation
/// slots, edge and transform widgets, text and access sub-nodes).
#[doc(hidden)]
pub fn sub(base: Id, tag: impl Hash + Debug) -> Id {
    base.with(Sub(tag))
}

/// The id of a runtime singleton named `name`, optionally under a `scope` such as a viewport.
#[doc(hidden)]
pub fn global(name: &'static str, scope: impl Hash + Debug) -> Id {
    Id::unique(name).with(scope)
}

/// Cross-widget facts, registered in one pass and read in the next. Runtime services only.
pub struct Facts<T>(PhantomData<T>);

/// The facts of one viewport: this pass's registrations and last pass's.
struct FactRecord<T> {
    pass: u64,
    this: Vec<T>,
    last: Vec<T>,
}

impl<T> Default for FactRecord<T> {
    fn default() -> Self {
        FactRecord { pass: 0, this: Vec::new(), last: Vec::new() }
    }
}

impl<T: Clone + Send + Sync + 'static> Facts<T> {
    /// Registers `fact` for the next pass of the running viewport.
    #[doc(hidden)]
    pub fn register(ctx: &Context, fact: T) {
        Self::with_record(ctx, |record| record.this.push(fact));
    }

    /// The facts the running viewport's previous pass registered, in registration order.
    #[doc(hidden)]
    pub fn of_last_pass(ctx: &Context) -> Vec<T> {
        Self::with_record(ctx, |record| record.last.clone())
    }

    fn with_record<R>(ctx: &Context, f: impl FnOnce(&mut FactRecord<T>) -> R) -> R {
        let viewport = pass::current_viewport(ctx);
        let pass_nr = pass::pass_nr(ctx, viewport);
        PerViewport::<FactRecord<T>>::with(ctx, viewport, |record| {
            if record.pass != pass_nr {
                if record.pass + 1 == pass_nr {
                    record.last = std::mem::take(&mut record.this);
                } else {
                    record.last.clear();
                    record.this.clear();
                }
                record.pass = pass_nr;
            }
            f(record)
        })
    }
}
