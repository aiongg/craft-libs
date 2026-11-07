//! `List` and its rows: `ListRow`, `FindingsList`, `EventList` and `UndoList`.

mod events;
mod findings;
pub(crate) mod row;
#[cfg(feature = "testing")]
mod stories;
mod toggle;
mod tree;
mod undo;
mod view;
mod view_words;
#[cfg(feature = "testing")]
mod view_stories;
mod words;

pub use events::{EventLevel, EventList, EventListStyle, EventLog, EventLogState, LevelCounts, LogEvent};
pub use findings::{Finding, Findings, FindingsList, FindingsListStyle, FindingsSummary, FindingsSummaryStyle, Severity, summary_text};
pub use row::{ListRow, ListRowStyle, RowEvent, RowInk, RowOutput, ThumbPaint, ThumbSize, ThumbTarget, TrailingPaint};
pub use toggle::{ToggleCell, ToggleFace, ToggleState};
pub use tree::OpenSet;
pub use undo::{UndoList, UndoListStyle, UndoMark, UndoRow, UndoRowEvent, UndoRowOutput, UndoRowStyle, UndoRows, footer_text, undo_count};
pub use view::{List, ListStyle};
pub use view_words::{EventWords, FindingsWords, UndoWords};
pub use words::ListWords;
