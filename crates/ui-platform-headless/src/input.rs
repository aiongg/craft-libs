//! Input a check injects into a window: the neutral form scenarios are written in, and the
//! shortcuts that build it.

use craft_ui_rt::{
    FileDrag, Key, Margin, Mods, PointerButton, PointerEvent, PointerKind, PointerPhase, Pos2, Vec2, WheelPhase, WheelSource, WheelUnit, WindowEvent,
};
use std::ops::Range;

/// One pointer device: a mouse, a pen or a touch contact, with its button and held modifiers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub id: u64,
    pub kind: PointerKind,
    pub button: PointerButton,
    pub mods: Mods,
}

impl Contact {
    pub const MOUSE: Contact = Contact { id: 1, kind: PointerKind::Mouse, button: PointerButton::Primary, mods: Mods::NONE };
    pub const PEN: Contact = Contact { id: 2, kind: PointerKind::Pen, button: PointerButton::Primary, mods: Mods::NONE };

    /// A finger; each simultaneous contact has its own `id`.
    pub const fn touch(id: u64) -> Contact {
        Contact { id: 100 + id, kind: PointerKind::Touch, button: PointerButton::Primary, mods: Mods::NONE }
    }

    pub const fn with_mods(self, mods: Mods) -> Contact {
        Contact { mods, ..self }
    }

    pub const fn with_button(self, button: PointerButton) -> Contact {
        Contact { button, ..self }
    }
}

/// What a check injects into one window. Lifecycle entries (resize, scale, surface loss, focus)
/// also change what the platform reports about the window, as a real platform's would.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Pointer {
        contact: Contact,
        phase: PointerPhase,
        pos: Pos2,
    },
    /// A wheel step of `delta` logical points.
    Wheel {
        pos: Pos2,
        delta: Vec2,
        mods: Mods,
    },
    /// A touchpad pinch or rotation step.
    Pinch {
        pos: Pos2,
        zoom: f32,
        rotation: f32,
        mods: Mods,
    },
    Key {
        key: Key,
        mods: Mods,
        down: bool,
    },
    Text(String),
    ImePreedit {
        text: String,
        cursor: Option<Range<usize>>,
    },
    ImeCommit(String),
    /// Clipboard content arriving as if the window had asked for it.
    Paste(String),
    /// An OS file drag over the window.
    Files(FileDrag),
    /// The surface's top left on the virtual desktop, in physical pixels.
    Moved(Pos2),
    /// The surface's offset from the window's outer top left, in physical pixels.
    FrameInset(Vec2),
    /// An assistive technology's action on an access node.
    Access {
        action: accesskit::Action,
        target: accesskit::NodeId,
        data: Option<accesskit::ActionData>,
    },
    Focus(bool),
    Resize(Vec2),
    Scale(f32),
    SafeArea(Margin),
    SurfaceLost,
    SurfaceCreated,
    CloseRequested,
    /// An assistive technology connects (again): the next frame sends the whole tree.
    AccessActivated,
}

impl Input {
    pub fn pointer(contact: Contact, phase: PointerPhase, pos: Pos2) -> Input {
        Input::Pointer { contact, phase, pos }
    }

    pub fn mouse_move(pos: Pos2) -> Input {
        Input::pointer(Contact::MOUSE, PointerPhase::Move, pos)
    }

    pub fn mouse_down(pos: Pos2) -> Input {
        Input::pointer(Contact::MOUSE, PointerPhase::Down, pos)
    }

    pub fn mouse_up(pos: Pos2) -> Input {
        Input::pointer(Contact::MOUSE, PointerPhase::Up, pos)
    }

    /// A key press and release.
    pub fn key_press(key: Key, mods: Mods) -> [Input; 2] {
        [Input::Key { key, mods, down: true }, Input::Key { key, mods, down: false }]
    }

    /// An access action without data.
    pub fn action(action: accesskit::Action, target: accesskit::NodeId) -> Input {
        Input::Access { action, target, data: None }
    }

    /// The window event this becomes, for entries that are plain events.
    pub(crate) fn event(&self) -> WindowEvent {
        match self {
            Input::Pointer { contact, phase, pos } => {
                WindowEvent::Pointer(PointerEvent::new(contact.id, contact.kind, *phase, contact.button, *pos, contact.mods))
            }
            Input::Wheel { pos, delta, mods } => WindowEvent::Wheel {
                pos: *pos,
                delta: *delta,
                unit: WheelUnit::Point,
                phase: WheelPhase::Move,
                source: WheelSource::Unknown,
                mods: *mods,
            },
            Input::Pinch { pos, zoom, rotation, mods } => WindowEvent::Pinch { pos: *pos, zoom: *zoom, rotation: *rotation, mods: *mods },
            Input::Key { key, mods, down } => WindowEvent::Key { key: *key, mods: *mods, down: *down, repeat: false },
            Input::Text(t) => WindowEvent::Text(t.clone()),
            Input::ImePreedit { text, cursor } => WindowEvent::ImePreedit { text: text.clone(), cursor: cursor.clone() },
            Input::ImeCommit(t) => WindowEvent::ImeCommit(t.clone()),
            Input::Paste(t) => WindowEvent::Paste(t.clone()),
            Input::Files(drag) => WindowEvent::Files(drag.clone()),
            Input::Moved(p) => WindowEvent::Moved(*p),
            Input::FrameInset(v) => WindowEvent::FrameInset(*v),
            Input::Access { action, target, data } => WindowEvent::Access(accesskit::ActionRequest {
                action: *action,
                target_tree: accesskit::TreeId::ROOT,
                target_node: *target,
                data: data.clone(),
            }),
            Input::Focus(f) => WindowEvent::Focus(*f),
            Input::Resize(s) => WindowEvent::Resize(*s),
            Input::Scale(s) => WindowEvent::Scale(*s),
            Input::SafeArea(m) => WindowEvent::SafeArea(*m),
            Input::SurfaceLost => WindowEvent::SurfaceLost,
            Input::SurfaceCreated => WindowEvent::SurfaceCreated,
            Input::CloseRequested => WindowEvent::CloseRequested,
            Input::AccessActivated => WindowEvent::AccessActivated,
        }
    }
}
