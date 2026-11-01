//! Stories of the tables: a sortable file table, the metadata table and the event log.
//!
//! Every string a story shows (column labels, file names, metadata, event messages) goes
//! through [`TableStoryLabels`] or the shared [`StoryLabels`], so the gallery's pseudo-locale
//! test sees no stray English. The event story passes a fixed `now_ms`, so its ages don't change
//! between screenshots.

use egui::Ui;

use super::{EventLevel, EventLog, EventLogState, LogEvent, MetadataGroup, MetadataRow, MetadataTable, Table, TableColumn, TableEvent, TableLayout};
use crate::behavior::{Axis, DropPosition, Move, Reorder, RowInfo, Selection};
use crate::controls::Button;
use crate::controls::slider::with_values;
use crate::edit::EditEvent;
use crate::localized::{AgeStyle, Labels as _, LocaleNumbers, relative_age};
use crate::primitives::{Content, Text};
use crate::story::{Level, NoArgs, Story, StoryArgs, StoryLabels};
use crate::typed_story;

crate::words! {
    /// The sample text of the table stories, English by default.
    #[non_exhaustive]
    pub struct TableStoryLabels {
        files: "tableStory.files" = "Files";
        pages: "tableStory.pages" = "Pages";
        modified: "tableStory.modified" = "Modified";
        warnings: "tableStory.warnings" = "Warnings";
        megabytes: "tableStory.megabytes" = "{n} MB";
        report: "tableStory.report" = "Report";
        metadata: "tableStory.metadata" = "Photo metadata";
        group_camera: "tableStory.groupCamera" = "Camera";
        group_exposure: "tableStory.groupExposure" = "Exposure";
        group_file: "tableStory.groupFile" = "File";
        meta_make: "tableStory.metaMake" = "Make";
        meta_model: "tableStory.metaModel" = "Model";
        meta_lens: "tableStory.metaLens" = "Lens";
        meta_exposure_time: "tableStory.metaExposureTime" = "Exposure Time";
        meta_aperture: "tableStory.metaAperture" = "Aperture";
        meta_iso: "tableStory.metaIso" = "ISO speed";
        meta_focal: "tableStory.metaFocal" = "Focal Length";
        meta_creator: "tableStory.metaCreator" = "Creator";
        meta_title: "tableStory.metaTitle" = "Title";
        meta_software: "tableStory.metaSoftware" = "Software";
        events: "tableStory.events" = "Events";
        restore: "tableStory.restore" = "Restore";
        source_autosave: "tableStory.sourceAutosave" = "Auto-save";
        source_render: "tableStory.sourceRender" = "Render";
        source_export: "tableStory.sourceExport" = "Export";
        msg_saved: "tableStory.msgSaved" = "Document saved.";
        msg_opened: "tableStory.msgOpened" = "Opened the project.";
        msg_exported: "tableStory.msgExported" = "Export finished.";
        msg_synced: "tableStory.msgSynced" = "Settings synced.";
        msg_disk: "tableStory.msgDisk" = "Disk space is low.";
        msg_slow: "tableStory.msgSlow" = "The preview is rendering slowly.";
        msg_font: "tableStory.msgFont" = "A font is missing; a substitute is used.";
        msg_proxy: "tableStory.msgProxy" = "Using proxy media.";
        msg_render_failed: "tableStory.msgRenderFailed" = "Render failed: out of memory.";
        msg_export_failed: "tableStory.msgExportFailed" = "Export failed: the folder is read-only.";
        msg_save_failed: "tableStory.msgSaveFailed" = "Auto-save failed.";
        msg_sync_failed: "tableStory.msgSyncFailed" = "Sync failed: offline.";
    }
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/table/files", Level::Composites, "Table", "Files", FilesArgs { striped: true, movable: true }, show_files),
        typed_story!(
            "composites/metadata-table/metadata",
            Level::Composites,
            "Metadata table",
            "Metadata",
            MetadataArgs { column_picker: false },
            show_metadata
        ),
        typed_story!("composites/event-log/events", Level::Composites, "Event log", "Events", NoArgs {}, show_events),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FilesArgs {
    striped: bool,
    movable: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct MetadataArgs {
    #[arg(name = "column picker")]
    column_picker: bool,
}

/// The file table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    Name,
    Pages,
    Size,
    Modified,
    Warnings,
}

const FILES: usize = 200;

/// The pages of file `n`.
fn pages(n: usize) -> u64 {
    (n as u64 * 7) % 90 + 1
}

/// The size of file `n` in kilobytes.
fn kilobytes(n: usize) -> u64 {
    (n as u64 * 937) % 48_000 + 120
}

/// Seconds since file `n` was modified.
fn age(n: usize) -> u64 {
    (n as u64 * 7_919) % (86_400 * 400)
}

/// The warnings of file `n`; most have none.
fn warnings(n: usize) -> u64 {
    if n.is_multiple_of(7) { (n as u64 % 3) + 1 } else { 0 }
}

#[derive(Clone)]
struct Files {
    layout: TableLayout<Column>,
    rows: Vec<RowInfo<usize>>,
    /// Each file's name, by key.
    names: Vec<String>,
    selection: Selection<usize>,
    events: Vec<String>,
}

/// Moves the rows of `mv` before or after its target, in the order shown.
fn apply_move(rows: &mut Vec<RowInfo<usize>>, mv: &Move<usize>) {
    let moved: Vec<RowInfo<usize>> = rows.iter().filter(|row| mv.items.contains(&row.key)).cloned().collect();
    rows.retain(|row| !mv.items.contains(&row.key));
    let Some(target) = rows.iter().position(|row| row.key == mv.target) else { return };
    let at = if mv.position == DropPosition::After { target + 1 } else { target };
    rows.splice(at..at, moved);
}

/// 200 files with sorting by a header click, resizing and moving of columns (pointer and keys),
/// the header menu for hiding Size and Modified, and the selection. While no column sorts, rows
/// drag to a new place (Cmd+] and Cmd+[ move them too); F2 or a slow click on a selected name
/// renames the file. The last events are printed under the table.
fn show_files(ui: &mut Ui, args: &mut FilesArgs) {
    let words = TableStoryLabels::of(ui);
    let shared = StoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let ctx = ui.ctx().clone();
    let columns = [
        TableColumn::new(Column::Name, shared.name.as_str()).min_width(120.0),
        TableColumn::new(Column::Pages, words.pages.as_str()).min_width(60.0).numeric(),
        TableColumn::new(Column::Size, shared.size.as_str()).min_width(80.0).hideable(true).numeric(),
        TableColumn::new(Column::Modified, words.modified.as_str()).min_width(100.0).hideable(true),
        TableColumn::new(Column::Warnings, words.warnings.as_str()).min_width(80.0).numeric(),
    ];
    let initial = Files {
        layout: TableLayout::new(&columns),
        rows: (0..FILES).map(RowInfo::leaf).collect(),
        names: (0..FILES).map(|file| format!("{} {:03}.pdf", words.report, file + 1)).collect(),
        selection: Selection::only(0),
        events: Vec::new(),
    };
    with_values(ui, "story-files", [initial], |ui, [files]| {
        let Files { layout, rows, names, selection, events } = files;
        // Rows keep the user's order only while no column sorts.
        let unsorted = layout.sort.is_none();
        let out = Table::new("story-files", &words.files, &columns, layout, rows.as_slice())
            .selection(selection)
            .striped(args.striped)
            .movable(args.movable)
            .max_height(320.0)
            .row_name(|&file| names[file].clone())
            .reorder(Reorder::new(Axis::Vertical).accepts(move |_, _| unsorted))
            .renamable(Column::Name)
            .show(ui, |ui, &file, column, cx| match column {
                Column::Name => {
                    cx.name(ui, names[file].as_str());
                }
                Column::Pages => {
                    cx.label(ui, numbers.format(pages(file) as f64, 0..=0));
                }
                Column::Size => {
                    let megabytes = numbers.format(kilobytes(file) as f64 / 1024.0, 1..=1);
                    cx.label(ui, craft_i18n::fmt(&words.megabytes, &[("n", &megabytes)]));
                }
                Column::Modified => {
                    cx.label(ui, relative_age(&ctx, age(file), AgeStyle::Short));
                }
                Column::Warnings => {
                    if warnings(file) > 0 {
                        cx.label(ui, numbers.format(warnings(file) as f64, 0..=0));
                    }
                }
            });
        for event in out.events {
            if matches!(event, TableEvent::Unsorted) {
                rows.sort_by_key(|row| row.key);
            }
            match &event {
                TableEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => apply_move(rows, value),
                TableEvent::Renamed { row, text } if !text.is_empty() => names[*row].clone_from(text),
                _ => {}
            }
            if let TableEvent::Sort(sort) = &event {
                sort.apply(rows, |a, b, column| match column {
                    Column::Name => a.key.cmp(&b.key),
                    Column::Pages => pages(a.key).cmp(&pages(b.key)),
                    Column::Size => kilobytes(a.key).cmp(&kilobytes(b.key)),
                    Column::Modified => age(b.key).cmp(&age(a.key)),
                    Column::Warnings => warnings(a.key).cmp(&warnings(b.key)),
                });
            }
            events.push(format!("{event:?}"));
        }
        let keep = events.len().saturating_sub(3);
        events.drain(..keep);
        for line in events.iter() {
            ui.add(Text::new(line.as_str()).content(Content::Muted));
        }
    });
}

/// A photo's groups of fields, in the app's words; the values are what the camera wrote.
fn metadata_groups(words: &TableStoryLabels) -> Vec<MetadataGroup> {
    let group = |name: &str, rows: &[(&str, &str)]| MetadataGroup {
        name: name.to_owned(),
        rows: rows.iter().map(|(name, value)| MetadataRow::new(*name, *value)).collect(),
    };
    vec![
        group(&words.group_camera, &[(&words.meta_make, "FUJIFILM"), (&words.meta_model, "X100V"), (&words.meta_lens, "23 mm f/2")]),
        group(
            &words.group_exposure,
            &[(&words.meta_exposure_time, "1/250"), (&words.meta_aperture, "f/4"), (&words.meta_iso, "200"), (&words.meta_focal, "23 mm")],
        ),
        group(&words.group_file, &[(&words.meta_creator, "A. Rivera"), (&words.meta_title, "IMG_2041"), (&words.meta_software, "Ver 5.20")]),
    ]
}

/// The metadata of a photo with the filter field; an arg adds the column picker's checks.
fn show_metadata(ui: &mut Ui, args: &mut MetadataArgs) {
    let words = TableStoryLabels::of(ui);
    let groups = metadata_groups(&words);
    with_values(ui, "story-metadata", [(String::new(), vec![words.meta_make.clone(), words.meta_iso.clone()])], |ui, [(filter, chosen)]| {
        let mut table = MetadataTable::new("story-metadata", &groups).filter(filter).max_height(320.0);
        if args.column_picker {
            table = table.choose(chosen);
        }
        table.show(ui);
        if args.column_picker {
            ui.add(Text::new(chosen.join(", ")).content(Content::Muted));
        }
    });
}

/// The moment the event story reads as now.
const NOW_MS: u64 = 1_800_000_000_000;

/// The events of the log, oldest first.
fn log_events(words: &TableStoryLabels) -> Vec<LogEvent> {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 3_600;
    const DAY: u64 = 86_400;
    let source = |level: EventLevel| match level {
        EventLevel::Info => &words.source_autosave,
        EventLevel::Warning => &words.source_render,
        EventLevel::Error => &words.source_export,
    };
    let events: [(EventLevel, &String, u32, u64); 12] = [
        (EventLevel::Info, &words.msg_opened, 1, 400 * DAY),
        (EventLevel::Error, &words.msg_sync_failed, 1, 40 * DAY),
        (EventLevel::Warning, &words.msg_font, 1, 9 * DAY),
        (EventLevel::Info, &words.msg_exported, 1, 4 * DAY),
        (EventLevel::Error, &words.msg_export_failed, 1, 2 * DAY),
        (EventLevel::Warning, &words.msg_proxy, 1, DAY),
        (EventLevel::Info, &words.msg_synced, 1, 5 * HOUR),
        (EventLevel::Warning, &words.msg_disk, 3, 3 * HOUR),
        (EventLevel::Error, &words.msg_save_failed, 2, HOUR),
        (EventLevel::Warning, &words.msg_slow, 1, 12 * MINUTE),
        (EventLevel::Error, &words.msg_render_failed, 1, 4 * MINUTE),
        (EventLevel::Info, &words.msg_saved, 1, 30),
    ];
    events
        .into_iter()
        .enumerate()
        .map(|(n, (level, message, count, seconds_ago))| LogEvent {
            id: n as u64 + 1,
            level,
            source: source(level).clone(),
            message: message.clone(),
            count,
            time_ms: NOW_MS - seconds_ago * 1000,
        })
        .collect()
}

/// A dozen events of all three levels (two repeated); Clear All empties the log.
fn show_events(ui: &mut Ui, _args: &mut NoArgs) {
    let words = TableStoryLabels::of(ui);
    let initial = (log_events(&words), EventLogState::default());
    with_values(ui, "story-events", [initial], |ui, [(events, state)]| {
        if EventLog::new("story-events", events, state, NOW_MS).show(ui).clear_all {
            events.clear();
        }
        if events.is_empty() && ui.add(Button::new(&words.restore)).clicked() {
            *events = log_events(&words);
        }
    });
}
