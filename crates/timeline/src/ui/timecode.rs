//! The timecode field and the Timecode panel (N1.09, N1.14).

use craft_theme::ControlSize;
use craft_ui::behavior::scrub::travel;
use craft_ui::behavior::{ClaimedKey, Codec, KeyClaim, NotAValue, Scrub, Typing, TypingLook};
use craft_ui::controls::text_field::{BoxState, field_box};
use craft_ui::controls::{IconButton, TextField};
use craft_ui::edit::{EditPolicy, EditResponse, Gesture};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, margin_x, role_format, tabular};
use egui::accesskit::Role;
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::text_edit::TextEditState;
use egui::{Align, Align2, AsIdSalt, CursorIcon, EventFilter, IdSalt, Key, Layout, Modifiers, Response, Sense, Shape, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::style::TimelineStyle;
use crate::time::Tick;
use crate::timecode::{TimeDisplay, TimeFormat, TimecodeError, format_time, parse_time};

/// Frames the Up and Down keys step a field, without and with Shift.
const STEP_FRAMES: i64 = 1;
const SHIFT_STEP_FRAMES: i64 = 10;
/// How many times faster a Shift-drag scrubs (FilmCraft `widgets.rs:130`).
const SHIFT_SCRUB: f64 = 10.0;

const FIELD_KEYS: [ClaimedKey; 4] = [
    ClaimedKey::exact(Modifiers::NONE, Key::ArrowUp),
    ClaimedKey::exact(Modifiers::NONE, Key::ArrowDown),
    ClaimedKey::exact(Modifiers::SHIFT, Key::ArrowUp),
    ClaimedKey::exact(Modifiers::SHIFT, Key::ArrowDown),
];

/// A scrub in progress: where the pointer was last seen, the frames not yet applied, and the
/// pointer warp the screen-edge wrap waits for.
#[derive(Clone, Copy, Debug)]
struct Scrubbing {
    last_x: f32,
    frames: f64,
    warp: Option<f32>,
}

/// A field being typed in: the text the box showed last pass, and whether the field still has to
/// take the focus and select its text.
#[derive(Clone, Debug)]
struct Editing {
    value: Shown,
    text: String,
    fresh: bool,
}

/// The value a typing run edits: a time, and the text the box shows for it when that isn't the
/// time formatted (the text a story opens the field with).
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    time: Tick,
    text: Option<String>,
}

/// Writes a time in the field's display and reads typed text back with [`parse_time`], never
/// before `min`.
struct TimecodeCodec<'a> {
    format: &'a TimeFormat,
    min: Tick,
    labels: &'a TimecodeFieldLabels,
}

impl Codec for TimecodeCodec<'_> {
    type Value = Shown;

    fn format(&self, value: &Shown) -> String {
        value.text.clone().unwrap_or_else(|| format_time(value.time, self.format))
    }

    fn parse(&self, text: &str, start: &Shown) -> Result<Shown, NotAValue> {
        // Text as shown stands for the time itself, not the first tick of its frame.
        if text == self.format(start) {
            return Ok(Shown { time: start.time, text: None });
        }
        match parse_time(text, self.format, start.time) {
            Ok(time) => Ok(Shown { time: time.max(self.min), text: None }),
            Err(error) => {
                // Text made only of what a time is made of may still become one as the user types on.
                let partial = matches!(error, TimecodeError::Empty) || text.trim().chars().all(|c| c.is_ascii_digit() || ":;.,+-".contains(c));
                Err(NotAValue { reason: self.labels.message(&error), partial })
            }
        }
    }
}

craft_ui::words! {
    /// The names of the time displays, English by default (Premiere's Time Display menu).
    #[non_exhaustive]
    pub struct TimeDisplayLabels {
        timecode: "timeDisplay.timecode" = "Timecode";
        frames: "timeDisplay.frames" = "Frames";
        feet_16: "timeDisplay.feet16" = "Feet + Frames 16mm";
        feet_35: "timeDisplay.feet35" = "Feet + Frames 35mm";
        audio_samples: "timeDisplay.audioSamples" = "Audio Samples";
        seconds: "timeDisplay.seconds" = "Seconds";
    }
}

impl TimeDisplayLabels {
    /// The name of `display`.
    pub fn name(&self, display: TimeDisplay) -> &str {
        match display {
            TimeDisplay::Timecode => &self.timecode,
            TimeDisplay::Frames => &self.frames,
            TimeDisplay::Feet16 => &self.feet_16,
            TimeDisplay::Feet35 => &self.feet_35,
            TimeDisplay::AudioSamples => &self.audio_samples,
            TimeDisplay::Seconds => &self.seconds,
        }
    }
}

craft_ui::words! {
    /// The timecode field's words for typed text that isn't a time, English by default.
    #[non_exhaustive]
    pub struct TimecodeFieldLabels {
        empty: "timecodeField.empty" = "Enter a time.";
        not_a_time: "timecodeField.notATime" = "\u{201c}{input}\u{201d} is not a time.";
    }
}

impl TimecodeFieldLabels {
    /// The notice for `error`.
    pub fn message(&self, error: &TimecodeError) -> String {
        match error {
            TimecodeError::Empty => self.empty.clone(),
            TimecodeError::NotATime(input) => fmt(&self.not_a_time, &[("input", input)]),
        }
    }
}

/// What a [`TimecodeField`] did this pass.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event"]
pub struct TimecodeFieldOutput {
    /// The edit: a changed time as an edit event.
    pub edit: EditResponse<Tick>,
    /// Typed text that didn't parse, from an Enter or a click away; word it with
    /// [`TimecodeFieldLabels::message`]. Nothing is committed; the bad text stays in the field,
    /// marked invalid, until it is fixed or Esc restores the time.
    pub error: Option<TimecodeError>,
}

/// Premiere's big timecode readout and the smaller timecode fields of dialogs.
///
/// - Drag horizontally to scrub through the kernel's [`Scrub`] (the
///   pointer travel and the screen-edge wrap, as [`NumberField`](craft_ui::controls::NumberField)
///   scrubs): one frame per `style.scrub_step` of travel ([`TimelineStyle`]),
///   Shift ×10 (FilmCraft `widgets.rs:130`); a gesture of `EditEvent<Tick>`s, Esc cancels it. A
///   click without a drag edits (decisions §4.2, NumberField).
/// - Click (or Enter when focused) to type: the text is selected; Enter or a click away commits
///   through [`parse_time`] relative to the current value; Esc cancels. Left
///   and Right on the whole selected text go to its last digit. The text is a
///   [`Typing`](craft_ui::behavior::Typing) run: one gesture that commits the time once. A time
///   that doesn't parse commits nothing and is reported on [`TimecodeFieldOutput::error`] (the app
///   shows a notice); the text stays, marked invalid, until it is fixed or Esc restores the time.
/// - Never before `min` (the sequence start).
/// - One Tab stop; Up and Down step one frame (Shift: ten) as `Set`s.
/// - Text in the mono type role with tabular figures (FilmCraft's timecode face); `large` is the
///   Program monitor's and Timeline's readout, accent coloured.
/// - AccessKit: a spin button whose value is the formatted time.
pub struct TimecodeField<'a> {
    id_salt: IdSalt,
    time: Tick,
    format: &'a TimeFormat,
    min: Tick,
    large: bool,
    width: Option<f32>,
    typing: Option<String>,
    labels: Option<TimecodeFieldLabels>,
    display_labels: Option<TimeDisplayLabels>,
}

impl<'a> TimecodeField<'a> {
    /// A field showing `time` in `format`.
    pub fn new(id_salt: impl AsIdSalt, time: Tick, format: &'a TimeFormat) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            time,
            format,
            min: Tick::ZERO,
            large: false,
            width: None,
            typing: None,
            labels: None,
            display_labels: None,
        }
    }

    /// The earliest time the field accepts, such as the sequence start; zero by default.
    pub fn min(mut self, min: Tick) -> Self {
        self.min = min;
        self
    }

    /// The big accent readout of a monitor or the Timeline.
    pub fn large(mut self, on: bool) -> Self {
        self.large = on;
        self
    }

    /// The field's width in points; it fits its text by default.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Opens the field in its typing state with `text` the first time it is shown (for stories and
    /// screenshots); Esc or a commit closes it as usual and it doesn't open again.
    pub fn typing(mut self, text: impl Into<String>) -> Self {
        self.typing = Some(text.into());
        self
    }

    /// The words for typed-text errors and accessibility; `TimecodeFieldLabels::of` when not set.
    pub fn labels(mut self, labels: TimecodeFieldLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The display names, for the accessible name of the field.
    pub fn display_labels(mut self, labels: TimeDisplayLabels) -> Self {
        self.display_labels = Some(labels);
        self
    }

    /// The edit alone; [`show_full`](Self::show_full) adds the typed-text error.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Tick> {
        self.show_full(ui).edit
    }

    /// Show the field and report the edit and any typed-text error.
    // filmcraft: crates/ui-egui/src/widgets.rs:147 `timecode_field` (and the scrub speed above
    // it, `:130`).
    pub fn show_full(self, ui: &mut Ui) -> TimecodeFieldOutput {
        let Self { id_salt, time, format, min, large, width, typing: preset, labels, display_labels } = self;
        let display_labels = display_labels.unwrap_or_else(|| TimeDisplayLabels::of(ui));
        let labels = labels.unwrap_or_else(|| TimecodeFieldLabels::of(ui));
        let display_name = display_labels.name(format.display);
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let field = TextField::style(&tokens, ControlSize::Md);
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let typing_key = id.with("typing");
        let rate = format.rate;
        let role = if large { style.timecode_large_text } else { style.timecode_text };
        let color = if large { tokens.color.accent.text.to_egui() } else { field.text };
        let text_format = tabular(role_format(&tokens, role, color));
        let ctx = ui.ctx().clone();
        let layout = |text: &str| ctx.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(text.to_owned(), text_format.clone())));
        if let Some(text) = preset {
            let preset_key = id.with("preset");
            if ui.data(|d| d.get_temp::<bool>(preset_key)).is_none() {
                ui.data_mut(|d| {
                    d.insert_temp(preset_key, true);
                    d.insert_temp(typing_key, Editing { value: Shown { time, text: Some(text.clone()) }, text, fresh: true });
                });
            }
        }
        let typing = ui.data(|d| d.get_temp::<Editing>(typing_key));

        let shown_text = |t: Tick| format_time(t, format);
        let natural = layout(&shown_text(time)).size();
        let size = vec2(width.unwrap_or(natural.x + 2.0 * field.pad_x), field.height.max(natural.y));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

        if let Some(mut editing) = typing {
            let edit_id = id.with("edit");
            let codec = TimecodeCodec { format, min, labels: &labels };
            let host = Shown { time, text: editing.value.text.clone() };
            arrow_to_last_digit(ui, edit_id, &editing.text);
            let look = TypingLook {
                size: rect.size(),
                margin: margin_x(field.pad_x, field.pad_x),
                align: Align2::LEFT_CENTER,
                font: text_format.font_id.clone(),
                text: color,
                placeholder: field.placeholder,
                selection: field.selection,
                selected_text: field.selected_text,
                caret: field.caret,
            };
            // The box goes under the text, which `TextEdit` paints as it runs.
            let under = ui.painter().add(Shape::Noop);
            let typed = ui
                .scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                    Typing::with_codec(edit_id, display_name, host, codec).enabled(enabled).tabular(true).show(ui, &look)
                })
                .inner;
            let focused = typed.response().has_focus();
            let rejected = typed.rejected().is_some();
            ui.painter().set(under, Shape::Vec(field_box(ui, &field, rect, BoxState { enabled, focused: true, hovered: false, invalid: rejected })));
            editing.text = typed.text().to_owned();
            let escaped = typed.escaped();
            // The run's end: Enter, or the focus leaving a run under way. Esc ends it without a time.
            let ended = !editing.fresh && !escaped && (typed.submitted() || (!focused && typed.is_typing()));
            let error = if ended { parse_time(typed.text(), format, time).err() } else { None };
            let waiting = !focused && !typed.is_typing() && !rejected;
            let done = !editing.fresh && (escaped || (ended && error.is_none()) || waiting);
            if editing.fresh {
                typed.response().request_focus();
                let mut state = TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
                state.cursor.set_char_range(Some(CCursorRange::select_all_str(&editing.text)));
                state.store(ui.ctx(), edit_id);
                editing.fresh = false;
            }
            let edit = typed.end_as(ui, |shown| shown.time);
            if done {
                ui.data_mut(|d| d.remove::<Editing>(typing_key));
            } else {
                ui.data_mut(|d| d.insert_temp(typing_key, editing));
            }
            return TimecodeFieldOutput { edit, error };
        }

        let response = ui.interact(rect, id, Sense::click_and_drag());
        let mut gesture = Gesture::load(ui, id, &time, EditPolicy::default()).enabled(enabled);
        let scrub_key = id.with("scrubbing");
        let modifiers = ui.input(|i| i.modifiers);
        let frame_of = |t: Tick| rate.frame_at(t);
        let at_frame = |frame: i64| rate.tick_of(frame).max(min);
        if enabled {
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            {
                ui.memory_mut(|m| m.request_focus(id));
                ui.data_mut(|d| d.insert_temp(scrub_key, Scrubbing { last_x: origin.x, frames: 0.0, warp: None }));
                gesture.begin();
            }
            if response.dragged_by(egui::PointerButton::Primary)
                && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
                && let Some(mut scrub) = ui.data(|d| d.get_temp::<Scrubbing>(scrub_key))
            {
                let edges = ui.input(|i| i.viewport().fullscreen == Some(true)).then(|| {
                    let screen = ui.ctx().content_rect();
                    (screen.left(), screen.right())
                });
                let moved = travel(scrub.last_x, pos.x, scrub.warp, edges);
                if let Some(x) = moved.send {
                    Scrub::warp(ui.ctx(), pos2(x, pos.y));
                }
                let rate_of_travel = if modifiers.shift { SHIFT_SCRUB } else { 1.0 };
                scrub.frames += f64::from(moved.delta) / f64::from(style.scrub_step) * rate_of_travel;
                (scrub.last_x, scrub.warp) = (moved.last_x, moved.warp);
                let whole = scrub.frames.trunc();
                scrub.frames -= whole;
                if whole != 0.0 {
                    let from = frame_of(*gesture.shown());
                    gesture.update(at_frame(from.saturating_add(whole as i64)), modifiers);
                }
                ui.data_mut(|d| d.insert_temp(scrub_key, scrub));
            }
            if response.drag_stopped() {
                ui.data_mut(|d| d.remove::<Scrubbing>(scrub_key));
                gesture.commit();
            }
            let enter = response.has_focus() && ui.input_mut(|i| ClaimedKey::exact(Modifiers::NONE, Key::Enter).consume(i));
            if response.clicked() || enter {
                let shown = *gesture.shown();
                let text = shown_text(shown);
                ui.data_mut(|d| d.insert_temp(typing_key, Editing { value: Shown { time: shown, text: None }, text, fresh: true }));
                ui.ctx().request_repaint();
            }
            if response.has_focus() && !gesture.is_active() {
                ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { vertical_arrows: true, ..EventFilter::default() }));
                KeyClaim::claim(ui.ctx(), id, &FIELD_KEYS);
                let key = ui.input_mut(|i| {
                    FIELD_KEYS
                        .iter()
                        .find(|claimed| claimed.consume(i))
                        .map(|claimed| (claimed.shortcut.logical_key, claimed.shortcut.modifiers.shift))
                });
                if let Some((key, shift)) = key {
                    let frames = if shift { SHIFT_STEP_FRAMES } else { STEP_FRAMES };
                    let from = frame_of(*gesture.shown());
                    gesture.set(at_frame(if key == Key::ArrowUp { from.saturating_add(frames) } else { from.saturating_sub(frames) }));
                }
            }
        }

        let shown = *gesture.shown();
        let text = shown_text(shown);
        ui.painter().extend(field_box(ui, &field, rect, BoxState::of(&response, enabled, false)));
        let galley = layout(&text);
        let at = pos2(rect.left() + field.pad_x, rect.center().y - galley.size().y / 2.0);
        ui.painter().with_clip_rect(rect).galley(at, galley, if enabled { color } else { field.text_disabled });
        craft_ui::primitives::FocusRing::paint_radius(ui, rect, field.surface.radius, &response);
        response.widget_info(|| WidgetInfo::labeled(Role::SpinButton, enabled, display_name));
        ui.ctx().accesskit_node_builder(response.id, |node| node.set_value(text.as_str()));
        TimecodeFieldOutput { edit: EditResponse::new(response, gesture.finish()), error: None }
    }
}

/// Left or Right on the whole selected text puts the caret on its last digit; egui would jump to
/// the start (FilmCraft `widgets.rs:160-168`).
fn arrow_to_last_digit(ui: &Ui, edit_id: egui::Id, text: &str) {
    let arrow = ui.input(|i| i.modifiers.is_none() && (i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowRight)));
    let len = text.chars().count();
    if !arrow || len == 0 {
        return;
    }
    if let Some(mut state) = TextEditState::load(ui.ctx(), edit_id)
        && state.cursor.char_range().is_some_and(|range| range.sorted_cursors() == [CCursor::new(0), CCursor::new(len)])
    {
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(len))));
        state.store(ui.ctx(), edit_id);
    }
}

/// What a row's menu can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowCommand {
    Kind(TimecodeRowKind),
    Source(TimecodeSource),
    Display(Option<TimeDisplay>),
    Remove,
}

fn row_kind_label(labels: &TimecodePanelLabels, kind: TimecodeRowKind) -> &str {
    match kind {
        TimecodeRowKind::CurrentTime => &labels.current_time,
        TimecodeRowKind::MediaTime => &labels.media_time,
        TimecodeRowKind::Duration => &labels.duration,
        TimecodeRowKind::InOutDuration => &labels.in_out_duration,
        TimecodeRowKind::Remaining => &labels.remaining,
    }
}

fn row_source_label(labels: &TimecodePanelLabels, source: TimecodeSource) -> &str {
    match source {
        TimecodeSource::Active => &labels.active,
        TimecodeSource::Program => &labels.program,
        TimecodeSource::Source => &labels.source,
    }
}

/// A row's menu: its kind, its monitor, its display, and Remove Row while another row is left.
fn row_menu(labels: &TimecodePanelLabels, displays: &TimeDisplayLabels, row: &TimecodeRow, removable: bool) -> Vec<MenuEntry<RowCommand>> {
    let mut entries: Vec<MenuEntry<RowCommand>> = Vec::new();
    let kinds = [
        TimecodeRowKind::CurrentTime,
        TimecodeRowKind::MediaTime,
        TimecodeRowKind::Duration,
        TimecodeRowKind::InOutDuration,
        TimecodeRowKind::Remaining,
    ];
    entries.extend(kinds.map(|kind| MenuItem::new(row_kind_label(labels, kind), RowCommand::Kind(kind)).radio(row.kind == kind).into()));
    entries.push(MenuEntry::Separator);
    let sources = [TimecodeSource::Active, TimecodeSource::Program, TimecodeSource::Source];
    entries
        .extend(sources.map(|source| MenuItem::new(row_source_label(labels, source), RowCommand::Source(source)).radio(row.source == source).into()));
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(labels.sequence_display.as_str(), RowCommand::Display(None)).radio(row.display.is_none()).into());
    entries.extend(
        TimeDisplay::ALL
            .map(|display| MenuItem::new(displays.name(display), RowCommand::Display(Some(display))).radio(row.display == Some(display)).into()),
    );
    if removable {
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.remove_row.as_str(), RowCommand::Remove).into());
    }
    entries
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
        sequence_display: "timecodePanel.sequenceDisplay" = "Sequence Display";
        caption: "timecodePanel.caption" = "{kind} \u{b7} {source}";
        spoken: "timecodePanel.spoken" = "{caption}: {value}";
    }
}

/// What a Timecode panel row shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimecodeRowKind {
    #[default]
    /// The playhead time.
    CurrentTime,
    /// The source timecode of the clip under the playhead.
    MediaTime,
    /// The length of the content.
    Duration,
    /// The length between the In and Out marks.
    InOutDuration,
    /// The time left after the playhead.
    Remaining,
}

/// Which monitor a row reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimecodeSource {
    /// The Source monitor when it has the focus and an item, else the Program.
    #[default]
    Active,
    /// The Program monitor.
    Program,
    /// The Source monitor.
    Source,
}

/// One row of the Timecode panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimecodeRow {
    /// What the row shows.
    pub kind: TimecodeRowKind,
    /// Which monitor the row reads.
    pub source: TimecodeSource,
    /// `None`: the sequence's own display.
    pub display: Option<TimeDisplay>,
}

/// Premiere's Timecode panel: the first row in large type, the others smaller, each with its
/// caption ("Current Time · Program"). The app gives each row's text (it knows the clip under
/// the playhead). A secondary click on a row opens its menu (kind, source, display with
/// "Sequence Display" to follow the sequence again, Remove Row);
/// "+" adds a row. Rows are read-only text (AccessKit labels); the menu items and "+" are the Tab
/// stops; the Menu key and Shift+F10 open a focused row's menu.
pub struct TimecodePanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut TimecodePanelSettings,
    text: &'a dyn Fn(&TimecodeRow) -> String,
    labels: Option<TimecodePanelLabels>,
    display_labels: Option<TimeDisplayLabels>,
}

impl<'a> TimecodePanel<'a> {
    /// A panel over `settings`, with `text` giving each row's text.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut TimecodePanelSettings, text: &'a dyn Fn(&TimecodeRow) -> String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, text, labels: None, display_labels: None }
    }

    /// The words of the panel; `TimecodePanelLabels::of` when not set.
    pub fn labels(mut self, labels: TimecodePanelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The display names of the row menu.
    pub fn display_labels(mut self, labels: TimeDisplayLabels) -> Self {
        self.display_labels = Some(labels);
        self
    }

    /// Changes the settings directly (a panel preference, not a document edit) and says so on
    /// the output.
    // filmcraft: crates/ui-egui/src/panels/timecode.rs:104 `show`, `:21` `row_text`.
    pub fn show(self, ui: &mut Ui) -> TimecodePanelOutput {
        let Self { id_salt, settings, text, labels, display_labels } = self;
        let labels = labels.unwrap_or_else(|| TimecodePanelLabels::of(ui));
        let displays = display_labels.unwrap_or_else(|| TimeDisplayLabels::of(ui));
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let enabled = ui.is_enabled();
        let colors = &tokens.color;
        let caption_format = role_format(&tokens, TimelineStyle::of(&tokens).timecode_caption, colors.text.muted.to_egui());
        let mut picks: Vec<(usize, RowCommand)> = Vec::new();
        let mut add = false;
        let removable = settings.rows.len() > 1;
        let response = ui
            .vertical(|ui| {
                for (at, row) in settings.rows.iter().enumerate() {
                    let big = at == 0;
                    let value = text(row);
                    let caption =
                        fmt(&labels.caption, &[("kind", row_kind_label(&labels, row.kind)), ("source", row_source_label(&labels, row.source))]);
                    let (role, color) = if big {
                        (TimelineStyle::of(&tokens).timecode_large_text, colors.accent.text)
                    } else {
                        (TimelineStyle::of(&tokens).timecode_text, colors.text.normal)
                    };
                    let value_format = tabular(role_format(&tokens, role, color.to_egui()));
                    let value_galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(value.clone(), value_format.clone())));
                    let caption_galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(caption.clone(), caption_format.clone())));
                    let size = vec2(ui.available_width(), value_galley.size().y + caption_galley.size().y + ui.spacing().item_spacing.y);
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let row_response = ui.interact(rect, id.with(("row", at)), Sense::click());
                    let top = pos2(rect.center().x - value_galley.size().x / 2.0, rect.top());
                    let below =
                        pos2(rect.center().x - caption_galley.size().x / 2.0, rect.top() + value_galley.size().y + ui.spacing().item_spacing.y);
                    ui.painter().galley(top, value_galley, value_format.color);
                    ui.painter().galley(below, caption_galley, caption_format.color);
                    let spoken = fmt(&labels.spoken, &[("caption", caption.as_str()), ("value", value.as_str())]);
                    row_response.widget_info(|| WidgetInfo::new(Role::Label));
                    ui.ctx().accesskit_node_builder(row_response.id, |node| node.set_value(spoken.as_str()));
                    let entries = row_menu(&labels, &displays, row, removable);
                    if let Some(command) = ContextMenu::new(&entries).show(ui, &row_response).chosen {
                        picks.push((at, command));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    add = ui.add_enabled(enabled, IconButton::new(Icon::Plus, labels.add_row.as_str()).size(ControlSize::Sm)).clicked();
                });
            })
            .response;
        let mut settings_changed = false;
        for (at, command) in picks {
            let Some(row) = settings.rows.get_mut(at) else { continue };
            match command {
                RowCommand::Kind(kind) => row.kind = kind,
                RowCommand::Source(source) => row.source = source,
                RowCommand::Display(display) => row.display = display,
                RowCommand::Remove => {
                    settings.rows.remove(at);
                }
            }
            settings_changed = true;
        }
        if add {
            let from = settings.rows.last().copied().unwrap_or_default();
            settings.rows.push(TimecodeRow { kind: TimecodeRowKind::Duration, ..from });
            settings_changed = true;
        }
        TimecodePanelOutput { response, settings_changed }
    }
}

/// The Timecode panel's rows, saved by the app with its panel state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimecodePanelSettings {
    /// The rows, first row in large type.
    pub rows: Vec<TimecodeRow>,
}

/// What the Timecode panel did this pass.
#[must_use]
pub struct TimecodePanelOutput {
    /// The panel's response, for hover and focus.
    pub response: Response,
    /// A row was added, removed or changed; the app saves [`TimecodePanelSettings`].
    pub settings_changed: bool,
}
