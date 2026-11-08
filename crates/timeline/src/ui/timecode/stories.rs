//! Stories of the timecode field and the Timecode panel.

use super::TimecodePanelSettings;
use super::{TimecodeField, TimecodePanel, TimecodeRow, TimecodeRowKind, TimecodeSource, TimecodeText};
use crate::rates::FrameRate;
use crate::time::Tick;
use crate::timecode::{TimeDisplay, TimeFormat};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

/// 29.97 frames per second with drop-frame counting.
fn drop_frame() -> TimeFormat {
    TimeFormat::timecode(FrameRate::new(30_000, 1_001)).drop_frame(true)
}

#[story(TimecodeField<'static>, covers = [Default], drive = Drive::Never)]
fn timecode_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let start = Tick::from_seconds_f64(3_723.5);
    TimecodeField::new("Current time", Bind::bound(data.value("time", start)), drop_frame()).show(ui).into()
}

#[story(TimecodeField<'static>, covers = [], drive = Drive::Never)]
fn timecode_field_from_sequence_start(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let start = Tick::from_seconds_f64(3_600.0);
    TimecodeField::new("In point", Bind::bound(data.value("time", start)), drop_frame()).min(start).show(ui).into()
}

#[story(TimecodePanel<'static>, covers = [Default], drive = Drive::Never)]
fn timecode_panel(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rows = vec![
        TimecodeRow::new(TimecodeRowKind::CurrentTime).source(TimecodeSource::Program),
        TimecodeRow::new(TimecodeRowKind::Duration).source(TimecodeSource::Program),
        TimecodeRow::new(TimecodeRowKind::Remaining).source(TimecodeSource::Program).display(Some(TimeDisplay::Frames)),
    ];
    let text = TimecodeText::new(|row| match row.kind {
        TimecodeRowKind::CurrentTime => "00:01:02:03".to_owned(),
        TimecodeRowKind::Duration => "00:05:00:00".to_owned(),
        _ => "17,940".to_owned(),
    });
    TimecodePanel::new("Timecode", Bind::bound(data.value("settings", TimecodePanelSettings::new(rows))), text).show(ui).into()
}
