//! [`Kbd`]: a key chord in the platform's notation.

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::os::OperatingSystem;
use egui::text::LayoutJob;
use egui::{Color32, Key, KeyboardShortcut, ModifierNames, Modifiers, Rect, Response, Sense, Ui, Vec2, Widget, WidgetInfo, pos2, vec2};

use crate::primitives::{Content, Gap, Stack, Surface, SurfaceKind, SurfaceStyle};
use crate::story::{Args, Level, Story, variant_cluster};
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
    /// `kbd.text`: the keys on keycaps. Plain keys take the caller's [`Content`] colour.
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
/// symbols in Apple's order with no separator (`⌃⌥⇧⌘S`), elsewhere names joined with `+` (`Ctrl+Alt+Shift+S`),
/// as egui's [`ModifierNames`]. [`Kbd::keys`] shows the given key names joined with `+` on every
/// platform (`Shift+Click`). Plain, the chord is text in a [`Content`] colour, muted by default,
/// for menus and tooltips; [`keycaps`](Kbd::keycaps) puts each key in a small raised box for help
/// pages and the command palette.
///
/// A chord keeps its order right to left
/// ([`Unmirrored::KeyChord`](crate::direction::Unmirrored::KeyChord)). It has no input and no Tab
/// stop; AccessKit sees a label with the names (`Cmd+Shift+S`).
#[derive(Clone, Debug)]
pub struct Kbd {
    chord: Chord,
    keycaps: bool,
    content: Content,
    os: Option<OperatingSystem>,
}

impl Kbd {
    pub fn new(shortcut: &KeyboardShortcut) -> Self {
        Self { chord: Chord::Shortcut(*shortcut), keycaps: false, content: Content::Muted, os: None }
    }

    /// Keys named by the caller, in order, e.g. `["Shift", "Click"]`.
    pub fn keys<S: AsRef<str>>(keys: &[S]) -> Self {
        Self { chord: Chord::Keys(keys.iter().map(|k| k.as_ref().to_owned()).collect()), keycaps: false, content: Content::Muted, os: None }
    }

    /// The colour of plain keys, e.g. a highlighted menu row's content colour. Keycaps draw their
    /// keys in `kbd.text`.
    pub fn content(mut self, content: Content) -> Self {
        self.content = content;
        self
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

    /// The keys of `shortcut` for `os`, modifiers first in the platform's order: Apple's ⌃⌥⇧⌘ on
    /// macOS, Ctrl, Alt, Shift elsewhere.
    pub fn parts(shortcut: &KeyboardShortcut, os: OperatingSystem) -> Vec<String> {
        let mac = is_mac(os);
        let mut out = modifier_names(&shortcut.modifiers, mac, if mac { &ModifierNames::SYMBOLS } else { &ModifierNames::NAMES });
        out.push(key_name(shortcut.logical_key, mac));
        out
    }

    /// The chord as one line for `os`: a shortcut's macOS symbols with no separator, its names
    /// elsewhere and caller-named keys everywhere joined with `+`.
    pub fn text(&self, os: OperatingSystem) -> String {
        let keys = self.keys_for(os);
        let symbols = matches!(self.chord, Chord::Shortcut(_)) && is_mac(os);
        keys.join(if symbols { "" } else { "+" })
    }

    /// The names for a screen reader and AccessKit's `keyboard_shortcut`: `Shift+Cmd+S`, in the
    /// platform's order whatever its symbols.
    pub(crate) fn spoken(&self, os: OperatingSystem) -> String {
        match &self.chord {
            Chord::Shortcut(shortcut) => {
                let mut keys = modifier_names(&shortcut.modifiers, is_mac(os), &ModifierNames::NAMES);
                keys.push(shortcut.logical_key.name().to_owned());
                keys.join("+")
            }
            Chord::Keys(keys) => keys.join("+"),
        }
    }

    /// The platform the chord is written for: the one set with [`os`](Kbd::os), else `ctx`'s.
    pub(crate) fn os_in(&self, ctx: &egui::Context) -> OperatingSystem {
        self.os.unwrap_or_else(|| ctx.os())
    }

    fn keys_for(&self, os: OperatingSystem) -> Vec<String> {
        match &self.chord {
            Chord::Shortcut(shortcut) => Kbd::parts(shortcut, os),
            Chord::Keys(keys) => keys.clone(),
        }
    }
}

/// The names of the modifiers held in `m`, in the platform's order: ⌃⌥⇧⌘ on a Mac (Apple's, as
/// its menus write them), Ctrl, Alt, Shift elsewhere.
fn modifier_names(m: &Modifiers, mac: bool, names: &ModifierNames<'_>) -> Vec<String> {
    let held: [(bool, &str); 4] = if mac {
        [(m.ctrl, names.ctrl), (m.alt, names.mac_alt), (m.shift, names.shift), (m.mac_cmd || m.command, names.mac_cmd)]
    } else {
        [(m.ctrl || m.command, names.ctrl), (m.alt, names.alt), (m.shift, names.shift), (false, "")]
    };
    held.iter().filter(|(on, _)| *on).map(|(_, name)| (*name).to_owned()).collect()
}

pub(crate) fn is_mac(os: OperatingSystem) -> bool {
    matches!(os, OperatingSystem::Mac | OperatingSystem::IOS)
}

/// The key as the suite writes it (craft-keymap's `Chord::display`): arrows and editing keys as
/// Apple's glyphs on a Mac, names elsewhere, punctuation as the character it types.
fn key_name(key: Key, mac: bool) -> String {
    if mac {
        let glyph = match key {
            Key::ArrowUp => "↑",
            Key::ArrowDown => "↓",
            Key::ArrowLeft => "←",
            Key::ArrowRight => "→",
            Key::Enter => "⏎",
            Key::Backspace => "⌫",
            Key::Delete => "⌦",
            Key::Escape => "⎋",
            Key::Tab => "⇥",
            _ => "",
        };
        if !glyph.is_empty() {
            return glyph.to_owned();
        }
    }
    let name = match key {
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::PageUp => "Page Up",
        Key::PageDown => "Page Down",
        Key::Minus => "-",
        Key::Plus => "+",
        Key::Equals => "=",
        Key::Colon => ":",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Semicolon => ";",
        Key::Backslash => "\\",
        Key::Slash => "/",
        Key::Pipe => "|",
        Key::Questionmark => "?",
        Key::Exclamationmark => "!",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::OpenCurlyBracket => "{",
        Key::CloseCurlyBracket => "}",
        Key::Quote => "'",
        Key::Backtick => "`",
        _ => key.name(),
    };
    name.to_owned()
}

impl Widget for Kbd {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Kbd::style(&tokens);
        let os = self.os.unwrap_or_else(|| ui.ctx().os());
        let keys = self.keys_for(os);
        let color = if self.keycaps { style.text } else { self.content.color(&tokens).to_egui() };
        let format = role_format(&tokens, style.label, color);
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
            let galley = layout(self.text(os));
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
                ui.painter().galley(pos.round_to_pixels(ppp), galley, color);
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
        Story::new("controls/kbd/plain", Level::Controls, "Kbd", "Plain", Args::new, |ui, _| show(ui, false)),
        Story::new("controls/kbd/keycaps", Level::Controls, "Kbd", "Keycaps", Args::new, |ui, _| show(ui, true)),
    ]
}

/// A few chords in macOS and other notation side by side.
fn show(ui: &mut Ui, keycaps: bool) {
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (os, name) in [(OperatingSystem::Mac, "macOS"), (OperatingSystem::Nix, "Other")] {
            variant_cluster(ui, name, |ui| {
                for (modifiers, key) in CHORDS {
                    ui.add(Kbd::new(&KeyboardShortcut::new(modifiers, key)).keycaps(keycaps).os(os));
                }
                ui.add(Kbd::keys(&["Shift", "Click"]).keycaps(keycaps).os(os));
            });
        }
    });
}
