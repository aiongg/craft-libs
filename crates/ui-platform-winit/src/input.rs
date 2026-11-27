//! winit window events to `WindowEvent`s of one window. Positions and deltas are logical points.

use craft_ui_rt::{
    FileDrag, Key, Mods, OsFile, PlatformEvent, PointerButton, PointerEvent, PointerKind, PointerPhase, Pos2, SizeU, SystemTheme, Vec2, WheelPhase,
    WheelSource, WheelUnit, WindowEvent, WindowId,
};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent as W};
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
use winit::window::Theme;

use crate::shared::{Cell, push};

/// The mouse has pointer id 0; a touch contact's id is winit's id plus one.
const MOUSE_ID: u64 = 0;

fn mods_of(m: ModifiersState) -> Mods {
    if cfg!(target_os = "macos") {
        Mods { cmd: m.super_key(), ctrl: m.control_key(), alt: m.alt_key(), shift: m.shift_key() }
    } else {
        Mods { cmd: m.control_key(), ctrl: false, alt: m.alt_key(), shift: m.shift_key() }
    }
}

fn button_of(b: MouseButton) -> PointerButton {
    match b {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Extra1,
        MouseButton::Forward => PointerButton::Extra2,
        MouseButton::Other(_) => PointerButton::Primary,
    }
}

pub(crate) fn theme_of(theme: Theme) -> SystemTheme {
    match theme {
        Theme::Light => SystemTheme::Light,
        Theme::Dark => SystemTheme::Dark,
    }
}

/// The suite's key for a logical key; named keys go through their names (`ArrowUp` is `Up`).
fn key_of(key: &WKey) -> Option<Key> {
    match key {
        WKey::Named(NamedKey::ArrowUp) => Key::parse("Up"),
        WKey::Named(NamedKey::ArrowDown) => Key::parse("Down"),
        WKey::Named(NamedKey::ArrowLeft) => Key::parse("Left"),
        WKey::Named(NamedKey::ArrowRight) => Key::parse("Right"),
        WKey::Named(NamedKey::Space) => Key::parse("Space"),
        WKey::Named(named) => Key::parse(&format!("{named:?}")),
        WKey::Character(s) => s.chars().next().and_then(Key::from_char),
        _ => None,
    }
}

pub(crate) fn translate(shared: &Cell, id: WindowId, event: W) {
    let (scale, mods, cursor) = {
        let s = shared.borrow();
        let Some(e) = s.entry(id) else { return };
        (f64::from(e.scale), e.mods, e.cursor)
    };
    let logical = |p: winit::dpi::PhysicalPosition<f64>| Pos2::new((p.x / scale) as f32, (p.y / scale) as f32);
    let pointer = |pid, kind, phase, button, pos| WindowEvent::Pointer(PointerEvent::new(pid, kind, phase, button, pos, mods));
    let put = |ev: WindowEvent| push(shared, id, ev);
    match event {
        W::Resized(size) => {
            let px = SizeU::new(size.width, size.height);
            if let Some(e) = shared.borrow_mut().entry_mut(id) {
                e.size_px = px;
            }
            put(WindowEvent::Resize(px.to_logical(scale as f32)));
        }
        W::Moved(outer) => {
            // The event carries the outer corner; the runtime's `Moved` is the surface's.
            let inner = {
                let mut s = shared.borrow_mut();
                s.monitors_stale = true;
                s.entry(id).and_then(|e| e.window.as_ref()).and_then(|w| w.inner_position().ok())
            };
            let at = inner.unwrap_or(outer);
            put(WindowEvent::Moved(Pos2::new(at.x as f32, at.y as f32)));
        }
        W::ScaleFactorChanged { scale_factor, .. } => {
            if let Some(e) = shared.borrow_mut().entry_mut(id) {
                e.scale = scale_factor as f32;
            }
            shared.borrow_mut().monitors_stale = true;
            put(WindowEvent::Scale(scale_factor as f32));
        }
        W::Focused(focused) => put(WindowEvent::Focus(focused)),
        W::ThemeChanged(theme) => put(WindowEvent::Theme(theme_of(theme))),
        W::ModifiersChanged(m) => {
            if let Some(e) = shared.borrow_mut().entry_mut(id) {
                e.mods = mods_of(m.state());
            }
        }
        W::CursorMoved { position, .. } => {
            let pos = logical(position);
            if let Some(e) = shared.borrow_mut().entry_mut(id) {
                e.cursor = pos;
            }
            put(pointer(MOUSE_ID, PointerKind::Mouse, PointerPhase::Move, PointerButton::Primary, pos));
        }
        W::CursorLeft { .. } => put(pointer(MOUSE_ID, PointerKind::Mouse, PointerPhase::Leave, PointerButton::Primary, cursor)),
        W::MouseInput { state, button, .. } => {
            let phase = match state {
                ElementState::Pressed => PointerPhase::Down,
                ElementState::Released => PointerPhase::Up,
            };
            put(pointer(MOUSE_ID, PointerKind::Mouse, phase, button_of(button), cursor));
        }
        W::MouseWheel { delta, phase, .. } => {
            let (delta, unit) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (Vec2::new(x, y), WheelUnit::Line),
                MouseScrollDelta::PixelDelta(p) => (Vec2::new((p.x / scale) as f32, (p.y / scale) as f32), WheelUnit::Point),
            };
            let phase = match phase {
                TouchPhase::Started => WheelPhase::Start,
                TouchPhase::Moved => WheelPhase::Move,
                TouchPhase::Ended => WheelPhase::End,
                TouchPhase::Cancelled => WheelPhase::Cancel,
            };
            put(WindowEvent::Wheel { pos: cursor, delta, unit, phase, source: WheelSource::Unknown, mods });
        }
        W::PinchGesture { delta, .. } => put(WindowEvent::Pinch { pos: cursor, zoom: (delta as f32).exp(), rotation: 0.0, mods }),
        // winit's degrees turn counterclockwise; the runtime's radians clockwise.
        W::RotationGesture { delta, .. } => put(WindowEvent::Pinch { pos: cursor, zoom: 1.0, rotation: -delta.to_radians(), mods }),
        W::Touch(touch) => {
            let phase = match touch.phase {
                TouchPhase::Started => PointerPhase::Down,
                TouchPhase::Moved => PointerPhase::Move,
                TouchPhase::Ended => PointerPhase::Up,
                TouchPhase::Cancelled => PointerPhase::Cancel,
            };
            put(pointer(touch.id + 1, PointerKind::Touch, phase, PointerButton::Primary, logical(touch.location)));
        }
        W::KeyboardInput { event, .. } => {
            let down = event.state == ElementState::Pressed;
            if let WKey::Named(lock @ (NamedKey::CapsLock | NamedKey::NumLock)) = &event.logical_key
                && !event.repeat
            {
                lock_key(shared, id, *lock, down);
            }
            if let Some(key) = key_of(&event.logical_key) {
                put(WindowEvent::Key { key, mods, down, repeat: event.repeat });
            }
            if down
                && !mods.cmd
                && !mods.ctrl
                && let Some(text) = event.text.as_deref().filter(|t| !t.is_empty() && !t.chars().any(char::is_control))
            {
                put(WindowEvent::Text(text.to_owned()));
            }
        }
        W::Ime(ime) => match ime {
            Ime::Preedit(text, cursor) => put(WindowEvent::ImePreedit { text, cursor: cursor.map(|(a, b)| a..b) }),
            Ime::Commit(text) => put(WindowEvent::ImeCommit(text)),
            Ime::Enabled => put(WindowEvent::ImeEnabled),
            Ime::Disabled => {}
        },
        W::HoveredFile(path) => push_file(shared, id, path, false),
        W::HoveredFileCancelled => put(WindowEvent::Files(FileDrag::Leave)),
        W::DroppedFile(path) => push_file(shared, id, path, true),
        _ => {}
    }
}

/// winit reports no lock state, so it follows the lock keys, from off when the window opens. macOS reports the caps
/// lock key down while the lock is on; the other systems report a press for each toggle.
fn lock_key(shared: &Cell, id: WindowId, key: NamedKey, down: bool) {
    let (caps_lock, num_lock) = {
        let mut s = shared.borrow_mut();
        let Some(e) = s.entry_mut(id) else { return };
        match key {
            NamedKey::CapsLock if cfg!(target_os = "macos") => e.caps_lock = down,
            NamedKey::CapsLock if down => e.caps_lock = !e.caps_lock,
            NamedKey::NumLock if down => e.num_lock = !e.num_lock,
            _ => return,
        }
        (e.caps_lock, e.num_lock)
    };
    push(shared, id, WindowEvent::LockKeys { caps_lock, num_lock });
}

/// winit reports one event per file: the files of one OS drag that arrive before the runtime
/// polls join one `Enter` or `Drop`.
fn push_file(shared: &Cell, id: WindowId, path: std::path::PathBuf, dropped: bool) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let file = OsFile { name, path: Some(path), ..OsFile::default() };
    {
        let mut s = shared.borrow_mut();
        if let Some(PlatformEvent::Window(w, WindowEvent::Files(drag))) = s.events.last_mut()
            && *w == id
        {
            match drag {
                FileDrag::Enter { files, .. } if !dropped => return files.push(file),
                FileDrag::Drop { files, .. } if dropped => return files.push(file),
                _ => {}
            }
        }
    }
    let files = vec![file];
    push(shared, id, WindowEvent::Files(if dropped { FileDrag::Drop { files, pos: None } } else { FileDrag::Enter { files, pos: None } }));
}
