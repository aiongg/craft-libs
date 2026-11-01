//! [`AddBesideHandles`]: the "+" buttons beside the four sides of a selected object, which add an
//! artboard (Photoshop) or a connected object (FigJam) there. One widget for the Artboard tool and
//! the connector tools (tasks SH-T8 and TK3).
//!
//! The Artboard tool runs on the tool host and gets no `Ui`: it places, hit-tests and paints the
//! buttons through [`AddBesideHandles::buttons`], [`at`](AddBesideHandles::at) and
//! [`paint`](AddBesideHandles::paint). An overlay drawn with a `Ui` calls
//! [`show`](AddBesideHandles::show), which also senses clicks and keys.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Color32, IdSalt, Painter, Pos2, Stroke, Ui};
use kurbo::Rect;

use craft_ui::canvas::CanvasView;
use craft_ui::theme::{ToEgui, UiTokens};

craft_ui::words! {
    /// The "+" buttons' names, English by default. The host may name the group for what it adds
    /// ("Add artboard beside").
    #[non_exhaustive]
    pub struct AddBesideLabels {
        group: "addBeside.group" = "Add beside";
        above: "addBeside.above" = "Add above";
        right: "addBeside.right" = "Add right";
        below: "addBeside.below" = "Add below";
        left: "addBeside.left" = "Add left";
    }
}

/// A side of the object, in document space (never mirrored right to left).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddSide {
    Top,
    Right,
    Bottom,
    Left,
}

impl AddSide {
    /// Clockwise from the top: the buttons' keyboard order.
    pub const ALL: [AddSide; 4] = [AddSide::Top, AddSide::Right, AddSide::Bottom, AddSide::Left];
}

/// The "+" buttons' look and place, from tokens. The same at every Size (they sit on the
/// document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AddBesideStyle {
    /// From the object's edge to a button's centre, screen points: `add_beside.offset` (18).
    pub offset: f32,
    /// A button's radius: `add_beside.radius` (8).
    pub radius: f32,
    /// The plus's arm as a share of the radius: `add_beside.arm` (0.5).
    pub arm: f32,
    /// The plus: `add_beside.line` (1.5) of `accent.solid`.
    pub plus: Stroke,
    /// The ring: `add_beside.ring` (1) of `accent.solid`.
    pub ring: Stroke,
    /// The disc: `surface.raised`.
    pub fill: Color32,
    /// A hovered or focused disc: `accent.solid`, the plus then in `text.on_strong`.
    pub hot_fill: Color32,
    pub hot_plus: Color32,
}

/// What [`AddBesideHandles::show`] reports.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AddBesideResponse {
    /// A button was clicked, or Enter or Space pressed on the focused group.
    pub clicked: Option<AddSide>,
    /// ⌥ was held with the click: add with contents (an artboard's copy).
    pub copy: bool,
    /// The button under the pointer (the host shows its ghost).
    pub hovered: Option<AddSide>,
    /// The group has the keyboard focus.
    pub focused: bool,
}

/// The ghost the host draws for a hovered side.
type Ghost<'a> = dyn Fn(&Painter, AddSide) + 'a;

/// The "+" buttons beside the four sides of `object` (document units).
///
/// - Place: each centred on its side, [`AddBesideStyle::offset`] screen points outside the edge,
///   through the view (a turned view turns the sides with the object).
/// - Hidden: all of them when the object on screen is smaller than two radii on a side, or
///   [`hidden`](Self::hidden) is set (a drag runs); one when its disc touches a
///   [`clear_of`](Self::clear_of) rectangle (a size label, a toolbar) or leaves the canvas.
/// - Hover: the pointing hand cursor, the hot look, and the host's [`ghost`](Self::ghost) of what
///   would be added.
/// - Keyboard ([`show`](Self::show) only): the group is one Tab stop named by
///   [`AddBesideLabels::group`]; arrows pick a side, Enter or Space adds; each button is an
///   AccessKit `Button` ("Add above", …).
pub struct AddBesideHandles<'a> {
    id_salt: IdSalt,
    object: Rect,
    clear_of: &'a [egui::Rect],
    hidden: bool,
    ghost: Option<Box<Ghost<'a>>>,
    labels: Option<AddBesideLabels>,
}

impl<'a> AddBesideHandles<'a> {
    /// The buttons around `object`, in document units.
    pub fn new(id_salt: impl AsIdSalt, object: Rect) -> Self {
        AddBesideHandles { id_salt: IdSalt::new(id_salt), object, clear_of: &[], hidden: false, ghost: None, labels: None }
    }

    /// Screen rectangles no button may cover.
    pub fn clear_of(mut self, rects: &'a [egui::Rect]) -> Self {
        self.clear_of = rects;
        self
    }

    /// Hides every button (while a drag runs).
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Draws the host's ghost of what a hovered side would add (FigCraft: the next object and its
    /// connector, faint).
    pub fn ghost(mut self, ghost: impl Fn(&Painter, AddSide) + 'a) -> Self {
        self.ghost = Some(Box::new(ghost));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: AddBesideLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The buttons' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> AddBesideStyle {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:26-28 (`PLUS_OFFSET_PX` 18, `PLUS_RADIUS_PX`
        // 8), :274-279 (fill `card`, 1 pt accent ring, 1.5 pt plus, arm radius × 0.5).
        let (b, c) = (&tokens.component.add_beside, &tokens.color);
        let accent = c.accent.solid.to_egui();
        AddBesideStyle {
            offset: b.offset,
            radius: b.radius,
            arm: b.arm,
            plus: Stroke::new(b.line, accent),
            ring: Stroke::new(b.ring, accent),
            fill: c.surface.raised.to_egui(),
            hot_fill: accent,
            hot_plus: c.text.on_strong.to_egui(),
        }
    }

    /// The visible buttons' screen centres in [`AddSide::ALL`] order.
    pub fn buttons(&self, view: &CanvasView, style: &AddBesideStyle) -> Vec<(AddSide, Pos2)> {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:97-104 `plus_buttons` (offset over the zoom).
        // inv-figcraft: crates/ui-egui/src/canvas/board_connect.rs:45-52 `handle_at`, :57-59 (hidden
        // under two radii), :66-69 (clear of the canvas edge and the toolbar).
        todo!()
    }

    /// The button at the screen point `at`, within its radius.
    pub fn at(&self, view: &CanvasView, style: &AddBesideStyle, at: Pos2) -> Option<AddSide> {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:107-112 `plus_at`.
        todo!()
    }

    /// Draws the visible buttons, `hot` in the hot look with its ghost.
    pub fn paint(&self, painter: &Painter, view: &CanvasView, style: &AddBesideStyle, hot: Option<AddSide>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:269-281 (disc, ring, plus).
        // inv-figcraft: crates/ui-egui/src/canvas/board_connect.rs:76-84 (the hot look), :94-115 `preview`.
        todo!()
    }

    /// Draws the buttons, senses their clicks and the group's keys.
    pub fn show(self, ui: &mut Ui, view: &CanvasView) -> AddBesideResponse {
        // inv-figcraft: crates/ui-egui/src/canvas/board_connect.rs:54-92 `show`.
        // new: one `PartFocus` group (arrows pick, Enter or Space adds), `Sense::click` per disc.
        todo!()
    }
}
