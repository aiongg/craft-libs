//! What a window hides and reveals: the chrome toggles, the Tab reveal and the edge reveal of body-only mode.

use crate::contract::PaneEdge;
use crate::runtime::commands::{ChromeCommand, CommandSet as _, FireSource};
use crate::runtime::egui::viewports;
use crate::runtime::env::{EnvValue, RevealEdge};
use crate::runtime::frame::{ChromeToggle, FrameOutput};
use crate::shell::Slot;
use egui::{Context, Pos2, Rect, ViewportId, pos2};

/// Whether the window's panels are hidden while its tools and bars stay (Shift+Tab). The shell sets it beside its
/// `ShellCx`, and a body that hosts panels reads it with `Env::get`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PanelsHidden(pub bool);

impl PanelsHidden {
    /// The panels are hidden (`true`) or shown.
    pub const fn new(hidden: bool) -> Self {
        PanelsHidden(hidden)
    }
}

impl EnvValue for PanelsHidden {
    fn root() -> Self {
        PanelsHidden(false)
    }
}

/// The chrome toggles of this frame: the key stage's own, and the menu, palette and automation fires of the chrome
/// commands, which reach the app as ordinary fires with no window and are mapped to the focused window. Call it
/// once per frame with the output taken from the frame loop, and pass the result to each shell.
pub fn chrome_toggles(ctx: &Context, output: &FrameOutput) -> Vec<(ViewportId, ChromeToggle)> {
    let mut toggles = output.chrome_toggles.clone();
    let focused = viewports::reported(ctx).into_iter().find(|(_, facts)| facts.focused).map(|(viewport, _)| viewport);
    for fired in &output.fired {
        // A key fire already wrote its toggle in the key stage.
        if fired.source == FireSource::Key {
            continue;
        }
        if let (Some(command), Some(viewport)) = (ChromeCommand::from_ref(&fired.command), focused) {
            toggles.push((viewport, command.toggle()));
        }
    }
    toggles
}

/// The reveal state of one window.
///
/// - `hidden`: the user hid every panel, tool and bar in normal mode (Tab); the window then behaves as in body-only
///   mode, and Tab shows them again.
/// - `panels_hidden`: the panels are hidden and the tools and bars stay (Shift+Tab).
/// - `tab`: in body-only mode, Tab shows the slots built to reveal until the next Tab.
/// - `edges`: in body-only mode the pointer at the top, start or end edge shows the slots of that edge.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Reveal {
    hidden: bool,
    panels_hidden: bool,
    tab: bool,
    edges: [bool; 3],
    shown: Vec<(RevealEdge, Rect)>,
    applied: Option<u64>,
}

/// What the reveal depends on in this pass.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Tracking {
    /// The window shows the body only.
    pub only: bool,
    /// The reveal works at all; off for a presentation.
    pub allow_reveal: bool,
    /// The pointer at an edge reveals (the Auto-Show Hidden Panels preference).
    pub edge_reveal: bool,
}

/// Where the pointer is, and the strips it reveals from.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Aim {
    /// The pointer position in the window.
    pub pointer: Option<Pos2>,
    /// A button is held or a popup is open.
    pub frozen: bool,
    /// The shell's rect.
    pub window: Rect,
    /// The width of the strip along an edge.
    pub zone: f32,
    /// Start and end are mirrored.
    pub rtl: bool,
}

const fn at(edge: RevealEdge) -> usize {
    match edge {
        RevealEdge::Top => 0,
        RevealEdge::Start => 1,
        _ => 2,
    }
}

impl Tracking {
    /// The reveal's inputs for one pass.
    pub const fn new(only: bool, allow_reveal: bool, edge_reveal: bool) -> Self {
        Tracking { only, allow_reveal, edge_reveal }
    }
}

impl Aim {
    /// The pointer and the strips for one pass.
    pub const fn new(pointer: Option<Pos2>, frozen: bool, window: Rect, zone: f32, rtl: bool) -> Self {
        Aim { pointer, frozen, window, zone, rtl }
    }
}

const EDGES: [RevealEdge; 3] = [RevealEdge::Top, RevealEdge::Start, RevealEdge::End];

impl Reveal {
    /// Applies the toggles that name `window`, once per frame (`frame` is the frame number; a discard pass of the
    /// same frame applies nothing). `host_only` says the screen mode shows the body only; `allow_reveal` is off for a
    /// presentation.
    pub fn apply(&mut self, toggles: &[(ViewportId, ChromeToggle)], window: ViewportId, frame: u64, host_only: bool, allow_reveal: bool) {
        if self.applied == Some(frame) {
            return;
        }
        self.applied = Some(frame);
        for (viewport, toggle) in toggles {
            if *viewport == window {
                self.toggle(*toggle, host_only, allow_reveal);
            }
        }
    }

    fn toggle(&mut self, toggle: ChromeToggle, host_only: bool, allow_reveal: bool) {
        match toggle {
            ChromeToggle::Panels if host_only => {
                if allow_reveal {
                    self.tab = !self.tab;
                    if self.tab {
                        self.edges = [false; 3];
                    }
                }
            }
            ChromeToggle::Panels => {
                self.hidden = !(self.hidden || self.panels_hidden);
                self.panels_hidden = false;
                self.end_reveal();
            }
            ChromeToggle::PanelsKeepTools if !host_only && !self.hidden => self.panels_hidden = !self.panels_hidden,
            _ => {}
        }
    }

    /// Ends the Tab and edge reveal and the user's hiding: every screen-mode change does.
    pub fn clear(&mut self) {
        self.hidden = false;
        self.panels_hidden = false;
        self.end_reveal();
    }

    fn end_reveal(&mut self) {
        self.tab = false;
        self.edges = [false; 3];
    }

    /// Whether the window shows the body only: the screen mode says so or the user hid the chrome.
    pub fn only(&self, host_only: bool) -> bool {
        host_only || self.hidden
    }

    /// Whether the panels are hidden and the tools stay.
    pub fn panels_hidden(&self) -> bool {
        self.panels_hidden && !self.hidden
    }

    /// Whether the Tab reveal is on.
    pub fn tab(&self) -> bool {
        self.tab
    }

    /// Whether the edge reveal of `edge` is on.
    pub fn edge(&self, edge: RevealEdge) -> bool {
        self.edges[at(edge)]
    }

    /// Updates the reveal at the start of a pass. A held button or an open popup freezes the edge reveal: it neither
    /// starts nor ends.
    pub fn track(&mut self, mode: Tracking, aim: Aim) {
        if !mode.only || !mode.allow_reveal {
            self.end_reveal();
            return;
        }
        if self.tab || !mode.edge_reveal {
            self.edges = [false; 3];
            return;
        }
        if aim.frozen {
            return;
        }
        let pointer = aim.pointer.filter(|point| aim.window.contains(*point));
        for edge in EDGES {
            let in_zone = pointer.is_some_and(|point| strip(aim.window, edge, aim.zone, aim.rtl).contains(point));
            let over_slot =
                self.edges[at(edge)] && pointer.is_some_and(|point| self.shown.iter().any(|(shown, rect)| *shown == edge && rect.contains(point)));
            self.edges[at(edge)] = in_zone || over_slot;
        }
    }

    /// Whether `slot` on `edge` shows: its `visible` flag in normal mode; in body-only mode, if it reveals and a Tab
    /// or edge reveal shows it.
    pub fn shows(&self, slot: &Slot, edge: PaneEdge, only: bool, allow_reveal: bool) -> bool {
        if !only {
            return slot.is_visible();
        }
        allow_reveal && slot.reveals() && (self.tab || edge.reveal_edge().is_some_and(|edge| self.edges[at(edge)]))
    }

    /// Records the rects of the shown slots by the edge they sit on, for the next pass's hit test.
    pub fn record(&mut self, shown: Vec<(RevealEdge, Rect)>) {
        self.shown = shown;
    }
}

/// The strip of `zone` along `edge` of `window`.
fn strip(window: Rect, edge: RevealEdge, zone: f32, rtl: bool) -> Rect {
    match edge {
        RevealEdge::Top => Rect::from_min_max(window.min, pos2(window.max.x, window.min.y + zone)),
        _ => {
            let at_left = (edge == RevealEdge::Start) != rtl;
            if at_left {
                Rect::from_min_max(window.min, pos2(window.min.x + zone, window.max.y))
            } else {
                Rect::from_min_max(pos2(window.max.x - zone, window.min.y), window.max)
            }
        }
    }
}
