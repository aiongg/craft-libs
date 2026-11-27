//! The craft `Id`: a component's identity across frames, and the key of its saved and session state.
//!
//! The hashing is egui's (aHash with fixed seeds; `with` hashes the parent's value and the child's
//! salt), so every id keeps its value across the move off egui and state saved under an id keeps
//! its key.

use std::hash::{BuildHasher, Hash, Hasher};

/// The seeds of an id's hash (egui's `Id`).
const ID_SEEDS: (u64, u64, u64, u64) = (1, 2, 3, 4);
/// The seeds of a salt's hash (egui's `IdSalt`).
const SALT_SEEDS: (u64, u64, u64, u64) = (5, 6, 7, 8);

fn hasher(seeds: (u64, u64, u64, u64)) -> ahash::RandomState {
    ahash::RandomState::with_seeds(seeds.0, seeds.1, seeds.2, seeds.3)
}

/// A hash as a value: zero, which never names anything, becomes one (egui's rule).
const fn nonzero(hash: u64) -> u64 {
    if hash == 0 { 1 } else { hash }
}

/// A component's identity: a hash of its source, or of its parent's id and a salt.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(u64);

impl Id {
    /// The id of nothing in particular.
    pub const NULL: Id = Id(u64::MAX);

    /// The root id of `source`, which must be unique in the whole app.
    pub fn new(source: impl Hash) -> Id {
        Id(nonzero(hasher(ID_SEEDS).hash_one(&source)))
    }

    /// The id of the child `child` of this id.
    pub fn with(self, child: impl Hash) -> Id {
        self.with_salt(IdSalt::new(child))
    }

    /// The id of the child this id's `salt` names.
    pub fn with_salt(self, salt: IdSalt) -> Id {
        let mut hasher = hasher(ID_SEEDS).build_hasher();
        hasher.write_u64(self.0);
        hasher.write_u64(salt.0);
        Id(nonzero(hasher.finish()))
    }

    /// The id's hash value.
    pub fn value(self) -> u64 {
        self.0
    }

    /// The id's low 16 bits in hex, for logs.
    pub fn short_debug_format(&self) -> String {
        format!("{:04X}", self.0 as u16)
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Id::NULL { f.write_str("Id::NULL") } else { write!(f, "id_{:04X}", self.0 as u16) }
    }
}

/// A child's name under its parent: unique among the parent's children only.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdSalt(u64);

impl IdSalt {
    /// The salt of `source`.
    pub fn new(source: impl Hash) -> IdSalt {
        IdSalt(nonzero(hasher(SALT_SEEDS).hash_one(&source)))
    }

    /// The salt's hash value.
    pub fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Debug for IdSalt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "salt_{:04X}", self.0 as u16)
    }
}

impl From<&str> for IdSalt {
    fn from(source: &str) -> IdSalt {
        IdSalt::new(source)
    }
}

impl From<String> for IdSalt {
    fn from(source: String) -> IdSalt {
        IdSalt::new(source)
    }
}
