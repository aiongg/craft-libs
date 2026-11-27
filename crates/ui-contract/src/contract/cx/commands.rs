//! Command access (`CommandAccess`, `Commands`) on `Cx`, `BodyCx` and `ItemCx`.

use super::body::BodyCx;
use super::context::Cx;
use super::item::ItemCx;
use crate::contract::Component;
use crate::contract::Context;
use crate::primitives::Id;
use crate::runtime::commands::{CommandAccess, CommandChain, Commands};

impl<C: Component> CommandAccess for Cx<'_, C> {
    fn command_ctx(&self) -> &Context {
        self.core.context()
    }

    fn command_chain(&self) -> &CommandChain {
        self.core.env.command_chain()
    }

    fn command_owner(&self) -> Option<Id> {
        Some(self.core.id)
    }

    fn command_inbox(&mut self) -> Option<(&mut crate::runtime::commands::InboxIn, &mut crate::runtime::memory::MemoryStage, Id)> {
        let core = &mut *self.core;
        (!core.settled).then(|| (&mut core.inbox, &mut *core.memory, core.id))
    }
}

impl<C: Component> Commands for Cx<'_, C> {}

impl CommandAccess for BodyCx<'_> {
    fn command_ctx(&self) -> &Context {
        self.memory.ctx()
    }

    fn command_chain(&self) -> &CommandChain {
        self.env.command_chain()
    }

    fn command_owner(&self) -> Option<Id> {
        None
    }
}

impl Commands for BodyCx<'_> {}

impl<K> CommandAccess for ItemCx<'_, '_, K> {
    fn command_ctx(&self) -> &Context {
        self.memory.ctx()
    }

    fn command_chain(&self) -> &CommandChain {
        self.env.command_chain()
    }

    fn command_owner(&self) -> Option<Id> {
        None
    }
}

impl<K> Commands for ItemCx<'_, '_, K> {}
