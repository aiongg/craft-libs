//! `Inner::settle`: the body's memory, handed to the caller once after the real run.

use crate::contract::{Context, Id};
use crate::runtime::memory::{MemoryStage, StageAccess, sealed};
use std::cell::RefCell;

/// The body's memory as a real run left it. Set only when a real run called the body.
#[doc(hidden)]
pub struct SettleAt {
    pub(crate) stage: RefCell<Option<MemoryStage>>,
    pub(crate) ctx: Context,
    pub(crate) body: Id,
    /// Where the body's shown slots go once the settle ran or was dropped: its node's memory; `None` for a body with
    /// no node, whose shown slots go with the stage.
    home: Option<Home>,
}

type Home = std::rc::Rc<RefCell<crate::contract::rt::memory::Memory>>;

impl std::fmt::Debug for SettleAt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettleAt").field("body", &self.body).finish_non_exhaustive()
    }
}

impl SettleAt {
    /// The settle of the body `body` whose run left `stage`.
    pub(crate) fn new(stage: MemoryStage, body: Id, home: Option<Home>) -> SettleAt {
        SettleAt { ctx: stage.ctx().clone(), stage: RefCell::new(Some(stage)), body, home }
    }

    fn give_back(&self, stage: MemoryStage) {
        match &self.home {
            Some(home) => *home.borrow_mut() = stage.finish(),
            None => stage.write_back(),
        }
    }

    /// Runs `f` once on the body's memory; a second call finds none and gets a stage with nothing in it.
    pub(crate) fn run<T>(&self, f: impl FnOnce(&mut SettleCx<'_>) -> T) -> T {
        let Some(mut stage) = self.stage.borrow_mut().take() else {
            let mut stage = MemoryStage::open(&self.ctx, self.body);
            let value = f(&mut SettleCx { stage: &mut stage, id: self.body });
            stage.write_back();
            return value;
        };
        let value = f(&mut SettleCx { stage: &mut stage, id: self.body });
        self.give_back(stage);
        value
    }
}

impl Drop for SettleAt {
    fn drop(&mut self) {
        if let Some(stage) = self.stage.get_mut().take() {
            self.give_back(stage);
        }
    }
}

/// The body's memory after the real run: the same `Remember` access the body's context has, and the `Held` and
/// `Bind` reads and writes that take it. It claims no stage and does not mark the body as seen.
pub struct SettleCx<'a> {
    stage: &'a mut MemoryStage,
    id: Id,
}

impl sealed::Sealed for SettleCx<'_> {}

impl StageAccess for SettleCx<'_> {
    fn stage(&self) -> &MemoryStage {
        self.stage
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.stage
    }

    fn base(&self) -> Id {
        self.id
    }
}
