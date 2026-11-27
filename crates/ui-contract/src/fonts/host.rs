//! The hand-over of font stacks to the text engine: what changes the faces (an install, a fetched face, on-demand
//! script coverage, the OS interface face resolving) offers the stack the engine should set text in next; the host
//! gives it to the engine once per frame, before the frame runs.

use std::sync::{Arc, Mutex, PoisonError};

use craft_ui_rt::{FontStack, TextEngine, Waker};

/// The stack offered and not yet given, shared with the threads that offer one (a fetch, the system font index).
/// The host provides it above every window root and calls [`FontHost::give`] in its frame hook.
#[derive(Clone)]
pub struct FontHost {
    pending: Arc<Mutex<Option<FontStack>>>,
    waker: Option<Waker>,
}

impl FontHost {
    /// A hand-over that wakes the loop through `waker` when a stack is offered; `None` where the caller steps the loop
    /// itself (a test harness).
    pub fn new(waker: Option<Waker>) -> FontHost {
        FontHost { pending: Arc::default(), waker }
    }

    /// Offers `stack` in place of any stack offered and not yet given, and wakes the loop. Callable from any thread.
    pub fn offer(&self, stack: FontStack) {
        *self.pending.lock().unwrap_or_else(PoisonError::into_inner) = Some(stack);
        if let Some(waker) = &self.waker {
            waker.wake();
        }
    }

    /// Gives `engine` the stack offered last, if one is waiting; returns whether one was. The engine's stack generation
    /// moves, and the frame that follows lays out again what laid text out.
    pub fn give(&self, engine: &mut dyn TextEngine) -> bool {
        let pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner).take();
        match pending {
            Some(stack) => {
                engine.set_stack(&stack);
                true
            }
            None => false,
        }
    }
}
