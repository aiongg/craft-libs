//! [`Switch`]: an on/off setting that applies at once, as a track with a sliding thumb.

use craft_theme::{ControlSize, Easing, States, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Pos2, Rect, Ui, pos2, vec2};

use super::button::content_color;
use super::check::{IndicatorColors, Labelled, Layout, Paint, indicator_shapes, label_colors};
use super::checkbox::LONG_LABEL;
use super::{SIZE_NAMES, salt, size_arg};
use crate::direction::{self, Direction};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{Divider, Gap, Interactive, Stack, Surface, SurfaceKind, SurfaceStyle};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// A switch's sizes, colours and motion, from [`Switch::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwitchStyle {
    /// Height of one line: `check.height`.
    pub height: f32,
    /// Width of the track: `check.track`.
    pub width: f32,
    /// Height of the track: `check.size`.
    pub track_height: f32,
    /// Diameter of the thumb: the track height less `check.thumb_inset` on each side.
    pub thumb: f32,
    /// Between the track's edge and the thumb: `check.thumb_inset`.
    pub inset: f32,
    /// Between the track and the label: `check.gap`.
    pub gap: f32,
    /// `check.label`.
    pub label: TypeRole,
    /// The sunken surface the track draws on, for its edge (a border, or a bevel in bevel
    /// themes); the fill and the round ends are the track's own.
    pub surface: SurfaceStyle,
    /// Off: `check.off.*`; the mark is the thumb.
    pub off: IndicatorColors,
    /// On: `check.on.*`.
    pub on: IndicatorColors,
    /// The label: `text.normal`, `text.disabled`.
    pub text: States<Color32>,
    /// How long the thumb takes to cross, in seconds: `motion.move`, none with reduced motion.
    pub travel: f32,
    /// The thumb's easing: `motion.move`.
    pub easing: Easing,
}

/// An on/off setting that applies at once, with a label: `Switch::new("Autosave", &mut on).show(ui)`.
///
/// The thumb sits at the reading start when off and slides to the end when on, over
/// `motion.move` (at once with reduced motion). A click on the track or label, or Space while it
/// has focus, flips it; each change is one [`Set`](crate::edit::EditEvent::Set) with the modifiers
/// held. AccessKit reports a `Switch` named by the label, toggled, after this frame's click.
///
/// The track sits at the reading start on a sunken [`Surface`] with round ends: `check.off.*`
/// with an outline and a `check.off.mark` thumb, or `check.on.*` with a `check.on.mark` thumb.
#[derive(Debug)]
pub struct Switch<'a> {
    label: String,
    on: &'a mut bool,
    size: ControlSize,
    enabled: bool,
    id_salt: Option<IdSalt>,
}

impl<'a> Switch<'a> {
    pub fn new(label: impl Into<String>, on: &'a mut bool) -> Self {
        Self { label: label.into(), on, size: ControlSize::Md, enabled: true, id_salt: None }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether it takes input; a disabled switch draws `check.*.disabled`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Keeps focus and animations when the layout around it changes; else the `Ui`'s next
    /// automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// Shows the switch; the response is [changed](egui::Response::changed) on a flip.
    pub fn show(self, ui: &mut Ui) -> EditResponse<bool> {
        let tokens = Tokens::of(ui);
        let style = Switch::style(&tokens, self.size);
        let layout = Layout { height: style.height, gap: style.gap, label: style.label, indicator: vec2(style.width, style.track_height) };
        let laid = Labelled::layout(ui, &tokens, &layout, &self.label);
        let sensed = Interactive::new(salt(ui, self.id_salt), Role::Switch, self.label.as_str())
            .state_layer(false)
            .enabled(self.enabled)
            .interact(ui, laid.size);
        let mut gesture = Gesture::load(ui, sensed.response().id, &*self.on, EditPolicy::default()).enabled(sensed.response().enabled());
        let clicked = sensed.response().clicked();
        if clicked {
            gesture.set(!*self.on);
            *self.on = !*self.on;
        }
        let on = *self.on;
        let mut response = sensed.checked(on).finish(ui);
        if clicked {
            response.mark_changed();
        }
        let position =
            ui.ctx().animate_bool_with_time_and_easing(response.id.with("craft_ui::switch::thumb"), on, style.travel, style.easing.to_egui());
        if ui.is_rect_visible(response.rect) {
            let (track, label) = laid.place(ui, response.rect);
            let paint = Paint::now(ui, if on { &style.on } else { &style.off }, &response);
            let round = CornerRadius::from(style.track_height / 2.0);
            ui.painter().extend(indicator_shapes(ui, &style.surface, track, round, paint));
            let thumb = Switch::thumb(&style, track, position, if direction::is_rtl(ui) { Direction::Rtl } else { Direction::Ltr });
            ui.painter().circle_filled(thumb.round_to_pixels(ui.pixels_per_point()), style.thumb / 2.0, paint.mark);
            laid.paint_label(ui, label, content_color(ui, &style.text, &response, false));
        }
        EditResponse::new(laid.tooltip(response, &self.label), gesture.finish())
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> SwitchStyle {
        let c = &tokens.component.check;
        let track_height = c.size.get(size);
        let motion = tokens.motion.r#move;
        SwitchStyle {
            height: c.height.get(size),
            width: c.track.get(size),
            track_height,
            thumb: track_height - 2.0 * c.thumb_inset,
            inset: c.thumb_inset,
            gap: c.gap,
            label: c.label,
            surface: Surface::style(tokens, SurfaceKind::Sunken),
            off: IndicatorColors::from(&c.off),
            on: IndicatorColors::from(&c.on),
            text: label_colors(tokens),
            travel: motion.millis as f32 / 1000.0,
            easing: motion.easing,
        }
    }

    /// The thumb's centre in `track` when it is `position` of the way from off (0) to on (1):
    /// off at the reading start of `direction`, on at its end.
    pub fn thumb(style: &SwitchStyle, track: Rect, position: f32, direction: Direction) -> Pos2 {
        let reach = style.inset + style.thumb / 2.0;
        let (off, on) = match direction {
            Direction::Rtl => (track.right() - reach, track.left() + reach),
            _ => (track.left() + reach, track.right() - reach),
        };
        pos2(off + (on - off) * position.clamp(0.0, 1.0), track.center().y)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Switch"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1);
    vec![
        Story { id: "controls/switch/states", level: Level::Controls, component: "Switch", name: "States", args, show: show_states },
        Story { id: "controls/switch/settings", level: Level::Controls, component: "Switch", name: "Settings", args, show: show_settings },
    ]
}

/// Off and on, enabled and disabled, a long label. The enabled ones flip.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let state_id = ui.scope_id().with("switch-states");
    let mut on = ui.data(|d| d.get_temp::<[bool; 3]>(state_id).unwrap_or([false, true, true]));
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (i, label) in ["Off", "On", LONG_LABEL].into_iter().enumerate() {
            let _ = Switch::new(label, &mut on[i]).size(size).id_salt(("switch", i)).show(ui);
        }
        for (i, (label, mut fixed)) in [("Disabled (off)", false), ("Disabled (on)", true)].into_iter().enumerate() {
            let _ = Switch::new(label, &mut fixed).size(size).enabled(false).id_salt(("switch-disabled", i)).show(ui);
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, on));
}

const SETTINGS: [&str; 4] = ["Autosave every five minutes", "Show rulers", "Snap to the pixel grid", "Use the graphics card"];

/// A settings list: one switch per row between dividers, the last one disabled.
fn show_settings(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let state_id = ui.scope_id().with("switch-settings");
    let mut on = ui.data(|d| d.get_temp::<[bool; 4]>(state_id).unwrap_or([true, false, true, true]));
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (i, label) in SETTINGS.into_iter().enumerate() {
            if i > 0 {
                Divider::horizontal().show(ui);
            }
            let _ = Switch::new(label, &mut on[i]).size(size).enabled(i + 1 < SETTINGS.len()).id_salt(("switch-setting", i)).show(ui);
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, on));
}
