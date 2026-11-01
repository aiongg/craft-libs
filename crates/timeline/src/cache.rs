//! A byte-budgeted cache of rendered frames that spares the frames a playing preview shows next.
//!
//! Plain least-recently-used eviction drops the frames just ahead of the playhead first (rendered
//! a while ago, not shown yet) to keep the ones just shown, so a preview longer than the cache
//! plays into a gap that prefetch keeps refilling (EffectCraft #595). This cache evicts, in order:
//! frames at another scale than the one on screen, frames the playing preview never reaches, the
//! least recently shown; while a preview plays, the frame it reaches last.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Arc;

/// The memory a cached frame holds.
pub trait FrameSize {
    fn bytes(&self) -> usize;
}

/// The app's frame identity: what was rendered (a sequence and its revision, a comp's content
/// hash, a media item), at which frame, scale and options.
pub trait FrameCacheKey: Clone + Eq + Hash {
    /// The frame number.
    fn frame(&self) -> i64;

    /// The same key at another frame (the frames playback asks for next).
    fn at_frame(&self, frame: i64) -> Self;

    /// Whether `other` is a frame of the same picture series: everything equal but the frame.
    fn same_series(&self, other: &Self) -> bool;

    /// Whether `other` is at the same output scale (a zoom or resolution change makes every other
    /// scale's frames the first to go). Default: always.
    fn same_scale(&self, _other: &Self) -> bool {
        true
    }
}

/// Where a playing preview is and what it plays next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePlayhead<K> {
    /// A key of the series that plays; its frame is ignored.
    pub series: K,
    /// The frame on screen.
    pub current: i64,
    /// The range the preview plays, inclusive.
    pub start: i64,
    pub end: i64,
    /// Frames advanced per shown frame (the skip of a RAM preview, at least 1).
    pub step: i64,
    pub looping: bool,
}

impl<K: FrameCacheKey> FramePlayhead<K> {
    /// How many frames the preview shows before it reaches `key` (0: on screen); `None` for a
    /// frame it never reaches (another series, outside the range, behind it without looping).
    /// Looping, a frame behind it is reached after the range's end and around from its start.
    // effectcraft: crates/ui-egui/src/frames.rs:132 `Playhead::ahead`.
    pub fn ahead(&self, key: &K) -> Option<i64> {
        todo!()
    }
}

/// Frames by key within a byte budget. Not thread-safe by itself; [`crate::FrameServer`] keeps
/// one behind a mutex.
pub struct FrameCache<K, V> {
    map: HashMap<K, Arc<V>>,
    /// Least recently shown first.
    order: VecDeque<K>,
    bytes: usize,
    budget: usize,
    playhead: Option<FramePlayhead<K>>,
    on_screen: Option<K>,
}

impl<K: FrameCacheKey, V: FrameSize> FrameCache<K, V> {
    /// An empty cache holding up to `budget` bytes.
    pub fn new(budget: usize) -> Self {
        Self { map: HashMap::new(), order: VecDeque::new(), bytes: 0, budget, playhead: None, on_screen: None }
    }

    pub fn budget(&self) -> usize {
        self.budget
    }

    /// The bytes held now.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// A new budget (Settings › Memory); evicts down to it at once.
    pub fn set_budget(&mut self, bytes: usize) {
        todo!()
    }

    /// Keep `frame` under `key`, replacing any frame there. Past the budget, frames are evicted
    /// in the module's order until the cache holds 80 % of its budget, so a full cache doesn't
    /// evict on every insert (FilmCraft `frames.rs:389`). A frame larger than the whole budget is
    /// not kept.
    // effectcraft: crates/ui-egui/src/frames.rs:161 `insert`, `:169` `victim`, `:249`
    // `evict_to_budget`; filmcraft: crates/ui-egui/src/frames.rs:389 (the 80 % mark).
    pub fn insert(&mut self, key: K, frame: Arc<V>) {
        todo!()
    }

    /// The frame under `key`, marked as just shown (last in line for eviction).
    // effectcraft: crates/ui-egui/src/frames.rs:225 `touch`.
    pub fn get(&mut self, key: &K) -> Option<Arc<V>> {
        todo!()
    }

    /// Whether a frame is cached, without touching it.
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// The nearest earlier frame of `key`'s series within `max_back` frames (the exact one
    /// first), touched: a monitor holds it while the exact frame renders, so scrubbing never
    /// flashes black.
    // filmcraft: crates/ui-egui/src/frames.rs:727 `nearest`.
    pub fn nearest(&mut self, key: &K, max_back: i64) -> Option<(K, Arc<V>)> {
        todo!()
    }

    /// The cached frame numbers of `series`, sorted: the timeline draws them as the cache bar.
    // effectcraft: crates/ui-egui/src/frames.rs:811 `cached_frames`.
    pub fn cached_frames(&self, series: &K) -> Vec<i64> {
        todo!()
    }

    /// The playing preview, or `None` when playback stops.
    pub fn set_playhead(&mut self, playhead: Option<FramePlayhead<K>>) {
        self.playhead = playhead;
    }

    /// The frame on screen now (the newest urgent request): frames at other scales go first.
    pub fn set_on_screen(&mut self, key: Option<K>) {
        self.on_screen = key;
    }

    /// Drop every frame `drop` accepts (a revision the document left, a closed item).
    pub fn remove_where(&mut self, drop: impl Fn(&K) -> bool) {
        todo!()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.bytes = 0;
    }
}
