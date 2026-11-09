//! The regions of a body: an inline header that measures nothing, a full-height end region with the footer in it, a
//! spread row, the room of a Grid cell, and a filling body placed from this pass's header and footer.

use crate::contract::kit::PlainStyle;
use craft_behavior::{ColumnWidth, Sense};
use craft_ui::buttons::Button;
use craft_ui::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, EndRegion, FooterPlace, FooterRegion, HeaderRegion, Role, Show as _,
    ShowContainer as _, Sizing,
};
use craft_ui::display::Divider;
use craft_ui::layout::{BodyLayout, RowAlign};
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::runtime::env::{Direction, with_env};
use craft_ui::style::StyleCx;
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::layout::{allocate, body};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

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

/// Shows a `Regioned` at `AREA` with `props` for five frames and returns what its regions took.
fn regioned(props: ContainerProps, header: Option<Vec2>) -> Took {
    let took = RefCell::new(Took::default());
    let mut frames = Frames::new();
    frames.idle(5, |ui| {
        let shown =
            Regioned { common: Common::new(), props: props.clone(), header, end: Vec2::new(80.0, 40.0), footer: Vec2::new(70.0, 16.0), took: &took }
                .show_at(ui, AREA, |_, cx| cx.rect);
        took.borrow_mut().body = shown.inner;
    });
    took.into_inner()
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

#[trial]
fn an_inline_header_that_measures_nothing_leaves_the_body_the_whole_line() {
    let props = ContainerProps::new().sizing(Sizing::Fill).header(HeaderRegion::new().inline(true));
    let empty = regioned(props.clone(), None).body.expect("the body ran");
    assert!(empty.min.x <= AREA.min.x + 0.5 && empty.max.x >= AREA.max.x - 0.5, "an empty header takes none of the line: {empty:?}");
    let titled = regioned(props, Some(Vec2::new(60.0, 20.0)));
    let (header, body) = (titled.header.expect("the header ran"), titled.body.expect("the body ran"));
    assert!(
        body.min.x >= header.max.x && body.max.x >= AREA.max.x - 0.5,
        "a measured header keeps its width, the body the rest: {header:?} {body:?}"
    );
}

#[trial]
fn a_footer_in_a_full_height_end_region_sits_at_its_foot_or_top_beside_the_body() {
    let props = |place: FooterPlace| {
        ContainerProps::new()
            .sizing(Sizing::Fill)
            .header(HeaderRegion::new())
            .end(EndRegion::new().full_height(true))
            .footer(FooterRegion::new().place(place))
    };
    let foot = regioned(props(FooterPlace::EndBottom), Some(Vec2::new(100.0, 20.0)));
    let (header, body, end, footer) = (foot.header.unwrap(), foot.body.unwrap(), foot.end.unwrap(), foot.footer.unwrap());
    assert!(end.min.x >= body.max.x && footer.min.x >= body.max.x, "the end region and its footer lie past the body: {body:?} {end:?} {footer:?}");
    assert!(end.min.y >= header.max.y - 0.5 && footer.min.y >= end.max.y, "the footer lies under the end region's content: {end:?} {footer:?}");
    assert!((footer.max.y - AREA.max.y).abs() < 1.0, "the footer sits at the container's foot: {footer:?}");
    assert!((body.max.y - AREA.max.y).abs() < 1.0, "no footer band crosses under the body: {body:?}");

    let top = regioned(props(FooterPlace::EndTop), Some(Vec2::new(100.0, 20.0)));
    let (header, body, end, footer) = (top.header.unwrap(), top.body.unwrap(), top.end.unwrap(), top.footer.unwrap());
    assert!(footer.min.x >= body.max.x && (footer.min.y - header.max.y).abs() < 1.0, "a top footer sits under the header, past the body: {footer:?}");
    assert!(end.min.y >= footer.max.y, "the end region's content lies under a top footer: {footer:?} {end:?}");
}

const ITEM: Vec2 = Vec2::new(40.0, 10.0);

/// Runs `show` for five frames under `direction` and returns the last frame's region and rects.
fn lay_out(direction: Direction, show: impl Fn(&mut Ui) -> Vec<Rect>) -> (Rect, Vec<Rect>) {
    let mut frames = Frames::new();
    let region = Cell::new(Rect::NOTHING);
    let rects = RefCell::new(Vec::new());
    frames.idle(5, |ui| {
        region.set(ui.available_rect_before_wrap());
        *rects.borrow_mut() = with_env(ui, |o| o.direction = Some(direction), &show);
    });
    (region.get(), rects.into_inner())
}

#[trial]
fn a_spread_row_puts_its_ends_at_the_edges_and_shares_the_room_between() {
    let row = |ui: &mut Ui| body(ui, "spread", &BodyLayout::row().align(RowAlign::SpaceBetween), &[ITEM, ITEM, ITEM]);
    let (region, ltr) = lay_out(Direction::Ltr, row);
    assert!((ltr[0].min.x - region.min.x).abs() < 1.0 && (ltr[2].max.x - region.max.x).abs() < 1.0, "the ends at the edges: {ltr:?}");
    let (first, second) = (ltr[1].min.x - ltr[0].max.x, ltr[2].min.x - ltr[1].max.x);
    assert!((first - second).abs() < 1.0 && first > 0.0, "the room is shared equally: {ltr:?}");
    let (_, rtl) = lay_out(Direction::Rtl, row);
    assert!(
        (rtl[0].max.x - region.max.x).abs() < 1.0 && (rtl[2].min.x - region.min.x).abs() < 1.0,
        "RTL: the first child at the right edge: {rtl:?}"
    );
}

static CELLS: [ColumnWidth; 2] = [ColumnWidth::Content, ColumnWidth::Content];

#[trial]
fn a_filling_leaf_after_a_fill_cell_gets_its_cells_room() {
    let (filled, divider) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let mut frames = Frames::new();
    frames.idle(5, |ui| {
        Grid::new(&CELLS).fill().show_at(ui, AREA, |ui, _| {
            filled.set(Stack::new().fill().show(ui, |_, _| {}).output.rect);
            divider.set(Divider::vertical().show(ui).rect);
        });
    });
    let (filled, divider) = (filled.get(), divider.get());
    assert!(divider.min.x >= filled.max.x, "the divider takes the second cell: {filled:?} {divider:?}");
    assert!(
        divider.min.y.round() <= filled.min.y.round() && divider.max.y.round() >= filled.max.y.round(),
        "the divider fills its cell's height as the fill cell does: {filled:?} {divider:?}"
    );
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

#[trial]
fn a_filling_body_ends_at_a_footer_beside_a_full_height_end_region_in_the_first_pass() {
    // Beside a full-height end region the footer lies under the body only.
    let props = ContainerProps::new().sizing(Sizing::Fill).end(EndRegion::new().full_height(true)).footer(FooterRegion::new());
    let took = first_pass(props, None);
    let (body, footer) = (took.body.unwrap(), took.footer.unwrap());
    assert!(body.max.y <= footer.min.y + EDGE_SLACK, "the body ends at the footer's top: {body:?} {footer:?}");
    assert!(footer.max.y <= AREA.max.y + EDGE_SLACK, "the footer lies inside the container: {footer:?}");
}

#[trial]
fn tab_walks_a_filling_stack_through_header_body_and_footer() {
    let mut fixture = Fixture::new(|ui| {
        Stack::new()
            .fill()
            .header(|ui, _| {
                let _ = Button::new("Head").show(ui);
            })
            .footer(|ui, _| {
                Row::new().show(ui, |ui, _| {
                    let _ = Button::new("Foot").show(ui);
                });
            })
            .show(ui, |ui, _| {
                let _ = Button::new("Body").show(ui);
            });
    });
    fixture.room(Vec2::new(320.0, 200.0)).settle();
    let mut walked = Vec::new();
    for _ in 0..3 {
        fixture.key(Key::Tab, Modifiers::NONE);
        fixture.settle();
        let focused = fixture.focused().expect("Tab focuses a button");
        walked.push(fixture.tree().name_of(NodeKey::of(focused)).map(str::to_owned));
    }
    let expected = ["Head", "Body", "Foot"].map(|name| Some(name.to_owned()));
    assert_eq!(walked, expected, "Tab follows the visual order");
}

#[trial]
fn a_footer_sits_under_a_body_that_grew_this_pass() {
    let took = RefCell::new(Took::default());
    let mut frames = Frames::new();
    let mut run = |height: f32| {
        frames.frame(Vec::new(), |ui| {
            let shown = Regioned {
                common: Common::new(),
                props: ContainerProps::new().sizing(Sizing::Content).footer(FooterRegion::new()),
                header: None,
                end: Vec2::ZERO,
                footer: Vec2::new(70.0, 16.0),
                took: &took,
            }
            .show(ui, |ui, _| allocate(ui, Vec2::new(100.0, height)));
            took.borrow_mut().body = shown.inner;
        })
    };
    run(20.0);
    run(20.0);
    let passes = run(50.0);
    let (body, footer) = (took.borrow().body.unwrap(), took.borrow().footer.unwrap());
    assert_eq!(passes, 1, "the frame took one pass");
    assert!((footer.min.y - body.max.y).abs() < 1.0, "the footer starts where the grown body ends: {body:?} {footer:?}");
}

#[trial]
fn an_inline_header_line_is_as_high_as_its_tallest_part() {
    let took = RefCell::new(Took::default());
    let mut frames = Frames::new();
    frames.screen(Vec2::new(400.0, 300.0));
    let mut outer = Rect::NOTHING;
    for _ in 0..4 {
        frames.frame(Vec::new(), |ui| {
            let shown = Regioned {
                common: Common::new(),
                props: ContainerProps::new().sizing(Sizing::FillWidth).header(HeaderRegion::new().inline(true)),
                header: Some(Vec2::new(60.0, 20.0)),
                end: Vec2::ZERO,
                footer: Vec2::ZERO,
                took: &took,
            }
            .show(ui, |ui, _| allocate(ui, Vec2::new(100.0, 40.0)));
            outer = shown.output.rect;
            took.borrow_mut().body = shown.inner;
        });
    }
    let body = took.borrow().body.unwrap();
    eprintln!("outer {outer:?} body {body:?}");
    assert!(outer.height() < 100.0, "the container is as high as its body, not the room: {outer:?}");
}
