//! An inactive part (a switched-off clip): it paints with the Disabled state and the theme's disabled fill, and it
//! keeps sensing and an enabled access node.

use craft_behavior::{PartKey, PartRef, Sense};
use craft_theme::{Contrast, Mode, Rgba, State};
use craft_ui::contract::{Access, Common, Component, Cx, PaintUi, PartsCx, Pointer as _, Props as _, Role, Show as _};
use craft_ui::primitives::PaintShape;
use craft_ui::style::{ComponentStyle, DataColor, StyleCx};
use craft_ui::testing::style::tokens;
use craft_ui::testing::{Frames, trial};
use egui::{Color32, CornerRadius, Rect, Response, Shape, Vec2};
use std::cell::RefCell;

const BODY: PartKey = PartKey::new("body");
const INK: Color32 = Color32::from_rgb(12, 34, 56);

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
struct ClipLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// A leaf with one data-coloured part that can be switched off.
#[derive(craft_ui::contract::Component)]
struct Clip<'a> {
    #[common]
    common: Common,
    inactive: bool,
    states: &'a RefCell<Vec<State>>,
}

impl Component for Clip<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = ClipLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ClipStyleKey) -> ClipLook {
        ClipLook { metrics: craft_behavior::Metrics::new().min_size(Vec2::new(60.0, 20.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(BODY, rect).sense(Sense::CLICK).inactive(self.inactive);
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.node(BODY, Role::BUTTON).name = craft_ui::contract::Name::Text("Body".into());
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.states.borrow_mut().push(cx.part_state(PartRef::from(BODY)).state);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rect = cx.part(BODY).rect;
        let shape = PaintShape::Rect(rect, CornerRadius::ZERO);
        ui.draw_part(BODY).data_fill(&shape, DataColor::new(INK));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn color(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3])
}

/// The state the part reported last, the fills painted in the last frame and whether its access node is disabled.
fn run(inactive: bool) -> (State, Vec<Color32>, bool) {
    let mut frames = Frames::new();
    let states = RefCell::new(Vec::new());
    let mut show = |ui: &mut egui::Ui| {
        Clip { common: Common::new(), inactive, states: &states }.label("Clip").show(ui);
    };
    frames.idle(3, &mut show);
    let shapes = frames.frame_shapes(&mut show);
    let fills = shapes.iter().filter_map(|shape| if let Shape::Rect(rect) = shape { Some(rect.fill) } else { None }).collect();
    let tree = frames.tree();
    let disabled = tree.with_role(Role::BUTTON).into_iter().any(|key| tree.is_disabled_key(key));
    let state = *states.borrow().last().expect("the part updated");
    (state, fills, disabled)
}

#[trial]
fn an_inactive_part_reads_disabled_and_paints_the_disabled_fill_but_stays_enabled_for_access() {
    let disabled_fill = color(tokens(Mode::Light, Contrast::Standard).color.state.disabled_fill);
    let (state, fills, access_disabled) = run(true);
    assert_eq!(state, State::Disabled, "the part state reads Disabled");
    assert!(fills.contains(&disabled_fill) && !fills.contains(&INK), "the data fill is the theme's disabled fill: {fills:?}");
    assert!(!access_disabled, "the part's access node stays enabled");

    let (state, fills, _) = run(false);
    assert_ne!(state, State::Disabled);
    assert!(fills.contains(&INK), "an active part keeps its data colour");
}
