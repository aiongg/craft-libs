//! [`FakeTiles`]: the fake tile source of the grid's tests and stories (craft-browse's too).

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use super::{TileContent, TileSource};

/// Sets the content of a [`FakeTiles`] key.
pub type DescribeTile = Box<dyn Fn(usize, &mut TileContent) + Send>;

/// Fake tiles: keys are ids in display order, content comes from the key.
///
/// A tile is named `Tile {key}`, rated `rated` unless `ratings` holds its key, and enabled unless
/// `disabled` lists its key. `describe` adds the rest (flags, badges, states). For stories and
/// tests, beside [`FakeThumbSource`](crate::canvas::textures::FakeThumbSource).
pub struct FakeTiles {
    /// The keys in display order.
    pub keys: Vec<usize>,
    /// Counts changes that move tiles, for [`TileSource::aspects_revision`].
    pub revision: u64,
    /// Ratings the grid's events set, by key.
    pub ratings: HashMap<usize, u8>,
    /// The rating of a tile `describe` and `ratings` leave unrated.
    pub rated: Option<u8>,
    /// The keys of disabled tiles.
    pub disabled: Vec<usize>,
    /// The picture width over height of a key.
    pub aspect: Box<dyn Fn(usize) -> f32 + Send>,
    /// Sets the content of a key after the defaults.
    pub describe: DescribeTile,
    /// The positions the grid asked content for.
    pub asked: RefCell<BTreeSet<usize>>,
}

impl FakeTiles {
    /// `count` square tiles with the keys `0..count`.
    pub fn new(count: usize) -> Self {
        Self {
            keys: (0..count).collect(),
            revision: 0,
            ratings: HashMap::new(),
            rated: None,
            disabled: Vec::new(),
            aspect: Box::new(|_| 1.0),
            describe: Box::new(|_, _| {}),
            asked: RefCell::default(),
        }
    }

    /// Records that tiles moved or changed shape.
    pub fn changed(&mut self) {
        self.revision += 1;
    }
}

impl crate::behavior::DisplayOrder<usize> for FakeTiles {
    fn len(&self) -> usize {
        self.keys.len()
    }

    fn key(&self, at: usize) -> &usize {
        &self.keys[at]
    }
}

impl TileSource<usize> for FakeTiles {
    fn aspect(&self, at: usize) -> f32 {
        (self.aspect)(self.keys[at])
    }

    fn aspects_revision(&self) -> u64 {
        self.revision * 1_000_003 + self.keys.len() as u64
    }

    fn content(&self, at: usize, content: &mut TileContent) {
        self.asked.borrow_mut().insert(at);
        let key = self.keys[at];
        content.name = format!("Tile {key}");
        content.rating = self.rated;
        (self.describe)(key, content);
        if let Some(rating) = self.ratings.get(&key) {
            content.rating = Some(*rating);
        }
        content.enabled &= !self.disabled.contains(&key);
    }
}
