//! Behavior tests for the icon rail, driven by egui pointer and key events. The harness's root ui
//! has an 8 pt margin, so in an 800 pt wide window the rail and panes span x 8..792.

use egui::accesskit::{Role, Toggled};
use egui::{Event, ImageSource, Key, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::{Harness, kittest::NodeT, kittest::Queryable};

use super::*;

const L: f32 = 8.0;
const R: f32 = 792.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Tool {
    Comments,
    Bookmarks,
    Pages,
    Layers,
    Files,
    Tags,
}

const TOOLS: [Tool; 6] = [Tool::Comments, Tool::Bookmarks, Tool::Pages, Tool::Layers, Tool::Files, Tool::Tags];

fn items() -> Vec<RailItem<Tool>> {
    TOOLS.iter().map(|t| RailItem::new(*t, ImageSource::Uri(format!("rail-test://{t:?}").into()), format!("{t:?}"))).collect()
}

/// The one-slot task pane the rail opens, at the rail's edge.
struct Content {
    edge: Edge,
}

impl PaneContent<Tool> for Content {
    fn title(&self, pane: &Tool) -> String {
        format!("{pane:?} pane")
    }
    fn body(&mut self, ui: &mut Ui, pane: &Tool) {
        ui.label(format!("{pane:?} body"));
    }
    fn default_state(&self, _: &Tool) -> PaneState {
        PaneState::docked(self.edge, 250.0)
    }
    fn movable(&self, _: &Tool) -> bool {
        false
    }
}

/// The app side: which pane is open, and every request the rail sent.
struct App {
    items: Vec<RailItem<Tool>>,
    open: Option<Tool>,
    log: Vec<RailRequest<Tool>>,
    edge: Edge,
    rtl: bool,
    /// Show the open pane with a `PaneHost` after the rail.
    pane: bool,
    foot: bool,
    out: Option<RailOutput<Tool>>,
    document: Rect,
}

impl App {
    fn new() -> Self {
        Self {
            items: items(),
            open: None,
            log: Vec::new(),
            edge: Edge::End,
            rtl: false,
            pane: false,
            foot: false,
            out: None,
            document: Rect::NOTHING,
        }
    }

    fn frame(&mut self, ui: &mut Ui) {
        let foot = self.foot;
        let out = Rail::new("rail").edge(self.edge).rtl(self.rtl).show(ui, &self.items, self.open.as_ref(), |ui| {
            if foot {
                let _ = ui.button("Foot");
            }
        });
        if self.pane {
            let open: Vec<Tool> = self.open.into_iter().collect();
            PaneHost::new("panes").rtl(self.rtl).policy(SidePolicy::OneSlot).show(ui, &open, |_| None, &mut Content { edge: self.edge });
        }
        self.document = egui::CentralPanel::default().show(ui, |ui| ui.label("Document")).response.rect;
        match &out.request {
            Some(RailRequest::Open(p)) => self.open = Some(*p),
            Some(RailRequest::Close(_)) => self.open = None,
            None => {}
        }
        self.log.extend(out.request.clone());
        self.out = Some(out);
    }

    fn out(&self) -> &RailOutput<Tool> {
        self.out.as_ref().unwrap()
    }
}

fn harness(app: App, height: f32) -> Harness<'static, App> {
    let mut h = Harness::builder().with_size(vec2(800.0, height)).build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.input_mut().focused = true;
    h.run_steps(2);
    h
}

fn item(h: &Harness<'_, App>, tool: Tool) -> Rect {
    h.ctx.read_response(Rail::new("rail").item_id(&tool)).unwrap().rect
}

fn click(h: &mut Harness<'_, App>, at: Pos2) {
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    h.input_mut().events.extend([Event::PointerMoved(at), button(true)]);
    h.run_steps(1);
    h.input_mut().events.push(button(false));
    h.run_steps(2);
}

fn click_item(h: &mut Harness<'_, App>, tool: Tool) {
    let at = item(h, tool).center();
    click(h, at);
}

fn key(h: &mut Harness<'_, App>, key: Key) {
    h.input_mut().events.push(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.input_mut().events.push(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn focused(h: &Harness<'_, App>) -> Option<Tool> {
    h.state().out().focused
}

#[test]
fn a_click_opens_the_open_item_closes_and_another_swaps() {
    let mut h = harness(App::new(), 600.0);
    click_item(&mut h, Tool::Comments);
    assert_eq!(h.state().log, [RailRequest::Open(Tool::Comments)]);
    assert_eq!(h.state().open, Some(Tool::Comments));
    click_item(&mut h, Tool::Comments);
    assert_eq!(h.state().log[1..], [RailRequest::Close(Tool::Comments)]);
    assert_eq!(h.state().open, None);
    click_item(&mut h, Tool::Comments);
    click_item(&mut h, Tool::Pages);
    assert_eq!(h.state().log[2..], [RailRequest::Open(Tool::Comments), RailRequest::Open(Tool::Pages)]);
    assert_eq!(h.state().open, Some(Tool::Pages));
}

#[test]
fn a_disabled_item_sends_nothing_and_the_marker_shows_only_while_closed() {
    let mut app = App::new();
    app.items[1].enabled = false;
    app.items[0].marker = true;
    app.items[3].marker = true;
    app.open = Some(Tool::Comments);
    let mut h = harness(app, 600.0);
    click_item(&mut h, Tool::Bookmarks);
    assert!(h.state().log.is_empty());
    assert_eq!(h.state().open, Some(Tool::Comments));

    let markers = |h: &Harness<'_, App>| -> Vec<Pos2> {
        h.output()
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Circle(circle) if circle.radius == 3.0 => Some(circle.center),
                _ => None,
            })
            .collect()
    };
    let shown = markers(&h);
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(item(&h, Tool::Layers).contains(shown[0]));

    // Closed, Comments shows its marker too.
    click_item(&mut h, Tool::Comments);
    assert_eq!(h.state().open, None);
    let shown = markers(&h);
    assert_eq!(shown.len(), 2, "{shown:?}");
    assert!(shown.iter().any(|p| item(&h, Tool::Comments).contains(*p)));
}

#[test]
fn the_rail_is_outermost_and_the_open_pane_sits_between_it_and_the_document() {
    for (edge, rtl, left) in [(Edge::End, false, false), (Edge::End, true, true), (Edge::Start, false, true), (Edge::Start, true, false)] {
        let mut app = App::new();
        app.edge = edge;
        app.rtl = rtl;
        app.pane = true;
        app.open = Some(Tool::Comments);
        let h = harness(app, 600.0);
        let rail = h.state().out().rect;
        let pane_id = Id::new("panes").with(("pane", &Tool::Comments));
        // The resize handle lies on the pane's inner edge; the pane is 250 wide.
        let inner = h.ctx.read_response(pane_id.with("resize")).unwrap().rect.center().x;
        let pane = if left {
            Rect::from_x_y_ranges(inner - 250.0..=inner, rail.y_range())
        } else {
            Rect::from_x_y_ranges(inner..=inner + 250.0, rail.y_range())
        };
        let title = h.ctx.read_response(pane_id.with("title")).unwrap().rect;
        let document = h.state().document;
        let case = format!("{edge:?} rtl {rtl}: rail {rail:?}, pane {pane:?}, title {title:?}, document {document:?}");
        assert_eq!(rail.width(), RAIL_WIDTH, "{case}");
        assert!(pane.x_range().contains(title.left()) && pane.x_range().contains(title.right()), "{case}");
        if left {
            assert_eq!(rail.left(), L, "{case}");
            assert_eq!(pane.left(), rail.right(), "{case}");
            assert!(document.left() >= pane.right(), "{case}");
        } else {
            assert_eq!(rail.right(), R, "{case}");
            assert_eq!(pane.right(), rail.left(), "{case}");
            assert!(document.right() <= pane.left(), "{case}");
        }
        assert!(rail.x_range().contains(item(&h, Tool::Comments).center().x), "{case}");
    }
}

#[test]
fn in_a_short_window_the_items_scroll_and_the_foot_stays_visible() {
    let mut app = App::new();
    app.foot = true;
    let mut h = harness(app, 200.0);
    let foot = h.get_by_label("Foot").rect();
    assert!(foot.bottom() <= 192.0, "{foot:?}");
    assert!(item(&h, Tool::Comments).top() < 30.0);
    assert!(item(&h, Tool::Tags).bottom() > foot.top(), "the last item starts out of view");

    let over = item(&h, Tool::Comments).center();
    h.input_mut().events.extend([
        Event::PointerMoved(over),
        Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: vec2(0.0, -500.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE },
    ]);
    h.run_steps(3);
    let tags = item(&h, Tool::Tags);
    assert!(tags.bottom() <= h.get_by_label("Foot").rect().top(), "{tags:?} above the foot after scrolling");
    assert_eq!(h.get_by_label("Foot").rect(), foot);
}

#[test]
fn items_are_named_toggle_buttons_and_the_keyboard_moves_between_and_activates_them() {
    let mut app = App::new();
    app.items[1].enabled = false;
    app.open = Some(Tool::Comments);
    let mut h = harness(app, 600.0);
    let comments = h.get_by_label("Comments").accesskit_node();
    assert_eq!(comments.role(), Role::Button);
    assert_eq!(comments.toggled(), Some(Toggled::True));
    assert!(!comments.is_disabled());
    let pages = h.get_by_label("Pages").accesskit_node();
    assert_eq!(pages.role(), Role::Button);
    assert_eq!(pages.toggled(), Some(Toggled::False));
    assert!(h.get_by_label("Bookmarks").accesskit_node().is_disabled());

    h.ctx.memory_mut(|m| m.request_focus(Rail::new("rail").item_id(&Tool::Comments)));
    h.run_steps(2);
    assert_eq!(focused(&h), Some(Tool::Comments));
    key(&mut h, Key::ArrowUp);
    assert_eq!(focused(&h), Some(Tool::Comments), "Up on the first item stays");
    key(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h), Some(Tool::Pages), "Down skips the disabled item");
    key(&mut h, Key::Enter);
    assert_eq!(h.state().log, [RailRequest::Open(Tool::Pages)]);
    key(&mut h, Key::Space);
    assert_eq!(h.state().log[1..], [RailRequest::Close(Tool::Pages)]);
    key(&mut h, Key::ArrowUp);
    assert_eq!(focused(&h), Some(Tool::Comments));
    for _ in 0..TOOLS.len() {
        key(&mut h, Key::ArrowDown);
    }
    assert_eq!(focused(&h), Some(Tool::Tags), "Down on the last item stays");
}

#[test]
fn an_empty_rail_draws_and_bad_sizes_fall_back() {
    let mut app = App::new();
    app.items.clear();
    let h = harness(app, 600.0);
    let rail = h.state().out().rect;
    assert_eq!((rail.right(), rail.width()), (R, RAIL_WIDTH));
    assert_eq!(h.state().out().request, None);

    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).build_ui(|ui| {
        let out = Rail::new("bad").width(f32::NAN).button_size(f32::INFINITY).show(ui, &items(), None, |_| {});
        assert_eq!(out.rect.width(), RAIL_WIDTH);
    });
    h.run_steps(2);
}
