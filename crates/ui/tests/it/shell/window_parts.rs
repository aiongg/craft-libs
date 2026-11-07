//! The caption record: exclusive parts tagged with their window role read back through
//! `WindowHost::caption_regions`.

use crate::contract::kit::PlainStyle;
use crate::windows_kit::Windows;
use craft_behavior::{PartKey, Sense};
use craft_ui::contract::{Common, Component, Cx, MeasureCx, PartsCx, Props as _, Role, Show as _};
use craft_ui::runtime::windows::{ResizeEdge, WindowHit, WindowHost};
use craft_ui::shell::Caption;
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Response, Vec2, ViewportId, pos2, vec2};

const MOVE: PartKey = PartKey::new("move");
const CLOSE: PartKey = PartKey::new("close");
const EDGE: PartKey = PartKey::new("edge");

const BAR: Vec2 = Vec2::new(400.0, 32.0);
const BUTTON: f32 = 32.0;
const GRIP: f32 = 4.0;

/// A title bar: a move band over the whole bar, a close button at its end and a grip along its top edge.
#[derive(craft_ui::contract::Component)]
struct Bar {
    #[common]
    common: Common,
}

impl Component for Bar {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &craft_ui::style::StyleCx<'_>, _: BarStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        BAR
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(MOVE, rect).sense(Sense::DRAG).exclusive(WindowHit::Move).pointer_only("the window command set moves the window");
        cx.part(CLOSE, close_of(rect)).sense(Sense::CLICK).exclusive(WindowHit::Caption(Caption::Close)).pointer_only("the Close command");
        cx.part(EDGE, top_of(rect)).sense(Sense::DRAG).exclusive(WindowHit::Resize(ResizeEdge::Top)).pointer_only("the Size command");
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn close_of(bar: Rect) -> Rect {
    Rect::from_min_size(pos2(bar.max.x - BUTTON, bar.min.y), vec2(BUTTON, bar.height()))
}

fn top_of(bar: Rect) -> Rect {
    Rect::from_min_size(bar.min, vec2(bar.width(), GRIP))
}

/// Parts tagged exclusive show up in the window's record with their role, and the hit test prefers a button to an
/// edge and an edge to the drag area. A window that registered none, or that stopped, has no record.
#[trial]
fn exclusive_parts_are_the_caption_record() {
    let windows = Windows::new();
    let host = WindowHost::of(&windows.ctx);
    let other = ViewportId::from_hash_of("other");
    let mut bar = Rect::NOTHING;
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| bar = Bar { common: Common::new() }.id_salt("bar").show(ui).rect);
    }
    let regions = host.caption_regions(ViewportId::ROOT).expect("the bar registered its parts");
    assert_eq!(regions.buttons, [(Caption::Close, close_of(bar))]);
    assert_eq!(regions.edges, [(ResizeEdge::Top, top_of(bar))]);
    assert_eq!(regions.moves, [bar]);
    let at = |x: f32, y: f32| Pos2::new(bar.min.x + x, bar.min.y + y);
    assert_eq!(regions.hit(close_of(bar).center()), Some(WindowHit::Caption(Caption::Close)), "a button wins over the drag area");
    assert_eq!(regions.hit(at(10.0, 1.0)), Some(WindowHit::Resize(ResizeEdge::Top)), "an edge wins over the drag area");
    assert_eq!(regions.hit(at(10.0, 16.0)), Some(WindowHit::Move));
    assert_eq!(regions.hit(at(10.0, 100.0)), None);
    assert_eq!(host.caption_regions(other), None, "another window has its own record");
    for _ in 0..3 {
        windows.frame(vec![], vec![], |_| {});
    }
    assert_eq!(host.caption_regions(ViewportId::ROOT), None, "a bar that stops registering leaves no record");
}
