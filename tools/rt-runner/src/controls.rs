//! `IconButton` and `ToggleCell`: the row's and footer's pointer parts (look sheet 3.2, 3.3,
//! 3.7, 3.12).

use std::rc::Rc;

use accesskit::{Action, Node, Role, Toggled};
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, Element, Event, EventCx, Gesture, Handled, ImageKey, Key, LayoutCx, PaintCx, Rect, Rgba, Sense, Stroke, Vec2,
    hash_key,
};

use crate::icons::{self, Icon};
use crate::look::{Look, icon_square};

pub type Callback = Rc<dyn Fn(&mut EventCx<'_>)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonKind {
    /// The trailing lock indicator: glyph 12 on a 16 plate, faint ink at rest.
    Indicator,
    /// A footer action: glyph 16 (craft's `GlyphRole::Tool`, `size.icon.md`) on a 24 plate, a Tab stop.
    Footer,
}

impl ButtonKind {
    /// The glyph's side in points.
    fn glyph(self, look: &Look) -> f32 {
        match self {
            ButtonKind::Indicator => look.glyph,
            ButtonKind::Footer => look.icon,
        }
    }
}

/// A ghost icon button: transparent plate, hover and pressed fills, a centred glyph.
pub struct IconButton {
    icon: Icon,
    kind: ButtonKind,
    name: Rc<str>,
    on_press: Option<Callback>,
}

impl IconButton {
    pub fn new(kind: ButtonKind, icon: Icon, name: impl Into<Rc<str>>) -> Self {
        IconButton { icon, kind, name: name.into(), on_press: None }
    }

    pub fn on_press(mut self, f: impl Fn(&mut EventCx<'_>) + 'static) -> Self {
        self.on_press = Some(Rc::new(f));
        self
    }

    fn press(&self, cx: &mut EventCx<'_>) {
        if let Some(f) = &self.on_press {
            f(cx);
        }
    }
}

pub struct ButtonLocal {
    mask: ImageKey,
}

impl Element for IconButton {
    type Output = ();
    type Local = Option<ButtonLocal>;

    fn props_key(&self) -> u64 {
        hash_key(&(self.icon, self.kind, &*self.name))
    }

    fn init(&self, _: &mut BuildCx<'_>) -> Option<ButtonLocal> {
        None
    }

    fn build(&self, local: &mut Option<ButtonLocal>, cx: &mut BuildCx<'_>) {
        let look = Look::of(&cx.tokens());
        let scale = cx.env().scale.get(cx);
        let (px, stroke) = icons::pixels(self.kind.glyph(&look), scale, look.icon_stroke);
        *local = Some(ButtonLocal { mask: icons::mask(cx, self.icon, px, stroke) });
    }

    fn layout(&self, _: &mut Option<ButtonLocal>, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let look = Look::of(&cx.tokens());
        let side = match self.kind {
            ButtonKind::Indicator => look.icon,
            ButtonKind::Footer => look.control,
        };
        c.constrain(Vec2::new(side, side))
    }

    fn paint(&self, local: &Option<ButtonLocal>, cx: &mut PaintCx<'_>) {
        let look = Look::of(&cx.tokens());
        let scale = cx.scale();
        let bounds = cx.bounds();
        // A press leaves the focus where it was: craft's buttons take it by Tab only.
        cx.hit(bounds, Sense::HOVER | Sense::CLICK | Sense::KEEP_FOCUS, 0);
        let state = cx.interaction();
        if state.pressed {
            cx.rect(bounds, look.radius_control, look.pressed, None);
        } else if state.hovered {
            cx.rect(bounds, look.radius_control, look.hover, None);
        }
        let ink = match self.kind {
            ButtonKind::Indicator if !(state.hovered || state.pressed) => look.text_faint,
            _ => look.text,
        };
        if let Some(local) = local {
            let (square, _) = icon_square(bounds.center(), self.kind.glyph(&look), scale);
            cx.image(local.mask, square, ink);
        }
        if state.focus_visible {
            let ring = Stroke::new(look.focus_width, look.focus);
            cx.over(|cx| cx.rect(bounds, look.radius_control, Rgba::TRANSPARENT, Some(ring)));
        }
    }

    fn event(&self, _: &mut Option<ButtonLocal>, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => {
                if let Gesture::Up { click: true, .. } = p.gesture {
                    self.press(cx);
                }
                Handled::Yes
            }
            Event::Key(k) if k.down && matches!(k.key, Key::Enter | Key::Space) => {
                self.press(cx);
                Handled::Yes
            }
            Event::Access(a) if a.action == Action::Click => {
                self.press(cx);
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, _: &Option<ButtonLocal>, _: &mut AccessCx<'_>) -> Option<Node> {
        let mut node = Node::new(Role::Button);
        node.set_label(&*self.name);
        node.add_action(Action::Click);
        if self.kind == ButtonKind::Footer {
            // craft's footer actions are toggle buttons, off unless an action says it is on.
            node.set_toggled(Toggled::False);
            node.add_action(Action::Focus);
        }
        Some(node)
    }

    fn focusable(&self) -> bool {
        self.kind == ButtonKind::Footer
    }
}

/// The face of a toggle cell: its on icon, and the icon it shows under the pointer while off
/// (`None`: empty when off).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToggleFace {
    pub on: Icon,
    pub off_hover: Option<Icon>,
}

/// The eye or lock cell of a row. A press flips it at once; its access node is the row's.
pub struct ToggleCell {
    face: ToggleFace,
    on: bool,
    on_selected_row: bool,
    on_toggle: Option<Callback>,
}

impl ToggleCell {
    pub fn new(face: ToggleFace, on: bool, on_selected_row: bool) -> Self {
        ToggleCell { face, on, on_selected_row, on_toggle: None }
    }

    pub fn on_toggle(mut self, f: impl Fn(&mut EventCx<'_>) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

pub struct ToggleLocal {
    on: ImageKey,
    off_hover: Option<ImageKey>,
}

impl Element for ToggleCell {
    type Output = ();
    type Local = Option<ToggleLocal>;

    fn props_key(&self) -> u64 {
        hash_key(&(self.face, self.on, self.on_selected_row))
    }

    fn init(&self, _: &mut BuildCx<'_>) -> Option<ToggleLocal> {
        None
    }

    fn build(&self, local: &mut Option<ToggleLocal>, cx: &mut BuildCx<'_>) {
        let look = Look::of(&cx.tokens());
        let scale = cx.env().scale.get(cx);
        let (px, stroke) = icons::pixels(look.icon, scale, look.icon_stroke);
        *local = Some(ToggleLocal {
            on: icons::mask(cx, self.face.on, px, stroke),
            off_hover: self.face.off_hover.map(|icon| icons::mask(cx, icon, px, stroke)),
        });
    }

    fn layout(&self, _: &mut Option<ToggleLocal>, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let look = Look::of(&cx.tokens());
        c.constrain(Vec2::new(look.toggle_width, look.row_height))
    }

    fn paint(&self, local: &Option<ToggleLocal>, cx: &mut PaintCx<'_>) {
        let look = Look::of(&cx.tokens());
        let scale = cx.scale();
        let bounds: Rect = cx.bounds();
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
        let Some(local) = local else { return };
        let state = cx.interaction();
        let hovered = state.hovered || state.pressed;
        let mask = match (self.on, hovered) {
            (true, _) => local.on,
            (false, true) => match local.off_hover {
                Some(mask) => mask,
                None => return,
            },
            (false, false) => return,
        };
        let ink = if self.on_selected_row {
            look.text_on_selected
        } else if hovered {
            look.text
        } else {
            look.text_muted
        };
        let (square, _) = icon_square(bounds.center(), look.icon, scale);
        cx.image(mask, square, ink);
    }

    fn event(&self, _: &mut Option<ToggleLocal>, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        if let Event::Pointer(p) = ev {
            if matches!(p.gesture, Gesture::Down { .. })
                && let Some(f) = &self.on_toggle
            {
                f(cx);
            }
            return Handled::Yes;
        }
        Handled::No
    }
}
