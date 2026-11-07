//! `Kbd`: a key chord as text, plain or in a keycap.

use crate::contract::{Access, Common, Component, Cx, MeasureCx, Name, PaintUi, Props, Role, Styled as _};
use crate::primitives::{ChordText, TextRun};
use crate::runtime::commands::{CommandRef, ResolvedCommand};
use crate::runtime::env::Label;
use crate::runtime::windows::os_of;
use crate::style::{ComponentStyle, Corners, StyleCx, SurfaceKind, TextLook, TypeRole};
use craft_behavior::{Insets, Metrics, Sense};
use craft_theme::Stateful;
use egui::{Context, Response, Vec2};

/// The style of a chord.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct KbdStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The keys.
    #[style(text)]
    text: TextLook,
}

/// A key chord. It shows the chord of the bound command in the platform's notation (`Ctrl+Shift+S`, `⇧⌘S`); with
/// no command it shows the text it was given, written by the caller. A chord keeps its left-to-right order under
/// right-to-left. The keycap form draws the chord in a small raised box. AccessKit reports a label with the chord.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Kbd {
    #[common]
    common: Common,
    chord: Label,
    /// Draws the chord in a raised box, for help pages and the command palette.
    #[style_key]
    #[prop]
    keycap: bool,
}

impl Kbd {
    /// A chord written as `chord`, which is also its accessible name.
    pub fn new(chord: impl Into<Label>) -> Self {
        let chord = chord.into();
        Kbd { common: Common::new(), chord: chord.clone(), keycap: false }.label(chord)
    }

    /// The chord of `command`, in the platform's notation; a command with no chord shows nothing.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        <Kbd as Props>::command(Kbd::new(Label::Empty), command)
    }

    /// The text shown: the bound command's chord for the platform, else the given chord.
    fn shown(&self, command: Option<&ResolvedCommand>, ctx: &Context) -> String {
        match (&self.common.command, command) {
            (Some(_), Some(command)) => command.chord.as_ref().map_or_else(String::new, |chord| chord.display(os_of(ctx))),
            _ => self.chord.as_str().to_owned(),
        }
    }
}

impl Component for Kbd {
    const ROLE: Role = Role::LABEL;
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    const OVERLAYS: bool = false;
    type Style = KbdStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, key: KbdStyleKey) -> KbdStyle {
        let metrics = if key.keycap {
            let line = cx.tokens.typography.get(TypeRole::Caption).line_height;
            let (pad_x, pad_y) = (cx.tokens.space.gap.inline, cx.tokens.stroke.thin);
            Metrics::new().padding(Insets::new(pad_x, pad_x, pad_y, pad_y)).min_size(Vec2::splat(line + 2.0 * pad_y))
        } else {
            Metrics::new()
        };
        KbdStyle { metrics, text: TextLook::new(TypeRole::Caption, Stateful::new(cx.color(cx.tokens.color.text.muted))) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let ctx = cx.ui.ctx();
        let command = self.common.command.clone().map(|command| crate::runtime::commands::resolve(ctx, command));
        let text = ChordText::new(self.shown(command.as_ref(), ctx));
        cx.layout(&text.run().role(cx.style.text.role)).size()
    }

    fn sense(&self) -> Sense {
        super::display_sense(&self.common, false)
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if self.common.command.is_some() {
            access.name(Name::Text(self.shown(cx.command.as_deref(), &cx.ctx)));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let draw = ui.draw();
        if self.keycap {
            draw.surface(cx.rect, SurfaceKind::Raised, Corners::All);
        }
        let text = ChordText::new(self.shown(cx.command.as_deref(), &cx.ctx));
        let run = text.run().role(cx.style.text.role).color(cx.resolve(&cx.style.text.color));
        let layout = draw.layout(&run, cx.content_rect.width());
        draw.galley(cx.content_rect.center() - layout.size() / 2.0, &layout);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
