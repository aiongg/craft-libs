//! Putting the panel on a runtime: the one setup every run mode shares.

use std::sync::Arc;

use craft_host::target::Os;
use craft_ui_rt::{Runtime, WindowId};

use crate::doc::DocCell;
use crate::fixture::Doc;
use crate::look::pro_dark_tokens;
use crate::panel::LayersPanel;

/// A runtime whose window `window` shows the panel over `doc`, and the cell holding the document
/// the panel edits. The platform's window gives the size and scale when the driver is made.
pub fn mount(window: WindowId, doc: Doc) -> (Runtime, DocCell) {
    let mut runtime = Runtime::new(pro_dark_tokens());
    let cell: DocCell = runtime.store_mut().create(Arc::new(doc));
    runtime.set_root(window, LayersPanel { doc: cell, os: Os::current() });
    (runtime, cell)
}
