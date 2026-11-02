//! Stories of the "+" buttons (task ST-SH1; task RG2 registers them). A box on the story page has
//! the buttons round it; the story shows what an app draws for the hovered side, a ghost of the
//! board the button would add, and reports the click. Sample words are story words.
//!
//! - `primitives/add-beside-handles/buttons`: four buttons round a box; the ghost on hover (`AddBesideResponse::hovered`
//!   is the side); a rectangle the buttons keep clear of (`clear_of`: the neighbouring board hides
//!   the button that would sit on it); a box too small for buttons; Alt on a click asks for a copy.
//!
//! RG2 registers `stories()` and the component `AddBesideHandles`; RG1 adds
//! [`AddBesideStoryLabels`] to `WORDS`.

use egui::{Pos2, Shape, Stroke, Ui};
use kurbo::{Rect, Vec2};

use super::add_beside::{AddBesideHandles, AddBesideLabels, AddSide};
use craft_ui::canvas::{CanvasView, Pasteboard, PasteboardColor};
use craft_ui::controls::Button;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryLabels, Surface, lock, shared};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["AddBesideHandles"];

craft_ui::words! {
    /// The captions of the add-beside story, English by default.
    #[non_exhaustive]
    pub(crate) struct AddBesideStoryLabels {
        clicked: "addBesideStory.clicked" = "Clicked: {side}";
        copy: "addBesideStory.copy" = "Clicked: {side}, with a copy";
        hovered: "addBesideStory.hovered" = "Pointing at: {side}";
        none: "addBesideStory.none" = "No button pressed";
    }
}

/// The space between a board and the one a button would add (story-local: no token sizes a
/// document).
const GAP: f64 = 40.0;

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/add-beside-handles/buttons",
            Level::Primitives,
            "AddBesideHandles",
            "Buttons",
            AddBesideArgs { view: ViewArgs::default(), width: 200.0, height: 140.0, neighbour: false, hidden: false, connecting: false },
            show
        )
        .on(Surface::Canvas),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AddBesideArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// The box's size in document units; under two button radii on screen it has no buttons.
    #[arg(range = 4.0..=500.0)]
    width: f64,
    #[arg(range = 4.0..=360.0)]
    height: f64,
    /// A board to the right of the box: the button that would sit on it is left out.
    neighbour: bool,
    /// Hides the buttons, as during a drag.
    hidden: bool,
    /// Names the group for adding a connected shape, as beside a connector's source.
    connecting: bool,
}

fn side_name(labels: &AddBesideLabels, side: AddSide) -> String {
    match side {
        AddSide::Top => labels.above.clone(),
        AddSide::Right => labels.right.clone(),
        AddSide::Bottom => labels.below.clone(),
        AddSide::Left => labels.left.clone(),
    }
}

/// The board a button on `side` would add: the box moved by its size and the gap.
fn beside(object: Rect, side: AddSide) -> Rect {
    let by = match side {
        AddSide::Top => Vec2::new(0.0, -(object.height() + GAP)),
        AddSide::Right => Vec2::new(object.width() + GAP, 0.0),
        AddSide::Bottom => Vec2::new(0.0, object.height() + GAP),
        AddSide::Left => Vec2::new(-(object.width() + GAP), 0.0),
    };
    object + by
}

fn screen_rect(view: &CanvasView, rect: Rect) -> egui::Rect {
    egui::Rect::from_points(&view.quad(rect.abs()))
}

fn show(ui: &mut Ui, args: &mut AddBesideArgs) {
    let words = AddBesideStoryLabels::of(ui);
    let labels = AddBesideLabels::of(ui);
    let story = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &story.document);
    let object = Rect::from_center_size(DOC.center(), (args.width, args.height));
    let neighbour = beside(object, AddSide::Right);
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let outline = Pasteboard::style(&tokens, PasteboardColor::Default).outline;
    let board = |rect: Rect| {
        let quad: Vec<Pos2> = canvas.view.quad(rect.abs()).to_vec();
        painter.add(Shape::convex_polygon(quad, tokens.color.surface.raised.to_egui(), outline));
    };
    board(object);
    let clear = [screen_rect(&canvas.view, neighbour)];
    if args.neighbour {
        board(neighbour);
    }
    let accent = Stroke::new(AddBesideHandles::style(&tokens).ring.width, tokens.color.accent.solid.to_egui());
    let view = canvas.view;
    let ghost = |painter: &egui::Painter, side: AddSide| {
        painter.add(Shape::closed_line(view.quad(beside(object, side).abs()).to_vec(), accent));
    };
    let mut handles = AddBesideHandles::new("add-beside", object).hidden(args.hidden).connecting(args.connecting).ghost(ghost);
    if args.neighbour {
        handles = handles.clear_of(&clear);
    }
    let out = handles.show(ui, &canvas.view);

    let state = shared(ui, "add-beside", String::new);
    let mut note = lock(&state);
    if let Some(side) = out.clicked {
        let name = side_name(&labels, side);
        *note = craft_i18n::fmt(if out.copy { &words.copy } else { &words.clicked }, &[("side", &name)]);
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(story.reset.as_str())).clicked() {
            note.clear();
        }
        let text = match (out.hovered, note.is_empty()) {
            (Some(side), _) => craft_i18n::fmt(&words.hovered, &[("side", &side_name(&labels, side))]),
            (None, false) => note.clone(),
            (None, true) => words.none.clone(),
        };
        ui.add(Text::new(text).content(Content::Muted));
    });
}
