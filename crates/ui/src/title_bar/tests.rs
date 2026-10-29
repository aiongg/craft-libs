use craft_window::{Button, Layout};
use egui::{
    Color32, Context, CursorIcon, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, ResizeDirection, ViewportCommand, ViewportId, pos2, vec2,
};

use super::captions::{metrics, theme_icon};
use super::resize::subtract;
use super::*;
use crate::theme::{Base, Tokens, TokensOf as _, install};

/// The bar height of the suite default theme (Compact).
const H: f32 = 32.0;
const HEIGHT: f32 = 400.0;

fn windows() -> Frame {
    Frame::Integrated(Captions { family: CaptionFamily::Windows, layout: Layout::default() })
}

#[derive(Default)]
struct Recorded {
    buttons: Vec<(Caption, Rect)>,
    drag: Vec<Rect>,
}

impl CaptionRegions for Recorded {
    fn button(&mut self, caption: Caption, rect: Rect) {
        self.buttons.push((caption, rect));
    }
    fn drag(&mut self, rect: Rect) {
        self.drag.push(rect);
    }
}

/// A headless window `width` pt wide whose bar is the title bar, with a "File" button 10 pt into
/// the content slot.
struct Win {
    ctx: Context,
    frame: Frame,
    width: f32,
    time: f64,
    maximized: bool,
    fullscreen: bool,
    response: TitleBarResponse,
    slots: Option<BarSlots>,
    regions: Recorded,
    /// A split bar with this title and groups of these widths, instead of the "File" button.
    titled: Option<(Title, f32, f32)>,
    /// An `egui::Modal` is open and the shell calls the drag overlay.
    modal: bool,
    /// A modal window of the rect is open.
    modal_rect: Option<Rect>,
    popup: bool,
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Win {
    fn new(frame: Frame, width: f32) -> Self {
        Self {
            ctx: Context::default(),
            frame,
            width,
            time: 0.0,
            maximized: false,
            fullscreen: false,
            response: TitleBarResponse::default(),
            slots: None,
            regions: Recorded::default(),
            titled: None,
            modal: false,
            modal_rect: None,
            popup: false,
            shapes: Vec::new(),
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> FullOutput {
        self.time += 0.05;
        install(&self.ctx, &Base::suite_default());
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(self.width, HEIGHT))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let v = input.viewports.entry(ViewportId::ROOT).or_default();
        v.maximized = Some(self.maximized);
        v.fullscreen = Some(self.fullscreen);
        // What the app's `raw_input_hook` does.
        release_after_os_resize(&self.ctx, &mut input);
        let (frame, slots) = (&self.frame, &mut self.slots);
        self.regions = Recorded::default();
        let regions = &mut self.regions;
        let (titled, modal, modal_rect, popup) = (&self.titled, self.modal, self.modal_rect, self.popup);
        let mut response = TitleBarResponse::default();
        let mut out = self.ctx.run_ui(input, |ui| {
            let bar = TitleBar::new(frame).regions(regions);
            response = match titled {
                None => bar.show(ui, |ui, s| {
                    *slots = Some(s);
                    let _ = ui.put(Rect::from_min_size(pos2(s.content.left() + 10.0, 0.0), vec2(60.0, H)), egui::Button::new("File"));
                }),
                Some((title, lead, trail)) => bar.title(title.clone()).show_titled(
                    ui,
                    |ui, s| {
                        *slots = Some(s);
                        ui.allocate_exact_size(vec2(*lead, H), egui::Sense::click());
                        LeadingEnd::from_ui(ui)
                    },
                    |ui, _| {
                        ui.allocate_exact_size(vec2(*trail, H), egui::Sense::click());
                    },
                ),
            };
            if popup {
                egui::Popup::open_id(ui.ctx(), egui::Id::unique("test-popup"));
            }
            if modal {
                egui::Modal::new(egui::Id::unique("test-modal")).show(ui.ctx(), |ui| {
                    ui.label("A dialog");
                });
                if let Some(rect) = modal_rect {
                    ModalRects::push(ui.ctx(), ViewportId::ROOT, rect);
                }
                TitleBar::modal_drag_overlay(ui.ctx(), ViewportId::ROOT, &ModalRects::take(ui.ctx(), ViewportId::ROOT));
            }
        });
        self.shapes = out.shapes.clone();
        // A click answers on the frame of its release; keep it for the gesture.
        if response != TitleBarResponse::default() {
            self.response = response;
        }
        out.textures_delta.clear();
        out
    }

    /// Move to `path[0]`, press, move along `path`, release: the viewport commands sent.
    fn gesture(&mut self, path: &[Pos2]) -> Vec<ViewportCommand> {
        self.gesture_by(path, PointerButton::Primary)
    }

    fn gesture_by(&mut self, path: &[Pos2], btn: PointerButton) -> Vec<ViewportCommand> {
        let mut events = vec![vec![Event::PointerMoved(path[0])], vec![button(path[0], true, btn)]];
        events.extend(path[1..].iter().map(|p| vec![Event::PointerMoved(*p)]));
        events.push(vec![button(path.last().copied().unwrap_or(path[0]), false, btn)]);
        events.into_iter().flat_map(|e| commands(self.frame(e))).collect()
    }

    fn click(&mut self, p: Pos2) -> Vec<ViewportCommand> {
        self.response = TitleBarResponse::default();
        self.gesture(&[p])
    }

    fn drag(&mut self, p: Pos2) -> usize {
        let cmds = self.gesture(&[p, p + vec2(12.0, 4.0), p + vec2(30.0, 8.0)]);
        cmds.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count()
    }

    fn caption(&self, c: Caption) -> Rect {
        self.ctx.read_response(caption_id(ViewportId::ROOT, c)).map_or(Rect::NOTHING, |r| r.rect)
    }

    fn content(&self) -> Rect {
        self.slots.map_or(Rect::NOTHING, |s| s.content)
    }

    fn file(&self) -> Pos2 {
        self.content().left_top() + vec2(40.0, H / 2.0)
    }
}

fn commands(mut out: FullOutput) -> Vec<ViewportCommand> {
    out.viewport_output.remove(&ViewportId::ROOT).map(|v| v.commands).unwrap_or_default()
}

fn button(pos: Pos2, pressed: bool, button: PointerButton) -> Event {
    Event::PointerButton { pos, button, pressed, modifiers: Default::default() }
}

fn resizes(cmds: &[ViewportCommand]) -> Vec<ResizeDirection> {
    cmds.iter().filter_map(|c| if let ViewportCommand::BeginResize(d) = c { Some(*d) } else { None }).collect()
}

#[test]
fn windows_buttons_take_the_right_end_and_send_commands() {
    let mut w = Win::new(windows(), 800.0);
    w.frame(vec![]);
    w.frame(vec![]);
    let [min, max, close] = [Caption::Minimize, Caption::Maximize, Caption::Close].map(|c| w.caption(c));
    assert_eq!((close.right(), close.top(), close.height()), (800.0, 0.0, H), "flush with the corner, full height");
    assert!([min, max, close].iter().all(|r| r.width() == WINDOWS_BUTTON_WIDTH));
    assert_eq!((min.right(), max.right()), (max.left(), close.left()));
    assert_eq!(w.content(), Rect::from_min_max(Pos2::ZERO, pos2(800.0 - 3.0 * WINDOWS_BUTTON_WIDTH, H)));
    assert_eq!(w.regions.buttons.iter().map(|b| b.1).collect::<Vec<_>>(), vec![min, max, close]);
    assert_eq!(w.regions.drag, vec![Rect::from_min_size(Pos2::ZERO, vec2(800.0, H))]);

    assert!(w.click(min.center()).contains(&ViewportCommand::Minimized(true)));
    assert_eq!(w.response.caption_clicked, Some(Caption::Minimize));
    assert!(w.click(max.center()).contains(&ViewportCommand::Maximized(true)));
    w.maximized = true;
    assert!(w.click(max.center()).contains(&ViewportCommand::Maximized(false)), "Restore");
}

#[test]
fn close_requests_and_never_closes() {
    let mut w = Win::new(windows(), 800.0);
    w.frame(vec![]);
    let cmds = w.click(w.caption(Caption::Close).center());
    assert_eq!((w.response.caption_clicked, w.response.close_requested), (Some(Caption::Close), true));
    assert!(!cmds.iter().any(|c| matches!(c, ViewportCommand::Close)), "{cmds:?}");
    // Maximized, the window's corner pixel is Close.
    w.maximized = true;
    w.click(pos2(799.5, 0.5));
    assert!(w.response.close_requested);
}

#[test]
fn the_empty_area_drags_and_controls_do_not() {
    let mut w = Win::new(windows(), 800.0);
    w.frame(vec![]);
    let empty = pos2(400.0, H / 2.0);
    assert_eq!(w.drag(empty), 1, "drag on the empty bar");
    assert_eq!(w.drag(w.file()), 0, "drag on the app's button");
    assert_eq!(w.drag(w.caption(Caption::Minimize).center()), 0, "Windows' caption buttons don't drag");
    assert_eq!(w.gesture_by(&[empty, empty + vec2(30.0, 8.0)], PointerButton::Middle), vec![], "middle-button drag");
    let mut cmds = w.click(empty);
    cmds.extend(w.click(empty));
    assert!(cmds.contains(&ViewportCommand::Maximized(true)), "double-click maximizes: {cmds:?}");
}

#[test]
fn resize_zones_never_cover_the_buttons_or_controls() {
    let mut w = Win::new(windows(), 800.0);
    w.frame(vec![]);
    let close = w.caption(Caption::Close);
    // The window's top-right corner and right edge beside Close.
    for p in [pos2(799.0, 1.0), pos2(797.0, 20.0), pos2(close.center().x, 2.0)] {
        let cmds = w.click(p);
        assert!(resizes(&cmds).is_empty() && w.response.close_requested, "{p:?}: {cmds:?}");
    }
    // The top edge over the app's button presses the button.
    let top_of_file = pos2(w.file().x, 2.0);
    assert!(resizes(&w.gesture(&[top_of_file, top_of_file + vec2(0.0, 6.0)])).is_empty());
    // The top edge over the empty bar and the left edge beside the bar still resize.
    assert_eq!(resizes(&w.gesture(&[pos2(400.0, 2.0), pos2(400.0, 8.0)])), vec![ResizeDirection::North]);
    assert_eq!(resizes(&w.gesture(&[pos2(2.0, H / 2.0), pos2(8.0, H / 2.0)])), vec![ResizeDirection::West]);
    assert_eq!(resizes(&w.gesture(&[pos2(2.0, 2.0), pos2(8.0, 8.0)])), vec![ResizeDirection::NorthWest]);
}

#[test]
fn edge_zones_resize_unless_maximized() {
    use ResizeDirection::*;
    let mut w = Win::new(windows(), 1000.0);
    w.frame(vec![]);
    for (p, dir) in [
        (pos2(1.0, 300.0), West),
        (pos2(998.0, 300.0), East),
        (pos2(500.0, 1.0), North),
        (pos2(500.0, 398.0), South),
        (pos2(3.0, 397.0), SouthWest),
        (pos2(998.0, 398.0), SouthEast),
    ] {
        assert_eq!(w.gesture(&[p, p + vec2(6.0, 6.0)]), vec![ViewportCommand::BeginResize(dir)], "at {p:?}");
    }
    // The first motion after those shows the default cursor for a frame.
    w.frame(vec![Event::PointerMoved(pos2(500.0, 300.0))]);
    for (p, cursor) in [
        (pos2(7.0, 300.0), CursorIcon::ResizeWest),
        (pos2(993.0, 300.0), CursorIcon::ResizeEast),
        (pos2(500.0, 7.0), CursorIcon::ResizeNorth),
        (pos2(500.0, 393.0), CursorIcon::ResizeSouth),
        (pos2(3.0, 397.0), CursorIcon::ResizeNeSw),
        (pos2(998.0, 398.0), CursorIcon::ResizeNwSe),
    ] {
        assert_eq!(w.frame(vec![Event::PointerMoved(p)]).platform_output.cursor_icon, cursor, "at {p:?}");
    }
    // Clear of the zone and of egui's interact radius around it.
    assert!(resizes(&w.gesture(&[pos2(20.0, 300.0), pos2(26.0, 306.0)])).is_empty());
    w.maximized = true;
    assert!(resizes(&w.gesture(&[pos2(1.0, 300.0), pos2(8.0, 300.0)])).is_empty());
}

/// KDE Wayland keeps the button release of an OS resize. egui then kept the edge pressed (no hover
/// until the next click) and egui-winit never re-sent the resize cursor that the compositor had
/// replaced: the cursor came back only on every other press.
#[test]
fn after_an_os_resize_the_edge_hovers_and_shows_its_cursor_again() {
    let mut w = Win::new(windows(), 1000.0);
    w.frame(vec![]);
    let p = pos2(3.0, 300.0);
    // Press and drag; the compositor takes over, so no release ever arrives.
    let mut cmds = vec![];
    for e in [vec![Event::PointerMoved(p)], vec![button(p, true, PointerButton::Primary)], vec![Event::PointerMoved(p + vec2(6.0, 0.0))]] {
        cmds.extend(commands(w.frame(e)));
    }
    assert!(cmds.contains(&ViewportCommand::BeginResize(ResizeDirection::West)), "{cmds:?}");
    w.frame(vec![]);
    assert!(!w.ctx.input(|i| i.pointer.primary_down()), "the press ends with the hand-off");
    let out = w.frame(vec![Event::PointerMoved(p)]);
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::Default);
    let out = w.frame(vec![Event::PointerMoved(p + vec2(1.0, 0.0))]);
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::ResizeWest);
    assert!(w.gesture(&[p, p + vec2(6.0, 0.0)]).contains(&ViewportCommand::BeginResize(ResizeDirection::West)));
}

/// Where the OS does send the release of a resize (Windows), no second one is added: a release
/// right after it could count as a click on what lies under the edge.
#[test]
fn a_release_the_os_sends_is_not_doubled() {
    let mut w = Win::new(windows(), 1000.0);
    w.frame(vec![]);
    let p = pos2(3.0, 300.0);
    w.frame(vec![Event::PointerMoved(p)]);
    w.frame(vec![button(p, true, PointerButton::Primary)]);
    w.frame(vec![Event::PointerMoved(p + vec2(6.0, 0.0))]);
    let mut raw = RawInput { events: vec![button(p + vec2(6.0, 0.0), false, PointerButton::Primary)], ..Default::default() };
    release_after_os_resize(&w.ctx, &mut raw);
    assert_eq!(raw.events.len(), 1, "{:?}", raw.events);
    // Or it arrived in an earlier input: egui has the button up, nothing to end.
    let mut w = Win::new(windows(), 1000.0);
    w.frame(vec![]);
    w.gesture(&[p, p + vec2(6.0, 0.0)]);
    let mut raw = RawInput::default();
    release_after_os_resize(&w.ctx, &mut raw);
    assert!(raw.events.is_empty(), "{:?}", raw.events);
}

/// The window manager keeps the button release of an OS move just as of a resize, so egui kept
/// the drag: every other move failed, and so did a resize right after one.
#[test]
fn after_an_os_move_the_bar_and_the_edges_take_the_next_press() {
    fn handed_off(w: &mut Win, path: &[Pos2]) -> usize {
        let mut events = vec![vec![Event::PointerMoved(path[0])], vec![button(path[0], true, PointerButton::Primary)]];
        events.extend(path[1..].iter().map(|p| vec![Event::PointerMoved(*p)]));
        events.push(vec![]);
        events.into_iter().flat_map(|e| commands(w.frame(e))).filter(|c| matches!(c, ViewportCommand::StartDrag)).count()
    }
    let mut w = Win::new(windows(), 1000.0);
    w.frame(vec![]);
    let empty = pos2(500.0, H / 2.0);
    let drag = [empty, empty + vec2(12.0, 4.0), empty + vec2(30.0, 8.0)];
    assert_eq!(handed_off(&mut w, &drag), 1);
    assert!(!w.ctx.input(|i| i.pointer.primary_down()), "the press ends with the hand-off");
    assert_eq!(w.drag(empty), 1, "the next move");
    assert_eq!(handed_off(&mut w, &drag), 1);
    let edge = pos2(3.0, 300.0);
    assert!(w.gesture(&[edge, edge + vec2(6.0, 0.0)]).contains(&ViewportCommand::BeginResize(ResizeDirection::West)), "a resize after a move");
}

#[test]
fn full_screen_has_no_inset_buttons_or_drag() {
    for frame in [windows(), Frame::Inset(MacInset::default())] {
        let mut w = Win::new(frame, 800.0);
        w.fullscreen = true;
        w.frame(vec![]);
        w.frame(vec![]);
        assert_eq!(w.content(), Rect::from_min_size(Pos2::ZERO, vec2(800.0, H)), "{:?}", w.frame);
        assert!(w.ctx.read_response(caption_id(ViewportId::ROOT, Caption::Close)).is_none());
        assert_eq!(w.drag(pos2(400.0, H / 2.0)), 0);
        assert!(resizes(&w.gesture(&[pos2(2.0, 200.0), pos2(8.0, 200.0)])).is_empty());
        assert!(w.regions.drag.is_empty() && w.regions.buttons.is_empty());
    }
}

#[test]
fn native_frame_draws_no_buttons() {
    let mut w = Win::new(Frame::Native, 800.0);
    w.frame(vec![]);
    w.frame(vec![]);
    assert_eq!(w.content(), Rect::from_min_size(Pos2::ZERO, vec2(800.0, H)));
    assert!(w.ctx.read_response(caption_id(ViewportId::ROOT, Caption::Close)).is_none());
    assert!(w.regions.drag.is_empty() && w.regions.buttons.is_empty());
    assert_eq!(w.drag(pos2(400.0, H / 2.0)), 0, "the OS bar moves the window");
    assert!(resizes(&w.gesture(&[pos2(2.0, 200.0), pos2(8.0, 200.0)])).is_empty());
}

#[test]
fn mac_inset_reserves_the_leading_width() {
    let mut w = Win::new(Frame::Inset(MacInset { leading: 78.0, centre_from_top: 14.0 }), 800.0);
    w.frame(vec![]);
    assert_eq!(
        w.slots,
        Some(BarSlots {
            content: Rect::from_min_max(pos2(78.0, 0.0), pos2(800.0, H)),
            leading_caption: 78.0,
            trailing_caption: 0.0,
            centre_line: 14.0,
            height: H,
            gap: Rect::NOTHING
        })
    );
    assert!(w.ctx.read_response(caption_id(ViewportId::ROOT, Caption::Close)).is_none());
    assert_eq!(w.drag(pos2(400.0, H / 2.0)), 1);
    assert!(resizes(&w.gesture(&[pos2(2.0, 200.0), pos2(8.0, 200.0)])).is_empty(), "AppKit resizes");
}

#[test]
fn linux_buttons_sit_where_the_layout_says_and_drag_the_window() {
    let layout = Layout { left: vec![Button::Close], right: vec![Button::Minimize, Button::Maximize] };
    for family in [CaptionFamily::Adwaita, CaptionFamily::Breeze, CaptionFamily::Generic] {
        let (size, gap, end) = metrics(family);
        let width = 1100.0;
        let mut w = Win::new(Frame::Integrated(Captions { family, layout: layout.clone() }), width);
        w.frame(vec![]);
        w.frame(vec![]);
        let [min, max, close] = [Caption::Minimize, Caption::Maximize, Caption::Close].map(|c| w.caption(c));
        // Flush with the window's edges, the edge gap outermost, the hit areas touching.
        assert_eq!((close.left(), close.width()), (0.0, end + size + gap), "{family:?}");
        assert_eq!((max.right(), max.width()), (width, gap + size + end), "{family:?}");
        assert_eq!((min.right(), min.width()), (max.left(), 2.0 * gap + size), "{family:?}");
        assert_eq!((w.content().left(), w.content().right()), (close.right(), min.left()), "{family:?}");
        assert!(w.click(max.center()).contains(&ViewportCommand::Maximized(true)));
        w.click(close.center());
        assert!(w.response.close_requested, "{family:?}");
        // A drag that starts on a button moves the window instead of pressing it.
        let at = min.center();
        let cmds = w.gesture(&[at, at + vec2(12.0, 4.0), at + vec2(30.0, 8.0)]);
        assert_eq!(cmds.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count(), 1, "{cmds:?}");
        assert!(!cmds.contains(&ViewportCommand::Minimized(true)), "{cmds:?}");
    }
}

#[test]
fn right_to_left_mirrors_windows_captions_only() {
    let mut w = Win::new(windows(), 800.0);
    crate::direction::apply(&w.ctx, crate::direction::Direction::Rtl);
    w.frame(vec![]);
    w.frame(vec![]);
    let [min, max, close] = [Caption::Minimize, Caption::Maximize, Caption::Close].map(|c| w.caption(c));
    assert_eq!((close.left(), close.right(), max.right(), min.right()), (0.0, max.left(), min.left(), 3.0 * WINDOWS_BUTTON_WIDTH));
    assert_eq!((w.content().left(), w.content().right()), (3.0 * WINDOWS_BUTTON_WIDTH, 800.0));
    // A desktop's layout is the user's: GNOME's stays as set.
    let mut w = Win::new(Frame::Integrated(Captions { family: CaptionFamily::Adwaita, layout: Layout::default() }), 800.0);
    crate::direction::apply(&w.ctx, crate::direction::Direction::Rtl);
    w.frame(vec![]);
    w.frame(vec![]);
    assert_eq!(w.caption(Caption::Close).right(), 800.0);
}

/// A side without buttons reserves nothing.
#[test]
fn only_sides_with_buttons_take_room() {
    let mut w = Win::new(Frame::Integrated(Captions { family: CaptionFamily::Adwaita, layout: Layout::parse("appmenu:close") }), 1000.0);
    w.frame(vec![]);
    assert_eq!((w.content().left(), w.content().right()), (0.0, 1000.0 - (6.5 + 24.0 + 10.0)));
}

/// Theme glyphs upload once per theme and size; an icon the theme lacks, a wrong-sized mask or an
/// absurd size gives `None`, so the button draws its own glyph.
#[test]
fn theme_icons_are_cached_and_fall_back() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&calls);
    let render = move |icon: CaptionIcon, px: u32| {
        counter.fetch_add(1, Ordering::Relaxed);
        match icon {
            CaptionIcon::Close => Some(vec![255; (px * px) as usize]),
            CaptionIcon::Minimize => Some(vec![255; 3]),
            _ => None,
        }
    };
    let icons = CaptionIcons { theme: "Test".into(), render: std::sync::Arc::new(render) };
    let ctx = Context::default();
    let mut out = ctx.run_ui(RawInput::default(), |ui| {
        let tex = theme_icon(ui.ctx(), &icons, CaptionIcon::Close, 16);
        assert_eq!(tex.map(|t| t.size()), Some([16, 16]));
        assert!(theme_icon(ui.ctx(), &icons, CaptionIcon::Close, 16).is_some());
        assert!(theme_icon(ui.ctx(), &icons, CaptionIcon::Restore, 16).is_none());
        assert!(theme_icon(ui.ctx(), &icons, CaptionIcon::Restore, 16).is_none());
        assert!(theme_icon(ui.ctx(), &icons, CaptionIcon::Minimize, 16).is_none(), "a mask of the wrong size");
        assert!(theme_icon(ui.ctx(), &icons, CaptionIcon::Close, 0).is_none());
        assert!(theme_icon(ui.ctx(), &icons, CaptionIcon::Close, 100_000).is_none());
    });
    out.textures_delta.clear();
    assert_eq!(calls.load(Ordering::Relaxed), 3, "each icon and size rendered once; absurd sizes never");
}

#[test]
fn subtract_leaves_only_the_parts_outside_the_holes() {
    let r = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 8.0));
    let pieces = subtract(r, &[Rect::from_min_max(pos2(20.0, 0.0), pos2(40.0, 32.0)), Rect::from_min_max(pos2(90.0, -5.0), pos2(120.0, 5.0))]);
    let area: f32 = pieces.iter().map(|p| p.area()).sum();
    assert_eq!(area, 800.0 - 20.0 * 8.0 - 10.0 * 5.0);
    for p in [pos2(30.0, 4.0), pos2(95.0, 2.0)] {
        assert!(!pieces.iter().any(|r| r.contains(p)), "{p:?} is in a hole");
    }
    for p in [pos2(10.0, 4.0), pos2(60.0, 4.0), pos2(95.0, 7.0)] {
        assert!(pieces.iter().any(|r| r.contains(p)), "{p:?} is outside the holes");
    }
}

/// Undecorated windows get the bar's resize zones in every family until the window procedure
/// answers the hit test itself, as the app says by the builder flag or through the regions sink.
#[test]
fn resize_zones_stay_until_a_native_hit_test_takes_over() {
    struct Sink(bool);
    impl CaptionRegions for Sink {
        fn native_hit_test(&self) -> bool {
            self.0
        }
    }
    /// Whether a drag from the left edge below the bar resizes the window.
    fn resizes_from_the_edge(flag: bool, sink: bool) -> bool {
        let (ctx, edge) = (Context::default(), pos2(2.0, 200.0));
        let steps = [
            vec![],
            vec![Event::PointerMoved(edge)],
            vec![button(edge, true, PointerButton::Primary)],
            vec![Event::PointerMoved(edge + vec2(6.0, 0.0))],
        ];
        let mut cmds = vec![];
        for (i, events) in steps.into_iter().enumerate() {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))),
                time: Some(i as f64 * 0.05),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                TitleBar::new(&windows()).native_hit_test(flag).regions(&mut Sink(sink)).show(ui, |_, _| {});
            });
            // No renderer here to take the font atlas.
            out.textures_delta.clear();
            cmds.extend(commands(out));
        }
        !resizes(&cmds).is_empty()
    }
    assert!(resizes_from_the_edge(false, false), "Windows captions, no native hit test");
    assert!(!resizes_from_the_edge(true, false), "the app's flag");
    assert!(!resizes_from_the_edge(false, true), "the regions sink");
}

/// The zones' clip must not be the panel the bar sits in.
#[test]
fn resize_zones_work_below_a_top_panel() {
    let (ctx, edge) = (Context::default(), pos2(798.0, 200.0));
    let steps =
        [vec![], vec![Event::PointerMoved(edge)], vec![button(edge, true, PointerButton::Primary)], vec![Event::PointerMoved(edge - vec2(6.0, 0.0))]];
    let mut cmds = vec![];
    for (i, events) in steps.into_iter().enumerate() {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))),
            time: Some(i as f64 * 0.05),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::Panel::top("bar").show(ui, |ui| {
                TitleBar::new(&windows()).show(ui, |_, _| {});
            });
        });
        out.textures_delta.clear();
        cmds.extend(commands(out));
    }
    assert_eq!(resizes(&cmds), vec![ResizeDirection::East], "{cmds:?}");
}

fn slots_of(frame: Frame, rtl: bool) -> BarSlots {
    let mut w = Win::new(frame, 800.0);
    if rtl {
        crate::direction::apply(&w.ctx, crate::direction::Direction::Rtl);
    }
    w.frame(vec![]);
    w.frame(vec![]);
    w.slots.unwrap()
}

#[test]
fn bar_slots_report_the_caption_widths() {
    let w = 3.0 * WINDOWS_BUTTON_WIDTH;
    let s = slots_of(windows(), false);
    assert_eq!((s.leading_caption, s.trailing_caption), (0.0, w));
    assert_eq!((s.content.left(), s.content.right()), (0.0, 800.0 - w));

    let s = slots_of(Frame::Inset(MacInset { leading: 78.0, centre_from_top: 14.0 }), false);
    assert_eq!((s.leading_caption, s.trailing_caption, s.content.left()), (78.0, 0.0, 78.0));

    let gnome = Frame::Integrated(Captions { family: CaptionFamily::Adwaita, layout: Layout::parse("close:appmenu") });
    let s = slots_of(gnome.clone(), false);
    assert!(s.leading_caption > 0.0 && s.trailing_caption == 0.0, "{s:?}");
    assert_eq!((s.content.left(), s.content.right()), (s.leading_caption, 800.0));
    // RTL: the physical left side is the trailing one.
    let s = slots_of(gnome, true);
    assert!(s.trailing_caption > 0.0 && s.leading_caption == 0.0, "{s:?}");
    assert_eq!(s.content.left(), s.trailing_caption);

    let s = slots_of(windows(), true);
    assert_eq!((s.leading_caption, s.trailing_caption), (0.0, w), "the buttons move to the physical left, the trailing end");
    assert_eq!((s.content.left(), s.content.right()), (w, 800.0));
}

#[test]
fn separator_is_a_hairline_on_the_bars_bottom_edge() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))), time: Some(0.0), ..Default::default() };
    let mut colour = Color32::TRANSPARENT;
    let mut out = ctx.run_ui(input, |ui| {
        colour = TitleBar::style(&Tokens::of(ui)).separator;
        TitleBar::new(&windows()).separator(true).show(ui, |_, _| {});
    });
    out.textures_delta.clear();
    let found = out
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == colour && r.rect == Rect::from_min_max(pos2(0.0, H - 1.0), pos2(800.0, H))));
    assert!(found);
}

#[test]
fn zones_alone_resize_without_a_bar() {
    fn resizes_from_the_edge(frame: &Frame, native: bool) -> bool {
        let (ctx, edge) = (Context::default(), pos2(2.0, 200.0));
        let steps = [
            vec![],
            vec![Event::PointerMoved(edge)],
            vec![button(edge, true, PointerButton::Primary)],
            vec![Event::PointerMoved(edge + vec2(6.0, 0.0))],
        ];
        let mut cmds = vec![];
        for (i, events) in steps.into_iter().enumerate() {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))),
                time: Some(i as f64 * 0.05),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| resize_zones_only(ui, frame, native));
            out.textures_delta.clear();
            cmds.extend(commands(out));
        }
        !resizes(&cmds).is_empty()
    }
    assert!(resizes_from_the_edge(&windows(), false));
    assert!(!resizes_from_the_edge(&Frame::Native, false));
    assert!(!resizes_from_the_edge(&windows(), true));
}

fn tokens(size: craft_theme::Size, mode: craft_theme::Mode, theme: &craft_theme::ThemeDef) -> crate::theme::UiTokens {
    use craft_theme::{Appearance, Contrast, TextSize, resolve};
    let appearance = Appearance { mode, contrast: Contrast::Standard, size, text_size: TextSize::default(), reduced_motion: false };
    crate::theme::UiTokens::new(resolve(theme, appearance, None).tokens, appearance, mode, crate::theme::FontFamilies::default())
}

/// The style is the `title_bar` family in every built-in theme, mode and size; the family's
/// contrast pairs run in the component contrast tests of `craft-theme`.
#[test]
fn the_style_is_the_family_in_every_built_in() {
    use crate::theme::ToEgui as _;
    use craft_theme::{Mode, Size};
    for theme in craft_theme::builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            for size in [Size::Compact, Size::Comfortable] {
                let t = tokens(size, mode, theme);
                let (s, f) = (TitleBar::style(&t), &t.component.title_bar);
                assert_eq!((s.height, s.pad, s.edge, s.corner), (f.height, f.pad, f.edge, f.corner), "{}", theme.id.as_str());
                let colours = [s.bar, s.title, s.glyph, s.glyph_inactive, s.hover, s.pressed, s.separator];
                let family = [f.bar, f.title, f.glyph, f.glyph_inactive, f.hover, f.pressed, f.separator].map(|c| c.to_egui());
                assert_eq!(colours, family, "{}", theme.id.as_str());
            }
        }
    }
}

/// The bar is `size.bar.title` tall; with Adwaita's buttons it is at least libadwaita's 47 pt.
#[test]
fn the_bar_height_follows_the_size_preset_and_adwaita() {
    use craft_theme::Size;
    let adwaita = Frame::Integrated(Captions { family: CaptionFamily::Adwaita, layout: Layout::default() });
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let height = |frame: &Frame, size: Size| {
        let mut got = None;
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))), ..Default::default() };
        ctx.run_ui(input, |ui| {
            crate::theme::scope(
                ui,
                |o| o.size = Some(size),
                |ui| {
                    TitleBar::new(frame).show(ui, |_, s| got = Some(s.height));
                },
            );
        })
        .textures_delta
        .clear();
        got.unwrap_or_default()
    };
    assert_eq!(height(&windows(), Size::Compact), 32.0);
    assert_eq!(height(&windows(), Size::Comfortable), 38.0);
    assert_eq!(height(&adwaita, Size::Compact), 47.0);
    assert_eq!(height(&adwaita, Size::Comfortable), 47.0);
    assert_eq!(height(&Frame::Native, Size::Comfortable), 38.0);
}

/// Two windows: the hand-off of one OS move does not end the press in the other, and each bar's
/// caption buttons have ids of their own.
#[test]
fn two_windows_keep_their_hand_offs_and_caption_ids_apart() {
    let (a, b) = (ViewportId::ROOT, ViewportId::from_hash_of("second"));
    let ctx = Context::default();
    let frame = windows();
    let time = std::cell::Cell::new(0.0);
    let run = |viewport: ViewportId, salt: &str, events: Vec<Event>| -> Vec<ViewportCommand> {
        install(&ctx, &Base::suite_default());
        time.set(time.get() + 0.05);
        let mut input = RawInput {
            viewport_id: viewport,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))),
            time: Some(time.get()),
            events,
            ..Default::default()
        };
        input.viewports.entry(viewport).or_default();
        release_after_os_resize(&ctx, &mut input);
        let mut out = ctx.run_ui(input, |ui| {
            TitleBar::new(&frame).id_salt(salt).show(ui, |_, _| {});
        });
        out.textures_delta.clear();
        out.viewport_output.remove(&viewport).map(|v| v.commands).unwrap_or_default()
    };
    let empty = pos2(400.0, H / 2.0);
    run(a, "a", vec![]);
    run(b, "b", vec![]);
    let mut cmds = vec![];
    for events in
        [vec![Event::PointerMoved(empty)], vec![button(empty, true, PointerButton::Primary)], vec![Event::PointerMoved(empty + vec2(12.0, 4.0))]]
    {
        cmds.extend(run(a, "a", events));
    }
    assert!(cmds.contains(&ViewportCommand::StartDrag), "{cmds:?}");
    run(b, "b", vec![Event::PointerMoved(empty)]);
    run(b, "b", vec![button(empty, true, PointerButton::Primary)]);
    run(b, "b", vec![]);
    assert!(ctx.input_for(b, |i| i.pointer.primary_down()), "the other window's press stays");
    run(a, "a", vec![]);
    assert!(!ctx.input_for(a, |i| i.pointer.primary_down()), "the window that handed off to the OS ends its press");

    assert_ne!(caption_id("a", Caption::Close), caption_id("b", Caption::Close));
    assert_ne!(caption_id("a", Caption::Close), caption_id(a, Caption::Close));
    assert!(ctx.read_response(caption_id("a", Caption::Close)).is_some());
    assert!(ctx.read_response(caption_id(a, Caption::Close)).is_none());
}

// DS-36c: the title, the split slots, the drag overlay under a modal.

const TITLE: &str = "Poster.psd";
const LONG: &str = "Poster.psd and a document name too long for the gap";

fn titled(frame: Frame, width: f32, lead: f32, trail: f32) -> Win {
    let mut w = Win::new(frame, width);
    w.titled = Some((Title::new(TITLE), lead, trail));
    w
}

/// The title as the last frame painted it.
fn painted_title(w: &Win) -> Option<Rect> {
    w.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == TITLE => Some(Rect::from_min_size(t.pos, t.galley.size())),
        _ => None,
    })
}

fn starts(cmds: &[ViewportCommand]) -> usize {
    cmds.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count()
}

#[test]
fn title_x_centres_slides_or_gives_up() {
    use super::title::title_x;
    // Wide window: centred.
    assert_eq!(title_x(800.0, 420.0, 1300.0, 60.0, 16.0), Some(770.0));
    // Centre would hit the menus: slid right into the gap.
    assert_eq!(title_x(400.0, 420.0, 1300.0, 60.0, 16.0), Some(436.0));
    // Centre would hit the controls: slid left.
    assert_eq!(title_x(1280.0, 420.0, 1300.0, 60.0, 16.0), Some(1224.0));
    // No room for the whole title.
    assert_eq!(title_x(400.0, 420.0, 480.0, 60.0, 16.0), None);
    assert_eq!(title_x(f32::NAN, 420.0, 1300.0, 60.0, 16.0), Some(436.0));
    assert_eq!(title_x(400.0, f32::INFINITY, 1300.0, 60.0, 16.0), None);
}

#[test]
fn the_title_centres_between_the_groups_and_the_gap_is_reported() {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    w.frame(vec![]);
    w.frame(vec![]);
    let pad = crate::theme::current(&w.ctx).component.title_bar.pad;
    let rect = painted_title(&w).expect("title");
    assert!((rect.center().x - 400.0).abs() < 0.01, "{rect:?}");
    assert!((rect.center().y - H / 2.0).abs() < 0.5, "{rect:?}");
    let right = 800.0 - 3.0 * WINDOWS_BUTTON_WIDTH;
    let gap = w.slots.expect("slots").gap;
    assert_eq!((gap.left(), gap.right(), gap.top(), gap.bottom()), (pad + 100.0, right - 100.0, 0.0, H));

    // A narrower window: what the last pass measured at another width is not used.
    w.width = 700.0;
    w.frame(vec![]);
    assert_eq!(w.slots.expect("slots").gap, Rect::NOTHING);
    w.frame(vec![]);
    assert!(w.slots.expect("slots").gap.is_positive());
}

#[test]
fn a_title_that_does_not_fit_is_elided_at_the_menus_and_a_small_gap_hides_it() {
    // Gap 108..262: 154 pt, more than the 80 pt minimum, less than the title needs.
    let mut w = titled(windows(), 500.0, 100.0, 100.0);
    w.titled = Some((Title::new(LONG).unsaved(true), 100.0, 100.0));
    w.frame(vec![]);
    w.frame(vec![]);
    let gap = w.slots.expect("slots").gap;
    let shown = w
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with(TITLE) => Some(t.clone()),
            _ => None,
        })
        .expect("an elided title");
    let rect = Rect::from_min_size(shown.pos, shown.galley.size());
    assert_eq!(rect.left(), gap.left() + 16.0, "starts beside the menus");
    assert!(rect.right() <= gap.right() - 16.0 + 0.01, "{rect:?} in {gap:?}");
    let drawn: String = shown.galley.rows.iter().map(|r| r.text()).collect();
    assert!(drawn.ends_with('\u{2026}'), "{drawn:?}");

    // 54 pt is under the minimum.
    let mut w = titled(windows(), 400.0, 100.0, 100.0);
    w.frame(vec![]);
    w.frame(vec![]);
    assert!(painted_title(&w).is_none());
}

#[test]
fn the_title_mirrors_in_a_right_to_left_ui() {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    crate::direction::apply(&w.ctx, crate::direction::Direction::Rtl);
    w.frame(vec![]);
    w.frame(vec![]);
    let caption = 3.0 * WINDOWS_BUTTON_WIDTH;
    let pad = crate::theme::current(&w.ctx).component.title_bar.pad;
    // The menus' group at the right end, the controls' at the left beside the buttons.
    let gap = w.slots.expect("slots").gap;
    assert_eq!((gap.left(), gap.right()), (caption + 100.0, 800.0 - pad - 100.0));
    assert!(painted_title(&w).is_some());

    // Elided: it starts at the menus' side, the right.
    let mut w = titled(windows(), 500.0, 100.0, 100.0);
    crate::direction::apply(&w.ctx, crate::direction::Direction::Rtl);
    w.titled = Some((Title::new(LONG).unsaved(true), 100.0, 100.0));
    w.frame(vec![]);
    w.frame(vec![]);
    let gap = w.slots.expect("slots").gap;
    let rect = w
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with(TITLE) => Some(Rect::from_min_size(t.pos, t.galley.size())),
            _ => None,
        })
        .expect("an elided title");
    assert!((rect.right() - (gap.right() - 16.0)).abs() < 0.01, "{rect:?} in {gap:?}");
}

#[test]
fn a_native_frame_draws_no_title_and_pads_both_ends() {
    let mut w = titled(Frame::Native, 800.0, 100.0, 100.0);
    w.frame(vec![]);
    w.frame(vec![]);
    assert!(painted_title(&w).is_none());
    let pad = crate::theme::current(&w.ctx).component.title_bar.pad;
    let s = w.slots.expect("slots");
    assert_eq!((s.content.left(), s.content.right()), (pad, 800.0 - pad));

    // The macOS inset replaces the margin at the leading end; the trailing end keeps its own.
    let mut w = titled(Frame::Inset(MacInset { leading: 78.0, centre_from_top: 14.0 }), 800.0, 100.0, 100.0);
    w.frame(vec![]);
    let s = w.slots.expect("slots");
    assert_eq!((s.content.left(), s.content.right()), (78.0, 800.0 - pad));
    assert!(painted_title(&w).is_some(), "the inset window draws its own title");
}

#[test]
fn a_press_on_the_title_drags_the_window() {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    w.frame(vec![]);
    w.frame(vec![]);
    let at = painted_title(&w).expect("title").center();
    assert_eq!(w.drag(at), 1);
}

#[test]
fn the_drag_area_is_a_title_bar_node_named_by_the_title_and_not_clickable() {
    use egui::accesskit::{Action, Role};
    let mut w = Win::new(windows(), 800.0);
    w.titled = Some((Title::new(TITLE).unsaved(true), 100.0, 100.0));
    w.ctx.enable_accesskit();
    let mut update = None;
    for _ in 0..2 {
        update = w.frame(vec![]).platform_output.accesskit_update;
    }
    let nodes = update.expect("an update").nodes;
    let node = nodes.iter().map(|(_, n)| n).find(|n| n.role() == Role::TitleBar).expect("a title bar node");
    assert_eq!(node.label(), Some("Poster.psd \u{2022}"));
    assert!(!node.supports_action(Action::Click) && !node.supports_action(Action::Focus));
}

#[test]
fn the_bar_background_is_painted_unless_the_app_does_it() {
    let fill = |background: bool| {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, HEIGHT))), time: Some(0.0), ..Default::default() };
        let mut colour = Color32::TRANSPARENT;
        let mut out = ctx.run_ui(input, |ui| {
            colour = TitleBar::style(&Tokens::of(ui)).bar;
            TitleBar::new(&windows()).background(background).show(ui, |_, _| {});
        });
        out.textures_delta.clear();
        let bar = Rect::from_min_size(Pos2::ZERO, vec2(800.0, H));
        out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == colour && r.rect == bar))
    };
    assert!(fill(true));
    assert!(!fill(false));
}

#[test]
fn each_window_keeps_its_own_gap() {
    let mut a = titled(windows(), 800.0, 100.0, 100.0);
    let mut b = titled(windows(), 800.0, 200.0, 50.0);
    for _ in 0..2 {
        a.frame(vec![]);
        b.frame(vec![]);
    }
    assert_ne!(a.slots.expect("slots").gap, b.slots.expect("slots").gap);
    // One context, two window keys.
    let ctx = Context::default();
    let (one, two) = (captions::window_scope("one"), captions::window_scope("two"));
    measure::store(&ctx, one, 800.0, Rect::from_min_max(pos2(1.0, 0.0), pos2(2.0, 3.0)));
    measure::store(&ctx, two, 800.0, Rect::from_min_max(pos2(4.0, 0.0), pos2(5.0, 3.0)));
    assert_eq!(measure::drawn(&ctx, one).map(|r| r.left()), Some(1.0));
    assert_eq!(measure::drawn(&ctx, two).map(|r| r.left()), Some(4.0));
}

/// A modal window is open, so the bar's own drag area gets no input; the overlay over the gap does.
fn under_a_modal() -> Win {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    w.modal = true;
    w.frame(vec![]);
    w.frame(vec![]);
    w.frame(vec![]);
    w
}

const IN_THE_GAP: Pos2 = pos2(300.0, H / 2.0);

#[test]
fn a_drag_in_the_gap_under_a_modal_moves_the_window_once() {
    let mut w = under_a_modal();
    assert_eq!(w.drag(IN_THE_GAP), 1);
    // The bar's controls get nothing under the modal, and the overlay does not reach them.
    let controls = w.gesture(&[pos2(30.0, H / 2.0), pos2(40.0, H / 2.0), pos2(60.0, H / 2.0)]);
    assert_eq!(starts(&controls), 0);
}

#[test]
fn a_middle_button_drag_under_a_modal_moves_nothing() {
    let mut w = under_a_modal();
    let cmds = w.gesture_by(&[IN_THE_GAP, IN_THE_GAP + vec2(12.0, 4.0), IN_THE_GAP + vec2(30.0, 8.0)], PointerButton::Middle);
    assert_eq!(starts(&cmds), 0);
}

#[test]
fn a_double_click_in_the_gap_under_a_modal_maximizes() {
    let mut w = under_a_modal();
    let cmds: Vec<_> = (0..2).flat_map(|_| w.gesture(&[IN_THE_GAP])).collect();
    assert!(cmds.iter().any(|c| matches!(c, ViewportCommand::Maximized(true))), "{cmds:?}");
}

#[test]
fn a_dialog_rect_over_the_gap_turns_the_overlay_off() {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    w.modal = true;
    w.modal_rect = Some(Rect::from_min_max(pos2(250.0, 0.0), pos2(450.0, 80.0)));
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(w.drag(IN_THE_GAP), 0, "a dialog dragged into the title bar stays in front");
}

#[test]
fn an_open_popup_turns_the_overlay_off() {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    w.modal = true;
    w.popup = true;
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(w.drag(IN_THE_GAP), 0);
}

#[test]
fn without_a_modal_there_is_no_overlay_and_the_bar_drags_by_itself() {
    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(w.drag(IN_THE_GAP), 1);
    assert!(w.ctx.memory(|m| m.areas().visible_layer_ids()).iter().all(|l| l.order != egui::Order::Tooltip));
}

#[test]
fn the_overlay_needs_a_titled_bar_outside_full_screen() {
    let mut w = titled(Frame::Native, 800.0, 100.0, 100.0);
    w.modal = true;
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(w.drag(IN_THE_GAP), 0, "the OS bar moves a native window");

    let mut w = titled(windows(), 800.0, 100.0, 100.0);
    w.modal = true;
    w.fullscreen = true;
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(w.drag(IN_THE_GAP), 0);
}

#[test]
fn the_menu_titles_and_the_help_search_field_stay_clear_of_the_gap() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let frame = windows();
    let menus = crate::menu::bar::story_bar();
    let mut time = 0.0;
    // The titles' rect, the gap the pass was given and the search field's bounds, if shown.
    let mut run = |events: Vec<Event>| {
        time += 0.05;
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, HEIGHT))), time: Some(time), events, ..Default::default() };
        let (mut titles, mut gap) = (Rect::NOTHING, Rect::NOTHING);
        let mut out = ctx.run_ui(input, |ui| {
            TitleBar::new(&frame).title(Title::new(TITLE)).show_titled(
                ui,
                |ui, s| {
                    gap = s.gap;
                    titles = crate::menu::MenuBar::new(&menus).help_search(menus.len() - 1).show(ui).titles;
                    LeadingEnd::from_ui(ui)
                },
                |ui, _| {
                    ui.allocate_exact_size(vec2(60.0, H), egui::Sense::click());
                },
            );
        });
        out.textures_delta.clear();
        let field = out.platform_output.accesskit_update.take().and_then(|update| {
            update.nodes.iter().map(|(_, n)| n).find(|n| matches!(n.role(), Role::TextInput | Role::SearchInput)).and_then(|n| n.bounds())
        });
        (titles, gap, field)
    };
    for _ in 0..3 {
        run(vec![]);
    }
    let (titles, ..) = run(vec![]);
    let help = pos2(titles.right() - 10.0, H / 2.0);
    run(vec![Event::PointerMoved(help)]);
    run(vec![button(help, true, PointerButton::Primary)]);
    run(vec![button(help, false, PointerButton::Primary)]);
    let mut shown = None;
    let (mut titles, mut gap) = (titles, Rect::NOTHING);
    for _ in 0..3 {
        let (t, g, f) = run(vec![]);
        (titles, gap) = (t, g);
        shown = shown.or(f);
    }
    let b = shown.expect("the open Help menu shows its search field");
    let field = Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32));
    assert!(!field.intersects(titles) && !field.intersects(gap), "{field:?} vs titles {titles:?}, gap {gap:?}");
    assert_eq!(gap.left(), titles.right(), "the gap starts at the titles' inner edge, not the bar's");
}
