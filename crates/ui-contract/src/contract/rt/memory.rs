//! The memory service on nodes (docs/contract-rt.md section 10, "Memory"): a component's slots
//! (`Remember`) live in its node's `Local`, so they are freed with the node and no global map by id
//! exists. A node that is kept keeps its slots; one that leaves the tree drops them. The saved tier
//! (slots that persist across sessions) stays in the host store keyed by `Id`, outside the tree.

use std::any::{Any, TypeId};

use crate::primitives::Id;

use crate::runtime::memory::Slot;

/// A node's memory slots, freed with the node.
#[derive(Default)]
pub(crate) struct Memory(Vec<(TypeId, Id, Box<dyn Any>)>);

impl Memory {
    /// The slot of type `T` under `id`, created with its default on first use.
    pub(crate) fn shown<T: Slot>(&mut self, id: Id) -> &mut T {
        let at = match self.0.iter().position(|(t, i, _)| *t == TypeId::of::<T>() && *i == id) {
            Some(at) => at,
            None => {
                self.0.push((TypeId::of::<T>(), id, Box::new(T::default())));
                self.0.len() - 1
            }
        };
        self.0[at].2.downcast_mut::<T>().expect("a slot holds the type it was created with")
    }

    /// The slot of type `T` under `id`, moved out.
    pub(crate) fn take<T: Slot>(&mut self, id: Id) -> Option<T> {
        let at = self.0.iter().position(|(t, i, _)| *t == TypeId::of::<T>() && *i == id)?;
        self.0.swap_remove(at).2.downcast::<T>().ok().map(|value| *value)
    }

    /// A copy of the slot of type `T` under `id`, or its default.
    pub(crate) fn recall<T: Slot>(&self, id: Id) -> T {
        self.0.iter().find(|(t, i, _)| *t == TypeId::of::<T>() && *i == id).and_then(|(_, _, v)| v.downcast_ref::<T>()).cloned().unwrap_or_default()
    }
}
