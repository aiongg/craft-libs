//! [`Kbd`]: a key chord in the platform's notation.

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::os::OperatingSystem;
use egui::text::LayoutJob;
use egui::{Color32, Key, KeyboardShortcut, ModifierNames, Modifiers, Rect, Response, Sense, Ui, Vec2, Widget, WidgetInfo, pos2, vec2};

use crate::primitives::{Content, Gap, Row, Stack, Surface, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};

/// A key chord's sizes and colours, from [`Kbd::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct KbdStyle {
    /// Height of a keycap: `kbd.height`.
    pub height: f32,
    /// At the start and end of a keycap's text: `kbd.pad_x`.
    pub pad_x: f32,
    /// Between keycaps: half of `kbd.pad_x`.
    pub gap: f32,
    /// `kbd.label`.
    pub label: TypeRole,
    /// `kbd.text`: the keys, plain or on keycaps.
    pub text: Color32,
    /// A keycap: a raised surface with `kbd.fill` and `kbd.radius`.
    pub keycap: SurfaceStyle,
}

#[derive(Clone, Debug)]
enum Chord {
    Shortcut(KeyboardShortcut),
    Keys(Vec<String>),
}

/// A key chord: `ui.add(Kbd::new(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S)))`.
///
/// Given a [`KeyboardShortcut`], it writes the modifiers in the platform's notation: macOS
/// symbols (`⌃⇧⌥⌘S`), elsewhere names joined with `+` (`Ctrl+Alt+Shift+S`), as egui's
/// [`ModifierNames`]. [`Kbd::keys`] shows the given key names, joined the same way. Plain, the
/// chord is muted text for menus and tooltips; [`keycaps`](Kbd::keycaps) puts each key in a small
/// raised box for help pages and the command palette.
///
/// It has no input and no Tab stop; AccessKit sees a label with the names (`Cmd+Shift+S`).
#[derive(Clone, Debug)]
pub struct Kbd {
    chord: Chord,
    keycaps: bool,
    os: Option<OperatingSystem>,
}

impl Kbd {
    pub fn new(shortcut: &KeyboardShortcut) -> Self {
        Self { chord: Chord::Shortcut(*shortcut), keycaps: false, os: None }
    }

    /// Keys named by the caller, in order, e.g. `["Shift", "Click"]`.
    pub fn keys<S: AsRef<str>>(keys: &[S]) -> Self {
        Self { chord: Chord::Keys(keys.iter().map(|k| k.as_ref().to_owned()).collect()), keycaps: false, os: None }
    }

    /// Each key in a small raised box.
    pub fn keycaps(mut self, keycaps: bool) -> Self {
        self.keycaps = keycaps;
        self
    }

    /// Writes the chord for `os` instead of the context's, for previews and help that shows
    /// another platform.
    pub fn os(mut self, os: OperatingSystem) -> Self {
        self.os = Some(os);
        self
    }

    pub fn style(tokens: &UiTokens) -> KbdStyle {
        let k = &tokens.component.kbd;
        let mut keycap = Surface::style(tokens, SurfaceKind::Raised);
        keycap.fill = k.fill.to_egui();
        keycap.radius = corner_radius(k.radius);
        KbdStyle { height: k.height, pad_x: k.pad_x, gap: k.pad_x / 2.0, label: k.label, text: k.text.to_egui(), keycap }
    }

    /// The keys of `shortcut` for `os`, modifiers first in the platform's order.
    pub fn parts(shortcut: &KeyboardShortcut, os: OperatingSystem) -> Vec<String> {
        let mac = is_mac(os);
        let names = if mac { ModifierNames::SYMBOLS } else { ModifierNames::NAMES };
        let m: &Modifiers = &shortcut.modifiers;
        let mut out = Vec::new();
        let mut add = |on: bool, name: &str| {
            if on {
                out.push(name.to_owned());
            }
        };
        if mac {
            add(m.ctrl, names.ctrl);
            add(m.shift, names.shift);
            add(m.alt, names.mac_alt);
            add(m.mac_cmd || m.command, names.mac_cmd);
        } else {
            add(m.ctrl || m.command, names.ctrl);
            add(m.alt, names.alt);
            add(m.shift, names.shift);
        }
        out.push(key_name(shortcut.logical_key));
        out
    }

    /// `keys` as one line: no separator between macOS symbols, `+` elsewhere.
    pub fn text(keys: &[String], os: OperatingSystem) -> String {
        keys.join(if is_mac(os) { "" } else { "+" })
    }

    /// The names for a screen reader: `Cmd+Shift+S`, whatever the platform's symbols.
    fn spoken(&self, os: OperatingSystem) -> String {
        match &self.chord {
            Chord::Shortcut(shortcut) => shortcut.format(&ModifierNames::NAMES, is_mac(os)),
            Chord::Keys(keys) => keys.join("+"),
        }
    }

    fn keys_for(&self, os: OperatingSystem) -> Vec<String> {
        match &self.chord {
            Chord::Shortcut(shortcut) => Kbd::parts(shortcut, os),
            Chord::Keys(keys) => keys.clone(),
        }
    }
}

pub(crate) fn is_mac(os: OperatingSystem) -> bool {
    matches!(os, OperatingSystem::Mac | OperatingSystem::IOS)
}

fn key_name(key: Key) -> String {
    key.symbol_or_name().to_owned()
}

impl Widget for Kbd {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Kbd::style(&tokens);
        let os = self.os.unwrap_or_else(|| ui.ctx().os());
        let keys = self.keys_for(os);
        let format = role_format(&tokens, style.label, style.text);
        let layout = |text: String| ui.painter().layout_job(LayoutJob::single_section(text, format.clone()));

        // (galley, rect relative to the chord's start) per box; plain text is one box.
        let (boxes, size): (Vec<_>, Vec2) = if self.keycaps {
            let mut x = 0.0;
            let boxes: Vec<_> = keys
                .iter()
                .map(|key| {
                    let galley = layout(key.clone());
                    let width = (galley.size().x + 2.0 * style.pad_x).max(style.height);
                    let rect = Rect::from_min_size(pos2(x, 0.0), vec2(width, style.height));
                    x += width + style.gap;
                    (galley, rect)
                })
                .collect();
            let width = boxes.last().map_or(0.0, |(_, r)| r.max.x);
            (boxes, vec2(width, style.height))
        } else {
            let galley = layout(Kbd::text(&keys, os));
            let size = galley.size();
            (vec![(galley, Rect::from_min_size(pos2(0.0, 0.0), size))], size)
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, ui.is_enabled(), self.spoken(os)));
        if ui.is_rect_visible(rect) {
            let ppp = ui.pixels_per_point();
            for (galley, local) in boxes {
                let cell = local.translate(rect.min.to_vec2());
                if self.keycaps {
                    ui.painter().extend(style.keycap.shapes(cell, ppp));
                }
                let pos = Rect::from_center_size(cell.center(), galley.size()).min;
                ui.painter().galley(pos.round_to_pixels(ppp), galley, style.text);
            }
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Kbd"];

const CHORDS: [(Modifiers, Key); 4] = [
    (Modifiers::COMMAND, Key::S),
    (Modifiers { command: true, shift: true, ..Modifiers::NONE }, Key::Z),
    (Modifiers { alt: true, ctrl: true, shift: true, mac_cmd: true, command: false }, Key::K),
    (Modifiers::NONE, Key::Escape),
];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls/kbd/plain",
            level: Level::Controls,
            component: "Kbd",
            name: "Plain",
            args: || Args::new(),
            show: |ui, _| show(ui, false),
        },
        Story {
            id: "controls/kbd/keycaps",
            level: Level::Controls,
            component: "Kbd",
            name: "Keycaps",
            args: || Args::new(),
            show: |ui, _| show(ui, true),
        },
    ]
}

/// A few chords in macOS and other notation side by side.
fn show(ui: &mut Ui, keycaps: bool) {
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (os, name) in [(OperatingSystem::Mac, "macOS"), (OperatingSystem::Nix, "Other")] {
            Row::new(Gap::Section).show(ui, |ui| {
                ui.add(Text::new(name).content(Content::Muted));
                for (modifiers, key) in CHORDS {
                    ui.add(Kbd::new(&KeyboardShortcut::new(modifiers, key)).keycaps(keycaps).os(os));
                }
                ui.add(Kbd::keys(&["Shift", "Click"]).keycaps(keycaps).os(os));
            });
        }
    });
}
