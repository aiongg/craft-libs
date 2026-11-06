//! Markers on a ruler or a clip bar, and the Marker dialog (N1.12).

use craft_ui::behavior::{ClaimedKey, KeyClaim};
use craft_ui::composites::FormGrid;
use craft_ui::controls::{Swatch, TextField, TooltipExt as _};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::{EditEvent, EditPolicy, Gesture, GestureDraft};
use craft_ui::icons::{Icon, IconSize};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Corner, FocusRing};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format, scale};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{
    AsIdSalt, Context, CursorIcon, EventFilter, Id, IdSalt, Key, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, Ui,
    WidgetInfo, pos2, vec2,
};

use super::LabelPalette;
use super::style::TimelineStyle;
use crate::clips::{SnapTargets, snap_time};
use crate::markers::{Marker, MarkerColor, MarkerId};
use crate::time::{Tick, TimeView};
use crate::timecode::{TimeFormat, format_time};

use super::timecode::TimecodeField;

/// The fill of a range marker's duration bar, as a share of the label colour (EffectCraft
/// `markers_ui.rs:296`). A drawing detail (decisions §3.4).
const DURATION_TINT: f32 = 0.5;

/// What a marker gesture asks of the app.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MarkerEdit {
    /// A drag of the marker: its new time (snapped), as one gesture; Esc cancels.
    Move {
        /// The marker that moves.
        id: MarkerId,
        /// The new start time.
        to: EditEvent<Tick>,
    },
    /// An Alt-drag: its new duration, as one gesture.
    SetDuration {
        /// The marker whose duration changes.
        id: MarkerId,
        /// The new duration.
        duration: EditEvent<Tick>,
    },
    /// Cmd-click (Ctrl-click on Windows and Linux), or Delete with the marker picked.
    Delete(MarkerId),
    /// Double-click or Enter: open the Marker dialog on it.
    Open(MarkerId),
    /// A marker dragged out of the marker bin and dropped at `at`, or Add Marker (M).
    Add {
        /// Where the new marker starts.
        at: Tick,
    },
}

/// A row of markers in `rect` (the ruler's marker band, a clip or layer bar): a flag
/// `style.marker_width` wide ([`TimelineStyle`]) pointing down at its time
/// (the point at [`MARKER_POINT`] of the height), coloured by its label, a duration bar for
/// range markers, the name beside it when there is room.
///
/// After Effects' gestures (EffectCraft `markers_ui.rs:1`): drag moves (snapping to `snap`),
/// Alt-drag sets the duration, Cmd-click (Ctrl-click on Windows and Linux) deletes, a
/// double-click opens the dialog, a secondary click is reported for the app's menu (Settings,
/// Convert, Delete). With [`bin`](Self::bin) a marker dragged out of the bin at the right end
/// adds one where it is dropped. Tooltips show name and comment. Keyboard: the strip is one Tab
/// stop (the markers and the bin are pointer-only), Left / Right pick the previous / next
/// marker, Enter opens it, Delete and Backspace delete it (claimed through
/// [`KeyClaim`]); each chord matches its modifiers exactly.
/// AccessKit: a `ListBox` named by [`MarkerStripLabels`]' title, holding the markers named by
/// name and time.
pub struct MarkerStrip<'a> {
    id_salt: IdSalt,
    markers: &'a [Marker],
    view: &'a TimeView,
    snap: Option<&'a SnapTargets>,
    palette: Option<&'a LabelPalette>,
    bin: Option<Rect>,
    format: Option<TimeFormat>,
    labels: Option<MarkerStripLabels>,
}

impl<'a> MarkerStrip<'a> {
    /// A strip for `markers` on the time `view`.
    pub fn new(id_salt: impl AsIdSalt, markers: &'a [Marker], view: &'a TimeView) -> Self {
        Self { id_salt: IdSalt::new(id_salt), markers, view, snap: None, palette: None, bin: None, format: None, labels: None }
    }

    /// The accessibility words; `MarkerStripLabels::of` when not set.
    pub fn labels(mut self, labels: MarkerStripLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The snap targets for drags; `None` turns snapping off.
    pub fn snap(mut self, targets: Option<&'a SnapTargets>) -> Self {
        self.snap = targets;
        self
    }

    /// The palette that gives the markers' label colours; the default palette when not set.
    pub fn palette(mut self, palette: &'a LabelPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// The marker bin (After Effects' at the right end of the ruler, EffectCraft `:429`).
    pub fn bin(mut self, rect: Rect) -> Self {
        self.bin = Some(rect);
        self
    }

    /// The time format of the markers' accessible names (name and time); without one the names
    /// are the markers' names alone.
    pub fn format(mut self, format: TimeFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Show the strip in `rect` and report what the user did.
    // effectcraft: crates/ui-egui/src/panels/markers_ui.rs:274 `marker_shape`, `:282` `marker`,
    // `:253` `snap`, `:380` `name_box_x`, `:408` `comp_markers`, `:429` `marker_bin`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> MarkerStripResponse {
        let Self { id_salt, markers, view, snap, palette, bin, format, labels } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let labels = labels.unwrap_or_else(|| MarkerStripLabels::of(ui));
        let owned;
        let palette = match palette {
            Some(palette) => palette,
            None => {
                owned = LabelPalette::default();
                &owned
            }
        };
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let response = ui.interact(rect, id, Sense::focusable_noninteractive());
        response.widget_info(|| WidgetInfo::labeled(Role::ListBox, enabled, labels.title.as_str()));

        let mut order: Vec<&Marker> = markers.iter().collect();
        order.sort_by_key(|marker| marker.start);
        let drag_key = id.with("drag");
        let picked_key = id.with("picked");
        let (move_id, length_id) = (id.with("move"), id.with("duration"));
        let reported = |gesture: Id| Gesture::<Tick>::reported(ui, gesture).map_or(Tick::ZERO, |(_, delta)| delta);
        let mut moving = Gesture::load(ui, move_id, &reported(move_id), EditPolicy::default()).enabled(enabled);
        let mut sizing = Gesture::load(ui, length_id, &reported(length_id), EditPolicy::default()).enabled(enabled);
        if !moving.is_active() && !sizing.is_active() {
            ui.data_mut(|d| d.remove::<Drag>(drag_key));
        }
        let mut drag = ui.data(|d| d.get_temp::<Drag>(drag_key));
        let mut picked = ui.data(|d| d.get_temp::<MarkerId>(picked_key)).filter(|picked| markers.iter().any(|m| m.id == *picked));
        let modifiers = ui.input(|i| i.modifiers);
        let half_hit = style.marker_hit / 2.0;
        let mut out = MarkerStripResponse { response, edits: Vec::new(), context_menu: None };

        // Where a dragged time lands: on a snap target other than the marker's own place.
        let settle = |t: Tick, own: Option<Tick>| match snap {
            Some(targets) => {
                let mut others = SnapTargets::new();
                others.extend(targets.times().iter().copied().filter(|target| Some(*target) != own));
                snap_time(&others, t.max(Tick::ZERO), view, style.snap_distance).0
            }
            None => t.max(Tick::ZERO),
        };
        let (moved, resized) = (*moving.shown(), *sizing.shown());
        let shown = |marker: &Marker, drag: Option<Drag>, moved: Tick, resized: Tick| match drag {
            Some(grab) if grab.id == marker.id => (grab.start + moved, grab.length + resized),
            _ => (marker.start, marker.duration),
        };

        if enabled {
            for marker in &order {
                let (start, _) = shown(marker, drag, moved, resized);
                let x = view.x_of(start, rect.left());
                let grabbed = drag.is_some_and(|grab| grab.id == marker.id);
                if !grabbed && (x < rect.left() - half_hit || x > rect.right() + half_hit) {
                    continue;
                }
                let hit = Rect::from_min_max(pos2(x - half_hit, rect.top()), pos2(x + half_hit, rect.bottom()));
                let marker_response = ui.interact(hit, id.with(("marker", marker.id.0)), Sense::CLICK | Sense::DRAG);
                if marker_response.drag_started_by(PointerButton::Primary) {
                    out.response.request_focus();
                    picked = Some(marker.id);
                    let grab = Drag { id: marker.id, length_drag: modifiers.alt, start: marker.start, length: marker.duration };
                    ui.data_mut(|d| d.insert_temp(drag_key, grab));
                    drag = Some(grab);
                    if grab.length_drag {
                        sizing.begin();
                    } else {
                        moving.begin();
                    }
                }
                if let Some(grab) = drag.filter(|grab| grab.id == marker.id)
                    && marker_response.dragged_by(PointerButton::Primary)
                    && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
                {
                    let at = settle(view.time_at(pos.x, rect.left()), Some(grab.start));
                    if grab.length_drag {
                        sizing.update(at.max(grab.start) - grab.start - grab.length, modifiers);
                    } else {
                        moving.update(at - grab.start, modifiers);
                    }
                }
                if marker_response.drag_stopped() {
                    moving.commit();
                    sizing.commit();
                }
                if marker_response.clicked_by(PointerButton::Primary) {
                    out.response.request_focus();
                    picked = Some(marker.id);
                    if modifiers.command {
                        out.edits.push(MarkerEdit::Delete(marker.id));
                    }
                }
                if marker_response.double_clicked_by(PointerButton::Primary) {
                    out.edits.push(MarkerEdit::Open(marker.id));
                }
                if marker_response.secondary_clicked() {
                    out.context_menu = Some(marker.id);
                }
                let name = match &format {
                    Some(format) => format!("{}, {}", marker.name, format_time(marker.start, format)),
                    None => marker.name.clone(),
                };
                let on = picked == Some(marker.id);
                marker_response.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, enabled, on, name.as_str()));
                let tip =
                    [marker.name.as_str(), marker.comment.as_str()].iter().filter(|text| !text.is_empty()).copied().collect::<Vec<_>>().join("\n");
                marker_response.tooltip(if tip.is_empty() { labels.title.clone() } else { tip });
            }
            if out.response.has_focus() && !order.is_empty() {
                pick_by_key(ui, id, &order, &mut picked, &mut out.edits);
            }
            ui.data_mut(|d| match picked {
                Some(picked) => {
                    d.insert_temp(picked_key, picked);
                }
                None => {
                    d.remove::<MarkerId>(picked_key);
                }
            });
        }

        let (moved, resized) = (*moving.shown(), *sizing.shown());

        // The drag's events, in the markers' own times.
        if let Some(grab) = drag {
            let to = moving.finish_as(|delta| grab.start + delta);
            let duration = sizing.finish_as(|delta| grab.length + delta);
            out.edits.extend(to.map(|to| MarkerEdit::Move { id: grab.id, to }));
            out.edits.extend(duration.map(|duration| MarkerEdit::SetDuration { id: grab.id, duration }));
        } else {
            let _ = (moving.finish(), sizing.finish());
        }

        // Paint: duration bars and names first, then every flag above them.
        let painter = ui.painter_at(rect);
        let color = &tokens.color;
        let hairline = scale::Stroke::Hairline.width(ui.pixels_per_point());
        let height = rect.height().min(style.marker_height);
        let text = role_format(&tokens, style.marker_text, color.text.normal.to_egui());
        let radius = Corner::Control.radius(&tokens);
        let placed: Vec<(&Marker, Tick, Tick)> = order
            .iter()
            .map(|marker| {
                let (start, length) = shown(marker, drag, moved, resized);
                (*marker, start, length)
            })
            .collect();
        for &(marker, start, length) in &placed {
            let x = view.x_of(start, rect.left());
            let tint = palette.color(marker.color);
            let end = (length > Tick::ZERO).then(|| view.x_of(start + length, rect.left()));
            if let Some(end) = end {
                let bar = Rect::from_min_max(pos2(x, rect.top() + hairline), pos2(end, rect.top() + height - hairline));
                painter.rect_filled(bar, egui::CornerRadius::ZERO, tint.gamma_multiply(DURATION_TINT));
            }
            if marker.name.is_empty() {
                continue;
            }
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(marker.name.clone(), text.clone())));
            let width = galley.size().x + 2.0 * tokens.space.gap.inline;
            if let Some(left) = name_box_x(x, width, end, rect, tokens.space.gap.inline, tokens.space.gap.tight) {
                let name_box = Rect::from_min_size(pos2(left, rect.top()), vec2(width, height.max(galley.size().y)));
                painter.rect(
                    name_box,
                    radius,
                    color.surface.sunken.to_egui(),
                    Stroke::new(hairline, color.line.control_edge.to_egui()),
                    egui::StrokeKind::Inside,
                );
                painter.galley(pos2(name_box.left() + tokens.space.gap.inline, name_box.center().y - galley.size().y / 2.0), galley, text.color);
            }
        }
        for &(marker, start, length) in &placed {
            let x = view.x_of(start, rect.left());
            let tint = palette.color(marker.color);
            let flag = flag(x, rect.top(), style.marker_width, height);
            painter.add(Shape::convex_polygon(flag, tint, Stroke::NONE));
            if length > Tick::ZERO {
                let end = view.x_of(start + length, rect.left());
                painter.line_segment(
                    [pos2(end, rect.top()), pos2(end, rect.top() + height)],
                    Stroke::new(scale::Stroke::W1.width(ui.pixels_per_point()), tint),
                );
            }
        }
        if let Some(picked) = picked
            && out.response.has_focus()
            && let Some(&(_, start, _)) = placed.iter().find(|(marker, ..)| marker.id == picked)
        {
            let ring = Rect::from_center_size(pos2(view.x_of(start, rect.left()), rect.top() + height / 2.0), vec2(style.marker_hit, height));
            FocusRing::paint_radius(ui, ring, radius, &out.response);
        } else {
            FocusRing::paint_radius(ui, rect, radius, &out.response);
        }

        if let Some(bin) = bin {
            let bin_cx = BinCx {
                bin,
                strip: rect,
                height,
                flag_width: style.marker_width,
                enabled,
                labels: &labels,
                tint: palette.color(MarkerColor::default()),
            };
            if let Some(at) = marker_bin(ui, id.with("bin"), &bin_cx, view, &settle, &tokens) {
                out.edits.push(MarkerEdit::Add { at });
            }
        }
        out
    }
}

/// Whether `key` was pressed with no modifier this pass, and consumes it; an extra Shift or Alt
/// is another chord.
fn pressed(ui: &Ui, key: Key) -> bool {
    ui.input_mut(|i| ClaimedKey::exact(Modifiers::NONE, key).consume(i))
}

/// Left and Right pick the previous and next marker, Enter opens the picked one, Delete and
/// Backspace delete it.
fn pick_by_key(ui: &Ui, id: Id, order: &[&Marker], picked: &mut Option<MarkerId>, edits: &mut Vec<MarkerEdit>) {
    ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, ..EventFilter::default() }));
    KeyClaim::claim(ui.ctx(), id, &[ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)]);
    let at = picked.and_then(|picked| order.iter().position(|m| m.id == picked));
    if pressed(ui, Key::ArrowRight) {
        *picked = Some(order[at.map_or(0, |at| (at + 1).min(order.len() - 1))].id);
    }
    if pressed(ui, Key::ArrowLeft) {
        *picked = Some(order[at.map_or(order.len() - 1, |at| at.saturating_sub(1))].id);
    }
    if let Some(picked) = *picked {
        if pressed(ui, Key::Enter) {
            edits.push(MarkerEdit::Open(picked));
        }
        if pressed(ui, Key::Delete) || pressed(ui, Key::Backspace) {
            edits.push(MarkerEdit::Delete(picked));
        }
    }
}

/// The flag's outline: a tab `width` wide pointing down at `x`, `height` tall from `top`
/// (EffectCraft `markers_ui.rs:274`).
fn flag(x: f32, top: f32, width: f32, height: f32) -> Vec<Pos2> {
    let half = width / 2.0;
    let point = top + height * MARKER_POINT;
    vec![pos2(x - half, top), pos2(x + half, top), pos2(x + half, point), pos2(x, top + height), pos2(x - half, point)]
}

/// The marker bin and what its drag needs.
struct BinCx<'a> {
    bin: Rect,
    strip: Rect,
    height: f32,
    flag_width: f32,
    enabled: bool,
    labels: &'a MarkerStripLabels,
    tint: egui::Color32,
}

/// The marker bin at the right end of the ruler: a marker dragged out of it follows the pointer
/// along the strip, and dropping it there answers the time to add one at.
// effectcraft: crates/ui-egui/src/panels/markers_ui.rs:429 `marker_bin`.
fn marker_bin(ui: &mut Ui, id: Id, cx: &BinCx<'_>, view: &TimeView, settle: &dyn Fn(Tick, Option<Tick>) -> Tick, tokens: &UiTokens) -> Option<Tick> {
    let BinCx { bin, strip, height, flag_width, enabled, labels, tint } = *cx;
    let color = &tokens.color;
    let response = ui.interact(bin, id, Sense::DRAG);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, labels.bin.as_str()));
    let response = response.tooltip(labels.bin_hint.clone());
    let hot = response.hovered() || response.dragged();
    if hot {
        ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
    }
    craft_ui::icons::paint(ui, bin, Icon::Flag, IconSize::Md, if hot { color.accent.solid } else { color.text.muted }.to_egui());
    let pos = response.interact_pointer_pos().filter(|pos| pos.x >= strip.left() && pos.x < strip.right())?;
    let at = settle(view.time_at(pos.x, strip.left()), None);
    if response.dragged() {
        let flag = flag(view.x_of(at, strip.left()), strip.top(), flag_width, height);
        ui.painter_at(strip).add(Shape::convex_polygon(flag, tint, Stroke::NONE));
    }
    response.drag_stopped().then_some(at)
}

/// A marker drag under way: which marker, whether it sets the duration, and the marker's start
/// and duration when it was grabbed (the gestures carry the change from these).
#[derive(Clone, Copy)]
struct Drag {
    id: MarkerId,
    length_drag: bool,
    start: Tick,
    length: Tick,
}

/// The left edge of a marker's name box: after the flag, after a range marker's end cap when that
/// is in the way, else before the flag; `None` when it fits nowhere in `clip`.
// effectcraft: crates/ui-egui/src/panels/markers_ui.rs:380 `name_box_x`.
fn name_box_x(x: f32, width: f32, end: Option<f32>, clip: Rect, gap: f32, end_gap: f32) -> Option<f32> {
    let start = x + gap;
    let right = match end {
        Some(end) if end > start && end < start + width => end + end_gap,
        _ => start,
    };
    if right + width <= clip.right() {
        return Some(right);
    }
    let left = x - gap - width;
    (left >= clip.left()).then_some(left)
}

/// Where a marker's point sits, as a share of the strip's height (EffectCraft `markers_ui.rs:274`).
/// A proportion of the strip, not a size.
pub const MARKER_POINT: f32 = 0.6;

/// What the strip did this pass. A drag reports its gesture in [`MarkerEdit::Move`] and
/// [`MarkerEdit::SetDuration`]: the app previews `Update`s and applies the `Commit`.
pub struct MarkerStripResponse {
    /// The strip's response, for hover and focus.
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<MarkerEdit>,
    /// The marker that got a secondary click; the app opens its menu.
    pub context_menu: Option<MarkerId>,
}

craft_ui::words! {
    /// The marker strip's words, English by default.
    #[non_exhaustive]
    pub struct MarkerStripLabels {
        title: "markerStrip.title" = "Markers";
        bin: "markerStrip.bin" = "Marker bin";
        bin_hint: "markerStrip.binHint" = "Marker bin: drag a marker to the time ruler";
    }
}

craft_ui::words! {
    /// The Marker dialog's words, English by default.
    #[non_exhaustive]
    pub struct MarkerDialogLabels {
        title: "markerDialog.title" = "Marker";
        name: "markerDialog.name" = "Name:";
        comment: "markerDialog.comment" = "Comments:";
        time: "markerDialog.time" = "Time:";
        duration: "markerDialog.duration" = "Duration:";
        color: "markerDialog.color" = "Marker Color:";
        delete: "markerDialog.delete" = "Delete";
        previous: "markerDialog.previous" = "Previous";
        next: "markerDialog.next" = "Next";
    }
}

/// The Comments field's height in lines (EffectCraft `markers_ui.rs:151`).
const COMMENT_ROWS: usize = 3;

/// The marker as the dialog edits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerDraft {
    /// The marker's name.
    pub name: String,
    /// The marker's comment.
    pub comment: String,
    /// The marker's start time.
    pub start: Tick,
    /// The marker's duration; zero for a point marker.
    pub duration: Tick,
    /// The marker's colour label.
    pub color: MarkerColor,
}

/// What the Marker dialog answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerDialogResult {
    /// The marker the dialog edited.
    pub marker: MarkerId,
    /// What the user chose.
    pub action: MarkerDialogAction,
}

/// OK with the edited values, Delete, or Previous / Next (Premiere's: the edits are kept and the
/// app opens the dialog again on the neighbouring marker).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkerDialogAction {
    /// OK: keep the edited values.
    Save(MarkerDraft),
    /// Remove the marker.
    Delete,
    /// Keep the edits and open the dialog on the previous marker.
    SaveAndPrevious(MarkerDraft),
    /// Keep the edits and open the dialog on the next marker.
    SaveAndNext(MarkerDraft),
}

/// Premiere's Marker dialog on the dialog host: Name, Time and Duration (timecode fields),
/// Comments (multi-line), Marker Color (the label swatches), Delete, and OK / Cancel. Enter in a
/// single-line field is OK; Tab walks the fields (the dialog's field Tab, decisions §4.1).
/// Result: a [`MarkerDialogResult`].
pub struct MarkerDialog {
    id: MarkerId,
    draft: MarkerDraft,
    format: TimeFormat,
    palette: LabelPalette,
    labels: Option<MarkerDialogLabels>,
    state: DialogState,
    /// What each field held before the gesture running in it, to restore on Esc.
    drafts: FieldDrafts,
}

/// One [`GestureDraft`] per field of the dialog.
#[derive(Default)]
struct FieldDrafts {
    name: GestureDraft<String>,
    comment: GestureDraft<String>,
    start: GestureDraft<Tick>,
    duration: GestureDraft<Tick>,
}

impl MarkerDialog {
    /// A dialog editing `marker`, with times shown in `format`.
    pub fn new(marker: &Marker, format: TimeFormat) -> Self {
        let draft = MarkerDraft {
            name: marker.name.clone(),
            comment: marker.comment.clone(),
            start: marker.start,
            duration: marker.duration,
            color: marker.color,
        };
        Self {
            id: marker.id,
            draft,
            format,
            palette: LabelPalette::default(),
            labels: None,
            state: DialogState::new(),
            drafts: FieldDrafts::default(),
        }
    }

    /// The palette that gives the swatches' colours; the default palette when not set.
    pub fn palette(mut self, palette: LabelPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The dialog's words; the default words when not set.
    pub fn labels(mut self, labels: MarkerDialogLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl StackDialog for MarkerDialog {
    type Result = MarkerDialogResult;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("timeline-marker")
    }

    // effectcraft: crates/ui-egui/src/panels/markers_ui.rs:124 `dialog`, `:47` `MarkerDraft`;
    // layout as Premiere's Marker dialog, on `Dialog` + `FormGrid` + `Footer`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>> {
        let labels = self.labels.clone().unwrap_or_else(|| MarkerDialogLabels::of_ctx(ctx));
        let footer = Footer::side().extra(labels.delete.as_str()).extra(labels.previous.as_str()).extra(labels.next.as_str());
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), labels.title.as_str())
            .stack(stack)
            .width(Width::Standard)
            .initial_focus(InitialFocus::Field(self.id().with("name")))
            .footer(footer)
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels));
        self.state = state;
        let answer = |action| Some(DialogResult::Ok(MarkerDialogResult { marker: self.id, action }));
        if let Some(extra) = shown.extra {
            return match extra {
                0 => answer(MarkerDialogAction::Delete),
                1 => answer(MarkerDialogAction::SaveAndPrevious(self.draft.clone())),
                _ => answer(MarkerDialogAction::SaveAndNext(self.draft.clone())),
            };
        }
        match shown.outcome {
            Some(Outcome::Ok) => answer(MarkerDialogAction::Save(self.draft.clone())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

impl MarkerDialog {
    fn body(&mut self, ui: &mut Ui, labels: &MarkerDialogLabels) {
        let name_id = self.id().with("name");
        FormGrid::new("marker-dialog").show(ui, |form| {
            let name = form.row(labels.name.as_str(), |ui, _| {
                TextField::new(("marker", "name"), labels.name.as_str(), self.draft.name.as_str()).id(name_id).show(ui)
            });
            if let Some(event) = name.inner.event {
                self.drafts.name.apply(&mut self.draft.name, event);
            }
            let time = form.row(labels.time.as_str(), |ui, _| TimecodeField::new(("marker", "time"), self.draft.start, &self.format).show(ui));
            if let Some(event) = time.inner.event {
                self.drafts.start.apply(&mut self.draft.start, event);
            }
            let length =
                form.row(labels.duration.as_str(), |ui, _| TimecodeField::new(("marker", "duration"), self.draft.duration, &self.format).show(ui));
            if let Some(event) = length.inner.event {
                self.drafts.duration.apply(&mut self.draft.duration, event);
            }
            let comment = form.row(labels.comment.as_str(), |ui, _| {
                TextField::new(("marker", "comment"), labels.comment.as_str(), self.draft.comment.as_str()).multiline(COMMENT_ROWS).show(ui)
            });
            if let Some(event) = comment.inner.event {
                self.drafts.comment.apply(&mut self.draft.comment, event);
            }
            let mut chosen = None;
            form.row(labels.color.as_str(), |ui, _| {
                ui.horizontal(|ui| {
                    for (index, color) in self.palette.0.iter().enumerate() {
                        let label = MarkerColor(index as u8);
                        let on = self.draft.color == label;
                        let swatch = Swatch::new(("marker", "color", index), format!("{}", index + 1), *color).radio(on).position_in_set(index);
                        if ui.add(swatch).clicked() {
                            chosen = Some(label);
                        }
                    }
                })
                .response
            });
            if let Some(color) = chosen {
                self.draft.color = color;
            }
        });
    }
}
