//! The time ruler and the timeline scroll bar (N1.01).

use craft_theme::TypeRole;
use craft_ui::behavior::{ClaimedKey, KeyClaim};
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::FocusRing;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format, tabular};
use egui::accesskit::Role;
use egui::{
    AsIdSalt, CornerRadius, CursorIcon, EventFilter, Id, IdSalt, Key, Modifiers, PointerButton, Rect, Response, Sense, Shape, Stroke, Ui, WidgetInfo,
    pos2, vec2,
};

use super::markers::{MarkerEdit, MarkerStrip};
use super::style::TimelineStyle;
use crate::clips::SnapTargets;
use crate::markers::Marker;
use crate::rates::FrameRate;
use crate::time::{Tick, TimeRange, TimeView, VIEW_EXTENT_PAST_END};
use crate::timecode::{TimeFormat, format_time};

craft_ui::words! {
    /// The ruler's accessible names, English by default.
    #[non_exhaustive]
    pub struct TimelineRulerLabels {
        /// The ruler's AccessKit name; its value is the playhead's time.
        ruler: "timelineRuler.ruler" = "Time Ruler";
        work_area: "timelineRuler.workArea" = "Work Area";
        work_area_start: "timelineRuler.workAreaStart" = "Work Area Start";
        work_area_end: "timelineRuler.workAreaEnd" = "Work Area End";
        scroll_bar: "timelineRuler.scrollBar" = "Zoom and Scroll";
    }
}

// The ruler's rows as shares of its height: the layout has no sizes of its own, so it scales with
// `style.ruler_height` (FilmCraft: markers on the top row, labels at 24 of 44 pt, ticks on the
// bottom edge, a 2 pt cache bar).
const MARKER_BAND: f32 = 0.3;
const WORK_BAND: (f32, f32) = (0.3, 0.42);
const LABEL_LINE: f32 = 0.58;
const MAJOR_TICK: f32 = 0.22;
const MINOR_TICK: f32 = 0.09;
const CACHE_BAR: f32 = 0.05;
/// The playhead's head: half its width and its height.
const HEAD_HALF_WIDTH: f32 = 0.14;
const HEAD_HEIGHT: f32 = 0.3;
/// The work area's end caps: their width.
const CAP_WIDTH: f32 = 0.07;
/// Natural-log zoom per point of wheel travel over the ruler.
const WHEEL_ZOOM: f64 = 0.005;
/// Frames one arrow key steps, without and with Shift.
const STEP_FRAMES: i64 = 1;
const SHIFT_STEP_FRAMES: i64 = 5;

/// Which part of the work-area bar a press took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkGrab {
    Move,
    Start,
    End,
}

/// The keys the ruler claims while focused.
const SCRUB_KEYS: [ClaimedKey; 8] = [
    ClaimedKey::exact(Modifiers::NONE, Key::ArrowLeft),
    ClaimedKey::exact(Modifiers::NONE, Key::ArrowRight),
    ClaimedKey::exact(Modifiers::NONE, Key::Home),
    ClaimedKey::exact(Modifiers::NONE, Key::End),
    ClaimedKey::exact(Modifiers::SHIFT, Key::ArrowLeft),
    ClaimedKey::exact(Modifiers::SHIFT, Key::ArrowRight),
    ClaimedKey::exact(Modifiers::SHIFT, Key::Home),
    ClaimedKey::exact(Modifiers::SHIFT, Key::End),
];

/// The scroll bar's knobs at the thumb's ends: their radius as a share of the bar's height.
const KNOB: f32 = 0.26;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollGrab {
    Middle,
    Left,
    Right,
}

/// A scroll-bar drag: the view and the thumb as they were at the press.
#[derive(Clone, Copy, Debug)]
struct ScrollHold {
    grab: ScrollGrab,
    from: TimeView,
    pointer: f32,
    left: f32,
    width: f32,
    total: f64,
}

/// The playhead an arrow key asked for: a frame (Shift: five) either way, or the start or the end.
fn arrow_target(ui: &Ui, id: Id, playhead: Tick, rate: FrameRate, end: Tick) -> Option<Tick> {
    ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, ..EventFilter::default() }));
    KeyClaim::claim(ui.ctx(), id, &SCRUB_KEYS);
    let (key, shift) = ui.input_mut(|i| {
        [true, false].into_iter().find_map(|shift| {
            let modifiers = if shift { Modifiers::SHIFT } else { Modifiers::NONE };
            [Key::ArrowLeft, Key::ArrowRight, Key::Home, Key::End].into_iter().find(|key| i.consume_key(modifiers, *key)).map(|key| (key, shift))
        })
    })?;
    let frames = if shift { SHIFT_STEP_FRAMES } else { STEP_FRAMES };
    let from = rate.frame_at(playhead);
    let to = match key {
        Key::ArrowLeft => rate.tick_of(from.saturating_sub(frames)),
        Key::ArrowRight => rate.tick_of(from.saturating_add(frames)),
        Key::Home => Tick::ZERO,
        _ => end,
    };
    Some(to.max(Tick::ZERO))
}

/// The work area after dragging `kind` of it by `moved` from `anchor`; no end gets closer to the
/// other than `frame`, and `settle` snaps a moving time.
fn dragged_range(kind: WorkGrab, anchor: TimeRange, moved: Tick, frame: Tick, settle: impl Fn(Tick) -> Tick) -> TimeRange {
    match kind {
        WorkGrab::Move => TimeRange::new(settle((anchor.start + moved).max(Tick::ZERO)), anchor.duration),
        WorkGrab::Start => {
            let start = settle((anchor.start + moved).clamp(Tick::ZERO, (anchor.end() - frame).max(Tick::ZERO)));
            TimeRange::from_bounds(start, anchor.end())
        }
        WorkGrab::End => TimeRange::from_bounds(anchor.start, settle((anchor.end() + moved).max(anchor.start + frame))),
    }
}

/// One ruler tick.
#[derive(Clone, Debug, PartialEq)]
pub struct RulerTick {
    pub at: Tick,
    /// The label of a major tick ("00:00:10:00", "5|1"); minor ticks have none.
    pub label: Option<String>,
    pub major: bool,
}

/// Ticks for a time scale the library doesn't know: SoundCraft's Bars|Beats from its tempo map.
pub trait RulerLabels {
    /// The ticks within `visible`, at `points_per_second`.
    fn ticks(&self, visible: TimeRange, points_per_second: f64) -> Vec<RulerTick>;
}

/// What the ruler counts in.
#[derive(Clone, Copy)]
pub enum RulerScale<'a> {
    /// Frames, labelled in the format's display (Premiere's timecode; After Effects' frame
    /// numbers with [`TimeDisplay::Frames`](crate::TimeDisplay::Frames)).
    Format(TimeFormat),
    /// The app's ticks.
    Custom(&'a dyn RulerLabels),
}

/// The ruler's two strides in frames, `(tick, label)`. `label` is the first of
/// `[1, 2, 5, 10, base/2, base, 2·base, 5·base, 10·base, 15·base, 30·base, 60·base, 120·base,
/// 300·base, 600·base, 1800·base, 3600·base]` frames whose width at `frame_points` (points per
/// frame) is at least `label_gap` (`timeline.ruler_label_gap`), so timestamps never touch;
/// `tick` is the finest divisor `label / k` (k from 9 down to 1) that leaves ticks at least
/// `tick_gap` apart (`timeline.ruler_tick_gap`), so every label sits on a tick. Steps past
/// `base` saturate (rates are file data).
// filmcraft: crates/ui-egui/src/panels/timeline/ruler.rs:12 `ruler_steps`.
pub fn ruler_steps(frame_points: f64, base: i64, label_gap: f32, tick_gap: f32) -> (i64, i64) {
    let big = |k: i64| base.saturating_mul(k);
    let ladder =
        [1, 2, 5, 10, base / 2, base, big(2), big(5), big(10), big(15), big(30), big(60), big(120), big(300), big(600), big(1800), big(3600)];
    let (label_gap, tick_gap) = (f64::from(label_gap), f64::from(tick_gap));
    let label = ladder.iter().copied().find(|step| *step as f64 * frame_points >= label_gap).unwrap_or(big(3600)).max(1);
    let tick = (1..=9)
        .rev()
        .filter(|k| label.rem_euclid(*k) == 0)
        .map(|k| label / k)
        .find(|step| *step as f64 * frame_points >= tick_gap)
        .unwrap_or(label)
        .max(1);
    (tick, label)
}

/// The ticks of a [`RulerScale::Format`] ruler over a view `width` points wide: one every
/// [`ruler_steps`] tick stride from the first one at or before the left edge, labelled every
/// label stride with [`format_time`](crate::format_time); the gaps from `style`.
// filmcraft: crates/ui-egui/src/panels/timeline/ruler.rs:24 `draw_top` (the tick loop).
pub fn ruler_ticks(format: &TimeFormat, view: &TimeView, width: f32, style: &super::TimelineStyle) -> Vec<RulerTick> {
    let (rate, pps) = (format.rate, view.points_per_second);
    if !(pps > 0.0 && pps.is_finite() && width.is_finite() && width > 0.0) {
        return Vec::new();
    }
    let (tick, label) = ruler_steps(pps / rate.as_f64(), rate.timecode_base(), style.ruler_label_gap, style.ruler_tick_gap);
    let last = rate.frame_at(view.time_at(width, 0.0)).saturating_add(1);
    let mut frame = rate.frame_at(view.start).max(0) / tick * tick;
    let mut ticks = Vec::new();
    while frame <= last {
        let at = rate.tick_of(frame);
        let major = frame % label == 0;
        ticks.push(RulerTick { at, label: major.then(|| format_time(at, format)), major });
        let Some(next) = frame.checked_add(tick) else { break };
        frame = next;
    }
    ticks
}

/// The ruler across the top of a timeline: ticks and labels, the playhead's head, In and Out
/// shading, the work-area bar, cache bars and the sequence markers (through
/// [`MarkerStrip`](super::MarkerStrip)).
///
/// - Press and drag scrubs the playhead (snapped to `snap` targets when given; Shift snaps when
///   no targets are given, as After Effects), reported as an `EditEvent<Tick>` gesture; Esc
///   during a scrub cancels it back to where it began.
/// - The work-area bar drags by its middle (moves) or an end (moves that end), Shift snaps
///   (`style.work_area_grab`; EffectCraft `timeline.rs:538`), reported as an `EditEvent<TimeRange>` gesture;
///   double-clicking it sets it to the visible span.
/// - Wheel over the ruler zooms about the pointer (Alt+wheel in Premiere zooms anywhere; the
///   panel decides), changing `view` directly.
/// - Keyboard: one Tab stop. Left and Right step the playhead one frame (Shift: five), Home and
///   End go to the start and the end; each a `Set` event. Visible focus ring around the ruler.
/// - AccessKit: a `Slider` named by [`TimelineRulerLabels::ruler`], value the playhead's text.
/// - Sizes from [`TimelineStyle`](super::TimelineStyle): `style.ruler_height` unless
///   [`height`](Self::height) says otherwise.
pub struct TimelineRuler<'a> {
    id_salt: IdSalt,
    view: &'a mut TimeView,
    scale: RulerScale<'a>,
    format: TimeFormat,
    playhead: Tick,
    end: Tick,
    in_out: (Option<Tick>, Option<Tick>),
    work_area: Option<TimeRange>,
    cached: &'a [TimeRange],
    disk_cached: &'a [TimeRange],
    markers: &'a [Marker],
    snap: Option<&'a SnapTargets>,
    height: Option<f32>,
    labels: Option<TimelineRulerLabels>,
}

impl<'a> TimelineRuler<'a> {
    /// A ruler over `view` showing `format`'s times, for content ending at `end`.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut TimeView, format: TimeFormat, end: Tick) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            view,
            scale: RulerScale::Format(format),
            format,
            playhead: Tick::ZERO,
            end,
            in_out: (None, None),
            work_area: None,
            cached: &[],
            disk_cached: &[],
            markers: &[],
            snap: None,
            height: None,
            labels: None,
        }
    }

    /// Custom ticks (Bars|Beats); scrubbing still reports ticks and shows `format`'s text.
    pub fn scale(mut self, scale: RulerScale<'a>) -> Self {
        self.scale = scale;
        self
    }

    pub fn playhead(mut self, t: Tick) -> Self {
        self.playhead = t;
        self
    }

    /// Shades outside Mark In … Mark Out.
    pub fn in_out(mut self, marks: (Option<Tick>, Option<Tick>)) -> Self {
        self.in_out = marks;
        self
    }

    pub fn work_area(mut self, range: Option<TimeRange>) -> Self {
        self.work_area = range;
        self
    }

    /// Ranges with frames in RAM (green bar) and on disk (blue bar), After Effects' cache bars
    /// (EffectCraft `timeline.rs:2379`); Premiere's render bar uses the same.
    pub fn cache_bars(mut self, ram: &'a [TimeRange], disk: &'a [TimeRange]) -> Self {
        self.cached = ram;
        self.disk_cached = disk;
        self
    }

    pub fn markers(mut self, markers: &'a [Marker]) -> Self {
        self.markers = markers;
        self
    }

    /// Scrubbing snaps to these targets (Premiere with Snap on).
    pub fn snap(mut self, targets: Option<&'a SnapTargets>) -> Self {
        self.snap = targets;
        self
    }

    /// Height in points, for a ruler stacked in a row of another height (SoundCraft's ruler
    /// rows); `style.ruler_height` by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn labels(mut self, labels: TimelineRulerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/timeline/ruler.rs:24 `draw_top` (ruler part), `:176`
    // `draw_markers`; effectcraft: crates/ui-egui/src/panels/timeline.rs:2308 `work_area_bar`,
    // `:2379` `cache_bars`, `:2483` `ruler_scrub`.
    pub fn show(self, ui: &mut Ui) -> TimelineRulerResponse {
        let Self { id_salt, view, scale, format, playhead, end, in_out, work_area, cached, disk_cached, markers, snap, height, labels } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let labels = labels.unwrap_or_else(|| TimelineRulerLabels::of(ui));
        let height = height.unwrap_or(style.ruler_height);
        let id = ui.make_persistent_id(id_salt);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let band =
            |from: f32, to: f32| Rect::from_min_max(pos2(rect.left(), rect.top() + from * height), pos2(rect.right(), rect.top() + to * height));
        let rate = format.rate;
        let modifiers = ui.input(|i| i.modifiers);
        let enabled = ui.is_enabled();
        let valid = view.points_per_second > 0.0 && view.points_per_second.is_finite() && rect.width() > 0.0;
        let mut out = TimelineRulerResponse { response, scrub: None, work_area: None, markers: Vec::new(), snapped_at: None };
        if !valid {
            return out;
        }

        // Wheel and pinch over the ruler zoom about the pointer.
        if let Some(pointer) = out.response.hover_pos().filter(|_| enabled) {
            let (scroll, pinch) = ui.input(|i| (f64::from(i.smooth_scroll_delta().y), f64::from(i.zoom_delta())));
            let factor = pinch * (scroll * WHEEL_ZOOM).exp();
            if factor != 1.0 && factor.is_finite() {
                view.zoom_about(factor, view.time_at(pointer.x, rect.left()), rect.width());
                ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
            }
        }
        let view: TimeView = *view;
        let (left, pps) = (rect.left(), view.points_per_second);
        let tolerance = Tick::from_seconds_f64(f64::from(style.snap_distance) / pps);

        // What Shift snaps to when the app gave no targets: what the ruler itself shows.
        let known = |skip_work_area: bool| -> Vec<Tick> {
            let mut times: Vec<Tick> = markers.iter().map(|marker| marker.start).collect();
            times.extend([in_out.0, in_out.1].into_iter().flatten());
            if !skip_work_area {
                times.extend(work_area.into_iter().flat_map(|area| [area.start, area.end()]));
            }
            times.push(playhead);
            times
        };
        let targets = |skip_work_area: bool| -> Option<Vec<Tick>> {
            match snap {
                Some(targets) => Some(targets.times().to_vec()),
                None => modifiers.shift.then(|| known(skip_work_area)),
            }
        };
        let nearest = |targets: &[Tick], t: Tick| targets.iter().copied().filter(|c| (*c - t).abs() <= tolerance).min_by_key(|c| (*c - t).abs());

        // The playhead: press and drag scrub, the arrow keys step.
        let mut scrub = Gesture::load(ui, id, &playhead, EditPolicy::default()).enabled(enabled);
        let time_at = |x: f32| (view.time_at(x, left).max(Tick::ZERO), targets(false));
        let (pressed, released) = ui.input(|i| (i.pointer.button_pressed(PointerButton::Primary), i.pointer.button_released(PointerButton::Primary)));
        let down = out.response.is_pointer_button_down_on();
        if down && pressed && enabled && !scrub.is_active() {
            scrub.begin();
        }
        if scrub.is_active()
            && (down || released)
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            let (t, targets) = time_at(pos.x);
            let hit = targets.and_then(|targets| nearest(&targets, t));
            out.snapped_at = hit;
            scrub.update(hit.unwrap_or_else(|| rate.snap_nearest(t)), modifiers);
        }
        if scrub.is_active() && released {
            scrub.commit();
        }
        if out.response.has_focus()
            && enabled
            && let Some(to) = arrow_target(ui, id, *scrub.shown(), rate, end)
        {
            scrub.set(to);
        }
        let shown = *scrub.shown();
        out.scrub = scrub.finish();
        if out.scrub.is_some() {
            ui.ctx().request_repaint();
        }

        // The work area: the bar moves, an end resizes.
        let mut work = work_area.map(|area| (area, Gesture::load(ui, id.with("work area"), &area, EditPolicy::default()).enabled(enabled)));
        let mut work_shown = work_area;
        if let Some((area, gesture)) = work.as_mut() {
            let grab = style.work_area_grab;
            let band = band(WORK_BAND.0, WORK_BAND.1);
            let shown = *gesture.shown();
            let (x0, x1) = (view.x_of(shown.start, left), view.x_of(shown.end(), left));
            let hit = band.expand2(vec2(0.0, grab / 2.0));
            let bar = hit.intersect(Rect::from_min_max(pos2(x0, hit.top()), pos2(x1, hit.bottom())));
            let end_zone = |x: f32| Rect::from_min_max(pos2(x - grab, hit.top()), pos2(x + grab, hit.bottom())).intersect(rect);
            let zones = [
                (WorkGrab::Move, bar, labels.work_area.as_str(), shown),
                (WorkGrab::Start, end_zone(x0), labels.work_area_start.as_str(), shown),
                (WorkGrab::End, end_zone(x1), labels.work_area_end.as_str(), shown),
            ];
            let anchor = gesture.anchor().copied().unwrap_or(*area);
            let origin = ui.input(|i| i.pointer.press_origin());
            let grab_key = id.with("work grab");
            for (kind, zone, name, shown) in zones {
                if zone.width() <= 0.0 {
                    continue;
                }
                let zone_response = ui.interact(zone, id.with(("work", kind as u8)), Sense::click_and_drag());
                let text = match kind {
                    WorkGrab::Move => format!("{} \u{2013} {}", format_time(shown.start, &format), format_time(shown.end(), &format)),
                    WorkGrab::Start => format_time(shown.start, &format),
                    WorkGrab::End => format_time(shown.end(), &format),
                };
                zone_response.widget_info(|| WidgetInfo::labeled(Role::Slider, enabled, name));
                ui.ctx().accesskit_node_builder(zone_response.id, |node| node.set_value(text.as_str()));
                if zone_response.hovered() || zone_response.dragged() {
                    ui.ctx().set_cursor_icon(if kind == WorkGrab::Move { CursorIcon::Grab } else { CursorIcon::ResizeHorizontal });
                }
                if zone_response.is_pointer_button_down_on() && pressed && enabled && !gesture.is_active() {
                    ui.data_mut(|d| d.insert_temp(grab_key, (kind, origin.map_or(0.0, |p| p.x))));
                    gesture.begin();
                }
                if zone_response.double_clicked() && enabled {
                    gesture.set(TimeRange::from_bounds(view.start, view.time_at(rect.right(), left)));
                }
            }
            if gesture.is_active()
                && ui.input(|i| i.pointer.primary_down() || released)
                && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
                && let Some((kind, press_x)) = ui.data(|d| d.get_temp::<(WorkGrab, f32)>(grab_key))
            {
                let moved = Tick::from_seconds_f64(f64::from(pos.x - press_x) / pps);
                let snap_to = targets(true);
                let settle = |t: Tick| match snap_to.as_deref().and_then(|targets| nearest(targets, t)) {
                    Some(hit) => hit,
                    None => rate.snap_nearest(t),
                };
                let next = dragged_range(kind, anchor, moved, rate.frame_duration(), settle);
                gesture.update(next, modifiers);
            }
            if gesture.is_active() && released {
                gesture.commit();
            }
            work_shown = Some(*gesture.shown());
        }
        out.work_area = work.and_then(|(_, gesture)| gesture.finish());

        // Paint.
        let painter = ui.painter().with_clip_rect(rect);
        let color = &tokens.color;
        let hairline = craft_theme::Stroke::Hairline.width(ui.pixels_per_point());
        painter.rect_filled(rect, CornerRadius::ZERO, color.surface.chrome.to_egui());
        let body = band(WORK_BAND.1, 1.0 - CACHE_BAR);
        if in_out.0.is_some() || in_out.1.is_some() {
            let from = in_out.0.unwrap_or(Tick::ZERO);
            let to = in_out.1.map_or(end, |out| out + rate.frame_duration());
            let span = Rect::from_min_max(pos2(view.x_of(from, left), body.top()), pos2(view.x_of(to, left), body.bottom()));
            painter.rect_filled(span.intersect(rect), CornerRadius::ZERO, color.state.selected.to_egui());
        }
        let strip = band(1.0 - CACHE_BAR, 1.0);
        for (ranges, fill) in [(disk_cached, color.data.blue), (cached, color.data.green)] {
            for range in ranges.iter().filter(|range| !range.is_empty()) {
                let bar = Rect::from_min_max(pos2(view.x_of(range.start, left), strip.top()), pos2(view.x_of(range.end(), left), strip.bottom()));
                painter.rect_filled(bar.intersect(rect), CornerRadius::ZERO, fill.to_egui());
            }
        }
        if let Some(area) = work_shown.filter(|area| !area.is_empty()) {
            let band = band(WORK_BAND.0, WORK_BAND.1);
            let bar = Rect::from_min_max(pos2(view.x_of(area.start, left), band.top()), pos2(view.x_of(area.end(), left), band.bottom()));
            painter.rect_filled(bar, CornerRadius::ZERO, color.text.muted.to_egui());
            for x in [bar.left(), bar.right()] {
                let cap = Rect::from_center_size(pos2(x, bar.center().y), vec2(CAP_WIDTH * height, bar.height()));
                painter.rect_filled(cap, CornerRadius::ZERO, color.accent.solid.to_egui());
            }
        }
        let visible = TimeRange::from_bounds(view.start, view.time_at(rect.right(), left));
        let ticks = match scale {
            RulerScale::Format(format) => ruler_ticks(&format, &view, rect.width(), &style),
            RulerScale::Custom(source) => source.ticks(visible, pps),
        };
        let text = tabular(role_format(&tokens, TypeRole::Caption, color.text.normal.to_egui()));
        let tick_color = color.text.muted.to_egui();
        for tick in &ticks {
            let x = view.x_of(tick.at, left);
            let size = if tick.major { MAJOR_TICK } else { MINOR_TICK } * height;
            painter.line_segment([pos2(x, strip.top()), pos2(x, strip.top() - size)], Stroke::new(hairline, tick_color));
            if let Some(label) = &tick.label {
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(egui::text::LayoutJob::single_section(label.clone(), text.clone())));
                let at = pos2(x - galley.size().x / 2.0, rect.top() + LABEL_LINE * height - galley.size().y / 2.0);
                painter.galley(at, galley, text.color);
            }
        }
        if !markers.is_empty() {
            let strip = MarkerStrip::new(id.with("markers"), markers, &view).snap(snap).show(ui, band(0.0, MARKER_BAND));
            out.markers = strip.edits;
        }
        let x = view.x_of(shown, left);
        let accent = color.accent.solid.to_egui();
        let half = HEAD_HALF_WIDTH * height;
        painter
            .line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(craft_theme::Stroke::W1.width(ui.pixels_per_point()), accent));
        let head = vec![pos2(x - half, rect.top()), pos2(x + half, rect.top()), pos2(x, rect.top() + HEAD_HEIGHT * height)];
        painter.add(Shape::convex_polygon(head, accent, Stroke::NONE));
        FocusRing::paint_radius(ui, rect, CornerRadius::ZERO, &out.response);

        let value = format_time(shown, &format);
        out.response.widget_info(|| WidgetInfo::labeled(Role::Slider, enabled, &labels.ruler));
        ui.ctx().accesskit_node_builder(out.response.id, |node| node.set_value(value.as_str()));
        out
    }
}

/// What the ruler did this pass.
pub struct TimelineRulerResponse {
    pub response: Response,
    /// The playhead scrub, a gesture or a key's `Set`.
    pub scrub: Option<EditEvent<Tick>>,
    pub work_area: Option<EditEvent<TimeRange>>,
    pub markers: Vec<MarkerEdit>,
    /// Where scrubbing snapped this pass (for a snap line across the tracks).
    pub snapped_at: Option<Tick>,
}

/// Premiere's timeline scroll bar: the thumb spans the visible time over the view's extent
/// ([`TimeView::max_start`]); dragging its middle scrolls, dragging an end zooms (that end moves,
/// the other stays); the thumb is at least `style.scroll_thumb_min` wide. Changes `view` directly (view changes
/// are not edits). Not a Tab stop: the panel's `=` / `-` and arrow keys zoom and scroll.
/// EffectCraft places it above the ruler as its time navigator (`timeline.rs:2245`).
pub struct TimelineScrollBar<'a> {
    id_salt: IdSalt,
    view: &'a mut TimeView,
    end: Tick,
}

impl<'a> TimelineScrollBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut TimeView, end: Tick) -> Self {
        Self { id_salt: IdSalt::new(id_salt), view, end }
    }

    // filmcraft: crates/ui-egui/src/panels/timeline/scrollbars.rs:6 `MIN_THUMB_W`, `:12`
    // `zoom_thumb`, `:24` `zoom_scrollbar`.
    pub fn show(self, ui: &mut Ui) -> Response {
        let Self { id_salt, view, end } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), style.scrollbar_height), Sense::hover());
        // Not a Tab stop: the panel's keys zoom and scroll.
        let response = ui.interact(bar, id, Sense::drag());
        let enabled = ui.is_enabled();
        let width = bar.width();
        if !(view.points_per_second > 0.0 && view.points_per_second.is_finite() && width > 0.0) {
            return response;
        }
        let total_for = |visible: f64| (end.max(Tick::ZERO) + VIEW_EXTENT_PAST_END).seconds().max(visible);
        // The thumb's left edge and width for a view starting at `start` seconds showing `visible`
        // seconds, and the seconds of scroll per point of thumb travel.
        let thumb_for = |start: f64, visible: f64| {
            let total = total_for(visible);
            let thumb = ((visible / total) as f32 * width).clamp(style.scroll_thumb_min.min(width), width);
            let (travel, range) = ((width - thumb).max(0.0), (total - visible).max(0.0));
            let at = if range > 0.0 { (start / range).clamp(0.0, 1.0) as f32 * travel } else { 0.0 };
            (bar.left() + at, thumb, if travel > 0.5 { range / f64::from(travel) } else { 0.0 })
        };
        let visible = f64::from(width) / view.points_per_second;
        let (left, thumb_width, per_point) = thumb_for(view.start.seconds(), visible);
        let thumb = Rect::from_min_max(pos2(left, bar.top()), pos2(left + thumb_width, bar.bottom()));
        let end_zone = bar.height() / 2.0;

        let key = id.with("grab");
        let origin = ui.input(|i| i.pointer.press_origin());
        if response.is_pointer_button_down_on()
            && enabled
            && ui.input(|i| i.pointer.button_pressed(PointerButton::Primary))
            && let Some(pos) = origin
        {
            // A press beside the thumb brings its middle under the pointer, then the drag goes on.
            let mut left = left;
            if !thumb.contains(pos) && per_point > 0.0 {
                let seconds = (f64::from(pos.x - bar.left() - thumb_width / 2.0) * per_point).clamp(0.0, view.max_start(end, width).seconds());
                left = thumb_for(seconds, visible).0;
                view.start = Tick::from_seconds_f64(seconds);
            }
            let grab = if thumb.contains(pos) && (pos.x - thumb.left()).abs() < end_zone {
                ScrollGrab::Left
            } else if thumb.contains(pos) && (pos.x - thumb.right()).abs() < end_zone {
                ScrollGrab::Right
            } else {
                ScrollGrab::Middle
            };
            let held = ScrollHold { grab, from: *view, pointer: pos.x, left, width: thumb_width, total: total_for(visible) };
            ui.data_mut(|d| d.insert_temp(key, held));
        }
        if let Some(held) = ui.data(|d| d.get_temp::<ScrollHold>(key)) {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                *view = held.from;
                ui.data_mut(|d| d.remove::<ScrollHold>(key));
            } else if response.is_pointer_button_down_on()
                && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            {
                let dx = pos.x - held.pointer;
                let floor = style.scroll_thumb_min.min(width);
                let (l, r) = match held.grab {
                    ScrollGrab::Middle => (held.left + dx, held.left + dx + held.width),
                    ScrollGrab::Left => ((held.left + dx).min(held.left + held.width - floor), held.left + held.width),
                    ScrollGrab::Right => (held.left, (held.left + held.width + dx).max(held.left + floor)),
                };
                let (l, r) = (l.clamp(bar.left(), bar.right()), r.clamp(bar.left(), bar.right()));
                let (l, new_width) = match held.grab {
                    ScrollGrab::Middle => (l.min(bar.right() - held.width), held.width),
                    _ => (l, (r - l).clamp(floor, width)),
                };
                let seen = if held.grab == ScrollGrab::Middle {
                    f64::from(width) / held.from.points_per_second
                } else {
                    f64::from(new_width / width) * held.total
                };
                let (travel, range) = ((width - new_width).max(0.0), (held.total - seen).max(0.0));
                let start = if travel > 0.5 { f64::from(((l - bar.left()) / travel).clamp(0.0, 1.0)) * range } else { 0.0 };
                if held.grab != ScrollGrab::Middle {
                    view.points_per_second = (f64::from(width) / seen).clamp(TimeView::MIN_ZOOM, TimeView::MAX_ZOOM);
                }
                view.start = Tick::from_seconds_f64(start).clamp(Tick::ZERO, view.max_start(end, width));
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                ui.data_mut(|d| d.remove::<ScrollHold>(key));
            }
        }

        let (left, thumb_width, _) = thumb_for(view.start.seconds(), f64::from(width) / view.points_per_second);
        let thumb = Rect::from_min_max(pos2(left, bar.top()), pos2(left + thumb_width, bar.bottom()));
        let color = &tokens.color;
        let painter = ui.painter_at(bar);
        let round = CornerRadius::same((bar.height() / 2.0) as u8);
        painter.rect_filled(bar, round, color.surface.sunken.to_egui());
        let hot = response.hovered() || response.dragged();
        painter.rect_filled(thumb, round, if hot { color.text.muted } else { color.text.faint }.to_egui());
        let radius = bar.height() * KNOB;
        for x in [thumb.left() + thumb.height() / 2.0, thumb.right() - thumb.height() / 2.0] {
            painter.circle(
                pos2(x, thumb.center().y),
                radius,
                color.surface.chrome.to_egui(),
                Stroke::new(craft_theme::Stroke::W1_5.width(ui.pixels_per_point()), color.text.normal.to_egui()),
            );
        }
        let name = TimelineRulerLabels::of(ui).scroll_bar;
        response.widget_info(|| WidgetInfo::labeled(Role::ScrollBar, enabled, &name));
        response
    }
}
