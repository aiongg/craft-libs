//! The run loop, one turn at a time. The platform calls [`Driver::step`] from whatever drives its
//! frames (a winit redraw, the browser's animation frame callback, a bench loop); `step` never
//! blocks and never sleeps, and asks for the next call through `Platform::request_frame` only when
//! something is dirty or animating.

use std::collections::BTreeSet;

use web_time::Instant;

use crate::build::BuildCx;
use crate::display::ImageStore;
use crate::element::Element;
use crate::id::WindowId;
use crate::input::ClipboardRequest;
use crate::platform::{Platform, PlatformEvent, Waker, WindowEvent, WindowSpec};
use crate::render::{RenderStats, Renderer};
use crate::runtime::{FrameReport, Runtime};
use crate::state::Store;
use crate::text::TextEngine;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepReport {
    /// A frame ran: something was dirty and a surface was there to draw on.
    pub ran: bool,
    pub frame: FrameReport,
    /// Summed over the windows drawn.
    pub render: RenderStats,
    pub render_ms: f64,
    pub events: u32,
    /// Windows whose pixels were drawn.
    pub windows_drawn: u32,
}

pub struct Driver<P: Platform, R: Renderer> {
    pub runtime: Runtime,
    pub platform: P,
    pub renderer: R,
    events: Vec<PlatformEvent>,
    /// The windows of one turn's end-of-step pass, kept to reuse its buffer.
    windows: Vec<WindowId>,
    attached: BTreeSet<WindowId>,
    wake: Option<WakeHandler>,
    input: Option<InputFilter>,
}

/// Runs on the UI thread at each [`PlatformEvent::Wake`] with the runtime's cells and images: the
/// host's job board delivers finished jobs into them ([`Driver::set_wake_handler`]).
pub type WakeHandler = Box<dyn FnMut(&mut Store, &mut ImageStore)>;

/// Sees each platform event before the runtime does, with the runtime: the app host's input hook
/// ([`Driver::set_input_filter`]). Returning true takes the event, and the runtime never gets it.
pub type InputFilter = Box<dyn FnMut(&PlatformEvent, &mut Runtime) -> bool>;

impl<P: Platform, R: Renderer> Driver<P, R> {
    /// Takes every platform window's size and scale and attaches the surfaces that exist.
    pub fn new(runtime: Runtime, platform: P, renderer: R) -> Self {
        let mut driver =
            Driver { runtime, platform, renderer, events: Vec::new(), windows: Vec::new(), attached: BTreeSet::new(), wake: None, input: None };
        for window in driver.platform.surfaces().windows() {
            driver.sync_window(window);
        }
        driver
    }

    pub fn is_attached(&self, window: WindowId) -> bool {
        self.attached.contains(&window)
    }

    /// A waker for background jobs: each wake delivers [`PlatformEvent::Wake`].
    pub fn waker(&self) -> Waker {
        self.platform.waker()
    }

    /// Sets what runs on the UI thread at each [`PlatformEvent::Wake`], before the step builds:
    /// delivering finished background jobs into cells and images, so the nodes that wait for them
    /// are marked in the same step.
    pub fn set_wake_handler(&mut self, handler: impl FnMut(&mut Store, &mut ImageStore) + 'static) {
        self.wake = Some(Box::new(handler));
    }

    /// Sets what sees each platform event before the runtime, in the step that polls it: the app's
    /// input hook. An event it takes never reaches the runtime; the driver's own bookkeeping
    /// (surfaces, wakes) still runs for it.
    pub fn set_input_filter(&mut self, filter: impl FnMut(&PlatformEvent, &mut Runtime) -> bool + 'static) {
        self.input = Some(Box::new(filter));
    }

    /// Asks every window to close ([`Runtime::quit`]).
    pub fn quit(&mut self) {
        self.runtime.quit();
    }

    /// Opens a window showing `root`.
    pub fn open_window<E: Element>(&mut self, spec: &WindowSpec, root: E) -> WindowId {
        let window = self.platform.create_window(spec);
        self.runtime.set_root(window, root);
        self.runtime.set_window_owner(window, spec.owner);
        self.runtime.set_window_key(window, spec.key.clone());
        self.sync_window(window);
        window
    }

    /// Opens the windows nodes asked for in this step and tells each node its window's id.
    fn open_queued_windows(&mut self) {
        for open in self.runtime.take_window_opens() {
            let window = self.platform.create_window(&open.spec);
            (open.root)(&mut self.runtime, window);
            self.runtime.set_window_owner(window, open.spec.owner);
            self.runtime.set_window_key(window, open.spec.key.clone());
            self.sync_window(window);
            self.runtime.notify(open.requester, &crate::event::Event::WindowOpened(window));
        }
    }

    /// One turn: platform events, dispatch, then build, layout, paint and render over dirty nodes
    /// only. `time` is the platform's frame time in seconds.
    pub fn step(&mut self, time: f64) -> StepReport {
        self.run_step(time, None)
    }

    /// [`Driver::step`] for windows with inline roots ([`Runtime::set_inline_root`]): `roots` is
    /// their content, passed to [`Runtime::frame_with`].
    pub fn step_with(&mut self, time: f64, roots: &mut dyn FnMut(WindowId, &mut BuildCx<'_>)) -> StepReport {
        self.run_step(time, Some(roots))
    }

    /// Opens a window with an inline root ([`Runtime::set_inline_root`]).
    pub fn open_inline_window(&mut self, spec: &WindowSpec) -> WindowId {
        let window = self.platform.create_window(spec);
        self.runtime.set_inline_root(window);
        self.runtime.set_window_key(window, spec.key.clone());
        self.sync_window(window);
        window
    }

    fn run_step(&mut self, time: f64, roots: Option<&mut dyn FnMut(WindowId, &mut BuildCx<'_>)>) -> StepReport {
        let mut report = StepReport::default();
        self.runtime.begin_input(time);
        let mut events = std::mem::take(&mut self.events);
        self.platform.poll(&mut events);
        report.events = u32::try_from(events.len()).unwrap_or(u32::MAX);
        for ev in &events {
            let taken = self.input.as_mut().is_some_and(|filter| filter(ev, &mut self.runtime));
            if !taken {
                self.runtime.handle_event(ev);
            }
            match ev {
                PlatformEvent::Suspend => {
                    for window in std::mem::take(&mut self.attached) {
                        self.renderer.lose_target(window);
                    }
                }
                PlatformEvent::Resume => {
                    for window in self.platform.surfaces().windows() {
                        self.attach(window);
                    }
                }
                PlatformEvent::Window(window, WindowEvent::SurfaceLost | WindowEvent::Closed) => {
                    self.attached.remove(window);
                    self.renderer.lose_target(*window);
                }
                PlatformEvent::Window(window, WindowEvent::SurfaceCreated | WindowEvent::Scale(_) | WindowEvent::Resize(_)) => {
                    self.attach(*window);
                }
                PlatformEvent::Wake => {
                    if let Some(handler) = self.wake.as_mut() {
                        let (store, images) = self.runtime.store_and_images_mut();
                        handler(store, images);
                    }
                }
                PlatformEvent::Window(..) => {}
            }
        }
        events.clear();
        self.events = events;
        self.runtime.tick_input();

        if !self.attached.is_empty() {
            self.runtime.wake_due(time);
            if self.runtime.wants_time() {
                self.runtime.set_time(time);
            }
            self.runtime.note_text_stack(self.renderer.text().stack_generation());
            if self.runtime.needs_frame() {
                report.frame = match roots {
                    Some(roots) => self.runtime.frame_with(self.renderer.text(), roots),
                    None => self.runtime.frame(self.renderer.text()),
                };
                let t = Instant::now();
                report.render.images_uploaded = self.renderer.sync_images(self.runtime.images());
                for &window in &self.attached {
                    if !self.runtime.has_damage(window) {
                        continue;
                    }
                    let stats = self.renderer.render(window, &self.runtime.frame_tree(window), self.runtime.damage(window));
                    report.render.draws += stats.draws;
                    report.render.prepared += stats.prepared;
                    report.render.reused += stats.reused;
                    report.render.prepare_ms += stats.prepare_ms;
                    report.windows_drawn += 1;
                }
                report.render_ms = t.elapsed().as_secs_f64() * 1000.0;
                self.runtime.end_frame();
                report.ran = true;
            }
        }
        let mut windows = std::mem::take(&mut self.windows);
        windows.clear();
        windows.extend(self.runtime.windows());
        for &window in &windows {
            if let Some(update) = self.runtime.take_access_update(window) {
                self.platform.access(window, update);
            }
            for req in self.runtime.take_ime_requests(window) {
                self.platform.ime(window, req);
            }
        }
        self.windows = windows;
        for (window, req) in self.runtime.take_clipboard_requests() {
            match req {
                ClipboardRequest::Set(text) => self.platform.clipboard().set_text(&text),
                ClipboardRequest::Read => self.platform.clipboard().request_text(window),
            }
        }
        for (window, cursor) in self.runtime.take_cursor_requests() {
            self.platform.set_cursor(window, cursor);
        }
        for (window, cmd) in self.runtime.take_window_commands() {
            self.platform.window(window, cmd);
        }
        self.open_queued_windows();
        let attached = !self.attached.is_empty();
        // A wake at or before this frame's time is due in the next frame (a build that waits one frame for its
        // picture to enter the image store): that is a frame request, not a timer.
        let wake = self.runtime.next_wake();
        let due = wake.is_some_and(|at| at <= self.runtime.time);
        let busy = due || self.runtime.needs_frame() || self.runtime.wants_time() || self.runtime.wants_input_tick();
        if attached && busy {
            self.platform.request_frame();
        }
        if attached
            && !due
            && let Some(at) = wake
        {
            self.platform.request_frame_at(at);
        }
        report
    }

    /// Brings the runtime's view of `window` (size, scale) in line with the platform's, and
    /// attaches its surface when it has one.
    fn sync_window(&mut self, window: WindowId) {
        let host = self.platform.surfaces();
        let scale = host.scale(window);
        let size = host.size_px(window).to_logical(scale);
        self.runtime.add_window(window, size, scale);
        self.runtime.handle_event(&PlatformEvent::Window(window, WindowEvent::Scale(scale)));
        self.runtime.handle_event(&PlatformEvent::Window(window, WindowEvent::Resize(size)));
        self.attach(window);
    }

    fn attach(&mut self, window: WindowId) {
        let host = self.platform.surfaces();
        match host.surface(window) {
            Some(surface) => {
                let (size, scale) = (host.size_px(window), host.scale(window));
                self.renderer.set_target(window, surface, size, scale);
                self.attached.insert(window);
                self.runtime.invalidate_window(window);
            }
            None => {
                self.attached.remove(&window);
            }
        }
    }
}
