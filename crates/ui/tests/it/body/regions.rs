//! The regions of a body: a filling body placed from this pass's header and footer.

use crate::contract::kit::PlainStyle;
use craft_behavior::Sense;
use craft_ui::contract::{BodyCx, Common, Component, Container, ContainerProps, Cx, FooterRegion, HeaderRegion, Role, ShowContainer as _, Sizing};
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::allocate;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Response, Ui, Vec2};
use std::cell::RefCell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(410.0, 210.0) };

/// The rects the regions of a `Regioned` took.
#[derive(Clone, Copy, Default)]
struct Took {
    header: Option<Rect>,
    body: Option<Rect>,
    end: Option<Rect>,
    footer: Option<Rect>,
}

/// A container whose header, end region and footer each fill a block of the size given (nothing for `None`).
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Regioned<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    header: Option<Vec2>,
    end: Vec2,
    footer: Vec2,
    took: &'a RefCell<Took>,
}

impl Component for Regioned<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: RegionedStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Regioned<'_> {
    fn header(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        self.took.borrow_mut().header = self.header.map(|size| allocate(ui, size));
    }

    fn end(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        self.took.borrow_mut().end = Some(allocate(ui, self.end));
    }

    fn footer(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        self.took.borrow_mut().footer = Some(allocate(ui, self.footer));
    }
}

/// Shows a filling `Regioned` at `AREA` with `props` for one frame and returns what its regions took in the frame's
/// first pass, before any discard pass: no region has a size from an earlier pass then.
fn first_pass(props: ContainerProps, header: Option<Vec2>) -> Took {
    let took = RefCell::new(Took::default());
    let passes = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    frames.frame(Vec::new(), |ui| {
        let shown =
            Regioned { common: Common::new(), props: props.clone(), header, end: Vec2::new(80.0, 40.0), footer: Vec2::new(70.0, 16.0), took: &took }
                .show_at(ui, AREA, |_, cx| cx.rect);
        took.borrow_mut().body = shown.inner;
        passes.borrow_mut().push(*took.borrow());
    });
    passes.into_inner().first().copied().expect("the frame ran a pass")
}

// Edges snap to whole pixels, so a rectangle may overhang its neighbour by half a point.
const EDGE_SLACK: f32 = 0.5;

#[trial]
fn a_filling_body_lies_between_its_header_and_its_footer_in_the_first_pass() {
    let props = ContainerProps::new().sizing(Sizing::Fill).header(HeaderRegion::new()).footer(FooterRegion::new());
    let took = first_pass(props, Some(Vec2::new(100.0, 20.0)));
    let (header, body, footer) = (took.header.unwrap(), took.body.unwrap(), took.footer.unwrap());
    assert!(body.min.y >= header.max.y - EDGE_SLACK, "the body starts under the header: {header:?} {body:?}");
    assert!(body.max.y <= footer.min.y + EDGE_SLACK, "the body ends at the footer's top: {body:?} {footer:?}");
    assert!(footer.max.y <= AREA.max.y + EDGE_SLACK, "the footer lies inside the container, at its foot: {footer:?}");
}
