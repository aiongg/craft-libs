//! A fixed-rate frame driver over a context.

use crate::runtime::egui::access::AccessTree;
use crate::runtime::egui::{areas, fixture};
use crate::runtime::commands::CommandInput;
use crate::runtime::env::RootEnv;
use crate::runtime::frame::{FrameInput, announce, set_input};
use crate::runtime::{InstallSpec, install};
use craft_host::Host;
use egui::{Context, Event, Ui};
use std::sync::Arc;
use std::time::Duration;

const DT: f64 = 1.0 / 60.0;

/// One context, advanced frame by frame at 60 Hz.
#[non_exhaustive]
pub struct Frames {
    /// The context the frames run on.
    pub ctx: Context,
    time: f64,
    root: RootEnv,
    cursor: egui::CursorIcon,
    repaint_delay: Duration,
    host: Option<Arc<Host>>,
    commands: CommandInput,
    tree: Option<AccessTree>,
}

impl Default for Frames {
    fn default() -> Self {
        Self::new()
    }
}

impl Frames {
    /// A fresh context with the runtime installed.
    pub fn new() -> Self {
        let mut frames = Self::uninstalled();
        frames.install();
        frames
    }

    /// A fresh context without the runtime, for frames that ran before `install`.
    pub fn uninstalled() -> Self {
        Frames {
            ctx: fixture::bare_context(),
            time: 0.0,
            root: RootEnv::default(),
            cursor: egui::CursorIcon::Default,
            repaint_delay: Duration::MAX,
            host: None,
            commands: CommandInput::new(),
            tree: None,
        }
    }

    /// Installs the runtime on the context.
    pub fn install(&mut self) {
        let installed = install(&self.ctx, InstallSpec::new().frame_layers(Some(crate::contract::ghost::paint)));
        debug_assert!(installed.is_ok(), "the context takes the runtime once");
    }

    /// The host every following frame is given.
    pub fn host(&mut self, host: Arc<Host>) {
        self.host = Some(host);
    }

    fn input(&self) -> FrameInput {
        let input = FrameInput::new(self.root.clone()).commands(self.commands.clone());
        match &self.host {
            Some(host) => input.host(host.clone()),
            None => input,
        }
    }

    /// The root values every following frame is given.
    pub fn root(&mut self, root: RootEnv) {
        self.root = root;
    }

    /// The command input every following frame is given.
    pub fn commands(&mut self, commands: CommandInput) {
        self.commands = commands;
    }

    /// One frame with `events`; `ui` runs once per pass. Returns the number of passes.
    pub fn frame(&mut self, events: Vec<Event>, mut ui: impl FnMut(&mut Ui)) -> usize {
        set_input(&self.ctx, self.input());
        let mut passes = 0;
        self.run(events, |root| {
            passes += 1;
            ui(root);
        });
        passes
    }

    /// One frame with no `set_input`, as an app that never calls it would drive it: the root values and the
    /// interface scale stay as they were. Returns whether the frame sent a close command.
    pub fn frame_without_input(&mut self, events: Vec<Event>, ui: impl FnMut(&mut Ui)) -> bool {
        self.run(events, ui)
    }

    /// One frame with no events; returns the shapes it painted, nested shapes flattened.
    pub fn frame_shapes(&mut self, mut ui: impl FnMut(&mut Ui)) -> Vec<egui::Shape> {
        set_input(&self.ctx, self.input());
        self.time += DT;
        fixture::run_frame_shapes(&self.ctx, self.time, &mut ui)
    }

    /// Moves the clock on by `seconds` before the next frame.
    pub fn advance(&mut self, seconds: f64) {
        self.time += seconds;
    }

    /// `n` frames with no events.
    pub fn idle(&mut self, n: usize, mut ui: impl FnMut(&mut Ui)) {
        for _ in 0..n {
            self.frame(Vec::new(), &mut ui);
        }
    }

    fn run(&mut self, events: Vec<Event>, ui: impl FnMut(&mut Ui)) -> bool {
        self.time += DT;
        let result = fixture::run_frame(&self.ctx, self.time, events, ui);
        self.cursor = result.cursor;
        self.repaint_delay = result.repaint_delay;
        self.tree = result.tree;
        result.closed
    }

    /// The AccessKit tree the last frame produced.
    pub fn tree(&mut self) -> AccessTree {
        self.tree.take().unwrap_or_else(AccessTree::none)
    }

    /// The wait the last frame asked for before the next pass: `Duration::ZERO` for an immediate repaint,
    /// `None` when it asked for none.
    pub fn repaint_delay(&self) -> Option<Duration> {
        (self.repaint_delay != Duration::MAX).then_some(self.repaint_delay)
    }

    /// Whether anything asked for a pass since the last frame ended.
    pub fn repaint_requested(&self) -> bool {
        fixture::repaint_requested(&self.ctx)
    }

    /// The system cursor the last frame's output asked for.
    pub fn cursor_icon(&self) -> egui::CursorIcon {
        self.cursor
    }
}

/// Adds `text` to the next frame output's announcements, as the runtime's access service does.
pub fn announce_text(ctx: &Context, text: &str) {
    announce(ctx, text.to_owned());
}

/// Shows an egui `Area` and runs `body` on its `Ui`.
pub fn show_area<R>(ctx: &Context, name: &'static str, body: impl FnOnce(&mut Ui) -> R) -> R {
    areas::show_area(ctx, name, body)
}
