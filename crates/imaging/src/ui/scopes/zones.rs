//! Tone zones ([`ToneZone`], [`ScopeZoneEdit`]): Camera Raw's and Lightroom's draggable ranges
//! over the histogram.

use craft_ui::controls::TooltipExt as _;
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::{Action, ActionData, Role};
use egui::{EventFilter, Key, Modifiers, Rect, Sense, UiBuilder};

use super::labels::ScopePanelLabels;

/// One tone zone over the plot: a range of the plot's width (0..=1) that edits one of the host's
/// controls. Zones are data: PhotoCraft's Camera Raw and LightCraft's Basic panel
/// disagree, so each app passes its own set until a Photoshop reference decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToneZone {
    /// A stable key the host maps to its control ("blacks", "shadows", "exposure", "highlights",
    /// "whites"); also LightCraft's control id.
    pub key: &'static str,
    /// Where the zone starts, as a share of the plot width.
    pub from: f32,
    /// Where it ends.
    pub to: f32,
}

impl ToneZone {
    /// Camera Raw's five zones: Blacks 0–0.10, Shadows 0.10–0.25, Exposure 0.25–0.75, Highlights
    /// 0.75–0.90, Whites 0.90–1.
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:59-65 `ToneZone::range`.
    pub const CAMERA_RAW: [ToneZone; 5] = [
        ToneZone { key: "blacks", from: 0.0, to: 0.10 },
        ToneZone { key: "shadows", from: 0.10, to: 0.25 },
        ToneZone { key: "exposure", from: 0.25, to: 0.75 },
        ToneZone { key: "highlights", from: 0.75, to: 0.90 },
        ToneZone { key: "whites", from: 0.90, to: 1.0 },
    ];

    /// Lightroom's five zones: Blacks 0–0.15, Shadows 0.15–0.35, Exposure 0.35–0.65, Highlights
    /// 0.65–0.85, Whites 0.85–1.
    // lightcraft: crates/ui-egui/src/panels/histogram.rs:33-39.
    pub const LIGHTROOM: [ToneZone; 5] = [
        ToneZone { key: "blacks", from: 0.0, to: 0.15 },
        ToneZone { key: "shadows", from: 0.15, to: 0.35 },
        ToneZone { key: "exposure", from: 0.35, to: 0.65 },
        ToneZone { key: "highlights", from: 0.65, to: 0.85 },
        ToneZone { key: "whites", from: 0.85, to: 1.0 },
    ];

    /// The zone's name: the [`ScopePanelLabels`] word for the five standard keys, else the key.
    pub fn label<'l>(&self, labels: &'l ScopePanelLabels) -> &'l str {
        // new: the blacks, shadows, exposure, highlights and whites words; any other key returns `self.key`.
        match self.key {
            "blacks" => &labels.blacks,
            "shadows" => &labels.shadows,
            "exposure" => &labels.exposure,
            "highlights" => &labels.highlights,
            "whites" => &labels.whites,
            other => other,
        }
    }

    /// The zone at `x`, a share of the plot width (clamped to 0..=1): the first zone whose `to`
    /// lies past `x`, the last one at 1.
    pub fn at(zones: &[ToneZone], x: f32) -> Option<usize> {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:33 `ToneZone::at`.
        let x = if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
        zones.iter().position(|zone| zone.to > x).or(zones.len().checked_sub(1))
    }
}

/// What the user did to one zone.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ZoneChange {
    /// A drag over the plot, one gesture: `Begin` at the press, `Update` per pass with the signed
    /// share of the plot width moved since the press (Shift drags a tenth as far: LightCraft's
    /// `FINE = 0.1`), `Commit` on release, `Cancel` with Esc. The host maps the share to its
    /// control (PhotoCraft: `share × limit × 2`, limit 5 for Exposure, else 100) and makes one
    /// undo step for the drag.
    Drag(EditEvent<f32>),
    /// Arrow keys with the plot focused, or a screen reader's Increment and Decrement: this many
    /// of the control's own steps (Right and Up +1, Left and Down −1, Shift ×10). One undo step
    /// each.
    Step(i32),
    /// A double-click on the zone: the host resets the control to its default. One undo step.
    Reset,
    /// A screen reader's SetValue, in the control's own units.
    SetValue(f64),
}

/// A tone-zone edit, reported in [`ScopePanelOutput::zone`](super::ScopePanelOutput::zone).
#[derive(Clone, Debug, PartialEq)]
pub struct ScopeZoneEdit {
    /// The zone's index in the slice passed to [`ScopePanel::zones`](super::ScopePanel::zones).
    pub zone: usize,
    /// The zone's key.
    pub key: &'static str,
    /// What happened.
    pub change: ZoneChange,
}

/// The zones' input and look over `plot`, read from `response`, the response of a
/// [`Histogram::interactive`](craft_ui::composites::Histogram::interactive) plot (one Tab stop): the hovered zone (or the pressed one during a drag)
/// shows its band in `text.normal` at `zone_alpha`, the `ResizeHorizontal` cursor and the tooltip
/// [`ScopePanelLabels::zone_tooltip`] with the host's value text; a press locks the zone under it
/// for the whole drag (LightCraft); the top `warning_size` strip belongs to the warning buttons.
/// With the plot focused, Page Up and Page Down move the focused zone (the keyboard's way to
/// reach every zone) and the arrow keys step it. AccessKit: a `Group` named
/// [`ScopePanelLabels::tone_zones`] holding one `Slider` per zone, named by its label, value text
/// the host's.
pub(super) fn zones(
    ui: &mut egui::Ui,
    response: &egui::Response,
    plot: egui::Rect,
    zones: &[ToneZone],
    values: &[String],
    labels: &ScopePanelLabels,
    style: &super::style::ScopePanelStyle,
) -> Option<ScopeZoneEdit> {
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:412-437 (hover band, tooltip, double-click reset, drag);
    // lightcraft: crates/ui-egui/src/panels/histogram.rs:41-44 (press locks the zone, Shift is a tenth).
    if zones.is_empty() || plot.width() <= 0.0 {
        return None;
    }
    let tokens = Tokens::of(ui);
    let id = response.id;
    let (drag_key, focus_key) = (id.with("zone-drag"), id.with("zone-focus"));
    let strip = plot.top() + style.warning_size;
    let share = |x: f32| ((x - plot.left()) / plot.width()).clamp(0.0, 1.0);
    let zone_at = |x: f32| ToneZone::at(zones, share(x));
    let mut drag: Option<ZoneDrag> = ui.data(|d| d.get_temp::<Option<ZoneDrag>>(drag_key)).flatten();
    let mut focused: usize = ui.data(|d| d.get_temp(focus_key)).filter(|z| *z < zones.len()).unwrap_or_else(|| ToneZone::at(zones, 0.5).unwrap_or(0));
    let pointer = response.interact_pointer_pos().or_else(|| response.hover_pos());
    let under = pointer.filter(|p| p.y > strip).and_then(|p| zone_at(p.x));
    let modifiers = ui.input(|i| i.modifiers);
    let mut change: Option<(usize, ZoneChange)> = None;

    let step = response.drag_delta().x / plot.width() * if modifiers.shift { FINE } else { 1.0 };
    let press = ui.input(|i| i.pointer.press_origin()).filter(|p| p.y > strip);
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(zone) = press.and_then(|p| zone_at(p.x))
    {
        let gesture = GestureId::next();
        drag = Some(ZoneDrag { gesture, zone, value: step });
        focused = zone;
        change = Some((zone, ZoneChange::Drag(EditEvent::begin(gesture))));
    } else if let Some(active) = drag.as_mut() {
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            change = Some((active.zone, ZoneChange::Drag(EditEvent::cancel(active.gesture, CancelReason::Escape))));
            drag = None;
        } else {
            active.value += step;
            if response.drag_stopped() {
                let event = EditEvent::commit(active.gesture, active.value, active.value != 0.0, modifiers);
                change = Some((active.zone, ZoneChange::Drag(event)));
                drag = None;
            } else if step != 0.0 {
                change = Some((active.zone, ZoneChange::Drag(EditEvent::update(active.gesture, active.value, modifiers))));
            }
        }
    } else if response.double_clicked()
        && let Some(zone) = under
    {
        change = Some((zone, ZoneChange::Reset));
    }

    if response.has_focus() {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, ARROWS));
        if drag.is_none() {
            let (moved, steps) = ui.input_mut(|i| {
                let moved = i32::from(i.consume_key(Modifiers::NONE, Key::PageUp)) - i32::from(i.consume_key(Modifiers::NONE, Key::PageDown));
                let mut steps = 0;
                for (modifiers, times) in [(Modifiers::SHIFT, 10), (Modifiers::NONE, 1)] {
                    for (key, direction) in [(Key::ArrowRight, 1), (Key::ArrowUp, 1), (Key::ArrowLeft, -1), (Key::ArrowDown, -1)] {
                        if i.consume_key(modifiers, key) {
                            steps += direction * times;
                        }
                    }
                }
                (moved, steps)
            });
            focused = focused.saturating_add_signed(moved as isize).min(zones.len() - 1);
            if steps != 0 {
                change = Some((focused, ZoneChange::Step(steps)));
            }
        }
    }

    let shown = drag.map(|d| d.zone).or(under).or(response.has_focus().then_some(focused));
    if let Some(zone) = shown {
        let (from, to) = (plot.left() + zones[zone].from * plot.width(), plot.left() + zones[zone].to * plot.width());
        let band = Rect::from_min_max(egui::pos2(from, plot.top()), egui::pos2(to, plot.bottom()));
        ui.painter().with_clip_rect(plot).rect_filled(band, 0.0, tokens.color.text.normal.to_egui().gamma_multiply(style.zone_alpha));
    }
    if (response.hovered() || response.dragged())
        && let Some(zone) = drag.map(|d| d.zone).or(under)
    {
        ui.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        let tip =
            craft_i18n::fmt(&labels.zone_tooltip, &[("zone", zones[zone].label(labels)), ("value", values.get(zone).map_or("", String::as_str))]);
        response.clone().tooltip(tip);
    }

    let nodes =
        ui.scope_builder(UiBuilder::new().max_rect(plot).accessibility_role(Role::Group).accessibility_label(labels.tone_zones.clone()), |ui| {
            let mut requested = None;
            for (at, zone) in zones.iter().enumerate() {
                let node = id.with(("zone-node", at));
                let rect = Rect::from_min_max(
                    egui::pos2(plot.left() + zone.from * plot.width(), plot.top()),
                    egui::pos2(plot.left() + zone.to * plot.width(), plot.bottom()),
                );
                Interactive::with_id(node, Role::Slider, zone.label(labels))
                    .sense(Sense::hover())
                    .enabled(ui.is_enabled())
                    .state_layer(false)
                    .focus_ring(false)
                    .interact_at(ui, rect)
                    .value(values.get(at).cloned().unwrap_or_default())
                    .numeric_mixed(-UNBOUNDED..=UNBOUNDED, 1.0)
                    .finish(ui);
                requested = requested.or_else(|| screen_reader_request(ui, node).map(|change| (at, change)));
            }
            requested
        });
    if change.is_none() {
        change = nodes.inner;
    }

    ui.data_mut(|d| {
        d.insert_temp(drag_key, drag);
        d.insert_temp(focus_key, focused);
    });
    change.map(|(zone, change)| ScopeZoneEdit { zone, key: zones[zone].key, change })
}

/// A drag in progress: the zone the press locked, and the share of the plot width moved so far.
#[derive(Clone, Copy)]
struct ZoneDrag {
    gesture: GestureId,
    zone: usize,
    value: f32,
}

/// Shift drags a tenth as far (LightCraft's `FINE`).
const FINE: f32 = 0.1;

/// The slider nodes accept any value: the zone's range belongs to the host's control.
const UNBOUNDED: f64 = 1.0e9;

/// The keys a focused plot keeps from focus navigation.
const ARROWS: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

/// What a screen reader asked of the slider `node` this pass: Increment and Decrement are one
/// step each, SetValue the control's own value.
fn screen_reader_request(ui: &egui::Ui, node: egui::Id) -> Option<ZoneChange> {
    if !ui.is_enabled() {
        return None;
    }
    ui.input(|i| {
        if let Some(value) = i
            .accesskit_action_requests(node, Action::SetValue)
            .filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                _ => None,
            })
            .last()
        {
            return Some(ZoneChange::SetValue(value));
        }
        let steps = i.num_accesskit_action_requests(node, Action::Increment) as i32 - i.num_accesskit_action_requests(node, Action::Decrement) as i32;
        (steps != 0).then_some(ZoneChange::Step(steps))
    })
}
