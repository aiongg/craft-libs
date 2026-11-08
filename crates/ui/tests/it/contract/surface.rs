//! `Sense::SURFACE`: a container root learns of a press on its own surface, where none of its children takes it,
//! without becoming a click target.

use super::kit::PlainStyle;
use craft_behavior::Sense;
use craft_ui::buttons::Button;
use craft_ui::contract::{Common, Component, Container, ContainerProps, Cx, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::Block;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Response, Ui, Vec2};
use std::cell::Cell;

/// A container that senses presses on its surface.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Surfaced {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Component for Surfaced {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SurfacedStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::SURFACE
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Surfaced {}

/// What one frame of the scene saw: the container's rect, the button's rect, a surface press and a button click.
struct Seen {
    rect: Cell<Rect>,
    button: Cell<Rect>,
    surface: Cell<bool>,
    clicked: Cell<bool>,
}

impl Seen {
    fn new() -> Self {
        Seen { rect: Cell::new(Rect::NOTHING), button: Cell::new(Rect::NOTHING), surface: Cell::new(false), clicked: Cell::new(false) }
    }
}

fn scene(seen: &Seen) -> impl FnMut(&mut Ui) + '_ {
    move |ui: &mut Ui| {
        let shown = Surfaced { common: Common::new(), props: ContainerProps::new() }.id_salt("group").show(ui, |ui, cx| {
            seen.surface.set(seen.surface.get() || cx.surface_pressed());
            let button = Button::new("Apply").show(ui);
            seen.button.set(button.rect);
            seen.clicked.set(seen.clicked.get() || button.clicked());
            Block::new(Vec2::new(200.0, 120.0)).show(ui);
        });
        seen.rect.set(shown.output.rect);
    }
}

/// A primary click at `at`: whether the container saw a press on its surface, and whether the button was clicked.
fn click_at(at: impl Fn(&Seen) -> Pos2) -> (bool, bool) {
    let seen = Seen::new();
    let mut frames = Frames::new();
    frames.idle(3, scene(&seen));
    let at = at(&seen);
    for events in [vec![moved(at)], vec![press(at)], vec![release(at)], vec![]] {
        frames.frame(events, scene(&seen));
    }
    (seen.surface.get(), seen.clicked.get())
}

#[trial]
fn a_press_on_a_containers_free_surface_is_reported_and_a_press_on_a_child_is_not() {
    let (surface, clicked) = click_at(|seen| seen.rect.get().right_bottom() - Vec2::splat(4.0));
    assert!(surface && !clicked, "a press on the free surface reaches the container");
    let (surface, clicked) = click_at(|seen| seen.button.get().center());
    assert!(!surface && clicked, "a press on the button is the button's");
}
