//! The experiment's behaviour (check) scenarios, bench scenarios and pixel states, as steps for the
//! test kit's [`Scenario`]: one step per frame of each gesture.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use craft_ui_platform_headless::Input;
use craft_ui_rt::{Key, Mods, Pos2};
use craft_ui_test_kit::rt::{Scenario, Step};

use crate::fixture::{PANEL_HEIGHT, PANEL_WIDTH};
use crate::gesture::{Gesture, Resolver, Target};

/// The behaviour scenarios run on this many layers (a short document, so the empty strip exists).
pub const CHECK_LAYERS: usize = 12;
/// The pixel states run on this many layers.
pub const DOC_LAYERS: usize = 20;
/// Frames run after a state's gestures before its shot: 1.33 s at 60 frames a second, past every
/// fade and in the middle of the caret's second shown half period.
pub const SETTLE_FRAMES: usize = 80;

/// The steps of one gesture. The first resolves the targets from the access tree as it stands and
/// keeps the frames after it for the rest, so a gesture aims at the layout it started from.
pub fn gesture_steps(label: &str, gesture: &Gesture) -> Vec<Step> {
    let frames = gesture.frames();
    let pending: Rc<RefCell<VecDeque<Vec<Input>>>> = Rc::default();
    let g = gesture.clone();
    let queue = pending.clone();
    let mut steps = vec![Step::resolved(label, move |snapshot| {
        let Some(window) = snapshot.windows.first() else { return Vec::new() };
        let mut all: VecDeque<Vec<Input>> = g.expand(&Resolver::new(window)).into();
        let first = all.pop_front().unwrap_or_default();
        *queue.borrow_mut() = all;
        first.into_iter().map(|i| (None, i)).collect()
    })];
    for _ in 1..frames {
        let queue = pending.clone();
        steps.push(Step::resolved(label, move |_| queue.borrow_mut().pop_front().unwrap_or_default().into_iter().map(|i| (None, i)).collect()));
    }
    steps
}

/// A scenario of gestures, and for each gesture the index of its last frame's step.
pub struct GestureScenario {
    pub scenario: Scenario,
    pub labels: Vec<String>,
    pub ends: Vec<usize>,
}

/// `settle` adds a frame without input after each gesture, which the check observes.
pub fn scenario(name: &str, gestures: &[(String, Gesture)], settle: bool) -> GestureScenario {
    let mut steps = Vec::new();
    let mut ends = Vec::new();
    for (label, gesture) in gestures {
        steps.extend(gesture_steps(label, gesture));
        if settle {
            steps.push(Step::idle(label));
        }
        ends.push(steps.len() - 1);
    }
    GestureScenario { scenario: Scenario::new(name, steps), labels: gestures.iter().map(|(l, _)| l.clone()).collect(), ends }
}

fn row(n: u32) -> Target {
    Target::Row(format!("Layer {n}"))
}
fn click(t: Target) -> Gesture {
    Gesture::Click { target: t, mods: Mods::NONE }
}
fn k(key: Key) -> Gesture {
    Gesture::Key { key, mods: Mods::NONE }
}
fn km(key: Key, mods: Mods) -> Gesture {
    Gesture::Key { key, mods }
}
fn s(label: &str, g: Gesture) -> (String, Gesture) {
    (label.into(), g)
}

const SHIFT: Mods = Mods::SHIFT;
const COMMAND: Mods = Mods::CMD;

/// Behaviour scenarios of `runner --check`, by name.
pub fn check_scenarios() -> Vec<(&'static str, Vec<(String, Gesture)>)> {
    vec![
        (
            "hover-toggles",
            vec![
                s("hover row 3", Gesture::Hover(row(3))),
                s("hover eye 3", Gesture::Hover(Target::Eye("Layer 3".into()))),
                s("click eye 3", click(Target::Eye("Layer 3".into()))),
                s("click eye 3 again", click(Target::Eye("Layer 3".into()))),
                s("click lock 4", click(Target::Lock("Layer 4".into()))),
                s("click lock 4 again", click(Target::Lock("Layer 4".into()))),
            ],
        ),
        (
            "select",
            vec![
                s("click row 2", click(row(2))),
                s("shift-click row 5", Gesture::Click { target: row(5), mods: SHIFT }),
                s("cmd-click row 3", Gesture::Click { target: row(3), mods: COMMAND }),
                s("cmd-click row 8", Gesture::Click { target: row(8), mods: COMMAND }),
                s("click empty space", click(Target::EmptyArea)),
            ],
        ),
        (
            "keyboard",
            vec![
                s("tab into list", k(Key::Tab)),
                s("down", k(Key::Down)),
                s("down", k(Key::Down)),
                s("shift-down", km(Key::Down, SHIFT)),
                s("shift-up", km(Key::Up, SHIFT)),
                s("end", k(Key::End)),
                s("home", k(Key::Home)),
                s("page down", k(Key::PageDown)),
                s("page up", k(Key::PageUp)),
                s("up", k(Key::Up)),
                s("cmd-] move up", km(Key::CloseBracket, COMMAND)),
                s("cmd-[ move down", km(Key::OpenBracket, COMMAND)),
                s("delete", k(Key::Delete)),
            ],
        ),
        (
            "rename",
            vec![
                s("double-click row 4", Gesture::DoubleClick(row(4))),
                s("type", Gesture::Text("Sky".into())),
                s("enter commits", k(Key::Enter)),
                s("click row 6", click(row(6))),
                s("f2", k(Key::F2)),
                s("type", Gesture::Text("zz".into())),
                s("escape cancels", k(Key::Escape)),
                s("f2 again", k(Key::F2)),
                s("ime", Gesture::Ime { preedit: "にほ".into(), commit: "日本".into() }),
                s("enter commits", k(Key::Enter)),
            ],
        ),
        (
            "drag",
            vec![
                s("drag row 2 after row 6", Gesture::Drag { from: row(2), to: row(6), after: true, alt: false, steps: 12 }),
                s("drag row 7 before row 1", Gesture::Drag { from: row(7), to: row(1), after: false, alt: false, steps: 12 }),
                s("alt-drag row 4 after row 9", Gesture::Drag { from: row(4), to: row(9), after: true, alt: true, steps: 12 }),
            ],
        ),
        (
            "scroll-footer",
            vec![
                s("wheel down", Gesture::Wheel { dy: 120.0 }),
                s("wheel down", Gesture::Wheel { dy: 120.0 }),
                s("wheel up", Gesture::Wheel { dy: -60.0 }),
                s("new layer", click(Target::FooterNew)),
                s("new layer", click(Target::FooterNew)),
                s("delete layer", click(Target::FooterDelete)),
            ],
        ),
    ]
}

/// One bench scenario: the gestures of one round (the rig is made again between rounds).
pub struct BenchScenario {
    pub name: &'static str,
    pub gestures: fn(layers: usize) -> Vec<Gesture>,
}

pub fn bench_scenarios() -> Vec<BenchScenario> {
    vec![
        BenchScenario { name: "idle", gestures: |_| vec![Gesture::Idle] },
        BenchScenario {
            name: "hover",
            gestures: |_| {
                let (w, h) = (PANEL_WIDTH as usize, PANEL_HEIGHT as usize);
                (0..60).map(|i| Gesture::HoverAt(Pos2::new(((i * 11) % w) as f32, ((i * 17) % h) as f32))).collect()
            },
        },
        BenchScenario { name: "scroll", gestures: |_| (0..20).map(|i| Gesture::Wheel { dy: if i < 10 { 48.0 } else { -48.0 } }).collect() },
        BenchScenario {
            name: "select",
            gestures: |_| vec![click(row(3)), Gesture::Click { target: row(8), mods: SHIFT }, Gesture::Click { target: row(5), mods: COMMAND }],
        },
        BenchScenario {
            name: "rename",
            gestures: |_| {
                let mut g = vec![Gesture::DoubleClick(row(4))];
                g.extend("rename test".chars().map(|c| Gesture::Text(c.to_string())));
                g.push(k(Key::Enter));
                g
            },
        },
        BenchScenario { name: "drag", gestures: |_| vec![Gesture::Drag { from: row(3), to: row(11), after: true, alt: false, steps: 30 }] },
        BenchScenario { name: "add", gestures: |_| (0..20).map(|_| click(Target::FooterNew)).collect() },
        BenchScenario {
            name: "touch",
            gestures: |_| {
                vec![
                    Gesture::TouchTap(row(3)),
                    Gesture::TouchLongPressDrag { from: row(3), to: row(8), steps: 20 },
                    Gesture::TouchScroll { dy: -200.0, steps: 20 },
                ]
            },
        },
    ]
}

/// The fixed pixel states, each from a fresh document: gestures before the settle frames.
pub fn states() -> Vec<(&'static str, Vec<Gesture>)> {
    let away = Gesture::HoverAt(Pos2::new(-50.0, -50.0));
    vec![
        ("rest", vec![]),
        ("hover", vec![Gesture::Hover(row(3))]),
        ("selected", vec![click(row(2)), Gesture::Click { target: row(4), mods: SHIFT }, away.clone()]),
        ("rename", vec![click(row(3)), k(Key::F2), away]),
    ]
}

/// A bench scenario's steps for `layers` layers: its gestures in order.
pub fn bench_steps(sc: &BenchScenario, layers: usize) -> Scenario {
    if sc.name == "idle" {
        return Scenario::new("idle", Vec::new());
    }
    let gestures: Vec<(String, Gesture)> = (sc.gestures)(layers).into_iter().map(|g| (sc.name.to_owned(), g)).collect();
    scenario(sc.name, &gestures, false).scenario
}
