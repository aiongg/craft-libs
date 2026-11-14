//! `PenSource`: the tablet's readings, which the runtime's one pointer feed pairs with each pointer move.

use craft_behavior::PointerSample;
use std::sync::{Arc, Mutex};

/// What the tablet hooks have written and the pass has not yet read.
#[derive(Default)]
#[allow(dead_code)] // P3(canvas-widgets-w4): cw.C73
struct Readings {
    queued: Vec<PointerSample>,
    current: Option<PointerSample>,
    barrel: bool,
}

/// A shared handle the app's platform tablet hooks write into and the runtime reads once per pass. Clones share one
/// queue.
#[derive(Clone, Default)]
pub struct PenSource {
    readings: Arc<Mutex<Readings>>,
}

impl PenSource {
    /// Queues one tablet reading in order; `None` says the pen left the tablet.
    pub fn set(&self, sample: Option<PointerSample>) {
        let _ = sample;
        todo!("cw.C73")
    }

    /// Sets whether the barrel button is down.
    pub fn set_barrel(&self, down: bool) {
        let _ = down;
        todo!("cw.C73")
    }

    /// Takes the readings queued since the last pass, in order.
    #[allow(dead_code)] // P3(canvas-widgets-w4): cw.C73
    pub(crate) fn drain(&self) -> Vec<PointerSample> {
        let _ = &self.readings;
        todo!("cw.C73")
    }

    /// The latest reading, if the pen is on the tablet.
    #[allow(dead_code)] // P3(canvas-widgets-w4): cw.C73
    pub(crate) fn current(&self) -> Option<PointerSample> {
        todo!("cw.C73")
    }

    /// Whether the barrel button is down.
    #[allow(dead_code)] // P3(canvas-widgets-w4): cw.C73
    pub(crate) fn barrel(&self) -> bool {
        todo!("cw.C73")
    }
}
