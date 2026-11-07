//! `Kbd`: a key chord as text, plain or one keycap per key.

use super::words::DisplayWords;
use crate::contract::{Access, Common, Component, Cx, MeasureCx, Name, PaintUi, Props, Role, Styled as _, WordSetCx as _};
use crate::primitives::ChordText;
use crate::runtime::commands::{CommandRef, ResolvedCommand};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleColor, StyleCx, SurfaceKind, SurfaceLook, TextLook, TypeRole};
use craft_behavior::{Insets, Metrics, Sense};
use craft_host::target::Os;
use craft_keymap::Layout;
use craft_theme::Stateful;
use egui::{Rect, Response, Vec2, pos2};

#[cfg(feature = "testing")]
mod stories;

/// The style of a chord.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct KbdStyle {
    /// The raised box of one keycap; the chord as text and the row of keycaps have none.
    #[style(surface)]
    keycap: Option<SurfaceLook>,
    #[style(metrics)]
    metrics: Metrics,
    /// The keys.
    #[style(text)]
    text: TextLook,
    /// Around the text of one keycap.
    key_pad: Insets,
    /// Between keycaps.
    key_gap: f32,
}

/// What a chord shows: one text per key, and whether they are written without a separator (macOS symbols).
struct Keys {
    parts: Vec<String>,
    glued: bool,
}

impl Keys {
    fn text(&self) -> String {
        self.parts.join(if self.glued { "" } else { "+" })
    }
}

/// The keys of a chord a caller wrote as text: `Shift+Click`, `Ctrl++`. A `+` with no key before it is the key.
fn written_keys(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c == '+' && !current.is_empty() {
            keys.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        keys.push(current);
    }
    keys
}

/// A key chord. It shows the chord of the bound command in the platform's notation (`Ctrl+Shift+S`, `⇧⌘S`); with
/// no command it shows the text it was given, written by the caller. A chord keeps its left-to-right order under
/// right-to-left. The keycap form draws each key in its own small raised box. AccessKit reports a label with the
/// chord, spoken for a command's chord (`Ctrl+Option+Shift+Cmd+K`).
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Kbd {
    #[common]
    common: Common,
    chord: Label,
    /// Draws each key in a raised box, for help pages and the command palette.
    #[style_key]
    #[prop]
    keycap: bool,
    /// One key of the keycap form: the box itself.
    #[style_key]
    single: bool,
}

impl Kbd {
    /// A chord written as `chord`, which is also its accessible name.
    pub fn new(chord: impl Into<Label>) -> Self {
        let chord = chord.into();
        Kbd { common: Common::new(), chord: chord.clone(), keycap: false, single: false }.label(chord)
    }

    /// The chord of `command`, in the platform's notation; a command with no chord shows nothing.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        <Kbd as Props>::command(Kbd::new(Label::Empty), command)
    }

    /// One keycap of a chord drawn by the keycap form.
    fn key(text: &str) -> Self {
        Kbd { single: true, keycap: true, ..Kbd::new(text.to_owned()) }
    }

    /// The keys shown: the bound command's chord for `os`, else the given chord. Key names that are words
    /// come from the catalog.
    fn keys(&self, command: Option<&ResolvedCommand>, os: Os, layout: &Layout, words: &DisplayWords) -> Keys {
        if self.single {
            return Keys { parts: vec![self.chord.as_str().to_owned()], glued: false };
        }
        match command {
            Some(command) => {
                let Some(chord) = command.chord.as_ref() else { return Keys { parts: Vec::new(), glued: false } };
                let mut parts = chord.parts_for(os, layout);
                if !os.is_mac()
                    && let (Some(word), Some(last)) = (words.key(chord.key), parts.last_mut())
                {
                    *last = word.to_owned();
                }
                Keys { parts, glued: os.is_mac() }
            }
            None => Keys { parts: written_keys(self.chord.as_str()), glued: false },
        }
    }

    /// The name for a screen reader: a command's chord spoken, else the text given.
    fn spoken(&self, command: Option<&ResolvedCommand>, os: Os) -> String {
        match command {
            Some(command) => command.chord.as_ref().map_or_else(String::new, |chord| chord.spoken(os)),
            None => self.chord.as_str().to_owned(),
        }
    }
}

impl Component for Kbd {
    const ROLE: Role = Role::LABEL;
    const PARTS: bool = false;
    const ALT_ROLES: &'static [Role] = &[Role::None];
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    const OVERLAYS: bool = false;
    type Style = KbdStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, key: KbdStyleKey) -> KbdStyle {
        let tokens = cx.tokens;
        let (pad_x, pad_y) = (tokens.space.gap.inline, tokens.space.pad.chip);
        let key_pad = Insets::new(pad_x, pad_x, pad_y, pad_y);
        let muted = Stateful::new(cx.color(tokens.color.text.muted));
        let (keycap, metrics, ink) = if key.single {
            let raised = cx.surface(SurfaceKind::Raised);
            let look = SurfaceLook::new(Stateful::new(StyleColor(raised.fill))).kind(SurfaceKind::Raised).radius(raised.radius);
            (Some(look), Metrics::new().padding(key_pad), Stateful::new(cx.color(tokens.color.text.normal)))
        } else {
            (None, Metrics::new(), cx.host_ink(muted))
        };
        KbdStyle { keycap, metrics, text: TextLook::new(TypeRole::Caption, ink), key_pad, key_gap: tokens.space.gap.tight }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let keys = self.keys(cx.command(), cx.env.shell().os, cx.env.keyboard_layout(), &cx.words::<DisplayWords>());
        let size = |text: &str| cx.layout(&ChordText::new(text).run().role(cx.style.text.role)).size();
        if !self.keycap || self.single {
            return size(&keys.text());
        }
        let boxes: Vec<Vec2> = keys
            .parts
            .iter()
            .map(|part| size(part) + Vec2::new(cx.style.key_pad.start + cx.style.key_pad.end, cx.style.key_pad.top + cx.style.key_pad.bottom))
            .collect();
        let gaps = cx.style.key_gap * boxes.len().saturating_sub(1) as f32;
        Vec2::new(boxes.iter().map(|size| size.x).sum::<f32>() + gaps, boxes.iter().map(|size| size.y).fold(0.0, f32::max))
    }

    fn sense(&self) -> Sense {
        super::display_sense(&self.common, false)
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if self.single {
            access.role(Role::None);
        } else if cx.command.is_some() {
            access.name(Name::Text(self.spoken(cx.command.as_deref(), cx.env.shell().os)));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let keys = self.keys(cx.command.as_deref(), cx.env.shell().os, cx.env.keyboard_layout(), &cx.words::<DisplayWords>());
        if keys.parts.is_empty() {
            return;
        }
        if self.keycap && !self.single {
            let pad = cx.style.key_pad;
            let sizes: Vec<Vec2> = {
                let draw = ui.draw();
                keys.parts
                    .iter()
                    .map(|part| {
                        let run = ChordText::new(part.as_str());
                        let text = draw.layout(&run.run().role(cx.style.text.role), f32::INFINITY).size();
                        text + Vec2::new(pad.start + pad.end, pad.top + pad.bottom)
                    })
                    .collect()
            };
            let mut x = cx.content_rect.min.x;
            for (at, (part, size)) in keys.parts.iter().zip(sizes).enumerate() {
                let top = cx.content_rect.center().y - size.y / 2.0;
                ui.child(at, Rect::from_min_size(pos2(x, top), size), Kbd::key(part));
                x += size.x + cx.style.key_gap;
            }
            return;
        }
        let text = ChordText::new(keys.text());
        let run = text.run().role(cx.style.text.role).color(cx.resolve(&cx.style.text.color));
        let draw = ui.draw();
        let layout = draw.layout(&run, cx.content_rect.width());
        draw.galley(cx.content_rect.center() - layout.size() / 2.0, &layout);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
