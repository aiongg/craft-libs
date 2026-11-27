//! Count trials for the egui embed bridge, on `egui::Context::run_ui` with synthetic input: a pass
//! runs, paints and forwards only what it must.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use craft_theme::{Appearance, Contrast, FontSlot, Mode, TextSize, builtins, resolve};
use craft_ui_embed_egui::{EmbedStats, EmbeddedWindow};
use craft_ui_rt::{
    BuildCx, Constraints, Element, Event, EventCx, FontSpec, Gesture, Handled, InteractFact, Key, LayoutCx, Overflow, PaintCx, Pos2, Rect, Rgba,
    Sense, Show, State, TextLayout, TextSpec, Tokens, Vec2,
};
use egui::{Modifiers, PointerButton, RawInput};

const ROW: f32 = 20.0;
const ROWS: usize = 5;
const PANEL: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(210.0, 10.0 + ROW * ROWS as f32) };

fn tokens() -> Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

#[derive(Default)]
struct Log {
    painted: Vec<usize>,
    keys: u32,
}

/// What the rows read and send: the selected row (set by the host) and the clicks (the outbox
/// the host applies).
#[derive(Clone, Copy)]
struct Model {
    selected: State<Option<usize>>,
    outbox: State<Vec<usize>>,
}

struct List {
    model: Model,
    log: Rc<RefCell<Log>>,
}

impl Element for List {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for index in 0..ROWS {
            cx.keyed(index, |cx| Row { index, model: self.model, log: self.log.clone() }.show(cx));
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for i in 0..cx.child_count() {
            let row = cx.child_at(i);
            cx.layout_child(row, Constraints::tight(Vec2::new(c.max.x, ROW)));
            cx.place_child(row, Vec2::new(0.0, i as f32 * ROW));
        }
        c.max
    }
}

struct Row {
    index: usize,
    model: Model,
    log: Rc<RefCell<Log>>,
}

impl Element for Row {
    type Output = ();
    type Local = Option<TextLayout>;

    fn props_key(&self) -> u64 {
        self.index as u64
    }

    fn init(&self, _: &mut BuildCx<'_>) -> Option<TextLayout> {
        None
    }

    fn layout(&self, local: &mut Option<TextLayout>, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let spec = TextSpec {
            text: format!("Layer {}", self.index).into(),
            font: FontSpec { slot: FontSlot::Ui, size: 12.0 },
            line_height: None,
            color: Rgba::WHITE,
            max_width: None,
            overflow: Overflow::default(),
        };
        *local = Some(cx.text_layout(&spec));
        c.max
    }

    fn paint(&self, local: &Option<TextLayout>, cx: &mut PaintCx<'_>) {
        self.log.borrow_mut().painted.push(self.index);
        let bounds = cx.bounds();
        let me = cx.node();
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
        let selected = self.model.selected.get(cx) == Some(self.index);
        if selected {
            cx.fill_rect(bounds, Rgba::rgb(40, 90, 200));
        } else if cx.read_interaction(me, InteractFact::Hovered) {
            cx.fill_rect(bounds, Rgba::rgb(60, 60, 60));
        }
        if let Some(text) = local {
            cx.text(text, Pos2::new(4.0, 3.0), None);
        }
    }

    fn event(&self, _: &mut Option<TextLayout>, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => {
                // Taking the press makes the row its owner, so the release comes back here.
                if matches!(p.gesture, Gesture::Up { click: true, .. }) {
                    self.model.outbox.update(cx, |q| q.push(self.index));
                }
                Handled::Yes
            }
            Event::Key(k) if k.key == Key::Space && k.down => {
                self.log.borrow_mut().keys += 1;
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}

struct Harness {
    ctx: egui::Context,
    embed: EmbeddedWindow,
    model: Model,
    log: Rc<RefCell<Log>>,
    repaints: Arc<AtomicU32>,
    time: f64,
}

/// One pass's result: the bridge's stats, egui's events left after the panel, and the meshes
/// egui got.
struct Pass {
    stats: EmbedStats,
    left: Vec<egui::Event>,
    meshes: usize,
}

impl Harness {
    fn new() -> Harness {
        let ctx = egui::Context::default();
        let repaints = Arc::new(AtomicU32::new(0));
        let counter = repaints.clone();
        ctx.set_request_repaint_callback(move |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        let mut embed = EmbeddedWindow::new(&ctx, egui::Id::unique("layers"), tokens());
        let store = embed.runtime_mut().store_mut();
        let model = Model { selected: store.create(None), outbox: store.create(Vec::new()) };
        let log = Rc::new(RefCell::new(Log::default()));
        let list_log = log.clone();
        embed.set_root(move |cx| {
            cx.child(List { model, log: list_log.clone() });
        });
        let mut h = Harness { ctx, embed, model, log, repaints, time: 0.0 };
        // Fonts, the first frame and egui's own first-pass repaints.
        for _ in 0..4 {
            h.pass(Vec::new());
        }
        h.repaints.store(0, Ordering::Relaxed);
        h.log.borrow_mut().painted.clear();
        h
    }

    fn pass(&mut self, events: Vec<egui::Event>) -> Pass {
        self.time += 1.0 / 60.0;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let mut stats = EmbedStats::default();
        let mut left = Vec::new();
        let embed = &mut self.embed;
        let output = self.ctx.run_ui(raw, |ui| {
            stats = embed.show(ui, PANEL).stats;
            left = ui.input(|i| i.events.clone());
        });
        let meshes = output.shapes.iter().filter(|s| matches!(s.shape, egui::Shape::Mesh(_))).count();
        output.drop_without_applying_deltas();
        Pass { stats, left, meshes }
    }

    /// The host's part of a round trip: applies the panel's clicks to the model.
    fn apply_outbox(&mut self) {
        let store = self.embed.runtime_mut().store_mut();
        let clicks = store.peek(self.model.outbox).clone();
        if let Some(&last) = clicks.last() {
            self.model.outbox.set(store, Vec::new());
            self.model.selected.set(store, Some(last));
        }
    }

    fn painted(&self) -> Vec<usize> {
        std::mem::take(&mut self.log.borrow_mut().painted)
    }
}

fn row_center(index: usize) -> Pos2 {
    Pos2::new(PANEL.center().x, PANEL.min.y + (index as f32 + 0.5) * ROW)
}

fn key(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }
}

#[test]
fn idle_passes_run_no_frame_and_ask_for_no_repaint() {
    let mut h = Harness::new();
    for _ in 0..10 {
        let pass = h.pass(Vec::new());
        assert_eq!(pass.stats.frames, 0, "an idle pass ran an rt frame");
        assert!(pass.meshes > 0, "the kept meshes are added on every pass");
    }
    assert_eq!(h.repaints.load(Ordering::Relaxed), 0, "idle passes asked egui for a repaint");
    assert!(h.painted().is_empty());
}

#[test]
fn hover_paints_only_the_hovered_row() {
    let mut h = Harness::new();
    let pass = h.pass(vec![egui::Event::PointerMoved(row_center(2))]);
    assert_eq!(pass.stats.frames, 1);
    assert_eq!(h.painted(), [2]);
    h.pass(vec![egui::Event::PointerMoved(row_center(3))]);
    let mut painted = h.painted();
    painted.sort_unstable();
    assert_eq!(painted, [2, 3], "the row left and the row entered repaint, no other");
}

#[test]
fn a_pointer_outside_the_panel_forwards_nothing() {
    let mut h = Harness::new();
    let outside = Pos2::new(300.0, 250.0);
    let pass = h.pass(vec![
        egui::Event::PointerMoved(outside),
        egui::Event::PointerButton { pos: outside, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        egui::Event::PointerButton { pos: outside, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: Vec2::new(0.0, -1.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        },
    ]);
    assert_eq!(pass.stats.events, 0);
    assert_eq!(pass.stats.frames, 0);
    assert!(h.painted().is_empty());
}

#[test]
fn keys_the_window_does_not_take_stay_in_egui_input() {
    let mut h = Harness::new();
    // egui's Tab focus move reaches the panel; the window then enters at its first stop.
    h.pass(vec![key(egui::Key::Tab, true), key(egui::Key::Tab, false)]);
    h.pass(Vec::new());
    assert!(h.ctx.memory(|m| m.has_focus(h.embed.id())), "Tab gave the panel egui focus");
    let pass = h.pass(vec![key(egui::Key::Space, true), key(egui::Key::B, true)]);
    assert_eq!(h.log.borrow().keys, 1, "the focused row took Space");
    let keys: Vec<egui::Key> = pass.left.iter().filter_map(|e| if let egui::Event::Key { key, .. } = e { Some(*key) } else { None }).collect();
    assert_eq!(keys, [egui::Key::B], "Space was taken, B stays for the app's shortcuts");
}

#[test]
fn a_click_runs_one_frame_and_the_host_resync_one_more() {
    let mut h = Harness::new();
    let at = row_center(1);
    let press = |pressed| egui::Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let pass = h.pass(vec![egui::Event::PointerMoved(at), press(true), press(false)]);
    assert_eq!(pass.stats.frames, 1, "the click ran one rt frame");
    assert_eq!(h.embed.runtime().store().peek(h.model.outbox), &vec![1]);
    h.apply_outbox();
    let pass = h.pass(Vec::new());
    assert_eq!(pass.stats.frames, 1, "the host's model change ran one rt frame");
    assert_eq!(h.pass(Vec::new()).stats.frames, 0);
}

#[test]
fn a_glyph_atlas_reset_remakes_the_meshes() {
    let mut h = Harness::new();
    // Different definitions make new fonts with a new atlas, as a full atlas's reset does.
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.entry(egui::FontFamily::Monospace).or_default().reverse();
    h.ctx.set_fonts(fonts);
    let pass = h.pass(Vec::new());
    assert_eq!(pass.stats.frames, 1, "the new atlas made the meshes again in the same pass");
    assert!(pass.stats.prepared >= ROWS as u32, "every row's text was tessellated again");
    assert!(h.painted().is_empty(), "no node painted again: only the meshes were remade");
    assert_eq!(h.pass(Vec::new()).stats.frames, 0);
}
