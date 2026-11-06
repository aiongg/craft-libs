//! A fixed-rate frame driver over a context with the runtime installed.

use craft_ui::runtime::egui::input::frame_input;
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::frame::{FrameInput, set_input};
use craft_ui::runtime::{InstallSpec, install};
use egui::{Context, Event, Pos2, Rect, Ui, Vec2};

const DT: f64 = 1.0 / 60.0;

/// One context, advanced frame by frame at 60 Hz.
#[non_exhaustive]
pub struct Frames {
    pub ctx: Context,
    time: f64,
    root: RootEnv,
}

impl Frames {
    pub fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, InstallSpec::new()).expect("a fresh context takes the runtime");
        Frames { ctx, time: 0.0, root: RootEnv::default() }
    }

    /// The root values every following frame is given.
    pub fn root(&mut self, root: RootEnv) {
        self.root = root;
    }

    /// One frame with `events`; `ui` runs once per pass. Returns the number of passes.
    pub fn frame(&mut self, events: Vec<Event>, mut ui: impl FnMut(&mut Ui)) -> usize {
        set_input(&self.ctx, FrameInput::new(self.root.clone()));
        self.time += DT;
        let input = frame_input(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)), self.time, events);
        let mut passes = 0;
        let mut output = self.ctx.run_ui(input, |root| {
            passes += 1;
            ui(root);
        });
        output.textures_delta.clear();
        passes
    }

    /// `n` frames with no events.
    pub fn idle(&mut self, n: usize, mut ui: impl FnMut(&mut Ui)) {
        for _ in 0..n {
            self.frame(Vec::new(), &mut ui);
        }
    }
}
