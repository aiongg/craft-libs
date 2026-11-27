//! The memory services the trials check: a stage fixture that implements `Remember`, `forget` and the id
//! derivations.

use crate::contract::rt::memory::Memory;
use crate::contract::{Context, Id, Ui};
use crate::runtime::memory::{self, MemoryStage, SaveKey, StageAccess, sealed};
use std::fmt::Debug;
use std::hash::Hash;

/// A memory stage opened the way the pipeline opens one for a component: over a node memory owned by `owner`.
/// `finish` is the write-back after the component's hooks. It implements `Remember` over `base`.
pub struct Stage {
    stage: MemoryStage,
    base: Id,
}

impl Stage {
    /// A stage whose entries belong to `base`.
    pub fn open(ctx: &Context, base: Id) -> Self {
        Self::open_owned(ctx, base, base)
    }

    /// A stage keyed by `base` whose node memory belongs to `owner`.
    pub fn open_owned(ctx: &Context, base: Id, owner: Id) -> Self {
        Stage { stage: MemoryStage::on_node(ctx, owner, Memory::default()), base }
    }

    /// The stage of a measuring run: it reads kept values and writes nothing.
    pub fn open_peek(ctx: &Context, base: Id) -> Self {
        Stage { stage: MemoryStage::open(ctx, base).peek_only(), base }
    }

    /// Writes every slot back to the window's memory.
    pub fn finish(self) {
        self.stage.write_back();
    }
}

impl sealed::Sealed for Stage {}

impl StageAccess for Stage {
    fn stage(&self) -> &MemoryStage {
        &self.stage
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        &mut self.stage
    }

    fn base(&self) -> Id {
        self.base
    }
}

/// Removes the session slots under `id`.
pub fn forget(ctx: &Context, id: Id) {
    memory::forget(ctx, id);
}

/// Removes the saved slots of `key`.
pub fn forget_saved(ctx: &Context, key: &SaveKey) {
    memory::forget_saved(ctx, key);
}

/// The runtime sub-id of `base` for `tag`.
pub fn sub(base: Id, tag: impl Hash + Debug) -> Id {
    memory::sub(base, tag)
}

/// The id of a runtime singleton named `name` under `scope`.
pub fn global(name: &'static str, scope: impl Hash + Debug) -> Id {
    memory::global(name, scope)
}

/// Runs `f` on a child `Ui` of `ui` under an id of its own.
pub fn scoped<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.unique_id();
    ui.push_id(id, f)
}

/// Runs `f` on a child `Ui` of `ui` with its own id scope named by `salt`.
pub fn scoped_by<R>(ui: &mut Ui, salt: impl Hash + Debug, f: impl FnOnce(&mut Ui) -> R) -> R {
    ui.push_id(salt, f)
}
