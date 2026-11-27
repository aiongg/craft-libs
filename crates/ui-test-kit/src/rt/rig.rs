//! A runtime, a headless platform and a renderer wired together, stepped on the platform's clock.

use std::sync::Arc;

use craft_theme::{Appearance, Contrast, Mode, TextSize, Tokens, builtins, resolve};
use craft_ui_platform_headless::{HeadlessPlatform, Input};
use craft_ui_rt::{Driver, Element, Renderer, Runtime, StepReport, Vec2, WindowId, WindowSpec};

use super::observe::Snapshot;

/// One simulated 60 Hz frame, in seconds.
pub const FRAME: f64 = 1.0 / 60.0;

/// The tokens the trials share: the first builtin theme, dark, compact, standard contrast.
pub fn test_tokens() -> Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

type StateFn = Arc<dyn Fn(&Runtime) -> String + Send + Sync>;

/// The unit a check drives. `driver.platform` is the headless platform, `driver.renderer` is
/// whatever renderer the check attaches (the recording one, or a pixel renderer).
pub struct Rig<R: Renderer> {
    pub driver: Driver<HeadlessPlatform, R>,
    /// The first window.
    pub main: WindowId,
    state: Option<StateFn>,
}

impl<R: Renderer> Rig<R> {
    /// A window of `size` logical points at `scale` showing `root`, with [`test_tokens`].
    pub fn new<E: Element>(root: E, size: Vec2, scale: f32, renderer: R) -> Rig<R> {
        Rig::with_tokens(root, size, scale, renderer, test_tokens())
    }

    pub fn with_tokens<E: Element>(root: E, size: Vec2, scale: f32, renderer: R, tokens: Tokens) -> Rig<R> {
        let platform = HeadlessPlatform::with_window(size, scale);
        let mut runtime = Runtime::new(tokens);
        runtime.set_root(HeadlessPlatform::MAIN, root);
        Rig { driver: Driver::new(runtime, platform, renderer), main: HeadlessPlatform::MAIN, state: None }
    }

    /// The app state a snapshot carries: whatever string the app's model reads back from the
    /// runtime's store. Compared as text between runs.
    pub fn with_state(mut self, state: impl Fn(&Runtime) -> String + Send + Sync + 'static) -> Rig<R> {
        self.state = Some(Arc::new(state));
        self
    }

    /// Opens another window.
    pub fn open_window<E: Element>(&mut self, title: &str, size: Vec2, root: E) -> WindowId {
        self.driver.open_window(&WindowSpec { size: Some(size), ..WindowSpec::new(title) }, root)
    }

    pub fn platform(&mut self) -> &mut HeadlessPlatform {
        &mut self.driver.platform
    }

    pub fn runtime(&self) -> &Runtime {
        &self.driver.runtime
    }

    pub fn inject(&mut self, window: WindowId, input: Input) {
        self.driver.platform.inject(window, input);
    }

    /// One turn at the platform's current time.
    pub fn step(&mut self) -> StepReport {
        let now = self.driver.platform.now();
        self.driver.step(now)
    }

    /// Advances the clock by `dt` seconds, then one turn.
    pub fn frame(&mut self, dt: f64) -> StepReport {
        self.driver.platform.advance(dt);
        self.step()
    }

    /// Frames of [`FRAME`] until one does not run (or `limit` frames); the number that ran. Timed
    /// wakes (a caret's blink) do not keep it going.
    pub fn settle(&mut self, limit: usize) -> usize {
        let mut ran = 0;
        while ran < limit && self.frame(FRAME).ran {
            ran += 1;
        }
        ran
    }

    /// Sets the clock to the time the runtime asked to be woken at, exactly, and runs a turn
    /// there. `None` when no wake is pending.
    pub fn run_to_wake(&mut self) -> Option<StepReport> {
        self.driver.platform.advance_to_wake()?;
        Some(self.step())
    }

    /// The access trees, focus and app state now.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot::of(&self.driver.runtime, &self.driver.platform, self.state.as_deref())
    }
}
