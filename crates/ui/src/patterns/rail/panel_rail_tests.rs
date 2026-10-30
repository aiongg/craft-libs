//! `PanelRail` tests: requests, the marker, keyboard focus, scrolling with a foot, edges and
//! AccessKit.

use egui::accesskit::{Orientation, Role, Toggled};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::panel_rail::stories;
use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::panes::Edge;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

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
const ICONS: [Icon; 6] = [Icon::Info, Icon::Navigation, Icon::Palette, Icon::Layers, Icon::Clock, Icon::Brush];

fn items() -> Vec<PanelRailItem<Tool>> {
    TOOLS.iter().zip(ICONS).map(|(t, icon)| PanelRailItem::new(*t, icon, format!("{t:?}"))).collect()
}

/// The host side: which panels are open, every request the rail sent and its last output.
struct App {
    items: Vec<PanelRailItem<Tool>>,
    open: Vec<Tool>,
    log: Vec<PanelRailRequest<Tool>>,
    edge: Edge,
    foot: bool,
    out: Option<PanelRailOutput<Tool>>,
}

impl App {
    fn new() -> Self {
        Self { items: items(), open: Vec::new(), log: Vec::new(), edge: Edge::End, foot: false, out: None }
    }

    fn frame(&mut self, ui: &mut Ui) {
        let foot = self.foot;
        let out = PanelRail::new("rail").edge(self.edge).label("Panels").show(ui, &self.items, &self.open, |ui| {
            if foot {
                let _ = ui.button("Foot");
            }
        });
        egui::CentralPanel::default().show(ui, |ui| ui.label("Document"));
        match &out.request {
            Some(PanelRailRequest::Open(p)) => self.open.push(*p),
            Some(PanelRailRequest::Close(p)) => self.open.retain(|o| o != p),
            None => {}
        }
        self.log.extend(out.request.clone());
        self.out = Some(out);
    }

    #[allow(clippy::unwrap_used)]
    fn out(&self) -> &PanelRailOutput<Tool> {
        self.out.as_ref().unwrap()
    }

    #[allow(clippy::unwrap_used)]
    fn rect(&self, tool: Tool) -> Rect {
        self.out().responses.iter().find(|(p, _)| *p == tool).unwrap().1.rect
    }
}

fn builder(dir: Direction, height: f32) -> HarnessBuilder<App> {
    Harness::<App>::builder().with_size(vec2(800.0, height)).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

fn harness_in(dir: Direction, app: App, height: f32) -> Harness<'static, App> {
    let mut h = builder(dir, height).build_ui_state(|ui, app: &mut App| app.frame(ui), app);
    h.input_mut().focused = true;
    h.run_steps(2);
    h
}

fn harness(app: App, height: f32) -> Harness<'static, App> {
    harness_in(Direction::Ltr, app, height)
}

fn click(h: &mut Harness<'_, App>, at: Pos2) {
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    h.input_mut().events.extend([Event::PointerMoved(at), button(true)]);
    h.run_steps(1);
    h.input_mut().events.push(button(false));
    h.run_steps(2);
}

fn click_item(h: &mut Harness<'_, App>, tool: Tool) {
    let at = h.state().rect(tool).center();
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

#[allow(clippy::unwrap_used)]
fn focus(h: &mut Harness<'_, App>, tool: Tool) {
    let id = h.state().out().responses.iter().find(|(p, _)| *p == tool).unwrap().1.id;
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.run_steps(2);
}

#[allow(clippy::expect_used)]
fn style() -> PanelRailStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(PanelRail::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

fn markers(h: &Harness<'_, App>) -> Vec<Pos2> {
    let radius = style().marker_radius;
    h.output()
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Circle(circle) if circle.radius == radius => Some(circle.center),
            _ => None,
        })
        .collect()
}

#[test]
fn a_click_opens_the_open_item_closes_and_another_adds() {
    let mut h = harness(App::new(), 600.0);
    click_item(&mut h, Tool::Comments);
    assert_eq!(h.state().log, [PanelRailRequest::Open(Tool::Comments)]);
    assert_eq!(h.state().open, [Tool::Comments]);
    click_item(&mut h, Tool::Comments);
    assert_eq!(h.state().log[1..], [PanelRailRequest::Close(Tool::Comments)]);
    assert!(h.state().open.is_empty());
    click_item(&mut h, Tool::Pages);
    assert_eq!(h.state().log[2..], [PanelRailRequest::Open(Tool::Pages)]);
}

#[test]
fn a_disabled_item_sends_nothing_and_the_marker_shows_only_while_closed() {
    let mut app = App::new();
    app.items[1].enabled = false;
    app.items[0] = app.items[0].clone().marker(true).marker_label("Has comments");
    app.items[3] = app.items[3].clone().marker(true).marker_label("Has layers");
    app.open = vec![Tool::Comments];
    let mut h = harness(app, 600.0);
    click_item(&mut h, Tool::Bookmarks);
    assert!(h.state().log.is_empty());

    let shown = markers(&h);
    assert_eq!(shown.len(), 1, "{shown:?}");
    let layers = h.state().rect(Tool::Layers);
    assert!(layers.contains(shown[0]));
    assert!(shown[0].x > layers.center().x, "the marker sits at the corner away from the canvas");
    let description = |h: &Harness<'_, App>, label: &str| h.get_by_label(label).accesskit_node().description();
    assert_eq!(description(&h, "Layers").as_deref(), Some("Has layers"));
    assert_eq!(description(&h, "Comments").as_deref(), Some("Comments"), "no marker description while the panel is open");

    click_item(&mut h, Tool::Comments);
    assert!(h.state().open.is_empty());
    assert_eq!(markers(&h).len(), 2);
    // Hovering Comments shows a tooltip with its label; move away before reading by label.
    h.input_mut().events.push(Event::PointerMoved(Pos2::new(400.0, 300.0)));
    h.run_steps(2);
    assert_eq!(description(&h, "Comments").as_deref(), Some("Has comments"));
}

#[test]
fn the_rail_sits_at_its_edge_with_the_marker_and_hairline_facing_away_from_the_canvas() {
    for (dir, edge, left) in [
        (Direction::Ltr, Edge::End, false),
        (Direction::Rtl, Edge::End, true),
        (Direction::Ltr, Edge::Start, true),
        (Direction::Rtl, Edge::Start, false),
    ] {
        let mut app = App::new();
        app.edge = edge;
        app.items[3] = app.items[3].clone().marker(true);
        let h = harness_in(dir, app, 600.0);
        let style = style();
        let rail = h.state().out().rect;
        let case = format!("{dir:?} {edge:?}: rail {rail:?}");
        assert_eq!(rail.width(), style.width, "{case}");
        let first = h.state().rect(Tool::Comments);
        assert_eq!((first.width(), first.height()), (style.button, style.button), "{case}");
        assert!(rail.x_range().contains(first.center().x), "{case}");
        assert_eq!(first.top() - rail.top(), style.pad_y, "{case}");
        if left {
            assert_eq!(rail.left(), 8.0, "{case}");
        } else {
            assert_eq!(rail.right(), 792.0, "{case}");
        }
        let layers = h.state().rect(Tool::Layers);
        let marker = markers(&h);
        assert_eq!(marker.len(), 1, "{case}");
        assert_eq!(marker[0].x < layers.center().x, left, "{case}");
        let line = h.output().shapes.iter().any(|s| {
            matches!(&s.shape, egui::Shape::LineSegment { points, stroke }
                if stroke.color == style.edge && points[0].x == points[1].x && (points[0].x - if left { rail.right() } else { rail.left() }).abs() <= 1.0)
        });
        assert!(line, "the hairline lies on the canvas side: {case}");
    }
}

#[test]
fn in_a_short_window_the_items_scroll_and_the_foot_stays_visible() {
    let mut app = App::new();
    app.foot = true;
    let mut h = harness(app, 200.0);
    let foot = h.get_by_label("Foot").rect();
    assert!(foot.bottom() <= 200.0, "{foot:?}");
    assert!(h.state().rect(Tool::Tags).bottom() > foot.top(), "the last item starts out of view");

    let over = h.state().rect(Tool::Comments).center();
    h.input_mut().events.extend([
        Event::PointerMoved(over),
        Event::MouseWheel {
            source: egui::MouseWheelSource::Unknown,
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -500.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.run_steps(3);
    assert!(h.state().rect(Tool::Tags).bottom() <= h.get_by_label("Foot").rect().top(), "the last item is above the foot after scrolling");
    assert_eq!(h.get_by_label("Foot").rect(), foot);
}

#[test]
fn the_rail_is_a_vertical_toolbar_of_selected_buttons_and_the_keyboard_moves_between_them() {
    let mut app = App::new();
    app.items[1].enabled = false;
    app.open = vec![Tool::Comments];
    let mut h = harness(app, 600.0);
    let toolbar = h.get_by_role(Role::Toolbar).accesskit_node();
    assert_eq!(toolbar.label().as_deref(), Some("Panels"));
    assert_eq!(toolbar.orientation(), Some(Orientation::Vertical));
    let comments = h.get_by_label("Comments").accesskit_node();
    assert_eq!(comments.role(), Role::Button);
    assert_eq!(comments.toggled(), Some(Toggled::True));
    let pages = h.get_by_label("Pages").accesskit_node();
    assert_eq!(pages.toggled(), Some(Toggled::False));
    assert!(h.get_by_label("Bookmarks").accesskit_node().is_disabled());

    focus(&mut h, Tool::Comments);
    assert_eq!(focused(&h), Some(Tool::Comments));
    key(&mut h, Key::ArrowUp);
    assert_eq!(focused(&h), Some(Tool::Comments), "Up on the first item stays");
    key(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h), Some(Tool::Pages), "Down skips the disabled item");
    key(&mut h, Key::Enter);
    assert_eq!(h.state().log, [PanelRailRequest::Open(Tool::Pages)]);
    key(&mut h, Key::Space);
    assert_eq!(h.state().log[1..], [PanelRailRequest::Close(Tool::Pages)]);
    for _ in 0..TOOLS.len() {
        key(&mut h, Key::ArrowDown);
    }
    assert_eq!(focused(&h), Some(Tool::Tags), "Down on the last item stays");
}

#[test]
fn a_disabled_item_takes_no_tab_focus_and_a_focused_item_disabled_hands_focus_on() {
    let mut app = App::new();
    app.items[1].enabled = false;
    let mut h = harness(app, 600.0);
    let focused_id = |h: &Harness<'_, App>| h.ctx.memory(|m| m.focused());
    let id = |h: &Harness<'_, App>, tool: Tool| h.state().out().responses.iter().find(|(p, _)| *p == tool).unwrap().1.id;
    focus(&mut h, Tool::Comments);
    key(&mut h, Key::Tab);
    assert_eq!(focused_id(&h), Some(id(&h, Tool::Pages)), "Tab skips the disabled item");

    h.state_mut().items[2].enabled = false;
    h.run_steps(2);
    assert_eq!(focused_id(&h), Some(id(&h, Tool::Layers)));
    assert_eq!(focused(&h), Some(Tool::Layers));

    focus(&mut h, Tool::Tags);
    h.state_mut().items[5].enabled = false;
    h.run_steps(2);
    assert_eq!(focused_id(&h), Some(id(&h, Tool::Files)));

    for item in &mut h.state_mut().items {
        item.enabled = false;
    }
    h.run_steps(2);
    assert_eq!((focused_id(&h), focused(&h)), (None, None));
}

#[test]
fn responses_carry_each_icon_once_and_an_empty_rail_draws() {
    let mut app = App::new();
    app.items.push(PanelRailItem::new(Tool::Comments, Icon::Info, "Comments again"));
    let h = harness(app, 600.0);
    let listed: Vec<Tool> = h.state().out().responses.iter().map(|(p, _)| *p).collect();
    assert_eq!(listed, TOOLS, "a panel listed twice is drawn once");

    let mut app = App::new();
    app.items.clear();
    let h = harness(app, 600.0);
    assert_eq!(h.state().out().rect.width(), style().width);
    assert!(h.state().out().responses.is_empty());
}

#[test]
fn every_story_names_every_control() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for story in stories() {
            let mut args = (story.args)();
            let show = story.show;
            let mut harness = builder(dir, 400.0).build_ui_state(move |ui, _: &mut App| show(ui, &mut args), App::new());
            harness.run();
            a11y::assert_labelled(&harness);
        }
    }
}
