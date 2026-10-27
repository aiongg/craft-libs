//! The eframe side: open the root window where it was ([`install`]) and remember where each window
//! is ([`Tracker`]).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use ::egui::{Context, Vec2, ViewportBuilder, ViewportInfo};

use super::{Defaults, NormalRect, Placement, Store, write_pending};

/// Seconds without a change before a [`Tracker`] writes.
const SAVE_DELAY: f64 = 1.0;

/// Held from `take_pending` until the write is done, so an older snapshot of a store never lands
/// on disk after a newer one when two windows save at once. One lock for the process: there is
/// one store per app.
static WRITES: Mutex<()> = Mutex::new(());

fn lock(store: &Mutex<Store>) -> MutexGuard<'_, Store> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Writes what `store` has not written yet, holding its lock only to take and to mark the write.
fn save(store: &Mutex<Store>) -> std::io::Result<bool> {
    let _writing = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(pending) = lock(store).take_pending() else { return Ok(false) };
    write_pending(&pending)?;
    lock(store).mark_saved(pending.generation);
    Ok(true)
}

fn positive(v: f32) -> Option<f32> {
    (v.is_finite() && v > 0.0).then_some(v)
}

/// `builder` with the size, window state and (approximate) position of `placement`, for a window
/// created while the interface zoom is `zoom` (`ctx.zoom_factor()`; 1 before the first frame).
///
/// egui takes builder sizes and positions in points at the current zoom. The position goes
/// through the placement's scale, which is exact on a single monitor or monitors of one scale.
pub fn apply(builder: ViewportBuilder, placement: &Placement, zoom: f32) -> ViewportBuilder {
    let zoom = positive(zoom).unwrap_or(1.0);
    let [w, h] = placement.normal.size;
    let mut builder = builder.with_inner_size([w / zoom, h / zoom]).with_maximized(placement.maximized);
    if placement.fullscreen {
        builder = builder.with_fullscreen(true);
    }
    if let Some([x, y]) = placement.position_px {
        let s = positive(placement.scale).unwrap_or(1.0) * zoom;
        builder = builder.with_position([x as f32 / s, y as f32 / s]);
    }
    builder
}

/// Opens the root window from what `store` has under `key` and returns the [`Tracker`] that keeps
/// it up to date. Call before `eframe::run_native`.
///
/// - The root viewport gets the saved size and maximized state, checked against `defaults` (no
///   monitors are known yet).
/// - eframe's own window persistence (`persist_window`) is turned off.
/// - A viewport hook (`NativeOptions::viewport_hooks`) runs [`super::place`] with the connected
///   monitors when the root window is created, before eframe shows it, and sets the position in
///   pixels, the size and the window state. Hooks already in `options` keep running after it.
#[cfg(not(target_arch = "wasm32"))]
pub fn install(options: &mut eframe::NativeOptions, store: &Arc<Mutex<Store>>, key: &str, defaults: &Defaults) -> Tracker {
    let saved = lock(store).get(key);
    let first = super::place(saved.as_ref(), &[], None, defaults);
    options.viewport = apply(std::mem::take(&mut options.viewport), &first, 1.0);
    options.persist_window = false;
    let opened = Arc::new(Mutex::new(None));
    let hook = native::Hook::new(saved, defaults.clone(), Arc::clone(&opened), options.viewport_hooks.take());
    options.viewport_hooks = Some(Arc::new(hook));
    let mut tracker = Tracker::new(store, key, defaults);
    tracker.opened = Some(opened);
    tracker
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};

    use ::egui::{RawInput, ViewportId};
    use eframe::ViewportHooks;
    use winit::dpi::{LogicalSize, PhysicalPosition};
    use winit::monitor::MonitorHandle;
    use winit::window::{Fullscreen, Window};

    use super::super::{Defaults, Monitor, MonitorKey, Placement, place};

    /// Places the root window once, while it is still hidden, then passes every call on.
    pub(super) struct Hook {
        saved: Option<Placement>,
        defaults: Defaults,
        placed: AtomicBool,
        /// Where the window was put, for the tracker's first frame.
        opened: Arc<Mutex<Option<Placement>>>,
        next: Option<Arc<dyn ViewportHooks>>,
    }

    impl Hook {
        pub(super) fn new(
            saved: Option<Placement>,
            defaults: Defaults,
            opened: Arc<Mutex<Option<Placement>>>,
            next: Option<Arc<dyn ViewportHooks>>,
        ) -> Self {
            Self { saved, defaults, placed: AtomicBool::new(false), opened, next }
        }
    }

    impl ViewportHooks for Hook {
        fn window_created(&self, viewport: ViewportId, window: &Window) {
            // eframe may recreate the root window (Android resume); only the first one is placed.
            if viewport == ViewportId::ROOT && !self.placed.swap(true, Ordering::Relaxed) {
                let placement = open(window, self.saved.as_ref(), &self.defaults);
                *self.opened.lock().unwrap_or_else(PoisonError::into_inner) = Some(placement);
            }
            if let Some(next) = &self.next {
                next.window_created(viewport, window);
            }
        }

        fn window_destroyed(&self, viewport: ViewportId) {
            if let Some(next) = &self.next {
                next.window_destroyed(viewport);
            }
        }

        fn raw_input(&self, ctx: &::egui::Context, raw_input: &mut RawInput) {
            if let Some(next) = &self.next {
                next.raw_input(ctx, raw_input);
            }
        }
    }

    fn monitor(handle: &MonitorHandle) -> Monitor {
        let size = handle.size();
        let pos = handle.position();
        let size = [size.width, size.height];
        Monitor::new(MonitorKey::new(handle.name().unwrap_or_default(), size), [pos.x, pos.y], size, handle.scale_factor() as f32)
    }

    fn open(window: &Window, saved: Option<&Placement>, defaults: &Defaults) -> Placement {
        let handles: Vec<MonitorHandle> = window.available_monitors().collect();
        let monitors: Vec<Monitor> = handles.iter().map(monitor).collect();
        let home = window.current_monitor().and_then(|current| handles.iter().position(|h| *h == current));
        let placement = place(saved, &monitors, home, defaults);
        if let Some([x, y]) = placement.position_px {
            window.set_outer_position(PhysicalPosition::new(x, y));
        }
        // Logical: if the move lands on a monitor of another scale, the platform's scale change
        // then keeps the size in points.
        let [w, h] = placement.normal.size;
        let _ = window.request_inner_size(LogicalSize::new(w, h));
        window.set_maximized(placement.maximized);
        if placement.fullscreen {
            let on = placement.monitor.as_ref().and_then(|key| monitors.iter().position(|m| &m.key == key)).and_then(|i| handles.get(i).cloned());
            window.set_fullscreen(Some(Fullscreen::Borderless(on)));
        }
        placement
    }
}

/// Keeps one window's entry in the store up to date: call [`frame`](Self::frame) every frame from
/// that window's own pass (for a child viewport, inside its viewport UI).
///
/// - The saved size is the inner size of the last state that was neither maximized, full screen
///   nor minimized, in points at interface zoom 1; the position is the outer top-left corner in
///   pixels, none with [`Defaults::no_position`] (Wayland) or where the platform reports none.
/// - Maximize and full screen keep that normal rectangle and set their flags; minimize changes
///   nothing.
/// - It writes 1 s after the last change, on a short-lived thread, and at once when the window is
///   asked to close and when the tracker is dropped.
pub struct Tracker {
    store: Arc<Mutex<Store>>,
    key: String,
    no_position: bool,
    /// The window as it would be saved now; `None` until a normal rectangle is known.
    current: Option<Placement>,
    /// What this tracker last handed to the store.
    stored: Option<Placement>,
    /// Input time of the last change not yet written.
    changed_at: Option<f64>,
    /// Filled by [`install`]'s hook with where the root window opened.
    opened: Option<Arc<Mutex<Option<Placement>>>>,
    writes: Vec<JoinHandle<()>>,
    #[cfg(test)]
    background_saves: u32,
}

impl Tracker {
    /// A tracker for the window saved under `key` in `store`. Use the one [`install`] returns for
    /// the root window.
    pub fn new(store: &Arc<Mutex<Store>>, key: &str, defaults: &Defaults) -> Self {
        let current = lock(store).get(key);
        Self {
            store: Arc::clone(store),
            key: key.to_owned(),
            no_position: defaults.no_position,
            stored: current.clone(),
            current,
            changed_at: None,
            opened: None,
            writes: Vec::new(),
            #[cfg(test)]
            background_saves: 0,
        }
    }

    /// The key this tracker saves under.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Reads this frame's window state from `ctx` (the window's own viewport).
    pub fn frame(&mut self, ctx: &Context) {
        let (info, size, now) = ctx.input(|i| (i.viewport().clone(), i.viewport_rect().size(), i.time));
        if let Some(wait) = self.observe(&info, size, ctx.zoom_factor(), now) {
            // A window left alone gets no frames: ask for the one that writes.
            ctx.request_repaint_after(Duration::from_secs_f64(wait));
        }
    }

    /// Writes now, waiting for the file: for an app's own exit path. [`Drop`] does the same and
    /// ignores the error.
    pub fn flush(&mut self) -> std::io::Result<bool> {
        self.changed_at = None;
        self.hand_over();
        for write in self.writes.drain(..) {
            let _ = write.join();
        }
        save(&self.store)
    }

    /// One frame: `size` is the viewport's size in points, used where `info` has no inner
    /// rectangle; `now` is the input time in seconds. Returns the seconds until the pending write,
    /// if one waits.
    fn observe(&mut self, info: &ViewportInfo, size: Vec2, zoom: f32, now: f64) -> Option<f64> {
        if let Some(p) = self.opened.as_ref().and_then(|o| o.lock().unwrap_or_else(PoisonError::into_inner).take()) {
            self.current = Some(p);
            self.opened = None;
        }
        if let Some(next) = self.next(info, size, zoom)
            && self.current.as_ref() != Some(&next)
        {
            self.current = Some(next);
            self.changed_at = Some(now);
        }
        if info.close_requested() {
            let _ = self.flush();
            return None;
        }
        let since = self.changed_at.map(|t| now - t)?;
        if since < SAVE_DELAY {
            return Some(SAVE_DELAY - since);
        }
        self.changed_at = None;
        self.save_in_background();
        None
    }

    /// `current` updated by one frame's state; `None` while no normal rectangle is known.
    fn next(&self, info: &ViewportInfo, size: Vec2, zoom: f32) -> Option<Placement> {
        let zoom = positive(zoom).unwrap_or(1.0);
        let scale = info.native_pixels_per_point.and_then(positive);
        let fullscreen = info.fullscreen == Some(true);
        let maximized = info.maximized == Some(true);
        let mut next = self.current.clone();
        if info.minimized == Some(true) {
            return next;
        }
        if !fullscreen && !maximized {
            let inner = info.inner_rect.map_or(size, |r| r.size()) * zoom;
            if inner.x >= 1.0 && inner.y >= 1.0 {
                let scale = scale.or(next.as_ref().map(|p| p.scale)).unwrap_or(1.0);
                let pixels = scale * zoom;
                let position =
                    info.outer_rect.filter(|_| !self.no_position).map(|r| [(r.min.x * pixels).round() as i32, (r.min.y * pixels).round() as i32]);
                let p = next.get_or_insert_with(|| Placement::new([inner.x, inner.y], scale));
                p.normal = NormalRect { pos: position.map_or([0.0; 2], |q| q.map(|v| v as f32 / scale)), size: [inner.x, inner.y] };
                p.position_px = position;
                p.scale = scale;
            }
        }
        let p = next.as_mut()?;
        p.fullscreen = fullscreen;
        // Leaving full screen returns to the window state from before it.
        if !fullscreen {
            p.maximized = maximized;
        }
        p.zoom = zoom;
        // A monitor key is only known from the hook; keep it while the monitor's size matches.
        if let (Some(key), Some(monitor), Some(s)) = (&p.monitor, info.monitor_size, scale) {
            let px = [(monitor.x * s * zoom).round() as u32, (monitor.y * s * zoom).round() as u32];
            if key.size_px != px {
                p.monitor = None;
            }
        }
        next
    }

    /// Gives the current placement to the store, if it changed since the last time.
    fn hand_over(&mut self) {
        if let Some(p) = &self.current
            && self.stored.as_ref() != Some(p)
        {
            lock(&self.store).set(&self.key, p);
            self.stored = Some(p.clone());
        }
    }

    fn save_in_background(&mut self) {
        self.hand_over();
        self.writes.retain(|w| !w.is_finished());
        #[cfg(test)]
        {
            self.background_saves += 1;
        }
        // No threads on the web, where the store is in memory anyway.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let store = Arc::clone(&self.store);
            if let Ok(write) = std::thread::Builder::new().name("craft-window-restore".into()).spawn(move || {
                let _ = save(&store);
            }) {
                self.writes.push(write);
                return;
            }
        }
        let _ = save(&self.store);
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use ::egui::{Rect, ViewportEvent, pos2, vec2};

    use super::*;
    use crate::restore::{MAIN, MonitorKey};

    fn info(pos: [f32; 2], size: [f32; 2], scale: f32) -> ViewportInfo {
        ViewportInfo {
            native_pixels_per_point: Some(scale),
            outer_rect: Some(Rect::from_min_size(pos2(pos[0], pos[1]), vec2(size[0], size[1] + 30.0))),
            inner_rect: Some(Rect::from_min_size(pos2(pos[0], pos[1] + 30.0), vec2(size[0], size[1]))),
            minimized: Some(false),
            maximized: Some(false),
            fullscreen: Some(false),
            ..ViewportInfo::default()
        }
    }

    fn file_store() -> (tempfile::TempDir, Arc<Mutex<Store>>) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(&dir.path().join("window.json"));
        (dir, Arc::new(Mutex::new(store)))
    }

    fn tracker(store: &Arc<Mutex<Store>>) -> Tracker {
        Tracker::new(store, MAIN, &Defaults::new([800.0, 600.0], [200.0, 100.0]))
    }

    fn on_disk(dir: &tempfile::TempDir) -> Option<Placement> {
        Store::at(&dir.path().join("window.json")).get(MAIN)
    }

    const NO_SIZE: Vec2 = Vec2::ZERO;

    #[test]
    fn moves_and_resizes_update_the_normal_rect() {
        let store = Arc::new(Mutex::new(Store::in_memory()));
        let mut t = tracker(&store);
        t.observe(&info([100.0, 50.0], [800.0, 600.0], 2.0), NO_SIZE, 1.0, 0.0);
        t.observe(&info([150.0, 70.0], [900.0, 650.0], 2.0), NO_SIZE, 1.0, 0.1);
        let p = t.current.clone().unwrap();
        assert_eq!(p.normal.size, [900.0, 650.0]);
        assert_eq!(p.position_px, Some([300, 140]));
        assert_eq!(p.normal.pos, [150.0, 70.0]);
        assert_eq!(p.scale, 2.0);
        assert!(!p.maximized && !p.fullscreen);
    }

    #[test]
    fn maximize_full_screen_and_minimize_keep_the_normal_rect() {
        let store = Arc::new(Mutex::new(Store::in_memory()));
        let mut t = tracker(&store);
        t.observe(&info([100.0, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, 0.0);
        let normal = t.current.clone().unwrap().normal;

        let mut max = info([0.0, 0.0], [1920.0, 1050.0], 1.0);
        max.maximized = Some(true);
        t.observe(&max, NO_SIZE, 1.0, 0.1);
        let p = t.current.clone().unwrap();
        assert!(p.maximized);
        assert_eq!((p.normal, p.position_px), (normal, Some([100, 50])));

        // Full screen from maximized: back to maximized after it.
        let mut full = info([0.0, 0.0], [1920.0, 1080.0], 1.0);
        full.fullscreen = Some(true);
        full.maximized = Some(false);
        t.observe(&full, NO_SIZE, 1.0, 0.2);
        let p = t.current.clone().unwrap();
        assert!(p.fullscreen && p.maximized);
        assert_eq!(p.normal, normal);

        // Minimized (Windows parks the window at -32000).
        let mut min = info([-32000.0, -32000.0], [160.0, 28.0], 1.0);
        min.minimized = Some(true);
        t.observe(&min, NO_SIZE, 1.0, 0.3);
        assert_eq!(t.current.clone().unwrap(), p);
    }

    #[test]
    fn sizes_are_saved_in_points_at_zoom_one() {
        let store = Arc::new(Mutex::new(Store::in_memory()));
        let mut t = tracker(&store);
        // At 150 % interface zoom on a 2× monitor, egui reports 800×600 pt for 1200×900 logical.
        t.observe(&info([100.0, 50.0], [800.0, 600.0], 2.0), NO_SIZE, 1.5, 0.0);
        let p = t.current.clone().unwrap();
        assert_eq!(p.normal.size, [1200.0, 900.0]);
        assert_eq!(p.position_px, Some([300, 150]));
        assert_eq!(p.zoom, 1.5);
        // Zooming back to 100 % changes no geometry.
        t.observe(&info([150.0, 75.0], [1200.0, 900.0], 2.0), NO_SIZE, 1.0, 0.1);
        let q = t.current.clone().unwrap();
        assert_eq!((q.normal.size, q.position_px, q.zoom), ([1200.0, 900.0], Some([300, 150]), 1.0));
    }

    #[test]
    fn wayland_stores_no_position() {
        let store = Arc::new(Mutex::new(Store::in_memory()));
        let defaults = Defaults::new([800.0, 600.0], [200.0, 100.0]).with_no_position(true);
        let mut t = Tracker::new(&store, MAIN, &defaults);
        t.observe(&info([100.0, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, 0.0);
        let p = t.current.clone().unwrap();
        assert_eq!((p.position_px, p.normal.pos, p.normal.size), (None, [0.0, 0.0], [800.0, 600.0]));

        // egui itself reports no rectangles there: the size comes from the viewport.
        let mut t = tracker(&store);
        let bare = ViewportInfo { native_pixels_per_point: Some(1.0), ..ViewportInfo::default() };
        t.observe(&bare, vec2(640.0, 480.0), 1.0, 0.0);
        let p = t.current.clone().unwrap();
        assert_eq!((p.position_px, p.normal.size), (None, [640.0, 480.0]));
    }

    #[test]
    fn a_monitor_key_stays_only_while_the_monitor_matches() {
        let store = Arc::new(Mutex::new(Store::in_memory()));
        let mut opened = Placement::new([800.0, 600.0], 1.0);
        opened.monitor = Some(MonitorKey::new("DP-1", [1920, 1080]));
        lock(&store).set(MAIN, &opened);
        let mut t = tracker(&store);
        let mut on = info([100.0, 50.0], [800.0, 600.0], 1.0);
        on.monitor_size = Some(vec2(1920.0, 1080.0));
        t.observe(&on, NO_SIZE, 1.0, 0.0);
        assert!(t.current.clone().unwrap().monitor.is_some());
        on.monitor_size = Some(vec2(2560.0, 1440.0));
        t.observe(&on, NO_SIZE, 1.0, 0.1);
        assert_eq!(t.current.clone().unwrap().monitor, None);
    }

    #[test]
    fn changes_within_the_delay_give_one_write() {
        let (dir, store) = file_store();
        let mut t = tracker(&store);
        let mut wait = None;
        for i in 0..10 {
            let x = 100.0 + i as f32 * 10.0;
            wait = t.observe(&info([x, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, f64::from(i) * 0.09);
        }
        assert!(wait.is_some_and(|w| w > 0.0), "a write is scheduled");
        assert_eq!(t.background_saves, 0);
        // The last change was at 0.81 s: quiet frames until 1 s after it.
        assert!(t.observe(&info([190.0, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, 1.5).is_some());
        assert_eq!(t.observe(&info([190.0, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, 1.85), None);
        assert_eq!(t.background_saves, 1);
        t.observe(&info([190.0, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, 3.0);
        assert_eq!(t.background_saves, 1, "nothing changed since");
        for w in t.writes.drain(..) {
            w.join().unwrap();
        }
        assert_eq!(on_disk(&dir).unwrap().position_px, Some([190, 50]));
        assert!(!lock(&store).is_dirty());
    }

    #[test]
    fn close_and_drop_write_at_once() {
        let (dir, store) = file_store();
        let mut t = tracker(&store);
        t.observe(&info([100.0, 50.0], [800.0, 600.0], 1.0), NO_SIZE, 1.0, 0.0);
        let mut closing = info([120.0, 50.0], [800.0, 600.0], 1.0);
        closing.events.push(ViewportEvent::Close);
        assert_eq!(t.observe(&closing, NO_SIZE, 1.0, 0.1), None);
        assert_eq!(t.background_saves, 0);
        assert_eq!(on_disk(&dir).unwrap().position_px, Some([120, 50]));

        t.observe(&info([140.0, 60.0], [700.0, 500.0], 1.0), NO_SIZE, 1.0, 0.2);
        drop(t);
        let p = on_disk(&dir).unwrap();
        assert_eq!((p.position_px, p.normal.size), (Some([140, 60]), [700.0, 500.0]));
    }

    #[test]
    fn apply_sets_size_state_and_position_at_the_current_zoom() {
        let mut p = Placement::new([1200.0, 900.0], 2.0);
        p.position_px = Some([400, 200]);
        p.maximized = true;
        let b = apply(ViewportBuilder::default().with_title("t"), &p, 1.5);
        assert_eq!(b.inner_size, Some(vec2(800.0, 600.0)));
        assert_eq!(b.position, Some(pos2(400.0 / 3.0, 200.0 / 3.0)));
        assert_eq!((b.maximized, b.fullscreen, b.title.as_deref()), (Some(true), None, Some("t")));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn install_opens_from_the_store_and_keeps_other_hooks() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Default)]
        struct Count(AtomicUsize);
        impl eframe::ViewportHooks for Count {
            fn raw_input(&self, _: &Context, _: &mut ::egui::RawInput) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }

        let store = Arc::new(Mutex::new(Store::in_memory()));
        let mut saved = Placement::new([1000.0, 700.0], 1.0);
        saved.maximized = true;
        lock(&store).set(MAIN, &saved);
        let count = Arc::new(Count::default());
        let mut options = eframe::NativeOptions { viewport_hooks: Some(Arc::clone(&count) as Arc<dyn eframe::ViewportHooks>), ..Default::default() };
        let tracker = install(&mut options, &store, MAIN, &Defaults::new([800.0, 600.0], [200.0, 100.0]));
        assert_eq!(tracker.key(), MAIN);
        assert!(!options.persist_window);
        assert_eq!((options.viewport.inner_size, options.viewport.maximized), (Some(vec2(1000.0, 700.0)), Some(true)));
        let hooks = options.viewport_hooks.unwrap();
        hooks.raw_input(&Context::default(), &mut ::egui::RawInput::default());
        assert_eq!(count.0.load(Ordering::Relaxed), 1);
    }
}
