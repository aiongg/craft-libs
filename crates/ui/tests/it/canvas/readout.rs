//! `canvas::readout`: rows of a label and a value beside the pointer.

use craft_behavior::Metrics;
use craft_i18n::Direction;
use craft_ui::canvas::readout;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, Props as _, Role, Show as _};
use craft_ui::runtime::env::Label;
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Response, Vec2};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PlainStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// Shows a readout of its rows at a point, as a gesture would each pass.
#[derive(Component)]
struct Gesture {
    #[common]
    common: Common,
    at: Pos2,
    rows: Vec<(Label, String)>,
}

impl Component for Gesture {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: GestureStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(10.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        readout(cx, self.at, &self.rows);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn rows() -> Vec<(Label, String)> {
    vec![(Label::Static("Width"), "120 px".to_owned()), (Label::Static("Angle"), "15.0".to_owned())]
}

/// A fixture showing the readout at the point `at` holds; `with_rows` is false for a gesture with nothing to say.
fn fixture(at: &Rc<Cell<Pos2>>, with_rows: bool) -> Fixture {
    let at = Rc::clone(at);
    Fixture::new(move |ui| {
        let rows = if with_rows { rows() } else { Vec::new() };
        Gesture { common: Common::new(), at: at.get(), rows }.id_salt("gesture").show_at(ui, Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0)));
    })
}

fn find(fixture: &Fixture, word: &str) -> Rect {
    fixture.texts().iter().find(|text| text.text == word).map(|text| text.rect).unwrap_or_else(|| panic!("{word} is painted"))
}

#[trial]
fn the_readout_shows_its_rows_beside_the_pointer_and_inside_the_window() {
    let at = Rc::new(Cell::new(Pos2::new(100.0, 100.0)));
    let mut fixture = fixture(&at, true);
    fixture.settle();
    let (width, value, angle) = (find(&fixture, "Width"), find(&fixture, "120 px"), find(&fixture, "Angle"));
    find(&fixture, "15.0");
    assert!(width.min.x > at.get().x && width.min.y > at.get().y, "beside the pointer, toward its lower end: {width:?}");
    assert!(value.min.x > width.max.x, "the value follows the label");
    assert!(angle.min.y >= width.max.y, "rows stack");

    let window = fixture.ctx().content_rect();
    at.set(window.max - Vec2::splat(4.0));
    fixture.settle();
    for word in ["Width", "120 px", "Angle", "15.0"] {
        assert!(window.contains_rect(find(&fixture, word)), "{word} stays inside the window at its corner");
    }
}

#[trial]
fn the_readout_mirrors_under_right_to_left() {
    let at = Rc::new(Cell::new(Pos2::new(300.0, 100.0)));
    let mut fixture = fixture(&at, true);
    fixture.direction(Direction::Rtl);
    fixture.settle();
    let (label, value) = (find(&fixture, "Width"), find(&fixture, "120 px"));
    assert!(label.max.x < at.get().x, "the readout sits toward the reading end, to the pointer's left: {label:?}");
    assert!(label.min.x > value.max.x, "the label starts at the right and the value ends at the left");
}
