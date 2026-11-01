//! The timecode field and the Timecode panel (N1.09, N1.14).

use craft_theme::{ControlSize, TypeRole};
use craft_ui::behavior::scrub::travel;
use craft_ui::behavior::{ClaimedKey, KeyClaim, Scrub};
use craft_ui::controls::text_field::{BoxState, field_box};
use craft_ui::controls::{IconButton, TextField};
use craft_ui::edit::{EditPolicy, EditResponse, Gesture};
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format, tabular};
use egui::accesskit::{Invalid, Role};
use egui::text::{CCursorRange, LayoutJob};
use egui::text_edit::TextEditState;
use egui::{Align, AsIdSalt, CursorIcon, EventFilter, IdSalt, Key, Layout, Modifiers, Response, Sense, TextEdit, Ui, WidgetInfo, pos2, vec2};

use super::style::TimelineStyle;
use crate::time::Tick;
use crate::timecode::{TimeDisplay, TimeFormat, format_time, parse_time};

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

/// The text of a field being typed in.
#[derive(Clone, Debug, Default)]
struct Typing {
    text: String,
    /// The text has not been selected yet (the field takes the focus first).
    select: bool,
    /// The last Enter or click-away held text that doesn't parse.
    invalid: bool,
}

/// Premiere's big timecode readout and the smaller timecode fields of dialogs.
///
/// - Drag horizontally to scrub through the kernel's [`Scrub`](craft_ui::behavior::Scrub) (the
///   pointer travel and the screen-edge wrap, as [`NumberField`](craft_ui::controls::NumberField)
///   scrubs): one frame per `style.scrub_step` of travel ([`TimelineStyle`](super::TimelineStyle)),
///   Shift ×10 (FilmCraft `widgets.rs:130`); a gesture of `EditEvent<Tick>`s, Esc cancels it. A
///   click without a drag edits (decisions §4.2, NumberField).
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
        let Self { id_salt, time, format, min, large, width } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let field = TextField::style(&tokens, ControlSize::Md);
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let typing_key = id.with("typing");
        let rate = format.rate;
        let role = if large { TypeRole::Heading } else { TypeRole::Mono };
        let color = if large { tokens.color.accent.text.to_egui() } else { field.text };
        let text_format = tabular(role_format(&tokens, role, color));
        let ctx = ui.ctx().clone();
        let layout = |text: &str| ctx.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(text.to_owned(), text_format.clone())));
        let typing = ui.data(|d| d.get_temp::<Typing>(typing_key));

        let shown_text = |t: Tick| format_time(t, format);
        let natural = layout(&shown_text(time)).size();
        let size = vec2(width.unwrap_or(natural.x + 2.0 * field.pad_x), field.height.max(natural.y));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

        if let Some(mut typed) = typing {
            // Typing: Enter or a click away commits, Esc cancels.
            let edit_id = id.with("edit");
            let mut gesture = Gesture::load(ui, id, &time, EditPolicy::default()).enabled(enabled);
            let state = BoxState { enabled, focused: true, hovered: false, invalid: typed.invalid };
            ui.painter().extend(field_box(ui, &field, rect, state));
            let inner = rect.shrink2(vec2(field.pad_x, 0.0));
            let edit = TextEdit::singleline(&mut typed.text)
                .id(edit_id)
                .font(text_format.font_id.clone())
                .text_color(color)
                .frame(egui::Frame::NONE)
                .desired_width(inner.width())
                .align(egui::Align2::LEFT_CENTER);
            let response = ui.put(inner, edit);
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(Role::TextInput);
                node.set_label(format.display.label());
            });
            if typed.select {
                let mut state = TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
                state.cursor.set_char_range(Some(CCursorRange::select_all_str(&typed.text)));
                state.store(ui.ctx(), edit_id);
                typed.select = false;
            }
            let escaped = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Escape));
            let entered = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let mut done = escaped;
            if (response.lost_focus() && !escaped) || entered {
                match parse_time(&typed.text, format, time) {
                    Ok(t) => {
                        gesture.set(t.max(min));
                        done = true;
                    }
                    Err(_) => {
                        typed.invalid = true;
                        response.request_focus();
                    }
                }
            } else if !response.has_focus() && !typed.select {
                response.request_focus();
            }
            if typed.invalid {
                ui.ctx().accesskit_node_builder(response.id, |node| node.set_invalid(Invalid::True));
            }
            if done {
                ui.data_mut(|d| d.remove::<Typing>(typing_key));
            } else {
                ui.data_mut(|d| d.insert_temp(typing_key, typed));
            }
            return EditResponse::new(response, gesture.finish()).submitted(entered).escaped(escaped);
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
            let enter = response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            if response.clicked() || enter {
                let current = shown_text(*gesture.shown());
                ui.data_mut(|d| d.insert_temp(typing_key, Typing { text: current, select: true, invalid: false }));
                ui.ctx().request_repaint();
            }
            if response.has_focus() && !gesture.is_active() {
                ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { vertical_arrows: true, ..EventFilter::default() }));
                KeyClaim::claim(ui.ctx(), id, &FIELD_KEYS);
                let key = ui.input_mut(|i| {
                    [true, false].into_iter().find_map(|shift| {
                        let modifiers = if shift { Modifiers::SHIFT } else { Modifiers::NONE };
                        [Key::ArrowUp, Key::ArrowDown].into_iter().find(|key| i.consume_key(modifiers, *key)).map(|key| (key, shift))
                    })
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
        response.widget_info(|| WidgetInfo::labeled(Role::SpinButton, enabled, format.display.label()));
        ui.ctx().accesskit_node_builder(response.id, |node| node.set_value(text.as_str()));
        EditResponse::new(response, gesture.finish())
    }
}

/// What a row's menu can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowCommand {
    Kind(TimecodeRowKind),
    Source(TimecodeSource),
    Display(TimeDisplay),
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
fn row_menu(labels: &TimecodePanelLabels, row: &TimecodeRow, removable: bool) -> Vec<MenuEntry<RowCommand>> {
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
    entries.extend(
        TimeDisplay::ALL.map(|display| MenuItem::new(display.label(), RowCommand::Display(display)).radio(row.display == Some(display)).into()),
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

/// One row of the Timecode panel.
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
/// stops; the Menu key and Shift+F10 open a focused row's menu.
pub struct TimecodePanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut TimecodePanelSettings,
    text: &'a dyn Fn(&TimecodeRow) -> String,
    labels: Option<TimecodePanelLabels>,
}

impl<'a> TimecodePanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut TimecodePanelSettings, text: &'a dyn Fn(&TimecodeRow) -> String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, text, labels: None }
    }

    pub fn labels(mut self, labels: TimecodePanelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Changes the settings directly (a panel preference, not a document edit) and says so on
    /// the output.
    // filmcraft: crates/ui-egui/src/panels/timecode.rs:104 `show`, `:21` `row_text`.
    pub fn show(self, ui: &mut Ui) -> TimecodePanelOutput {
        let Self { id_salt, settings, text, labels } = self;
        let labels = labels.unwrap_or_else(|| TimecodePanelLabels::of(ui));
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let enabled = ui.is_enabled();
        let colors = &tokens.color;
        let caption_format = role_format(&tokens, TypeRole::Caption, colors.text.muted.to_egui());
        let mut picks: Vec<(usize, RowCommand)> = Vec::new();
        let mut add = false;
        let removable = settings.rows.len() > 1;
        let response = ui
            .vertical(|ui| {
                for (at, row) in settings.rows.iter().enumerate() {
                    let big = at == 0;
                    let value = text(row);
                    let caption = format!("{} \u{b7} {}", row_kind_label(&labels, row.kind), row_source_label(&labels, row.source));
                    let (role, color) = if big { (TypeRole::Heading, colors.accent.text) } else { (TypeRole::Mono, colors.text.normal) };
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
                    let spoken = format!("{caption}: {value}");
                    row_response.widget_info(|| WidgetInfo::new(Role::Label));
                    ui.ctx().accesskit_node_builder(row_response.id, |node| node.set_value(spoken.as_str()));
                    let entries = row_menu(&labels, row, removable);
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
                RowCommand::Display(display) => row.display = Some(display),
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
    pub rows: Vec<TimecodeRow>,
}

/// What the Timecode panel did this pass.
#[must_use]
pub struct TimecodePanelOutput {
    pub response: Response,
    /// A row was added, removed or changed; the app saves [`TimecodePanelSettings`].
    pub settings_changed: bool,
}
