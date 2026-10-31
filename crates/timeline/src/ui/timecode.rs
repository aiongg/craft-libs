//! The timecode field and the Timecode panel (N1.09, N1.14).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditResponse;
use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::time::Tick;
use crate::timecode::{TimeDisplay, TimeFormat};

/// Premiere's big timecode readout and the smaller timecode fields of dialogs.
///
/// - Drag horizontally to scrub: one frame per point, Shift ×10 (FilmCraft `widgets.rs:130`);
///   a gesture of `EditEvent<Tick>`s, Esc cancels it.
/// - Click (or Enter when focused) to type: the text is selected; Enter or a click away commits
///   through [`parse_time`](crate::parse_time) relative to the current value; Esc cancels. A time
///   that doesn't parse keeps the field open with the error on `EditResponse` (the app shows a
///   notice) and commits nothing.
/// - Never before `min` (the sequence start).
/// - One Tab stop; Up and Down step one frame (Shift: ten) as `Set`s.
/// - Text in the `timecode` type role with tabular figures; `large` uses the display role
///   (the Program monitor's and Timeline's readout, accent coloured).
/// - AccessKit: a spin button whose value is the formatted time.
pub struct TimecodeField<'a> {
    id_salt: IdSalt,
    time: Tick,
    format: &'a TimeFormat,
    min: Tick,
    large: bool,
    width: Option<f32>,
}

impl<'a> TimecodeField<'a> {
    pub fn new(id_salt: impl AsIdSalt, time: Tick, format: &'a TimeFormat) -> Self {
        Self { id_salt: IdSalt::new(id_salt), time, format, min: Tick::ZERO, large: false, width: None }
    }

    pub fn min(mut self, min: Tick) -> Self {
        self.min = min;
        self
    }

    /// The big accent readout of a monitor or the Timeline.
    pub fn large(mut self, on: bool) -> Self {
        self.large = on;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    // filmcraft: crates/ui-egui/src/widgets.rs:147 `timecode_field` (and the scrub speed above
    // it, `:130`).
    pub fn show(self, ui: &mut Ui) -> EditResponse<Tick> {
        todo!()
    }
}

craft_ui::words! {
    /// The Timecode panel's words, English by default (FilmCraft's).
    #[non_exhaustive]
    pub struct TimecodePanelLabels {
        current_time: "timecodePanel.currentTime" = "Current Time";
        media_time: "timecodePanel.mediaTime" = "Media Time";
        duration: "timecodePanel.duration" = "Duration";
        in_out_duration: "timecodePanel.inOutDuration" = "In/Out Duration";
        remaining: "timecodePanel.remaining" = "Remaining";
        program: "timecodePanel.program" = "Program";
        source: "timecodePanel.source" = "Source";
        active: "timecodePanel.active" = "Active Monitor";
        add_row: "timecodePanel.addRow" = "Add a timecode row";
        remove_row: "timecodePanel.removeRow" = "Remove Row";
    }
}

/// What a Timecode panel row shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimecodeRowKind {
    #[default]
    CurrentTime,
    /// The source timecode of the clip under the playhead.
    MediaTime,
    Duration,
    InOutDuration,
    Remaining,
}

/// Which monitor a row reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimecodeSource {
    /// The Source monitor when it has the focus and an item, else the Program.
    #[default]
    Active,
    Program,
    Source,
}

/// A row's settings; the app keeps the list in its panel state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimecodeRow {
    pub kind: TimecodeRowKind,
    pub source: TimecodeSource,
    /// `None`: the sequence's own display.
    pub display: Option<TimeDisplay>,
}

/// Premiere's Timecode panel: the first row in large type, the others smaller, each with its
/// caption ("Current Time · Program"). The app gives each row's text (it knows the clip under
/// the playhead). A secondary click on a row opens its menu (kind, source, display, Remove Row);
/// "+" adds a row. Rows are read-only text (AccessKit labels); the menu items and "+" are the Tab
/// stops.
pub struct TimecodePanel<'a> {
    id_salt: IdSalt,
    rows: &'a mut Vec<TimecodeRow>,
    text: &'a dyn Fn(&TimecodeRow) -> String,
    labels: Option<TimecodePanelLabels>,
}

impl<'a> TimecodePanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, rows: &'a mut Vec<TimecodeRow>, text: &'a dyn Fn(&TimecodeRow) -> String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rows, text, labels: None }
    }

    pub fn labels(mut self, labels: TimecodePanelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Changes `rows` directly (panel settings, not document edits).
    // filmcraft: crates/ui-egui/src/panels/timecode.rs:104 `show`, `:21` `row_text`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
