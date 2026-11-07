//! `Rating`: stars the user clicks, a row of `max` parts over a value from 0 to `max`. A click on the current star
//! clears the rating; arrows, Home, End and the digits set it while the row has focus.

use super::words::ChoiceWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Name, Numeric, PaintUi, PartsCx,
    Pointer as _, Role, SetPosition, Styled as _, WordSetCx as _,
};
use crate::primitives::IconSpec;
use crate::runtime::focus::{ActivateKeys, KeepKeys, KeyUse};
use crate::style::{ComponentStyle, Flags, FocusLook, IconLook, Ink, StyleCx};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{Flow, IconId, Metrics, PartKey, Sense};
use craft_keymap::{Chord, Key};
use craft_theme::{Stateful, States};
use egui::{Rect, Vec2};

const STAR: PartKey = PartKey::new("star");

/// The digit keys, 0 to 5; a rating with fewer stars takes the digits up to its own maximum.
const DIGITS: [Chord; 6] = [
    Chord::plain(Key::Num0),
    Chord::plain(Key::Num1),
    Chord::plain(Key::Num2),
    Chord::plain(Key::Num3),
    Chord::plain(Key::Num4),
    Chord::plain(Key::Num5),
];

/// The style of a rating.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct RatingStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// A star the rating covers.
    #[style(icon)]
    lit: IconLook,
    /// A star past the rating.
    #[style(icon)]
    unlit: IconLook,
    /// The square one star takes.
    cell: f32,
}

/// A rating over a `u8` from 0 (unrated) to `max`. A `Mixed` binding shows no star lit.
#[derive(crate::contract::Component)]
pub struct Rating<'a> {
    #[common]
    common: Common,
    value: Bind<'a, u8>,
    max: u8,
}

impl<'a> Rating<'a> {
    /// A rating of `value` with `max` stars.
    pub fn new(value: Bind<'a, u8>, max: u8) -> Self {
        Rating { common: Common::new(), value, max }
    }

    fn current(&self, shown: Option<&u8>) -> u8 {
        shown.copied().unwrap_or(0).min(self.max)
    }

    fn star(at: u8) -> craft_behavior::PartRef {
        STAR.at(u64::from(at))
    }
}

impl Component for Rating<'_> {
    const ROLE: Role = Role::RADIO_GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_keep(KeepKeys::InlineArrows).with_activate(ActivateKeys::Neither);
    type Style = RatingStyle;
    type Output = EditResponse<u8>;

    fn style(cx: &StyleCx<'_>, _: RatingStyleKey) -> RatingStyle {
        let control = cx.control();
        let text = &cx.tokens.color.text;
        let look = |rest| IconLook::new(control.icon, Stateful::new(rest).off(States::new(rest, rest, rest, cx.color(text.disabled))));
        RatingStyle {
            metrics: Metrics::new().min_size(Vec2::splat(control.height)),
            focus: crate::controls::focus_look(cx),
            lit: look(cx.ink(Ink::OnPanel)),
            unlit: look(cx.color(text.muted)),
            cell: control.height,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.style.cell * f32::from(self.max), cx.style.cell)
    }

    fn sense(&self) -> Sense {
        Sense::FOCUS
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let current = self.current(cx.value());
        let mut flow = Flow::new(rect, cx.flow.is_rtl());
        for at in 0..self.max {
            let (cell, rest) = flow.take_start(rect.height());
            flow = rest;
            let part = cx.part(Self::star(at), cell).sense(Sense::CLICK).pointer_only("the arrow keys, Home, End and the digits set the rating");
            if at + 1 == current {
                part.flags(Flags::CHECKED);
            }
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, u8>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let current = self.current(cx.value());
        for at in 0..self.max {
            if cx.part(Self::star(at)).press.phase.is_click() {
                cx.set(if at + 1 == current { 0 } else { at + 1 });
            }
        }
        let steps = ValueSteps::new(f64::from(current), 0.0, f64::from(self.max), 1.0, 1.0);
        while let Some((mv, mods)) = cx.intents().take_move() {
            if let StepResult::Set(next) = step_value(&steps, ValueAxes::Horizontal, mv, mods) {
                let next = next as u8;
                if next != current {
                    cx.set(next);
                }
            }
        }
        for (digit, chord) in (0..=self.max).zip(DIGITS) {
            if cx.take_claimed(chord) && digit != current {
                cx.set(digit);
            }
        }
        for chord in DIGITS {
            cx.claim(chord);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<ChoiceWords>();
        let stars = |n: u8| words.stars.text(u64::from(n), &cx.format().integer(u64::from(n)));
        let current = self.current(cx.value());
        let text = if current == 0 { words.unrated.as_str().to_owned() } else { stars(current) };
        access.numeric(Numeric::new(f64::from(current), 0.0..=f64::from(self.max)).step(1.0).text(text));
        for at in 0..self.max {
            let node = access.node(Self::star(at), Role::RADIO_BUTTON);
            node.name = Name::Text(stars(at + 1));
            node.position = Some(SetPosition::new(usize::from(at)).of(usize::from(self.max)));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let current = self.current(cx.value());
        let hovered = (0..self.max).find(|at| cx.part(Self::star(*at)).hovered).map(|at| at + 1);
        let shown = hovered.unwrap_or(current);
        let draw = ui.draw();
        let mut flow = Flow::new(cx.rect, cx.flow.is_rtl());
        for at in 0..self.max {
            let (cell, rest) = flow.take_start(cx.style.cell);
            flow = rest;
            let (look, name) = if at < shown { (&cx.style.lit, "star_filled") } else { (&cx.style.unlit, "star") };
            let icon = Rect::from_center_size(cell.center(), Vec2::splat(look.side));
            draw.icon(icon, &IconSpec::svg(IconId::new(name)).color(cx.resolve(&look.color)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<u8> {
        cx.edit_response()
    }
}
