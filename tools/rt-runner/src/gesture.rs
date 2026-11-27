//! Gestures over named targets (the experiment harness's vocabulary), expanded into frames of
//! injected input. Targets resolve from the access tree of the first window.

use craft_ui_platform_headless::{Contact, Input};
use craft_ui_rt::{Key, Mods, PointerPhase, Pos2, Rect, Vec2};
use craft_ui_test_kit::rt::WindowSnapshot;

use crate::observe::role_name;

const MOUSE: Contact = Contact::MOUSE;
const TOUCH: Contact = Contact::touch(2);

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Row(String),
    Eye(String),
    Lock(String),
    FooterNew,
    FooterDelete,
    List,
    RenameField,
    /// The strip of the list below the last row; absent when the rows fill the list.
    EmptyArea,
}

fn rect_of(b: [f32; 4]) -> Rect {
    Rect::from_min_max(Pos2::new(b[0], b[1]), Pos2::new(b[2], b[3]))
}

/// Finds targets in a window's access tree: the list (role "list"), rows ("list_item" named for
/// the layer), toggles ("toggle": "Visibility {row}" and "Lock {row}"), footer buttons and the
/// rename field.
pub struct Resolver<'a> {
    window: &'a WindowSnapshot,
}

impl<'a> Resolver<'a> {
    pub fn new(window: &'a WindowSnapshot) -> Self {
        Resolver { window }
    }

    fn find(&self, role: &str, name: &str) -> Option<Rect> {
        self.window.access.iter().find(|n| role_name(&n.role) == role && n.name == name).map(|n| rect_of(n.bounds))
    }

    fn first(&self, role: &str) -> Option<&craft_ui_test_kit::rt::AccessNode> {
        self.window.access.iter().find(|n| role_name(&n.role) == role)
    }

    pub fn rect(&self, t: &Target) -> Option<Rect> {
        match t {
            Target::Row(n) => self.find("list_item", n),
            Target::Eye(n) => self.find("toggle", &format!("Visibility {n}")),
            Target::Lock(n) => self.find("toggle", &format!("Lock {n}")),
            Target::FooterNew => self.find("button", "New Layer"),
            Target::FooterDelete => self.find("button", "Delete Layer"),
            Target::List => self.first("list").map(|n| rect_of(n.bounds)),
            Target::RenameField => self.first("text_input").map(|n| rect_of(n.bounds)),
            Target::EmptyArea => {
                let list = self.rect(&Target::List)?;
                let last = self.window.access.iter().filter(|n| role_name(&n.role) == "list_item").map(|n| n.bounds[3]).fold(f32::MIN, f32::max);
                (list.max.y - last > 8.0)
                    .then(|| Rect::from_min_size(Pos2::new(list.min.x, last + 4.0), Vec2::new(list.width(), list.max.y - last - 4.0)))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Gesture {
    /// One frame without input.
    Idle,
    Hover(Target),
    HoverAt(Pos2),
    Click {
        target: Target,
        mods: Mods,
    },
    DoubleClick(Target),
    /// One wheel notch over the list; positive `dy` scrolls content down.
    Wheel {
        dy: f32,
    },
    Key {
        key: Key,
        mods: Mods,
    },
    Text(String),
    Ime {
        preedit: String,
        commit: String,
    },
    /// Press on `from`, move over `steps` frames to the upper (`after` false) or lower quarter of
    /// `to`, release. Alt held for a copy.
    Drag {
        from: Target,
        to: Target,
        after: bool,
        alt: bool,
        steps: usize,
    },
    TouchTap(Target),
    /// Hold on `from` past the long-press delay, then drag to `to`.
    TouchLongPressDrag {
        from: Target,
        to: Target,
        steps: usize,
    },
    /// Touch drag on the list by `dy` points over `steps` frames.
    TouchScroll {
        dy: f32,
        steps: usize,
    },
}

/// Frames of the long-press hold: past the delay (about 0.5 s).
const LONG_PRESS_FRAMES: usize = 36;

impl Gesture {
    /// How many frames [`Gesture::expand`] gives when every target resolves.
    pub fn frames(&self) -> usize {
        match self {
            Gesture::Idle | Gesture::Hover(_) | Gesture::HoverAt(_) | Gesture::Wheel { .. } | Gesture::Key { .. } | Gesture::Text(_) => 1,
            Gesture::Click { .. } | Gesture::Ime { .. } | Gesture::TouchTap(_) => 2,
            Gesture::DoubleClick(_) => 4,
            Gesture::Drag { steps, .. } | Gesture::TouchScroll { steps, .. } => steps + 2,
            Gesture::TouchLongPressDrag { steps, .. } => 1 + LONG_PRESS_FRAMES + steps + 1,
        }
    }
}

fn mv(contact: Contact, pos: Pos2, mods: Mods) -> Input {
    Input::pointer(contact.with_mods(mods), PointerPhase::Move, pos)
}
fn down(contact: Contact, pos: Pos2, mods: Mods) -> Input {
    Input::pointer(contact.with_mods(mods), PointerPhase::Down, pos)
}
fn up(contact: Contact, pos: Pos2, mods: Mods) -> Input {
    Input::pointer(contact.with_mods(mods), PointerPhase::Up, pos)
}
fn lerp(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

impl Gesture {
    /// Frames of input, in order. A target that does not resolve gives no frames.
    pub fn expand(&self, r: &Resolver) -> Vec<Vec<Input>> {
        let c = |t: &Target| r.rect(t).map(|x| x.center());
        match self {
            Gesture::Idle => vec![vec![]],
            Gesture::Hover(t) => c(t).map(|p| vec![vec![mv(MOUSE, p, Mods::NONE)]]).unwrap_or_default(),
            Gesture::HoverAt(p) => vec![vec![mv(MOUSE, *p, Mods::NONE)]],
            Gesture::Click { target, mods } => {
                let Some(p) = c(target) else { return vec![] };
                vec![vec![mv(MOUSE, p, *mods), down(MOUSE, p, *mods)], vec![up(MOUSE, p, *mods)]]
            }
            Gesture::DoubleClick(t) => {
                let Some(p) = c(t) else { return vec![] };
                vec![
                    vec![mv(MOUSE, p, Mods::NONE), down(MOUSE, p, Mods::NONE)],
                    vec![up(MOUSE, p, Mods::NONE)],
                    vec![down(MOUSE, p, Mods::NONE)],
                    vec![up(MOUSE, p, Mods::NONE)],
                ]
            }
            Gesture::Wheel { dy } => {
                let Some(p) = c(&Target::List) else { return vec![] };
                vec![vec![mv(MOUSE, p, Mods::NONE), Input::Wheel { pos: p, delta: Vec2::new(0.0, -dy), mods: Mods::NONE }]]
            }
            Gesture::Key { key, mods } => {
                vec![vec![Input::Key { key: *key, mods: *mods, down: true }, Input::Key { key: *key, mods: *mods, down: false }]]
            }
            Gesture::Text(s) => vec![vec![Input::Text(s.clone())]],
            Gesture::Ime { preedit, commit } => vec![
                vec![Input::ImePreedit { text: preedit.clone(), cursor: Some(preedit.len()..preedit.len()) }],
                vec![Input::ImeCommit(commit.clone())],
            ],
            Gesture::Drag { from, to, after, alt, steps } => {
                let (Some(a), Some(tr)) = (c(from), r.rect(to)) else { return vec![] };
                let drop = Pos2::new(tr.center().x, tr.min.y + tr.height() * if *after { 0.75 } else { 0.25 });
                let mods = if *alt { Mods { alt: true, ..Mods::NONE } } else { Mods::NONE };
                let mut frames = vec![vec![mv(MOUSE, a, mods), down(MOUSE, a, mods)]];
                for i in 1..=*steps {
                    frames.push(vec![mv(MOUSE, lerp(a, drop, i as f32 / *steps as f32), mods)]);
                }
                frames.push(vec![up(MOUSE, drop, mods)]);
                frames
            }
            Gesture::TouchTap(t) => {
                let Some(p) = c(t) else { return vec![] };
                vec![vec![down(TOUCH, p, Mods::NONE)], vec![up(TOUCH, p, Mods::NONE)]]
            }
            Gesture::TouchLongPressDrag { from, to, steps } => {
                let (Some(a), Some(b)) = (c(from), c(to)) else { return vec![] };
                let mut frames = vec![vec![down(TOUCH, a, Mods::NONE)]];
                frames.extend((0..LONG_PRESS_FRAMES).map(|_| vec![]));
                for i in 1..=*steps {
                    frames.push(vec![mv(TOUCH, lerp(a, b, i as f32 / *steps as f32), Mods::NONE)]);
                }
                frames.push(vec![up(TOUCH, b, Mods::NONE)]);
                frames
            }
            Gesture::TouchScroll { dy, steps } => {
                let Some(l) = r.rect(&Target::List) else { return vec![] };
                let a = l.center();
                let b = Pos2::new(a.x, a.y + dy);
                let mut frames = vec![vec![down(TOUCH, a, Mods::NONE)]];
                for i in 1..=*steps {
                    frames.push(vec![mv(TOUCH, lerp(a, b, i as f32 / *steps as f32), Mods::NONE)]);
                }
                frames.push(vec![up(TOUCH, b, Mods::NONE)]);
                frames
            }
        }
    }
}
