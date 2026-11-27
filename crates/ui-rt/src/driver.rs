//! The run loop, one turn at a time. The platform calls [`Driver::step`] from whatever drives its
//! frames (a winit redraw, the browser's animation frame callback, a bench loop); `step` never
//! blocks and never sleeps, and asks for the next call through `Platform::request_frame` only when
//! something is dirty or animating.

use std::collections::BTreeSet;

use web_time::Instant;

use crate::build::BuildCx;
use crate::element::Element;
use crate::id::WindowId;
use crate::input::ClipboardRequest;
use crate::platform::{Platform, PlatformEvent, Waker, WindowEvent, WindowSpec};
use crate::render::{RenderStats, Renderer};
use crate::runtime::{FrameReport, Runtime};

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
}

impl<P: Platform, R: Renderer> Driver<P, R> {
    /// Takes every platform window's size and scale and attaches the surfaces that exist.
    pub fn new(runtime: Runtime, platform: P, renderer: R) -> Self {
        let mut driver = Driver { runtime, platform, renderer, events: Vec::new(), windows: Vec::new(), attached: BTreeSet::new() };
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

    /// Opens a window showing `root`.
    pub fn open_window<E: Element>(&mut self, spec: &WindowSpec, root: E) -> WindowId {
        let window = self.platform.create_window(spec);
        self.runtime.set_root(window, root);
        self.runtime.set_window_owner(window, spec.owner);
        self.sync_window(window);
        window
    }

    /// Opens the windows nodes asked for in this step and tells each node its window's id.
    fn open_queued_windows(&mut self) {
        for open in self.runtime.take_window_opens() {
            let window = self.platform.create_window(&open.spec);
            (open.root)(&mut self.runtime, window);
            self.runtime.set_window_owner(window, open.spec.owner);
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
            self.runtime.handle_event(ev);
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
                PlatformEvent::Window(..) | PlatformEvent::Wake => {}
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
        let busy = self.runtime.needs_frame() || self.runtime.wants_time() || self.runtime.wants_input_tick();
        if attached && busy {
            self.platform.request_frame();
        }
        if attached && let Some(at) = self.runtime.next_wake() {
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
