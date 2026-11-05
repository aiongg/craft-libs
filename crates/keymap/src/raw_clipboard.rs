//! Clipboard keys egui's integrations drop or never send, read from a frame's raw input before
//! egui sees it ([`Dispatcher::raw_input`](crate::egui::Dispatcher::raw_input)).

use ::egui::{Context, Event, Id, Key, Modifiers, RawInput};

fn press(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

/// Adds the ⌘V press egui-winit swallows (egui issue #6329). With an image on the clipboard it
/// sends only the V release, and ⌘ is often let go first, so a V release whose press never
/// arrived, after ⌘ was held and before any other key, is that swallowed ⌘V. egui 0.37 also sends
/// `PasteImage` when the clipboard holds only an image. Both become the ⌘V press. A V pressed
/// and released on its own (a tool key) is no paste. The state between frames lives in `ctx`.
pub(crate) fn swallowed_paste(ctx: &Context, raw: &mut RawInput) {
    let seen = Id::new("craft-keymap-paste-key-seen");
    // A V press without ⌘ that reached us: its release is not a paste.
    let plain_v = Id::new("craft-keymap-plain-v-down");
    // ⌘ was held since the last key press without it: a swallowed ⌘V can be under way.
    let command_lately = Id::new("craft-keymap-paste-command-lately");
    // The modifiers held at each event: last frame's, updated as the events report changes.
    let mut mods = ctx.input(|i| i.modifiers);
    if mods.command {
        ctx.data_mut(|d| d.insert_temp(command_lately, true));
    }
    let mark_paste = |ctx: &Context| {
        ctx.data_mut(|d| {
            d.insert_temp(seen, true);
            d.remove_temp::<bool>(plain_v);
        });
    };
    let mut out = Vec::with_capacity(raw.events.len());
    for e in raw.events.drain(..) {
        if let Event::ModifiersChanged(m) | Event::Key { modifiers: m, .. } = &e {
            mods = *m;
            if mods.command {
                ctx.data_mut(|d| d.insert_temp(command_lately, true));
            }
        }
        match &e {
            Event::PasteImage(_) => {
                mark_paste(ctx);
                out.push(press(Key::V, if mods.command { mods } else { Modifiers::COMMAND }));
                continue;
            }
            Event::Paste(_) => mark_paste(ctx),
            Event::Key { key: Key::V, pressed: true, modifiers, .. } if modifiers.command => mark_paste(ctx),
            Event::Key { key, pressed: true, modifiers, .. } if !modifiers.command => {
                ctx.data_mut(|d| {
                    d.remove_temp::<bool>(command_lately);
                    if *key == Key::V {
                        d.insert_temp(plain_v, true);
                    }
                });
            }
            // A paste key's release ends that press, with or without ⌘ still held (else a stale
            // mark would swallow the next image paste); without a press it is the paste itself.
            Event::Key { key: Key::V, pressed: false, modifiers, .. } => {
                let had_press = ctx.data_mut(|d| d.remove_temp::<bool>(seen)).unwrap_or(false);
                let had_plain_press = ctx.data_mut(|d| d.remove_temp::<bool>(plain_v)).unwrap_or(false);
                // Taken: one swallowed ⌘V is one paste.
                let lately = ctx.data_mut(|d| d.remove_temp::<bool>(command_lately)).unwrap_or(false);
                // The web always forwards the key-down, so there a lone release is no paste.
                let held = if modifiers.command {
                    Some(*modifiers)
                } else {
                    (lately && !had_plain_press && !cfg!(target_arch = "wasm32")).then_some(Modifiers::COMMAND)
                };
                if let Some(held) = held
                    && !had_press
                {
                    out.push(press(Key::V, held));
                }
            }
            _ => {}
        }
        out.push(e);
    }
    raw.events = out;
}

/// Turns Ctrl+Insert, Shift+Insert and Shift+Delete (Copy, Paste and Cut on Linux and Windows)
/// into what the chords they stand for would be. egui-winit sends the clipboard events for them
/// only on Windows, and the web browser pastes natively, so the presses reach egui as plain keys
/// only on a native Linux build (and Windows builds that turned `clipboard_shortcuts` off).
///
/// In a text field the press becomes egui's `Copy` or `Cut` event, or, for paste, a
/// `ViewportCommand::RequestPaste` (the window answers with a `Paste` event next frame) and the
/// press goes. Outside text fields it becomes the Cmd+C, Cmd+V or Cmd+X press, so the Copy,
/// Paste and Cut bindings run.
pub(crate) fn insert_chords(ctx: &Context, raw: &mut RawInput, typing: bool) {
    use ::egui::os::OperatingSystem as Os;
    if cfg!(target_arch = "wasm32") || !matches!(ctx.os(), Os::Nix | Os::Windows) {
        return;
    }
    let mut out = Vec::with_capacity(raw.events.len());
    for e in raw.events.drain(..) {
        let Event::Key { key, pressed: true, repeat: false, modifiers: m, .. } = &e else {
            out.push(e);
            continue;
        };
        let ctrl_only = m.ctrl && !m.shift && !m.alt && !m.mac_cmd;
        let shift_only = m.shift && !m.ctrl && !m.alt && !m.command && !m.mac_cmd;
        match key {
            Key::Insert if ctrl_only => out.push(if typing { Event::Copy } else { press(Key::C, Modifiers::COMMAND) }),
            Key::Delete if shift_only => out.push(if typing { Event::Cut } else { press(Key::X, Modifiers::COMMAND) }),
            Key::Insert if shift_only && typing => ctx.send_viewport_cmd(::egui::ViewportCommand::RequestPaste),
            Key::Insert if shift_only => out.push(press(Key::V, Modifiers::COMMAND)),
            _ => out.push(e),
        }
    }
    raw.events = out;
}
