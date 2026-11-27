//! State cells with read tracking. A read inside a phase records (node, phase) in the cell's
//! reader set; a write bumps the cell's version and marks exactly those pairs dirty. Readers
//! re-register when they run again, so a write clears the set.

use std::any::Any;
use std::collections::HashSet;
use std::hash::BuildHasherDefault;
use std::marker::PhantomData;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::id::{NodeId, StableHasher};

/// The phase a read happened in, which is the phase a write reruns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Build,
    Layout,
    Paint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Reader {
    Node(NodeId, Phase),
    /// A derived cell by slot index and generation.
    Derived(u32, u32),
}

/// The readers of one cell, each once, in the order they first read. A cell every node reads
/// (the scale, the tokens) has thousands, so past a few the membership test is a hash lookup.
#[derive(Default)]
struct Readers {
    list: SmallVec<[Reader; 4]>,
    /// Every reader in `list`, kept once `list` outgrows a linear scan.
    set: HashSet<Reader, BuildHasherDefault<StableHasher>>,
}

impl Readers {
    const SCAN: usize = 8;

    fn insert(&mut self, reader: Reader) {
        if self.list.len() < Self::SCAN {
            if !self.list.contains(&reader) {
                self.list.push(reader);
            }
            return;
        }
        if self.set.is_empty() {
            self.set.extend(self.list.iter().copied());
        }
        if self.set.insert(reader) {
            self.list.push(reader);
        }
    }

    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn clear(&mut self) {
        self.list.clear();
        self.set.clear();
    }

    fn take(&mut self) -> SmallVec<[Reader; 4]> {
        self.set.clear();
        std::mem::take(&mut self.list)
    }
}

type Compute = Rc<dyn Fn(&mut DeriveCx<'_>) -> Box<dyn Any>>;

struct DeriveInfo {
    compute: Compute,
    stale: bool,
    /// (slot, generation, version) of each input at the last compute.
    inputs: SmallVec<[(u32, u32, u64); 4]>,
}

struct Slot {
    generation: u32,
    version: u64,
    value: Option<Box<dyn Any>>,
    readers: Readers,
    derive: Option<DeriveInfo>,
}

/// The arena every cell lives in. Writes queue node marks here; the runtime applies them to the
/// tree before each pass.
#[derive(Default)]
pub struct Store {
    slots: Vec<Slot>,
    free: Vec<u32>,
    clock: u64,
    pub(crate) pending: Vec<(NodeId, Phase)>,
}

/// A handle to one cell. Copy; the value stays in the [`Store`].
pub struct State<T> {
    index: u32,
    generation: u32,
    _t: PhantomData<fn() -> T>,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for State<T> {}

impl<T> PartialEq for State<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> std::fmt::Debug for State<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "State({}v{})", self.index, self.generation)
    }
}

/// A cell computed from other cells. It is marked stale when an input is written and recomputes
/// on its next read; its readers are marked at the write, as for a plain cell.
pub struct Derived<T>(State<T>);

impl<T> Clone for Derived<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Derived<T> {}

/// Store access for a read, with the reader to record, if any.
pub struct Access<'a> {
    pub(crate) store: &'a mut Store,
    pub(crate) reader: Option<Reader>,
}

/// A context that can read cells. Build, layout and paint contexts record their node and phase.
pub trait Reads {
    fn access(&mut self) -> Access<'_>;
}

/// A context that can write cells. Paint cannot: a paint pass never changes state.
pub trait Writes {
    fn store_mut(&mut self) -> &mut Store;
}

/// Reads through the store directly record nothing: the app between frames, event handlers that
/// only look.
impl Reads for Store {
    fn access(&mut self) -> Access<'_> {
        Access { store: self, reader: None }
    }
}

impl Writes for Store {
    fn store_mut(&mut self) -> &mut Store {
        self
    }
}

/// The context a derived cell computes in: its reads become its inputs.
pub struct DeriveCx<'a> {
    store: &'a mut Store,
    me: Reader,
}

impl Reads for DeriveCx<'_> {
    fn access(&mut self) -> Access<'_> {
        Access { store: self.store, reader: Some(self.me) }
    }
}

impl Store {
    /// A new cell with no owner; it lives until [`Store::remove`].
    pub fn create<T: 'static>(&mut self, value: T) -> State<T> {
        let (index, generation) = self.alloc(Some(Box::new(value)), None);
        State { index, generation, _t: PhantomData }
    }

    /// A derived cell. `compute` reads its inputs through the context it is given.
    pub fn derive<T: 'static>(&mut self, compute: impl Fn(&mut DeriveCx<'_>) -> T + 'static) -> Derived<T> {
        let compute: Compute = Rc::new(move |cx| Box::new(compute(cx)));
        let info = DeriveInfo { compute, stale: true, inputs: SmallVec::new() };
        let (index, generation) = self.alloc(None, Some(info));
        Derived(State { index, generation, _t: PhantomData })
    }

    pub fn remove<T>(&mut self, state: State<T>) {
        self.free_slot(state.index, state.generation);
    }

    /// The cell's value without recording a read.
    pub fn peek<T: 'static>(&self, state: State<T>) -> &T {
        self.value(state.index, state.generation)
    }

    /// Whether anything reads the cell now. The run loop uses it for the frame clock: only
    /// frames that something animates from need a time.
    pub fn has_readers<T>(&self, state: State<T>) -> bool {
        self.slot(state.index, state.generation).is_some_and(|s| !s.readers.is_empty())
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    fn alloc(&mut self, value: Option<Box<dyn Any>>, derive: Option<DeriveInfo>) -> (u32, u32) {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = value;
            slot.derive = derive;
            slot.version = 0;
            (index, slot.generation)
        } else {
            let index = u32::try_from(self.slots.len()).unwrap_or(u32::MAX);
            self.slots.push(Slot { generation: 0, version: 0, value, readers: Readers::default(), derive });
            (index, 0)
        }
    }

    pub(crate) fn free_slot(&mut self, index: u32, generation: u32) {
        let Some(slot) = self.slots.get_mut(index as usize).filter(|s| s.generation == generation) else {
            return;
        };
        slot.generation = slot.generation.wrapping_add(1);
        slot.value = None;
        slot.derive = None;
        slot.readers.clear();
        self.free.push(index);
    }

    fn slot(&self, index: u32, generation: u32) -> Option<&Slot> {
        self.slots.get(index as usize).filter(|s| s.generation == generation)
    }

    fn slot_mut(&mut self, index: u32, generation: u32) -> &mut Slot {
        match self.slots.get_mut(index as usize) {
            Some(s) if s.generation == generation => s,
            _ => panic!("state cell {index} used after its owner dropped it"),
        }
    }

    fn value<T: 'static>(&self, index: u32, generation: u32) -> &T {
        match self.slot(index, generation).and_then(|s| s.value.as_deref()).and_then(|v| v.downcast_ref::<T>()) {
            Some(v) => v,
            None => panic!("state cell {index} read after its owner dropped it"),
        }
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// Records `reader` and brings a derived cell up to date.
    fn read(&mut self, index: u32, generation: u32, reader: Option<Reader>) {
        self.refresh(index, generation);
        let version = self.slot_mut(index, generation).version;
        let Some(reader) = reader else { return };
        let slot = self.slot_mut(index, generation);
        slot.readers.insert(reader);
        if let Reader::Derived(d, g) = reader
            && let Some(info) = self.slots.get_mut(d as usize).filter(|s| s.generation == g).and_then(|s| s.derive.as_mut())
        {
            info.inputs.push((index, generation, version));
        }
    }

    fn refresh(&mut self, index: u32, generation: u32) {
        let slot = self.slot_mut(index, generation);
        let Some(info) = slot.derive.as_ref() else {
            return;
        };
        let mut stale = info.stale || slot.value.is_none();
        if !stale {
            let inputs = info.inputs.clone();
            for (i, g, v) in inputs {
                if self.slot(i, g).is_none() {
                    stale = true;
                    break;
                }
                self.refresh(i, g);
                if self.slot(i, g).is_none_or(|s| s.version != v) {
                    stale = true;
                    break;
                }
            }
        }
        if !stale {
            return;
        }
        let slot = self.slot_mut(index, generation);
        let Some(info) = slot.derive.as_mut() else {
            return;
        };
        info.inputs.clear();
        let compute = info.compute.clone();
        let value = compute(&mut DeriveCx { store: self, me: Reader::Derived(index, generation) });
        let version = self.tick();
        let slot = self.slot_mut(index, generation);
        slot.value = Some(value);
        slot.version = version;
        if let Some(info) = slot.derive.as_mut() {
            info.stale = false;
        }
    }

    /// Bumps the version and marks every reader.
    fn touch(&mut self, index: u32, generation: u32) {
        let version = self.tick();
        let slot = self.slot_mut(index, generation);
        slot.version = version;
        self.invalidate(index);
    }

    fn invalidate(&mut self, index: u32) {
        let readers = self.slots[index as usize].readers.take();
        for r in readers {
            match r {
                Reader::Node(node, phase) => self.pending.push((node, phase)),
                Reader::Derived(d, g) => {
                    if let Some(info) = self.slots.get_mut(d as usize).filter(|s| s.generation == g).and_then(|s| s.derive.as_mut()) {
                        info.stale = true;
                        self.invalidate(d);
                    }
                }
            }
        }
    }
}

impl<T: 'static> State<T> {
    /// The value, recording the read for the context's node and phase.
    pub fn get(self, cx: &mut impl Reads) -> T
    where
        T: Clone,
    {
        self.with(cx, T::clone)
    }

    pub fn with<R>(self, cx: &mut impl Reads, f: impl FnOnce(&T) -> R) -> R {
        let Access { store, reader } = cx.access();
        store.read(self.index, self.generation, reader);
        f(store.value(self.index, self.generation))
    }

    pub fn set(self, cx: &mut impl Writes, value: T) {
        self.update(cx, |v| *v = value);
    }

    /// Writes only when the value differs, so readers of an unchanged cell stay clean.
    pub fn set_if_changed(self, cx: &mut impl Writes, value: T)
    where
        T: PartialEq,
    {
        let store = cx.store_mut();
        if *store.value::<T>(self.index, self.generation) != value {
            self.set(store, value);
        }
    }

    pub fn update(self, cx: &mut impl Writes, f: impl FnOnce(&mut T)) {
        let store = cx.store_mut();
        let slot = store.slot_mut(self.index, self.generation);
        match slot.value.as_deref_mut().and_then(|v| v.downcast_mut::<T>()) {
            Some(v) => f(v),
            None => panic!("state cell {} written after its owner dropped it", self.index),
        }
        store.touch(self.index, self.generation);
    }

    /// The version, which moves on every write.
    pub fn version(self, store: &Store) -> u64 {
        store.slot(self.index, self.generation).map_or(0, |s| s.version)
    }

    pub(crate) fn slot_ref(self) -> (u32, u32) {
        (self.index, self.generation)
    }
}

impl<T: 'static> Derived<T> {
    pub fn get(self, cx: &mut impl Reads) -> T
    where
        T: Clone,
    {
        self.0.get(cx)
    }

    pub fn with<R>(self, cx: &mut impl Reads, f: impl FnOnce(&T) -> R) -> R {
        self.0.with(cx, f)
    }

    pub fn remove(self, store: &mut Store) {
        store.remove(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readers_keep_first_read_order_and_each_reader_once() {
        let mut readers = Readers::default();
        let reader = |i: u64| Reader::Node(NodeId(i), Phase::Paint);
        for round in 0..3 {
            for i in 0..100 {
                readers.insert(reader(i));
            }
            assert_eq!(readers.list.len(), 100, "round {round}");
        }
        assert!(readers.list.iter().enumerate().all(|(i, r)| *r == reader(i as u64)));
        assert_eq!(readers.take().len(), 100);
        readers.insert(reader(7));
        assert_eq!(readers.list.len(), 1);
    }
}
