//! [`Segmented`]: one of a few options on a track, as a radio group.

use std::sync::Arc;

use craft_theme::{ControlSize, States, TypeRole};
use egui::accesskit::{Role, Toggled};
use egui::emath::GuiRounding as _;
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, CornerRadius, EventFilter, Galley, IdSalt, Key, Modifiers, Rect, Response, Sense, Ui, UiBuilder, Vec2, vec2};

use super::button::content_color;
use super::{SIZE_NAMES, icon_size, size_arg};
use crate::direction;
use crate::icons::{self, Icon};
use crate::primitives::{Content, Corner, Gap, Interactive, Stack, StateColors, Surface, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// A segmented control's sizes and colours, from [`Segmented::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SegmentedStyle {
    /// Height of the track: `segmented.height`.
    pub height: f32,
    /// At the start and end of each segment: `segmented.pad_x`.
    pub pad_x: f32,
    /// Side of an icon: `segmented.icon`.
    pub icon: f32,
    /// Between the track's edge and the segments: `segmented.inset`.
    pub inset: f32,
    /// `segmented.label`.
    pub label: TypeRole,
    /// The track: a sunken [`Surface`] (its edge and `Corner::Control`) filled with
    /// `segmented.track`.
    pub track: SurfaceStyle,
    /// Segments' corners: `Corner::Control`.
    pub radius: CornerRadius,
    /// `segmented.segment.fill`.
    pub fill: States<Color32>,
    /// `segmented.segment.text`.
    pub text: States<Color32>,
    /// `segmented.selected.fill`.
    pub selected_fill: States<Color32>,
    /// `segmented.selected.text`.
    pub selected_text: States<Color32>,
}

/// One of a few options, e.g. tool modes:
///
/// ```
/// # use craft_ui::controls::Segmented;
/// # #[derive(Clone, PartialEq)] enum Mode { New, Add, Subtract }
/// # egui::__run_test_ui(|ui| { let mut mode = Mode::New;
/// Segmented::new("selection-mode", "Selection mode", &mut mode)
///     .item(Mode::New, "New")
///     .item(Mode::Add, "Add")
///     .item(Mode::Subtract, "Subtract")
///     .show(ui);
/// # });
/// ```
///
/// Equal-width segments sit on a sunken track; the selected one shows `segmented.selected.*`.
/// It is a WAI-ARIA radio group: AccessKit `RadioGroup` named `label` with a `RadioButton` per
/// segment, the selected one checked. The group is one Tab stop (the selected segment, or the
/// first when none is); the arrow keys select the next or previous segment, wrapping, with Left
/// and Right swapped in a right-to-left interface, and Home and End the first and last.
/// Segments run from the reading start. An icon segment's label is its accessible name and
/// tooltip.
#[derive(Debug)]
pub struct Segmented<'a, T> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut T,
    items: Vec<Item<T>>,
    size: ControlSize,
    enabled: bool,
}

#[derive(Debug)]
struct Item<T> {
    value: T,
    label: String,
    icon: Option<Icon>,
}

impl<'a, T: PartialEq + Clone> Segmented<'a, T> {
    /// A group named `label` (its accessible name) choosing `value`; `id_salt` keeps its focus
    /// and animations when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a mut T) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), value, items: Vec::new(), size: ControlSize::Md, enabled: true }
    }

    /// A segment showing `label`, which selects `value`.
    pub fn item(mut self, value: T, label: impl Into<String>) -> Self {
        self.items.push(Item { value, label: label.into(), icon: None });
        self
    }

    /// A segment showing `icon`, which selects `value`; `label` is its accessible name and
    /// tooltip.
    pub fn icon_item(mut self, value: T, icon: Icon, label: impl Into<String>) -> Self {
        self.items.push(Item { value, label: label.into(), icon: Some(icon) });
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the control. The response covers the track; it is
    /// [changed](Response::changed) when the selection changed.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Segmented::style(&tokens, self.size);
        let format = role_format(&tokens, style.label, Color32::PLACEHOLDER);
        let galleys: Vec<Option<Arc<Galley>>> = self
            .items
            .iter()
            .map(|item| item.icon.is_none().then(|| ui.painter().layout_job(LayoutJob::single_section(item.label.clone(), format.clone()))))
            .collect();
        let content = galleys.iter().map(|g| g.as_ref().map_or(style.icon, |g| g.size().x)).fold(0.0, f32::max);
        let segment = vec2(content + 2.0 * style.pad_x, style.height - 2.0 * style.inset);
        let count = self.items.len();
        let (_, track) = ui.allocate_space(vec2(count as f32 * segment.x + 2.0 * style.inset, style.height));
        if ui.is_rect_visible(track) {
            ui.painter().extend(style.track.shapes(track, ui.pixels_per_point()));
        }

        let mut group = ui.new_child(
            UiBuilder::new()
                .id_salt(self.id_salt)
                .max_rect(track.shrink(style.inset))
                .layout(direction::row(ui))
                .accessibility_role(Role::RadioGroup)
                .accessibility_label(self.label.as_str()),
        );
        group.spacing_mut().item_spacing = Vec2::ZERO;
        let ids: Vec<_> = (0..count).map(|i| Interactive::id_in(&group, i)).collect();
        let enabled = self.enabled && ui.is_enabled();
        let mut selected = self.items.iter().position(|item| item.value == *self.value);
        let mut changed = false;

        // The selected segment (or the first) is the group's one Tab stop, and the keys move
        // the selection from it.
        let stop = selected.unwrap_or(0);
        if enabled && count > 0 && ui.memory(|m| m.has_focus(ids[stop])) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(ids[stop], EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() });
            });
            let (next, previous) = if direction::is_rtl(ui) { (Key::ArrowLeft, Key::ArrowRight) } else { (Key::ArrowRight, Key::ArrowLeft) };
            let pressed = |key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
            let target = if pressed(Key::Home) {
                Some(0)
            } else if pressed(Key::End) {
                Some(count - 1)
            } else if pressed(next) || pressed(Key::ArrowDown) {
                Some((stop + 1) % count)
            } else if pressed(previous) || pressed(Key::ArrowUp) {
                Some((stop + count - 1) % count)
            } else {
                None
            };
            if let Some(target) = target.filter(|t| Some(*t) != selected) {
                *self.value = self.items[target].value.clone();
                selected = Some(target);
                changed = true;
                ui.memory_mut(|m| m.request_focus(ids[target]));
            }
        }

        let stop = selected.unwrap_or(0);
        let mut clicked = None;
        let mut responses = Vec::with_capacity(count);
        for (i, (item, galley)) in self.items.iter().zip(&galleys).enumerate() {
            let on = selected == Some(i);
            let response = Interactive::new(i, Role::RadioButton, item.label.as_str())
                .sense(if i == stop { Sense::click() } else { Sense::CLICK })
                .checked(on)
                .colors(StateColors { fill: style.fill, border: None })
                .on_colors(StateColors { fill: style.selected_fill, border: None })
                .corner(Corner::Control)
                .enabled(enabled)
                .show(&mut group, segment);
            if response.clicked() && !on {
                clicked = Some(i);
            }
            let color = content_color(ui, if on { &style.selected_text } else { &style.text }, &response, true);
            if ui.is_rect_visible(response.rect) {
                let center = response.rect.center();
                match (galley, item.icon) {
                    (Some(galley), _) => {
                        let pos = (center - galley.size() / 2.0).round_to_pixels(ui.pixels_per_point());
                        ui.painter().galley(pos, Arc::clone(galley), color);
                    }
                    (None, Some(icon)) => {
                        let rect = Rect::from_center_size(center, vec2(style.icon, style.icon)).round_to_pixels(ui.pixels_per_point());
                        icons::paint(ui, rect, icon, icon_size(self.size), color);
                    }
                    (None, None) => {}
                }
            }
            responses.push(if item.icon.is_some() { response.on_hover_text(&item.label) } else { response });
        }

        // A click selects a segment after the ones before it reported their state: report the
        // new state of every segment, and draw it next frame.
        if let Some(target) = clicked {
            *self.value = self.items[target].value.clone();
            changed = true;
            for (i, id) in ids.iter().enumerate() {
                ui.ctx().accesskit_node_builder(*id, |node| node.set_toggled(if i == target { Toggled::True } else { Toggled::False }));
            }
            ui.ctx().request_repaint();
        }
        let mut response = match responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all.with_new_rect(track),
            None => ui.interact(track, ui.make_persistent_id(self.id_salt), Sense::hover()),
        };
        if changed {
            response.mark_changed();
        }
        response
    }
}

impl Segmented<'static, ()> {
    pub fn style(tokens: &UiTokens, size: ControlSize) -> SegmentedStyle {
        let s = &tokens.component.segmented;
        let mut track = Surface::style(tokens, SurfaceKind::Sunken);
        track.fill = s.track.to_egui();
        SegmentedStyle {
            height: s.height.get(size),
            pad_x: s.pad_x.get(size),
            icon: s.icon.get(size),
            inset: s.inset,
            label: s.label,
            track,
            radius: Corner::Control.radius(tokens),
            fill: s.segment.fill.to_egui(),
            text: s.segment.text.to_egui(),
            selected_fill: s.selected.fill.to_egui(),
            selected_text: s.selected.text.to_egui(),
        }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Segmented"];

const TEXT_ITEMS: [&str; 5] = ["New", "Add", "Subtract", "Intersect", "Exclude"];

const ICON_ITEMS: [(Icon, &str); 5] =
    [(Icon::Move, "Move"), (Icon::Hand, "Hand"), (Icon::Brush, "Brush"), (Icon::Eraser, "Eraser"), (Icon::Eyedropper, "Eyedropper")];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false);
    vec![
        Story { id: "controls/segmented/text", level: Level::Controls, component: "Segmented", name: "Text", args, show: show_text },
        Story { id: "controls/segmented/icons", level: Level::Controls, component: "Segmented", name: "Icons", args, show: show_icons },
    ]
}

/// The stories' selections, kept between frames.
fn with_choices(ui: &mut Ui, key: &str, show: impl FnOnce(&mut Ui, &mut [usize; 2])) {
    let id = ui.scope_id().with(key);
    let mut choices = ui.data(|d| d.get_temp::<[usize; 2]>(id).unwrap_or([0, 2]));
    show(ui, &mut choices);
    ui.data_mut(|d| d.insert_temp(id, choices));
}

/// Three and five text segments.
fn show_text(ui: &mut Ui, args: &mut Args) {
    let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
    with_choices(ui, "segmented-text", |ui, choices| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            for (n, choice) in [3, 5].into_iter().zip(choices.iter_mut()) {
                let mut control = Segmented::new(("segmented-text", n), format!("Combine ({n})"), choice).size(size).enabled(enabled);
                for (i, label) in TEXT_ITEMS.iter().take(n).enumerate() {
                    control = control.item(i, *label);
                }
                control.show(ui);
            }
            ui.add(Text::new(format!("Selected: {}, {}", TEXT_ITEMS[choices[0]], TEXT_ITEMS[choices[1]])).content(Content::Muted));
        });
    });
}

/// Three and five icon segments.
fn show_icons(ui: &mut Ui, args: &mut Args) {
    let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
    with_choices(ui, "segmented-icons", |ui, choices| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            for (n, choice) in [3, 5].into_iter().zip(choices.iter_mut()) {
                let mut control = Segmented::new(("segmented-icons", n), format!("Tool ({n})"), choice).size(size).enabled(enabled);
                for (i, (icon, label)) in ICON_ITEMS.iter().take(n).enumerate() {
                    control = control.icon_item(i, *icon, *label);
                }
                control.show(ui);
            }
        });
    });
}
