//! The macOS menu bar, through muda (AppKit menus behind a safe API; no GTK with default features
//! off). This module only builds and updates the `NSMenu`s; the layout and how keys reach egui are
//! in the platform-free part of the crate.
//!
//! - muda ids must be unique: a command shown twice (Keyboard Shortcuts in Edit and Window) has
//!   `id` for its first item and `id#2` for its second.
//! - A structure change ([`crate::structure_key`]: a new Open Recent file, a language switch)
//!   rebuilds the menu bar; labels, enabled and checked are updated in place. Opening or closing a
//!   document ([`crate::window_key`]) rebuilds only the Window menu: AppKit adds its own items to
//!   that menu (window tiling, the window list), so muda can't insert rows there by position.
//! - A chosen item is a click or a key equivalent: AppKit's current event says which. A key
//!   equivalent goes back to egui as the key press that was made (the event's modifiers, not the
//!   item's: AppKit may match ⌘R to a ⇧⌘R item), so the app's key rules still apply.
//! - Hide ([`crate::HIDE`]) is a custom item calling `NSApplication hide:`, because the predefined
//!   one always takes ⌘H.
//! - While a menu is open, changes wait until it closes: replacing or changing an `NSMenu` while
//!   AppKit tracks it can leave the displayed menu stranded.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{CheckMenuItem, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSEventType};

use crate::chord::Chord;
use crate::driver::{Backend, Event, NativeMenu};
use crate::model::{HIDE, Item, MINIMIZE, Menu, MenuBar, MenuRole, Node, Standard, structure_key, window_key};

/// Install the native menu bar for `bar` (a [`crate::mac_layout`] result) and return the driver
/// for it. Call it once, on the main thread, in eframe's creator closure, so winit's default menu
/// doesn't stay up. `None` when AppKit refused the menus: the app keeps its in-window menus.
///
/// The menu takes over muda's event handler, which is one per process: an app using this does
/// not also call `craft_quit::install_app_menu`.
pub fn install(ctx: &egui::Context, bar: &MenuBar) -> Option<NativeMenu> {
    MainThreadMarker::new()?;
    // Never crash for a menu.
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut menu = MacMenu::new(ctx);
        menu.build(bar);
        menu
    }));
    built.ok().map(|menu| NativeMenu::new(Box::new(menu)))
}

enum Handle {
    Plain(MenuItem),
    Check(CheckMenuItem),
}

/// One native item.
struct Entry {
    handle: Handle,
    /// The command id (several entries can share one).
    command: String,
    chord: Option<Chord>,
    label: String,
    enabled: bool,
}

/// How an item was chosen, read from AppKit's current event while it calls the handler.
struct Chosen {
    id: String,
    /// A key press (not a mouse click), and the modifiers held: ⌘, ⌃, ⌥, ⇧.
    key: bool,
    mods: [bool; 4],
}

struct MacMenu {
    ctx: egui::Context,
    menu: Option<muda::Menu>,
    /// By muda id.
    entries: HashMap<String, Entry>,
    rx: Receiver<Chosen>,
    structure: u64,
    /// The Window menu, the muda ids of its items, and its [`window_key`].
    window: Option<(Submenu, Vec<String>, u64)>,
    /// The latest menus, waiting for the open menu to close.
    pending: Option<MenuBar>,
}

impl MacMenu {
    fn new(ctx: &egui::Context) -> MacMenu {
        let (tx, rx) = channel();
        let repaint = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            // AppKit calls this on the main thread, from the event that chose the item: a key
            // equivalent is a key-down, a click a mouse-up (or ↩ inside the open menu).
            let event = MainThreadMarker::new().and_then(|mtm| NSApplication::sharedApplication(mtm).currentEvent());
            let key = event.as_ref().is_some_and(|ev| ev.r#type() == NSEventType::KeyDown);
            let flags = event.as_ref().map_or(NSEventModifierFlags::empty(), |ev| ev.modifierFlags());
            let mods = [NSEventModifierFlags::Command, NSEventModifierFlags::Control, NSEventModifierFlags::Option, NSEventModifierFlags::Shift]
                .map(|m| flags.contains(m));
            // The receiver goes away only with the app.
            let _ = tx.send(Chosen { id: e.id.0, key, mods });
            // Choosing an item is not window input, so nothing else wakes egui: without this the
            // item runs only with the next input event, 8 to 90 seconds later in measurements.
            repaint.request_repaint();
        }));
        MacMenu { ctx: ctx.clone(), menu: None, entries: HashMap::new(), rx, structure: 0, window: None, pending: None }
    }

    fn build(&mut self, bar: &MenuBar) {
        if let Some(old) = self.menu.take() {
            old.remove_for_nsapp();
        }
        self.entries.clear();
        self.window = None;
        let menu = muda::Menu::new();
        let mut special = Vec::new();
        for m in &bar.menus {
            let (sub, ids) = self.submenu(m);
            let _ = menu.append(&sub);
            if m.role == MenuRole::Window {
                self.window = Some((sub.clone(), ids, window_key(bar)));
            }
            if matches!(m.role, MenuRole::Window | MenuRole::Help) {
                special.push((m.role, sub));
            }
        }
        menu.init_for_nsapp();
        // Only once the menu is NSApp's: muda looks the submenus up in the installed main menu.
        for (role, sub) in special {
            match role {
                MenuRole::Window => sub.set_as_windows_menu_for_nsapp(),
                _ => sub.set_as_help_menu_for_nsapp(),
            }
        }
        self.menu = Some(menu);
        self.structure = structure_key(bar);
    }

    /// A top-level menu, and the muda ids of the items it added.
    fn submenu(&mut self, m: &Menu) -> (Submenu, Vec<String>) {
        let before: HashSet<String> = self.entries.keys().cloned().collect();
        let sub = Submenu::new(escape(&m.title), true);
        self.append(&sub, &m.children);
        let ids = self.entries.keys().filter(|id| !before.contains(*id)).cloned().collect();
        (sub, ids)
    }

    /// Replace the Window menu (a document opened or closed), leaving the other menus as they are.
    fn rebuild_window(&mut self, bar: &MenuBar) {
        let (Some(menu), Some((old, ids, _))) = (self.menu.clone(), self.window.take()) else { return self.build(bar) };
        let Some((at, spec)) = bar.menus.iter().enumerate().find(|(_, m)| m.role == MenuRole::Window) else { return self.build(bar) };
        if menu.remove(&old).is_err() {
            return self.build(bar);
        }
        for id in ids {
            self.entries.remove(&id);
        }
        let (sub, ids) = self.submenu(spec);
        if menu.insert(&sub, at).is_err() {
            return self.build(bar);
        }
        sub.set_as_windows_menu_for_nsapp();
        self.window = Some((sub, ids, window_key(bar)));
    }

    fn append(&mut self, sub: &Submenu, nodes: &[Node]) {
        for n in nodes {
            match n {
                Node::Separator => {
                    let _ = sub.append(&PredefinedMenuItem::separator());
                }
                Node::Submenu { label, children, .. } => {
                    let s = Submenu::new(escape(label), !children.is_empty());
                    self.append(&s, children);
                    let _ = sub.append(&s);
                }
                Node::Standard { kind, label } => {
                    let text = Some(label.as_str());
                    let p = match kind {
                        Standard::Services => PredefinedMenuItem::services(text),
                        Standard::HideOthers => PredefinedMenuItem::hide_others(text),
                        Standard::ShowAll => PredefinedMenuItem::show_all(text),
                        Standard::Minimize => PredefinedMenuItem::minimize(text),
                        Standard::Zoom => PredefinedMenuItem::maximize(text),
                        Standard::BringAllToFront => PredefinedMenuItem::bring_all_to_front(text),
                        Standard::CloseWindow => PredefinedMenuItem::close_window(text),
                    };
                    let _ = sub.append(&p);
                }
                Node::Item(it) => {
                    let id = unique_id(&it.id, |id| self.entries.contains_key(id));
                    let chord = it.shortcut.as_deref().and_then(Chord::parse);
                    let accel = chord.as_ref().filter(|c| c.native_ok()).and_then(accelerator);
                    let handle = match it.checked {
                        Some(c) => {
                            let h = CheckMenuItem::with_id(id.clone(), escape(&it.label), it.enabled, c, accel);
                            let _ = sub.append(&h);
                            Handle::Check(h)
                        }
                        None => {
                            let h = MenuItem::with_id(id.clone(), escape(&it.label), it.enabled, accel);
                            let _ = sub.append(&h);
                            Handle::Plain(h)
                        }
                    };
                    self.entries.insert(id, Entry { handle, command: it.id.clone(), chord, label: it.label.clone(), enabled: it.enabled });
                }
            }
        }
    }

    fn update(&mut self, bar: &MenuBar) {
        if self.menu.is_none() || structure_key(bar) != self.structure {
            self.build(bar);
            return;
        }
        if self.window.as_ref().map(|w| w.2) != Some(window_key(bar)) {
            self.rebuild_window(bar);
        }
        // Items by muda id: the n-th item of a command, depth first, is its n-th native item.
        let mut items: HashMap<String, &Item> = HashMap::new();
        for it in bar.items() {
            let id = unique_id(&it.id, |id| items.contains_key(id));
            items.insert(id, it);
        }
        for (id, e) in &mut self.entries {
            let Some(it) = items.get(id) else { continue };
            if e.label != it.label {
                let text = escape(&it.label);
                match &e.handle {
                    Handle::Plain(h) => h.set_text(text),
                    Handle::Check(h) => h.set_text(text),
                }
                e.label = it.label.clone();
            }
            if e.enabled != it.enabled {
                match &e.handle {
                    Handle::Plain(h) => h.set_enabled(it.enabled),
                    Handle::Check(h) => h.set_enabled(it.enabled),
                }
                e.enabled = it.enabled;
            }
            // Compare with AppKit's own state: muda toggles a check item when it's clicked.
            if let (Handle::Check(h), Some(c)) = (&e.handle, it.checked)
                && h.is_checked() != c
            {
                h.set_checked(c);
            }
        }
    }
}

impl Backend for MacMenu {
    fn sync(&mut self, bar: &MenuBar) {
        if self.tracking() {
            // The driver counts this sync as done and may not send these menus again.
            self.pending = Some(bar.clone());
            // Nothing else may wake egui when the menu closes (Escape goes to the menu).
            self.ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }
        self.pending = None;
        self.update(bar);
    }

    fn drain(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(chosen) = self.rx.try_recv() {
            // With asynchronous menu presentation the menu can stay on screen after its item ran.
            self.cancel_tracking();
            // Ids that aren't ours are AppKit's own items (Services, Zoom, …), already handled.
            let Some(e) = self.entries.get(&chosen.id) else { continue };
            if e.command == HIDE {
                if let Some(mtm) = MainThreadMarker::new() {
                    NSApplication::sharedApplication(mtm).hide(None);
                }
                continue;
            }
            // A key-down chose it: its key equivalent, unless it was ↩ or Space inside the open
            // menu (no ⌘ or ⌃, and the item's shortcut needs one). The key is the item's (AppKit
            // matched on it); the modifiers are the ones held. Minimize and craft-quit's Quit are
            // run by the driver, not by the app's key handling, so they stay clicks.
            let [cmd, ctrl, alt, shift] = chosen.mods;
            let runs_here = e.command == MINIMIZE || e.command == craft_quit::QUIT_MENU_ID;
            let key = e.chord.as_ref().filter(|c| chosen.key && !runs_here && (cmd || ctrl || !(c.cmd || c.ctrl))).map(|c| Chord {
                cmd,
                ctrl,
                alt,
                shift,
                key: c.key.clone(),
            });
            out.push(match key {
                Some(chord) => Event::Key(chord),
                None => Event::Click(e.command.clone()),
            });
        }
        // After the events: they were chosen from the menus as they were.
        if !self.tracking()
            && let Some(bar) = self.pending.take()
        {
            self.update(&bar);
        }
        out
    }

    /// A menu bar menu is open while one of its titles is highlighted.
    fn tracking(&self) -> bool {
        MainThreadMarker::new().and_then(|mtm| NSApplication::sharedApplication(mtm).mainMenu()).is_some_and(|m| m.highlightedItem().is_some())
    }

    fn cancel_tracking(&mut self) {
        if let Some(menu) = MainThreadMarker::new().and_then(|mtm| NSApplication::sharedApplication(mtm).mainMenu()) {
            menu.cancelTracking();
        }
    }
}

/// `id`, or `id#2`, `id#3`, … when `taken`.
fn unique_id(id: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut unique = id.to_string();
    let mut n = 2;
    while taken(&unique) {
        unique = format!("{id}#{n}");
        n += 1;
    }
    unique
}

/// `&` marks a mnemonic in muda labels.
fn escape(s: &str) -> String {
    s.replace('&', "&&")
}

/// A key equivalent for AppKit. `Cmd` is ⌘ (`META`) and `Ctrl` is ⌃: never folded together.
fn accelerator(c: &Chord) -> Option<Accelerator> {
    let mut mods = Modifiers::empty();
    for (on, m) in [(c.cmd, Modifiers::META), (c.ctrl, Modifiers::CONTROL), (c.alt, Modifiers::ALT), (c.shift, Modifiers::SHIFT)] {
        if on {
            mods |= m;
        }
    }
    const LETTERS: [Code; 26] = [
        Code::KeyA,
        Code::KeyB,
        Code::KeyC,
        Code::KeyD,
        Code::KeyE,
        Code::KeyF,
        Code::KeyG,
        Code::KeyH,
        Code::KeyI,
        Code::KeyJ,
        Code::KeyK,
        Code::KeyL,
        Code::KeyM,
        Code::KeyN,
        Code::KeyO,
        Code::KeyP,
        Code::KeyQ,
        Code::KeyR,
        Code::KeyS,
        Code::KeyT,
        Code::KeyU,
        Code::KeyV,
        Code::KeyW,
        Code::KeyX,
        Code::KeyY,
        Code::KeyZ,
    ];
    const DIGITS: [Code; 10] =
        [Code::Digit0, Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4, Code::Digit5, Code::Digit6, Code::Digit7, Code::Digit8, Code::Digit9];
    const F: [Code; 24] = [
        Code::F1,
        Code::F2,
        Code::F3,
        Code::F4,
        Code::F5,
        Code::F6,
        Code::F7,
        Code::F8,
        Code::F9,
        Code::F10,
        Code::F11,
        Code::F12,
        Code::F13,
        Code::F14,
        Code::F15,
        Code::F16,
        Code::F17,
        Code::F18,
        Code::F19,
        Code::F20,
        Code::F21,
        Code::F22,
        Code::F23,
        Code::F24,
    ];
    let k = c.key.as_str();
    let mut chars = k.chars();
    let code = match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_ascii_alphabetic() => *LETTERS.get((ch.to_ascii_uppercase() as usize).checked_sub('A' as usize)?)?,
        (Some(ch), None) if ch.is_ascii_digit() => *DIGITS.get((ch as usize).checked_sub('0' as usize)?)?,
        _ if c.is_function_key() => *F.get(k.get(1..)?.parse::<usize>().ok()?.checked_sub(1)?)?,
        _ => match k {
            "[" => Code::BracketLeft,
            "]" => Code::BracketRight,
            "\\" => Code::Backslash,
            "/" => Code::Slash,
            "=" => Code::Equal,
            "-" => Code::Minus,
            "'" => Code::Quote,
            ";" => Code::Semicolon,
            "," => Code::Comma,
            "." => Code::Period,
            "`" => Code::Backquote,
            "Delete" | "Backspace" => Code::Backspace,
            "Enter" if !mods.is_empty() => Code::Enter,
            "Tab" if !mods.is_empty() => Code::Tab,
            "Space" if !mods.is_empty() => Code::Space,
            _ => return None,
        },
    };
    Some(Accelerator::new(mods, code))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// The shortcuts the layout writes itself, and common app ones, become key equivalents.
    #[test]
    fn shortcuts_map_to_key_equivalents() {
        for s in
            ["Cmd+Q", "Cmd+H", "Ctrl+Cmd+H", "Ctrl+Cmd+M", "Cmd+,", "Cmd+Shift+Z", "Cmd+1", "Cmd+[", "Cmd+=", "Cmd+-", "Cmd+Delete", "F7", "Alt+F12"]
        {
            let c = Chord::parse(s).unwrap();
            assert!(c.native_ok() && accelerator(&c).is_some(), "{s} has no key equivalent");
        }
    }

    #[test]
    fn control_and_command_stay_distinct() {
        let a = accelerator(&Chord::parse("Cmd+M").unwrap()).unwrap();
        let b = accelerator(&Chord::parse("Ctrl+Cmd+M").unwrap()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn a_command_shown_twice_gets_a_second_id() {
        let mut taken = vec!["edit.keys".to_string()];
        let second = unique_id("edit.keys", |id| taken.iter().any(|t| t == id));
        taken.push(second.clone());
        assert_eq!((second.as_str(), unique_id("edit.keys", |id| taken.iter().any(|t| t == id)).as_str()), ("edit.keys#2", "edit.keys#3"));
    }
}
