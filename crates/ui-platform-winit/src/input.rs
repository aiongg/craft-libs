//! winit window events to `WindowEvent`s of one window. Positions and deltas are logical points.

use craft_ui_rt::{Key, Mods, PointerButton, PointerEvent, PointerKind, PointerPhase, Pos2, SizeU, SystemTheme, Vec2, WindowEvent, WindowId};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent as W};
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
use winit::window::Theme;

use crate::shared::{Cell, push};

/// Logical points one wheel notch scrolls.
const LINE_POINTS: f64 = 40.0;

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
    let pointer = |pid, kind, phase, button, pos| WindowEvent::Pointer(PointerEvent { id: pid, kind, phase, button, pos, mods });
    let put = |ev: WindowEvent| push(shared, id, ev);
    match event {
        W::Resized(size) => {
            let px = SizeU::new(size.width, size.height);
            if let Some(e) = shared.borrow_mut().entry_mut(id) {
                e.size_px = px;
            }
            put(WindowEvent::Resize(px.to_logical(scale as f32)));
        }
        W::Moved(pos) => {
            let p = pos.to_logical::<f64>(scale);
            put(WindowEvent::Moved(Pos2::new(p.x as f32, p.y as f32)));
            shared.borrow_mut().monitors_stale = true;
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
        W::MouseWheel { delta, .. } => {
            let delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => Vec2::new((f64::from(x) * LINE_POINTS) as f32, (f64::from(y) * LINE_POINTS) as f32),
                MouseScrollDelta::PixelDelta(p) => Vec2::new((p.x / scale) as f32, (p.y / scale) as f32),
            };
            put(WindowEvent::Wheel { pos: cursor, delta, mods });
        }
        W::PinchGesture { delta, .. } => put(WindowEvent::Pinch { pos: cursor, delta: delta as f32 }),
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
            Ime::Enabled | Ime::Disabled => {}
        },
        W::HoveredFile(path) => put(WindowEvent::FileHovered(path)),
        W::HoveredFileCancelled => put(WindowEvent::FileHoverCancelled),
        W::DroppedFile(path) => put(WindowEvent::FileDropped(path)),
        _ => {}
    }
}
