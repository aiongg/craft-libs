//! The winit application: window creation, event translation, AccessKit adapters and the redraw
//! that runs `Driver::step`.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use craft_ui_rt::{Driver, PlatformEvent, Pos2, Renderer, SizeU, StepReport, Vec2, WindowEvent, WindowLevel};
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent as W;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes};

use crate::shared::{Cell, Shared, monitor_info, push};
use crate::{UserEvent, WinitPlatform, apply, cursor, input};

type Setup<R> = Box<dyn FnOnce(WinitPlatform) -> Pin<Box<dyn Future<Output = Driver<WinitPlatform, R>>>>>;
type Step<R> = Box<dyn FnMut(&mut Driver<WinitPlatform, R>, f64)>;

struct App<R: Renderer + 'static> {
    proxy: EventLoopProxy<UserEvent>,
    shared: Cell,
    woken: Arc<AtomicBool>,
    setup: Option<Setup<R>>,
    step: Step<R>,
    driver: Rc<RefCell<Option<Driver<WinitPlatform, R>>>>,
    start: Instant,
}

fn attributes(spec: &craft_ui_rt::WindowSpec, canvas: Option<&str>) -> WindowAttributes {
    let size = spec.size.unwrap_or(Vec2::new(800.0, 600.0));
    let mut attrs = Window::default_attributes()
        .with_title(spec.title.clone())
        .with_inner_size(LogicalSize::new(f64::from(size.x), f64::from(size.y)))
        .with_resizable(spec.resizable)
        .with_decorations(spec.decorations)
        // AccessKit's adapter must exist before the window is first shown.
        .with_visible(false);
    if let Some(min) = spec.min_size {
        attrs = attrs.with_min_inner_size(LogicalSize::new(f64::from(min.x), f64::from(min.y)));
    }
    if let Some(p) = spec.position {
        attrs = attrs.with_position(LogicalPosition::new(f64::from(p.x), f64::from(p.y)));
    }
    if spec.level == WindowLevel::AboveOthers {
        attrs = attrs.with_window_level(winit::window::WindowLevel::AlwaysOnTop);
    }
    if !spec.taskbar {
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_skip_taskbar(true);
        }
        // A utility window is kept off the taskbar by the window managers that honour the type.
        #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android"), not(target_arch = "wasm32")))]
        {
            use winit::platform::x11::{WindowAttributesExtX11, WindowType};
            attrs = attrs.with_x11_window_type(vec![WindowType::Utility]);
        }
    }
    // The desktop groups windows and finds the launcher entry by it: the Wayland app id and the X11 class.
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android"), not(target_arch = "wasm32")))]
    if let Some(app_id) = &spec.app_id {
        use winit::platform::wayland::WindowAttributesExtWayland;
        use winit::platform::x11::WindowAttributesExtX11;
        attrs = WindowAttributesExtWayland::with_name(attrs, app_id.clone(), app_id.clone());
        attrs = WindowAttributesExtX11::with_name(attrs, app_id.clone(), app_id.clone());
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowAttributesExtWebSys;
        attrs = attrs.with_canvas(canvas.and_then(web_canvas)).with_append(true);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = canvas;
    attrs
}

/// Sends `State` and `FrameInset` when the window's maximized or full-screen state or its frame
/// changed since the last report (the first call always reports the state).
fn report_frame(shared: &Cell, id: craft_ui_rt::WindowId, window: &Window) {
    let state = (window.is_maximized(), window.fullscreen().is_some());
    // Physical pixels from the surface's corner back to the outer corner; errors where the
    // platform cannot tell (Wayland, the browser).
    let inset = match (window.inner_position(), window.outer_position()) {
        (Ok(inner), Ok(outer)) => Some(Vec2::new((inner.x - outer.x) as f32, (inner.y - outer.y) as f32)),
        _ => None,
    };
    let (send_state, send_inset) = {
        let mut s = shared.borrow_mut();
        let Some(e) = s.entry_mut(id) else { return };
        let send_state = e.reported_state != Some(state);
        e.reported_state = Some(state);
        let send_inset = inset.is_some() && e.reported_inset != inset;
        if inset.is_some() {
            e.reported_inset = inset;
        }
        (send_state, send_inset)
    };
    if send_state {
        push(shared, id, WindowEvent::State { maximized: state.0, fullscreen: state.1 });
    }
    if let (true, Some(inset)) = (send_inset, inset) {
        push(shared, id, WindowEvent::FrameInset(inset));
    }
}

#[cfg(target_arch = "wasm32")]
fn web_canvas(id: &str) -> Option<web_sys::HtmlCanvasElement> {
    use wasm_bindgen::JsCast;
    web_sys::window()?.document()?.get_element_by_id(id)?.dyn_into().ok()
}

impl<R: Renderer + 'static> App<R> {
    /// Makes the native windows the app asked for, with their AccessKit adapters, and reports
    /// each one's scale, size, theme and surface. Runs once per loop turn.
    fn create_pending(&mut self, event_loop: &ActiveEventLoop) {
        let todo: Vec<_> = {
            let s = self.shared.borrow();
            if !s.live {
                return;
            }
            s.entries.iter().filter(|e| e.window.is_none()).map(|e| (e.id, e.spec.clone(), e.canvas.clone())).collect()
        };
        for (id, spec, canvas) in todo {
            let window = match event_loop.create_window(attributes(&spec, canvas.as_deref())) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    log::error!("window creation failed: {e}");
                    self.shared.borrow_mut().entries.retain(|entry| entry.id != id);
                    self.shared.borrow_mut().events.push(PlatformEvent::Window(id, WindowEvent::Closed));
                    continue;
                }
            };
            let adapter = accesskit_winit::Adapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
            let queued = self.shared.borrow_mut().entry_mut(id).map(|e| std::mem::take(&mut e.queued)).unwrap_or_default();
            for cmd in &queued {
                apply(&window, cmd);
            }
            window.set_visible(spec.visible);
            let (size, scale) = (window.inner_size(), window.scale_factor() as f32);
            {
                let mut s = self.shared.borrow_mut();
                let Some(e) = s.entry_mut(id) else { continue };
                e.size_px = SizeU::new(size.width, size.height);
                e.scale = scale;
                e.window = Some(window.clone());
                e.adapter = Some(adapter);
                e.surface_live = true;
                s.monitors_stale = true;
            }
            push(&self.shared, id, WindowEvent::Scale(scale));
            push(&self.shared, id, WindowEvent::Resize(SizeU::new(size.width, size.height).to_logical(scale)));
            if let Some(theme) = window.theme() {
                push(&self.shared, id, WindowEvent::Theme(input::theme_of(theme)));
            }
            if let Ok(p) = window.inner_position() {
                push(&self.shared, id, WindowEvent::Moved(Pos2::new(p.x as f32, p.y as f32)));
            }
            report_frame(&self.shared, id, &window);
            push(&self.shared, id, WindowEvent::SurfaceCreated);
        }
    }

    fn create_cursors(&mut self, event_loop: &ActiveEventLoop) {
        let wanted = std::mem::take(&mut self.shared.borrow_mut().cursor_wanted);
        for (id, key, hotspot) in wanted {
            let image = self.shared.borrow().cursor_images.get(&key).cloned();
            let Some(source) = image.and_then(|i| cursor::source(&i, hotspot)) else { continue };
            let custom = event_loop.create_custom_cursor(source);
            let mut s = self.shared.borrow_mut();
            s.cursor_cache.insert((key, hotspot), custom.clone());
            if let Some(w) = s.native(id) {
                w.set_cursor(winit::window::Cursor::Custom(custom));
            }
        }
    }

    fn refresh_monitors(&mut self, event_loop: &ActiveEventLoop) {
        let primary = event_loop.primary_monitor();
        let list = event_loop.available_monitors().map(|m| monitor_info(&m, primary.as_ref())).collect();
        let mut s = self.shared.borrow_mut();
        s.monitors = list;
        s.monitors_stale = false;
    }

    fn start_driver(&mut self) {
        let Some(setup) = self.setup.take() else { return };
        let platform = WinitPlatform::new(self.shared.clone(), self.proxy.clone(), self.woken.clone());
        let future = setup(platform);
        let (slot, shared) = (self.driver.clone(), self.shared.clone());
        let finish = async move {
            let driver = future.await;
            *slot.borrow_mut() = Some(driver);
            shared.borrow_mut().frame_wanted = true;
        };
        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(finish);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(finish);
    }

    fn step(&mut self) {
        let time = self.start.elapsed().as_secs_f64();
        let Ok(mut slot) = self.driver.try_borrow_mut() else { return };
        if let Some(driver) = slot.as_mut() {
            (self.step)(driver, time);
        }
    }
}

impl<R: Renderer + 'static> ApplicationHandler<UserEvent> for App<R> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        self.shared.borrow_mut().live = true;
        self.refresh_monitors(event_loop);
        if self.setup.is_some() {
            self.start_driver();
        } else {
            self.shared.borrow_mut().events.push(PlatformEvent::Resume);
        }
        self.create_pending(event_loop);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        {
            let mut s = self.shared.borrow_mut();
            s.live = false;
            for e in &mut s.entries {
                e.surface_live = false;
                // Android destroys the native window with the surface; `resumed` makes a new one.
                #[cfg(target_os = "android")]
                {
                    e.window = None;
                    e.adapter = None;
                }
            }
            s.events.push(PlatformEvent::Suspend);
            s.frame_wanted = true;
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Wake => {
                self.woken.store(false, Ordering::Release);
                let mut s = self.shared.borrow_mut();
                s.events.push(PlatformEvent::Wake);
                s.frame_wanted = true;
            }
            UserEvent::Access(event) => {
                let Some(id) = self.shared.borrow().id_of(event.window_id) else { return };
                match event.window_event {
                    accesskit_winit::WindowEvent::ActionRequested(request) => push(&self.shared, id, WindowEvent::Access(request)),
                    accesskit_winit::WindowEvent::InitialTreeRequested => push(&self.shared, id, WindowEvent::AccessActivated),
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
            }
        }
    }

    /// Makes what the runtime asked for: windows, custom cursors, a redraw for a requested step,
    /// and a sleep until the wake time it set.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.create_pending(event_loop);
        self.create_cursors(event_loop);
        if self.shared.borrow().monitors_stale {
            self.refresh_monitors(event_loop);
        }
        let mut s = self.shared.borrow_mut();
        #[cfg(not(target_arch = "wasm32"))]
        if s.ever_opened && s.entries.is_empty() && self.driver.borrow().is_some() {
            event_loop.exit();
            return;
        }
        let mut flow = ControlFlow::Wait;
        if let Some(at) = s.wake_at {
            let due = self.start + std::time::Duration::from_secs_f64(at.max(0.0));
            if Instant::now() >= due {
                s.wake_at = None;
                s.frame_wanted = true;
            } else {
                flow = ControlFlow::WaitUntil(due);
            }
        }
        if s.frame_wanted
            && let Some(w) = s.entries.iter().find_map(|e| e.window.as_ref())
        {
            w.request_redraw();
            s.frame_wanted = false;
        }
        drop(s);
        event_loop.set_control_flow(flow);
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, native: winit::window::WindowId, event: W) {
        let Some(id) = self.shared.borrow().id_of(native) else { return };
        let (window, adapter) = {
            let mut s = self.shared.borrow_mut();
            let Some(e) = s.entry_mut(id) else { return };
            (e.window.clone(), e.adapter.take())
        };
        if let (Some(window), Some(mut adapter)) = (window.as_ref(), adapter) {
            adapter.process_event(window, &event);
            if let Some(e) = self.shared.borrow_mut().entry_mut(id) {
                e.adapter = Some(adapter);
            }
        }
        if matches!(event, W::Moved(_) | W::Resized(_) | W::Occluded(_))
            && let Some(window) = window.as_ref()
        {
            if let Ok(p) = window.inner_position() {
                push(&self.shared, id, WindowEvent::Moved(Pos2::new(p.x as f32, p.y as f32)));
            }
            report_frame(&self.shared, id, window);
        }
        match event {
            W::CloseRequested => push(&self.shared, id, WindowEvent::CloseRequested),
            W::RedrawRequested => self.step(),
            event => input::translate(&self.shared, id, event),
        }
    }
}

/// Runs the loop. `setup` runs once the event loop is active: it builds the renderer
/// (asynchronously, because the browser cannot block on the GPU) and the driver, and opens the
/// first windows with `Driver::open_window`. `after_step` runs after every `Driver::step`: the
/// place to poll app state (close requests go through `Runtime::request_close`). On the desktop `run`
/// returns when the last window closes; in the browser it returns at once and the page's event
/// loop keeps running.
pub fn run<R, F, Fut, H>(setup: F, mut after_step: H) -> Result<(), winit::error::EventLoopError>
where
    R: Renderer + 'static,
    F: FnOnce(WinitPlatform) -> Fut + 'static,
    Fut: Future<Output = Driver<WinitPlatform, R>> + 'static,
    H: FnMut(&mut Driver<WinitPlatform, R>, &StepReport) + 'static,
{
    run_with(setup, move |driver, time| {
        let report = driver.step(time);
        after_step(driver, &report);
    })
}

/// [`run`] for a host that takes the turn itself: `step` is called with the frame time where
/// `run` calls `Driver::step`, so a host with inline roots calls `Driver::step_with` and does its
/// own work around it.
pub fn run_with<R, F, Fut, S>(setup: F, step: S) -> Result<(), winit::error::EventLoopError>
where
    R: Renderer + 'static,
    F: FnOnce(WinitPlatform) -> Fut + 'static,
    Fut: Future<Output = Driver<WinitPlatform, R>> + 'static,
    S: FnMut(&mut Driver<WinitPlatform, R>, f64) + 'static,
{
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let app = App::<R> {
        proxy: event_loop.create_proxy(),
        shared: Rc::new(RefCell::new(Shared::default())),
        woken: Arc::default(),
        setup: Some(Box::new(move |p| Box::pin(setup(p)))),
        step: Box::new(step),
        driver: Rc::new(RefCell::new(None)),
        start: Instant::now(),
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = app;
        event_loop.run_app(&mut app)
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
        Ok(())
    }
}
