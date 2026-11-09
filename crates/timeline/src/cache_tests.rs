//! The frame cache.

use std::sync::Arc;

use crate::cache::{FrameCache, FrameCacheKey, FramePlayhead, FrameSize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TestKey {
    pub series: u8,
    pub frame: i64,
    pub scale: u8,
}

impl TestKey {
    pub(crate) fn new(series: u8, frame: i64) -> Self {
        Self { series, frame, scale: 1 }
    }
}

impl FrameCacheKey for TestKey {
    fn frame(&self) -> i64 {
        self.frame
    }

    fn at_frame(&self, frame: i64) -> Self {
        Self { frame, ..*self }
    }

    fn same_series(&self, other: &Self) -> bool {
        self.series == other.series && self.scale == other.scale
    }

    fn same_scale(&self, other: &Self) -> bool {
        self.scale == other.scale
    }
}

#[derive(Debug)]
pub(crate) struct TestFrame(pub usize);

impl FrameSize for TestFrame {
    fn bytes(&self) -> usize {
        self.0
    }
}

fn fill(cache: &mut FrameCache<TestKey, TestFrame>, frames: impl IntoIterator<Item = i64>) {
    for f in frames {
        cache.insert(TestKey::new(0, f), Arc::new(TestFrame(10)));
    }
}

fn playhead(current: i64, start: i64, end: i64, looping: bool) -> FramePlayhead<TestKey> {
    FramePlayhead { series: TestKey::new(0, 0), current, start, end, step: 1, looping }
}

// past the budget, the oldest shown frames go down to 80 %.
#[test]
fn evicts_to_eighty_percent_least_recently_shown_first() {
    let mut cache = FrameCache::new(100);
    fill(&mut cache, 0..10);
    assert_eq!(cache.bytes(), 100);
    cache.get(&TestKey::new(0, 0));
    fill(&mut cache, [10]);
    assert_eq!(cache.bytes(), 80);
    assert!(cache.contains(&TestKey::new(0, 0)));
    assert!((1..=3).all(|f| !cache.contains(&TestKey::new(0, f))));
    assert!((4..=10).all(|f| cache.contains(&TestKey::new(0, f))));
}

// with a playhead, unreached frames go first, then the one reached last; a frame at another scale than the on-screen
// one is the first victim.
#[test]
fn eviction_spares_next_frames_and_drops_other_scales_first() {
    let mut cache = FrameCache::new(50);
    cache.set_playhead(Some(playhead(5, 5, 20, false)));
    fill(&mut cache, [0, 1, 5, 6, 7, 8]);
    assert!(!cache.contains(&TestKey::new(0, 0)) && !cache.contains(&TestKey::new(0, 1)));
    assert!((5..=8).all(|f| cache.contains(&TestKey::new(0, f))));
    fill(&mut cache, [9, 10]);
    assert_eq!(cache.cached_frames(&TestKey::new(0, 0)), [5, 6, 7, 8]);

    let mut cache = FrameCache::new(30);
    cache.set_on_screen(Some(TestKey { series: 0, frame: 0, scale: 2 }));
    let at = |frame, scale| TestKey { series: 0, frame, scale };
    for key in [at(0, 2), at(1, 2), at(2, 1)] {
        cache.insert(key, Arc::new(TestFrame(10)));
    }
    cache.insert(at(3, 2), Arc::new(TestFrame(10)));
    assert!(!cache.contains(&at(2, 1)) && !cache.contains(&at(0, 2)));
    assert!(cache.contains(&at(1, 2)) && cache.contains(&at(3, 2)));
}

// nearest finds the exact frame, else the nearest earlier one within max_back.
#[test]
fn nearest_holds_an_earlier_frame() {
    let mut cache = FrameCache::new(1000);
    fill(&mut cache, [3, 5]);
    assert_eq!(cache.nearest(&TestKey::new(0, 5), 3).map(|(k, _)| k.frame), Some(5));
    assert_eq!(cache.nearest(&TestKey::new(0, 8), 3).map(|(k, _)| k.frame), Some(5));
    assert!(cache.nearest(&TestKey::new(0, 8), 2).is_none());
    assert!(cache.nearest(&TestKey::new(1, 8), 9).is_none());
}

// FramePlayhead::ahead counts past the end and around from the start when looping.
#[test]
fn ahead_counts_through_a_loop() {
    let mut p = FramePlayhead { step: 2, ..playhead(8, 2, 10, true) };
    assert_eq!(p.ahead(&TestKey::new(0, 8)), Some(0));
    assert_eq!(p.ahead(&TestKey::new(0, 10)), Some(1));
    assert_eq!(p.ahead(&TestKey::new(0, 4)), Some(3));
    assert_eq!(p.ahead(&TestKey::new(0, 11)), None);
    assert_eq!(p.ahead(&TestKey::new(1, 9)), None);
    p.looping = false;
    assert_eq!(p.ahead(&TestKey::new(0, 4)), None);
}
