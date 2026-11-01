//! [`EventLog`]: the warnings and errors of commands and background work, newest first, with a
//! level filter, counts and the selected event's details.

use std::sync::Arc;
use std::time::Duration;

use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Response, Sense, Ui, UiBuilder, vec2};
use serde::{Deserialize, Serialize};

use super::{List, ListEvent, ListRow};
use crate::behavior::{RowInfo, Selection, SelectionMode};
use crate::composites::Notice;
use crate::composites::notice::{paint_tone_icon, tone_icon};
use crate::controls::{Button, Select};
use crate::icons::IconSize;
use crate::localized::words::counts;
use crate::localized::{AgeStyle, LocaleNumbers, SharedLabels as _, Words, relative_age};
use crate::primitives::{Content, Text};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use craft_theme::{Tone, TypeRole};

crate::words! {
    /// The log's words, English by default.
    #[non_exhaustive]
    pub struct EventLogLabels {
        /// The level filter's name and its three choices (FilmCraft's Events panel).
        filter: "events.filter" = "Show";
        all: "events.all" = "All Events";
        warnings: "events.warnings" = "Warnings and Errors";
        errors: "events.errors" = "Errors Only";
        clear_all: "events.clearAll" = "Clear All";
        none: "events.none" = "No events.";
        /// The name of the list of events.
        list: "events.list" = "Events";
        /// An event that happened several times in a row: `{message}`, `{count}`.
        repeated: "events.repeated" = "{message}  (×{count})";
        info: "events.info" = "Info";
        warning: "events.warning" = "Warning";
        error: "events.error" = "Error";
        /// A row's accessible name: `{level}`, `{message}` and `{age}`.
        row_name: "events.rowName" = "{level}: {message}, {age}";
        /// The details line over the selected event's message: `{level}`, `{source}` and
        /// `{time}` (its age, [`relative_age`](crate::localized::relative_age)).
        details: "events.details" = "{level} · {source} · {time}";
        /// The count words beside the level glyphs, also their accessible names; each is a
        /// [`Counted`](crate::localized::words::Counted) word written with
        /// [`Words::counted`](crate::localized::Words::counted); their
        /// forms are in the kernel's [`COUNTS`](crate::localized::words::COUNTS).
        error_count: "events.errorCount" = "{n} errors";
        warning_count: "events.warningCount" = "{n} warnings";
        message_count: "events.messageCount" = "{n} messages";
    }
}

/// How bad an event is; ordered, so a filter keeps the levels at or above its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventLevel {
    #[default]
    Info,
    Warning,
    Error,
}

impl EventLevel {
    /// The notices' tone with the same meaning: the glyph and colour are theirs.
    fn tone(self) -> Tone {
        match self {
            Self::Info => Tone::Info,
            Self::Warning => Tone::Warn,
            Self::Error => Tone::Bad,
        }
    }
}

/// One event, in the app's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEvent {
    /// The app's id; it keeps the selection while new events arrive.
    pub id: u64,
    pub level: EventLevel,
    /// What reported it: "Auto-save", "Render", a command's name.
    pub source: String,
    pub message: String,
    /// How many times in a row it happened (1 for once).
    pub count: u32,
    /// When, in milliseconds since the Unix epoch (wasm has no `SystemTime::now`; the app reads
    /// its own clock, FilmCraft through `web_time`).
    pub time_ms: u64,
}

/// What the log keeps between passes; the app stores it with its panel state (FilmCraft's
/// `ui.panels.events`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLogState {
    /// The least level shown.
    pub min_level: EventLevel,
    /// The selected event's id.
    pub selected: Option<u64>,
}

/// What the log reported this pass: a composite of the bar, the rows and the details box.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EventLogOutput {
    /// The rows: their focus and their rect.
    pub response: Response,
    /// Clear All was pressed: the app empties its log (FilmCraft runs `events.clear`). The
    /// selection is already dropped.
    pub clear_all: bool,
}

/// FilmCraft's Events panel as a component.
///
/// ```ignore
/// let out = EventLog::new("events", &log_events, &mut panels.events, now_ms).show(ui);
/// if out.clear_all { session.execute("events.clear", json!({})) }
/// ```
///
/// - **Bar.** A [`Select`](crate::controls::Select) for the least level (All Events, Warnings and
///   Errors, Errors Only), then each level's glyph and count (error, warning, info) in
///   `text.muted` ([`EventLogLabels::error_count`], `warning_count`, `message_count`, with the locale's plural
///   forms), then Clear All at the end.
/// - **Rows** (newest first, those at or above the filter's level): a [`ListRow`](super::ListRow)
///   22 points high with the level's tone icon (the notices' info, warning and error icons in
///   their status colours) in the icon slot, the message
///   (with "(×n)" for a repeated one, [`EventLogLabels::repeated`]) clipped before the age, and
///   the age at the end in the `Caption` role and `text.faint`
///   ([`relative_age`](crate::localized::relative_age) of `now_ms - time_ms` in whole seconds; a
///   time after `now_ms` is 0). Striped.
///   A click selects a row, a click on the selected one deselects it.
/// - **Details.** With a row selected, a 70-point box under the rows in `field.fill.rest`:
///   [`EventLogLabels::details`] then the whole message, wrapping, selectable. A click shows or
///   hides the box in the pass of the click.
/// - **Empty.** [`EventLogLabels::none`] in `text.faint`.
/// - **Keys and AccessKit.** The rows are a [`List`](super::List) (one Tab stop, single
///   selection, ↑ ↓ move); each row reads "Warning: Disk almost full, 2 min ago" ([`EventLogLabels::row_name`]).
/// - The ages are whole minutes and up, so the log asks for a repaint a minute after the pass
///   while it shows events; the host passes the new `now_ms` then. No timer runs while the log
///   is hidden.
pub struct EventLog<'a> {
    id_salt: IdSalt,
    events: &'a [LogEvent],
    state: &'a mut EventLogState,
    now_ms: u64,
    labels: Option<EventLogLabels>,
}

impl<'a> EventLog<'a> {
    /// The log of `events` in the order they happened (oldest first, as the app appends them),
    /// with ages counted from `now_ms` (milliseconds since the Unix epoch, from the app's clock:
    /// craft-ui reads no wall clock, and `SystemTime::now` panics on wasm).
    pub fn new(id_salt: impl AsIdSalt, events: &'a [LogEvent], state: &'a mut EventLogState, now_ms: u64) -> Self {
        Self { id_salt: IdSalt::new(id_salt), events, state, now_ms, labels: None }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: EventLogLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/events.rs:52 events
    pub fn show(self, ui: &mut Ui) -> EventLogOutput {
        let Self { id_salt, events, state, now_ms, labels } = self;
        let labels = labels.map_or_else(|| EventLogLabels::shared(ui), Arc::new);
        let tokens = Tokens::of(ui);
        let log = tokens.component.event_log;
        let ctx = ui.ctx().clone();
        let words = Words::of_ctx(&ctx);
        let scope = ui.scope_builder(UiBuilder::new().id_salt(id_salt), |ui| {
            let mut clear_all = false;
            let mut level = state.min_level;
            crate::direction::horizontal(ui, |ui| {
                let _ = Select::new("filter", labels.filter.as_str(), &mut level)
                    .option(EventLevel::Info, labels.all.as_str())
                    .option(EventLevel::Warning, labels.warnings.as_str())
                    .option(EventLevel::Error, labels.errors.as_str())
                    .show(ui);
                for (glyph_level, count, text) in [
                    (EventLevel::Error, counts::EVENT_ERRORS, &labels.error_count),
                    (EventLevel::Warning, counts::EVENT_WARNINGS, &labels.warning_count),
                    (EventLevel::Info, counts::EVENT_MESSAGES, &labels.message_count),
                ] {
                    let n = events.iter().filter(|e| e.level == glyph_level).count() as u64;
                    let tone = glyph_level.tone();
                    tone_icon(ui, tone, IconSize::Sm, 0.0, Notice::style(&tokens, tone).fg);
                    ui.add(Text::new(words.counted(&ctx, count, text, n)).content(Content::Muted));
                }
                ui.with_layout(crate::direction::trailing(ui), |ui| {
                    clear_all = ui.add(Button::new(labels.clear_all.as_str())).clicked();
                });
            });
            state.min_level = level;
            if clear_all {
                state.selected = None;
            }

            // Newest first: the app appends, so the log reads backwards.
            let visible: Vec<&LogEvent> = events.iter().rev().filter(|e| e.level >= state.min_level).collect();
            let shown_before = state.selected.is_some_and(|id| visible.iter().any(|e| e.id == id));
            if visible.is_empty() {
                ui.add(Text::new(labels.none.as_str()).content(Content::Faint));
                return clear_all;
            }
            ctx.request_repaint_after(Duration::from_secs(60));

            let details_height = if shown_before { log.details_height + ui.spacing().item_spacing.y } else { 0.0 };
            let rows: Vec<RowInfo<u64>> = visible.iter().map(|e| RowInfo::leaf(e.id)).collect();
            let mut selection = state.selected.filter(|_| shown_before).map_or_else(Selection::new, Selection::only);
            let caption = role_format(&tokens, TypeRole::Caption, Color32::PLACEHOLDER).font_id;
            let numbers = LocaleNumbers::of_ctx(&ctx);
            let output = List::new("rows", labels.list.as_str(), rows.as_slice())
                .mode(SelectionMode::Single)
                .selection(&mut selection)
                .max_height((ui.available_height() - details_height).max(tokens.component.list.row_height_dense))
                .show(ui, |at, &id| {
                    let event = visible[at];
                    let age = relative_age(&ctx, now_ms.saturating_sub(event.time_ms) / 1000, AgeStyle::Long);
                    let message = match event.count {
                        0 | 1 => event.message.clone(),
                        n => {
                            craft_i18n::fmt(&labels.repeated, &[("message", event.message.as_str()), ("count", numbers.count(u64::from(n)).as_str())])
                        }
                    };
                    let name = craft_i18n::fmt(
                        &labels.row_name,
                        &[("level", level_word(&labels, event.level)), ("message", message.as_str()), ("age", age.as_str())],
                    );
                    let age_width = ctx.fonts_mut(|f| f.layout_no_wrap(age.clone(), caption.clone(), Color32::PLACEHOLDER).size().x).ceil();
                    let tone = event.level.tone();
                    let color = Notice::style(&tokens, tone).fg;
                    let glyph_tokens = &tokens;
                    ListRow::new(id, message)
                        .accessible_name(name)
                        .glyph(move |painter, rect| paint_tone_icon(painter, rect, glyph_tokens, tone, IconSize::Md, color))
                        .striped(at % 2 == 1)
                        .trailing_item(age_width, move |ui| {
                            ui.add(Text::new(age).role(TypeRole::Caption).content(Content::Faint));
                        })
                });
            let before = state.selected;
            for event in &output.events {
                match event {
                    ListEvent::Selection(change) => state.selected = change.selection.items.first().copied(),
                    // A click on the selected row deselects it.
                    ListEvent::Clicked(id) if before == Some(*id) => state.selected = None,
                    _ => {}
                }
            }

            // After the list's events, so a click shows the box in its own pass. The rows were laid out
            // with the room the old selection left; a changed box asks for the pass again.
            let selected = state.selected.and_then(|id| visible.iter().find(|e| e.id == id).copied());
            if selected.is_some() != shown_before {
                ctx.request_discard("event log details box shown or hidden");
            }
            if let Some(event) = selected {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), log.details_height), Sense::hover());
                let field = &tokens.component.field;
                ui.painter().rect_filled(rect, CornerRadius::from(field.radius), field.fill.rest.to_egui());
                let inner = rect.shrink2(vec2(field.pad_x, log.details_pad_y));
                let age = relative_age(&ctx, now_ms.saturating_sub(event.time_ms) / 1000, AgeStyle::Long);
                let line = craft_i18n::fmt(
                    &labels.details,
                    &[("level", level_word(&labels, event.level)), ("source", event.source.as_str()), ("time", age.as_str())],
                );
                ui.scope_builder(UiBuilder::new().max_rect(inner).id_salt("details"), |ui| {
                    ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                    ui.add(Text::new(line).role(TypeRole::Label).content(Content::Muted));
                    ui.add(Text::new(event.message.as_str()));
                });
            }
            clear_all
        });
        EventLogOutput { response: scope.response, clear_all: scope.inner }
    }
}

fn level_word(labels: &EventLogLabels, level: EventLevel) -> &str {
    match level {
        EventLevel::Info => &labels.info,
        EventLevel::Warning => &labels.warning,
        EventLevel::Error => &labels.error,
    }
}
