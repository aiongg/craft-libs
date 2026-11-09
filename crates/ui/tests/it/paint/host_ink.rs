//! A face reads the ink and the validity flags of the part it is the look of: `StyleCx::host_ink` paints the host's
//! text colour inside a face and the face's own colour alone.

use craft_behavior::{Metrics, PartKey, Sense};
use craft_theme::variant::Tone;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, PaintUi, PartsCx, Role, Show as _, Styled as _};
use craft_ui::primitives::PaintShape;
use craft_ui::style::{ComponentStyle, Flags, Stateful, States, StyleColor, StyleCx, TextLook, TypeRole};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, CornerRadius, Rect, Response, Vec2};
use std::cell::Cell;

const FACE: PartKey = PartKey::new("face");
const SIDE: f32 = 10.0;

thread_local! {
    /// The flags the last face on this thread painted with.
    static SEEN: Cell<Flags> = const { Cell::new(Flags::empty()) };
}

#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct SwatchStyle {
    #[style(text)]
    text: TextLook,
    #[style(metrics)]
    metrics: Metrics,
}

/// A host whose text colour is the normal text colour, or the colour on a selected fill while selected. It paints a
/// swatch of that colour and shows an `Ink` face on its part.
#[derive(Component)]
struct Host {
    #[common]
    common: Common,
    selected: bool,
    invalid: bool,
}

impl Component for Host {
    const ROLE: Role = Role::None;
    type Style = SwatchStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: HostStyleKey) -> SwatchStyle {
        let text = &cx.tokens.color.text;
        let on = cx.color(text.on_selected);
        let color = Stateful::new(cx.color(text.normal)).on(States::new(on, on, on, on));
        SwatchStyle { text: TextLook::new(TypeRole::Label, color), metrics: Metrics::new() }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(60.0, 30.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let flags = if self.selected { Flags::SELECTED } else { Flags::empty() };
        cx.part(FACE, Rect::from_min_size(rect.min + Vec2::splat(5.0), Vec2::splat(SIDE))).sense(Sense::HOVER).flags(flags);
    }

    fn flags(&self) -> Flags {
        let selected = if self.selected { Flags::SELECTED } else { Flags::empty() };
        selected | if self.invalid { Flags::INVALID } else { Flags::empty() }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let swatch = Rect::from_min_size(cx.rect.min + Vec2::new(30.0, 5.0), Vec2::splat(SIDE));
        ui.draw().fill(&PaintShape::Rect(swatch, CornerRadius::ZERO), cx.resolve(&cx.style.text.color));
        ui.face(FACE, Ink { common: Common::new() });
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct InkStyle {
    ink: Stateful<StyleColor>,
    #[style(metrics)]
    metrics: Metrics,
}

/// A face that fills its rect with the host's ink, or with the warning colour when it stands alone.
#[derive(Component)]
struct Ink {
    #[common]
    common: Common,
}

impl Component for Ink {
    const ROLE: Role = Role::None;
    type Style = InkStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: InkStyleKey) -> InkStyle {
        InkStyle { ink: cx.host_ink(Stateful::new(cx.tone(Tone::Warn))), metrics: Metrics::new().min_size(Vec2::splat(SIDE)) }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        SEEN.set(cx.state().flags);
        ui.draw().fill(&PaintShape::Rect(cx.rect, CornerRadius::ZERO), cx.resolve(&cx.style.ink));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn fill_at(shapes: &[egui::Shape], rect: Rect) -> Option<Color32> {
    shapes.iter().find_map(|shape| match shape {
        egui::Shape::Rect(shape) if shape.rect == rect => Some(shape.fill),
        _ => None,
    })
}

/// The fills of the host's swatch and of its face, in a settled frame.
fn host_fills(selected: bool, invalid: bool) -> (Color32, Color32) {
    let mut frames = Frames::new();
    let mut host = Rect::NOTHING;
    let mut shapes = Vec::new();
    for _ in 0..3 {
        shapes = frames.frame_shapes(|ui| host = Host { common: Common::new(), selected, invalid }.show(ui).rect);
    }
    let swatch = Rect::from_min_size(host.min + Vec2::new(30.0, 5.0), Vec2::splat(SIDE));
    let face = Rect::from_min_size(host.min + Vec2::splat(5.0), Vec2::splat(SIDE));
    (fill_at(&shapes, swatch).expect("the host's swatch"), fill_at(&shapes, face).expect("the face"))
}

fn alone_fill() -> Color32 {
    let mut frames = Frames::new();
    let mut rect = Rect::NOTHING;
    let mut shapes = Vec::new();
    for _ in 0..2 {
        shapes = frames.frame_shapes(|ui| rect = Ink { common: Common::new() }.show(ui).rect);
    }
    fill_at(&shapes, rect).expect("the face alone")
}

/// A face whose style uses `host_ink` paints the host's ink, which differs between a plain and a selected host, and
/// its own colour when shown alone.
#[trial]
fn a_face_reads_the_ink_and_validity_of_its_host() {
    {
        let (plain_host, plain_face) = host_fills(false, false);
        let (selected_host, selected_face) = host_fills(true, false);
        assert_eq!(plain_face, plain_host, "the face reads the host's ink");
        assert_eq!(selected_face, selected_host, "the face reads the host's ink while selected");
        assert_ne!(selected_face, plain_face, "the host's ink changes with its selection");
        let alone = alone_fill();
        assert!(alone != plain_face && alone != selected_face, "alone, the face has its own colour");
    }
    {
        host_fills(false, true);
        assert!(SEEN.get().contains(Flags::INVALID));
        host_fills(false, false);
        assert!(!SEEN.get().contains(Flags::INVALID));
    }
}
