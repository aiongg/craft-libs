//! The render (export) queue list (N1.15).

use std::cell::RefCell;
use std::time::Duration;

use super::style::TimelineStyle;
use craft_theme::{ButtonVariant, ControlSize};
use craft_ui::behavior::{Axis, ClaimedKey, KeyClaim, Move, Reorder, RowInfo, Selection, SelectionChange};
use craft_ui::controls::{Button, Progress};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::list::{List, ListEvent, ListRow, ThumbSize};
use craft_ui::localized::words::Counted;
use craft_ui::localized::{Labels as _, LocaleNumbers, Words, fmt};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::{Align2, AsIdSalt, Context, IdSalt, Key, Modifiers, Rect, Response, RichText, Ui, pos2, vec2};

craft_ui::words! {
    /// The queue's words, English by default (FilmCraft's).
    #[non_exhaustive]
    pub struct RenderQueueLabels {
        title: "renderQueue.title" = "Destinations";
        empty: "renderQueue.empty" = "Send exports here with Send to Queue, then start the queue.";
        ready: "renderQueue.ready" = "Ready";
        encoding: "renderQueue.encoding" = "Encoding {n}%";
        done: "renderQueue.done" = "Done";
        failed: "renderQueue.failed" = "Failed: {e}";
        cancelled: "renderQueue.cancelled" = "Cancelled";
        time_left: "renderQueue.timeLeft" = "{time} left";
        seconds: "renderQueue.seconds" = "{n} s";
        cancel: "renderQueue.cancel" = "Cancel";
        retry: "renderQueue.retry" = "Retry";
        remove: "renderQueue.remove" = "Remove";
        start: "renderQueue.start" = "Start Queue";
        stop: "renderQueue.stop" = "Stop";
        clear_finished: "renderQueue.clearFinished" = "Clear Finished";
    }
}

/// The seconds of a time left under a minute (FilmCraft's `45 s`): the count word of
/// [`RenderQueueLabels::seconds`].
pub const SECONDS: Counted = Counted::new("renderQueue.seconds", "{n} s", "{n} s");

/// An item's state.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderStatus {
    /// Waiting in the queue.
    Ready,
    /// Progress 0…1 and the time left when the encoder knows it.
    Encoding {
        /// The share done, from 0 to 1.
        progress: f32,
        /// The time left; `None` while the encoder cannot tell.
        remaining: Option<Duration>,
    },
    /// Finished.
    Done,
    /// The encoder's message, in the app's words.
    Failed(String),
    /// Cancelled by the user.
    Cancelled,
}

/// A queue item's id, the app's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderItemId(pub u64);

/// A queue row: "Sequence → file.mp4", the preset, the status.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderQueueItem {
    /// The item's identity.
    pub id: RenderItemId,
    /// The source name, such as the sequence.
    pub source: String,
    /// The output file name.
    pub file: String,
    /// The name of the export preset.
    pub preset: String,
    /// The item's state.
    pub status: RenderStatus,
}

/// What the queue asks of the app, in the order the user did it this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RenderQueueEvent {
    /// A drag of rows, or Cmd/Ctrl+] and [ on the selected rows (the list's reorder; Premiere's
    /// queue moves items by drag), as one gesture.
    Reorder(EditEvent<Move<RenderItemId>>),
    /// The selection changed.
    Selection(SelectionChange<RenderItemId>),
    /// Cancel the item.
    Cancel(RenderItemId),
    /// Run a failed or cancelled item again.
    Retry(RenderItemId),
    /// The row's Remove button, or Delete or Backspace on the selection (one event per item, in
    /// order).
    Remove(RenderItemId),
    /// Start the queue.
    Start,
    /// Stop the queue.
    Stop,
    /// Remove every finished item.
    ClearFinished,
}

/// FilmCraft's export queue (Premiere's queue in Export mode), drawn through the kernel's
/// [`List`] of [`ListRow`]s: each row's label
/// "source → file", its detail line "preset · status"; an Encoding row shows its progress with a
/// [`Progress`] bar and the time left on its own line
/// ([`RenderQueueLabels::time_left`] with the time written as FilmCraft's `format_eta`: `45 s`
/// ([`RenderQueueLabels::seconds`], a count word) under a minute, `m:ss` under an hour, then
/// `h:mm:ss`, digits through `LocaleNumbers`).
/// Trailing row buttons: Cancel (Ready or Encoding) or Retry (Failed or Cancelled), and Remove
/// (not while encoding). Under the list a row of buttons: Start Queue (when something is Ready
/// and the queue is stopped), Stop (while running), Clear Finished (when an item is Done, Failed
/// or Cancelled). The empty queue shows [`RenderQueueLabels::empty`].
///
/// Keyboard and AccessKit are the list's: one Tab stop, arrows move, Shift and Cmd/Ctrl select,
/// Cmd/Ctrl+] and [ move the selected rows (decisions §4.2, lists), Delete and Backspace remove
/// them; the
/// trailing buttons are row actions; the footer buttons are Tab stops after the list.
pub struct RenderQueueList<'a> {
    id_salt: IdSalt,
    items: &'a [RenderQueueItem],
    selection: &'a mut Selection<RenderItemId>,
    running: bool,
    labels: Option<RenderQueueLabels>,
}

impl<'a> RenderQueueList<'a> {
    /// A list of `items` with the app's `selection`; `running` tells whether the queue is encoding, which enables Stop and disables Start.
    pub fn new(id_salt: impl AsIdSalt, items: &'a [RenderQueueItem], selection: &'a mut Selection<RenderItemId>, running: bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), items, selection, running, labels: None }
    }

    /// The words of the list; `RenderQueueLabels::of` when not set.
    pub fn labels(mut self, labels: RenderQueueLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Show the list and report what the user did.
    // filmcraft: crates/ui-egui/src/panels/export_mode/queue.rs:7 `queue_column`;
    // crates/export/src/settings.rs:769 `format_eta`.
    pub fn show(self, ui: &mut Ui) -> RenderQueueListOutput {
        let Self { id_salt, items, selection, running, labels } = self;
        let labels = labels.unwrap_or_else(|| RenderQueueLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let tokens = Tokens::of(ui);
        let row_style = ListRow::<RenderItemId>::style(&tokens);
        let bar_style = Progress::style(&tokens);
        let button_height = Button::style(&tokens, ButtonVariant::Secondary, ControlSize::Md).height;
        let caption = role_format(&tokens, TimelineStyle::of(&tokens).render_queue_text, tokens.color.text.muted.to_egui());
        let caption_line = tokens.typography.get(TimelineStyle::of(&tokens).render_queue_text).line_height;
        let (caption_font, caption_color) = (caption.font_id.clone(), caption.color);
        let ctx = ui.ctx().clone();
        let gap = ui.spacing().item_spacing.y;
        let list_id = List::id_in(ui, id_salt);
        let events = RefCell::new(Vec::new());

        let footer = gap + button_height;
        let response = if items.is_empty() {
            let height = (ui.available_height() - footer).max(0.0);
            ui.allocate_ui(vec2(ui.available_width(), height), |ui| ui.label(RichText::new(&labels.empty).color(caption.color))).inner
        } else {
            let order: Vec<RowInfo<RenderItemId>> = items.iter().map(|item| RowInfo::leaf(item.id)).collect();
            let button = |text: &str| Button::new(text).size(ControlSize::Sm).natural_width(ui);
            let widths = [button(&labels.cancel).max(button(&labels.retry)), button(&labels.remove)];
            let encoding_height = (bar_style.height + gap + caption_line + 2.0 * row_style.gap).max(row_style.height(ThumbSize::None, true));
            let height = |at: usize| match items[at].status {
                RenderStatus::Encoding { .. } => encoding_height,
                _ => row_style.height(ThumbSize::None, true),
            };
            let max_height = (ui.available_height() - footer).max(0.0);
            let output = List::new(id_salt, &labels.title, order.as_slice())
                .selection(&mut *selection)
                .heights(height)
                .max_height(max_height)
                .reorder(Reorder::new(Axis::Vertical))
                .show(ui, |at, _| {
                    let item = &items[at];
                    let events = &events;
                    let status = match &item.status {
                        RenderStatus::Ready => labels.ready.clone(),
                        RenderStatus::Encoding { progress, .. } => {
                            fmt(&labels.encoding, &[("n", &numbers.format(f64::from(Progress::clamped(*progress)) * 100.0, 0..=0))])
                        }
                        RenderStatus::Done => labels.done.clone(),
                        RenderStatus::Failed(error) => fmt(&labels.failed, &[("e", error)]),
                        RenderStatus::Cancelled => labels.cancelled.clone(),
                    };
                    let mut row =
                        ListRow::new(item.id, format!("{} \u{2192} {}", item.source, item.file)).detail(format!("{} \u{b7} {status}", item.preset));
                    let id = item.id;
                    let (remove_text, cancel_text, retry_text) = (labels.remove.as_str(), labels.cancel.as_str(), labels.retry.as_str());
                    let encoding = matches!(item.status, RenderStatus::Encoding { .. });
                    row = row.trailing_item(widths[1], move |ui| {
                        let removed = ui.add(Button::new(remove_text).size(ControlSize::Sm).enabled(!encoding).min_width(widths[1])).clicked();
                        if removed {
                            events.borrow_mut().push(RenderQueueEvent::Remove(id));
                        }
                    });
                    let (label, event) = match item.status {
                        RenderStatus::Ready | RenderStatus::Encoding { .. } => (cancel_text, RenderQueueEvent::Cancel(id)),
                        _ => (retry_text, RenderQueueEvent::Retry(id)),
                    };
                    row = row.trailing_item(widths[0], move |ui| {
                        if ui.add(Button::new(label).size(ControlSize::Sm).min_width(widths[0])).clicked() {
                            events.borrow_mut().push(event);
                        }
                    });
                    if let RenderStatus::Encoding { progress, remaining } = item.status {
                        let left = remaining.map(|time| fmt(&labels.time_left, &[("time", &format_eta(&ctx, &labels.seconds, time))]));
                        let text_width = left.as_ref().map_or(0.0, |left| {
                            ctx.fonts_mut(|fonts| fonts.layout_no_wrap(left.clone(), caption_font.clone(), caption_color).size().x)
                        });
                        let font = caption_font.clone();
                        row = row.optional_item(bar_style.width.max(text_width), move |ui| {
                            let rect = ui.max_rect();
                            let lines = bar_style.height + left.as_ref().map_or(0.0, |_| gap + caption_line);
                            let top = rect.center().y - lines / 2.0;
                            let bar = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), bar_style.height));
                            ui.put(bar, Progress::new(progress).width(rect.width()));
                            if let Some(left) = left {
                                let at = pos2(rect.left(), bar.bottom() + gap);
                                ui.painter().text(at, Align2::LEFT_TOP, left, font, caption_color);
                            }
                        });
                    }
                    row
                });
            let mut out = events.borrow_mut();
            for event in output.events {
                match event {
                    ListEvent::Selection(change) => out.push(RenderQueueEvent::Selection(change)),
                    ListEvent::Reorder(edit) => out.push(RenderQueueEvent::Reorder(edit)),
                    _ => {}
                }
            }
            KeyClaim::claim(ui.ctx(), list_id, &CLAIMED);
            let delete = ui.memory(|memory| memory.has_focus(list_id)) && CLAIMED.map(|key| ui.input_mut(|input| key.consume(input))).contains(&true);
            if delete {
                for item in items.iter().filter(|item| selection.contains(&item.id) && !matches!(item.status, RenderStatus::Encoding { .. })) {
                    out.push(RenderQueueEvent::Remove(item.id));
                }
            }
            output.response
        };

        let ready = items.iter().any(|item| item.status == RenderStatus::Ready);
        let finished = items.iter().any(|item| matches!(item.status, RenderStatus::Done | RenderStatus::Failed(_) | RenderStatus::Cancelled));
        ui.add_space(gap);
        ui.with_layout(direction::row(ui), |ui| {
            let mut out = events.borrow_mut();
            let start = Button::new(&labels.start).variant(ButtonVariant::Primary).enabled(!running && ready);
            if ui.add(start).clicked() {
                out.push(RenderQueueEvent::Start);
            }
            if ui.add(Button::new(&labels.stop).enabled(running)).clicked() {
                out.push(RenderQueueEvent::Stop);
            }
            if ui.add(Button::new(&labels.clear_finished).enabled(finished)).clicked() {
                out.push(RenderQueueEvent::ClearFinished);
            }
        });
        RenderQueueListOutput { response, events: events.into_inner() }
    }
}

/// What the queue did this pass.
#[must_use]
pub struct RenderQueueListOutput {
    /// The list's response, for hover and focus.
    pub response: Response,
    /// What the user asked for this pass, in order.
    pub events: Vec<RenderQueueEvent>,
}

/// The keys a focused queue keeps from the app's shortcuts.
const CLAIMED: [ClaimedKey; 2] = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];

/// A time left, rounded up to the second, in the locale's digits: `45 s`, `2:05`, `1:02:05`.
pub(super) fn format_eta(ctx: &Context, seconds_word: &str, time: Duration) -> String {
    let numbers = LocaleNumbers::of_ctx(ctx);
    let seconds = time.as_secs().saturating_add(u64::from(time.subsec_nanos() > 0));
    let digits = |n: u64| numbers.format(n as f64, 0..=0);
    let two = |n: u64| if n < 10 { format!("{}{}", digits(0), digits(n)) } else { digits(n) };
    match seconds {
        0..60 => Words::of_ctx(ctx).counted(ctx, SECONDS, seconds_word, seconds),
        60..3600 => format!("{}:{}", digits(seconds / 60), two(seconds % 60)),
        _ => format!("{}:{}:{}", digits(seconds / 3600), two(seconds % 3600 / 60), two(seconds % 60)),
    }
}
