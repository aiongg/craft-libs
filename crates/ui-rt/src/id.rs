use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hash, Hasher};

/// A node's identity: its parent's id mixed with its key. Stable across frames, so it doubles as
/// the AccessKit node id and the renderer's cache key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

impl NodeId {
    pub(crate) fn child(self, key: ChildKey) -> NodeId {
        let mut h = StableHasher::default();
        self.0.hash(&mut h);
        key.hash(&mut h);
        NodeId(h.finish())
    }

    pub fn access_id(self) -> accesskit::NodeId {
        accesskit::NodeId(self.0)
    }
}

/// How a child is named under its parent: by position among unkeyed siblings, or by an explicit
/// key and its position inside that key's scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ChildKey {
    Pos(u32),
    Keyed(u64, u32),
    /// A child the node shows on one of its window's layers ([`crate::BuildCx::layer`]), by layer
    /// and key.
    Layer(u8, u64),
    /// A window's fixed nodes: its root, its layers, and the app root on the content layer.
    Fixed(u32),
}

/// A window, as the platform numbers it. Every window event, surface, root, frame and access tree
/// carries one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

impl WindowId {
    /// The id of the window's root node.
    pub fn root(self) -> NodeId {
        NodeId(hash_key(&("window", self.0)))
    }
}

/// Hashes a key or props value with a hasher that is the same in every run and on every target.
pub fn hash_key(value: &impl Hash) -> u64 {
    let mut h = StableHasher::default();
    value.hash(&mut h);
    h.finish()
}

/// FxHash's word mix with a SplitMix64 finish, so small integer keys still spread over all bits.
#[derive(Default)]
pub(crate) struct StableHasher(u64);

impl Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for w in words {
            self.write_u64(u64::from_le_bytes(*w));
        }
        if !rest.is_empty() {
            let mut w = [0u8; 8];
            w[..rest.len()].copy_from_slice(rest);
            self.write_u64(u64::from_le_bytes(w) ^ ((rest.len() as u64) << 59));
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }

    fn write_u8(&mut self, n: u8) {
        self.write_u64(u64::from(n));
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }

    fn finish(&self) -> u64 {
        let mut z = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// Node ids are already mixed hashes, so maps keyed by them skip rehashing.
#[derive(Default)]
pub(crate) struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 << 8) | u64::from(b);
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type IdMap<V> = HashMap<NodeId, V, BuildHasherDefault<IdHasher>>;
pub(crate) type IdSet = HashSet<NodeId, BuildHasherDefault<IdHasher>>;
