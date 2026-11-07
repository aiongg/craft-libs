//! `SplitButton`: an action button whose end part opens a menu of variants or presets.

use super::face::{self, FaceLook, FaceSpec};
use crate::contract::{
    Access, Bind, CollectionEvent, Common, Component, Cx, Focus as _, LeadOnOpen, MeasureCx, Name, Outcome, Overlay, OverlayUi, Overlays as _,
    PaintUi, PartsCx, Pointer as _, Props as _, Role, ShowCollection as _, Styled as _,
};
use crate::controls::select::{OptionList, OptionRow};
use crate::primitives::{IconSpec, PaintShape, StrokeWidth};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Corners, FocusLook, IconLook, Stateful, StyleColor, StyleCx, SurfaceLook, TextLook};
use craft_behavior::{DisplayOrder, Flow, HitLevel, HitPriority, IconId, Intent, ItemSource, Metrics, NavMove, PartKey, Selection, Sense, Value};
use craft_theme::{
    States, Variant,
    variant::{Emphasis, Tone},
};
use egui::{CornerRadius, Pos2, Rect, Response, Vec2};

const ARROW: PartKey = PartKey::new("arrow");

/// The style of a split button: one surface under both parts, each part's hover and pressed fill over it, and a
/// divider between them.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[style(variants = [Variant::SECONDARY, Variant::PRIMARY, Variant::GHOST, Variant::DANGER, Variant::look(Emphasis::Outline, Tone::Accent)])]
#[non_exhaustive]
pub struct SplitButtonStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The label.
    #[style(text)]
    text: TextLook,
    /// The leading icon.
    #[style(icon)]
    icon: IconLook,
    face: FaceLook,
    /// What each part adds over the surface: clear at rest, the variant's hover and pressed fills.
    part_fill: Stateful<StyleColor>,
    /// The line between the parts, in the variant's edge colour.
    divider: Stateful<StyleColor>,
    /// The divider's gap from the top and bottom edges.
    divider_inset: f32,
    radius: CornerRadius,
}

/// One entry of a split button's menu: the value its pick reports and the label it shows.
#[non_exhaustive]
pub struct MenuEntry<T> {
    /// The value reported when this entry is picked.
    pub value: T,
    /// The text the entry shows.
    pub label: Label,
}

impl<T> MenuEntry<T> {
    /// An entry reporting `value`, shown as `label`.
    pub fn new(value: T, label: impl Into<Label>) -> Self {
        MenuEntry { value, label: label.into() }
    }
}

/// What a split button reports: the main part's response (`clicked()` runs the action) and the entry picked from
/// the menu this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct SplitOutput<T> {
    /// The main part's response.
    pub response: Response,
    /// The entry the user picked.
    pub chosen: Option<T>,
}

impl<T: Value> Outcome for SplitOutput<T> {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A button whose end opens a menu. A press on the arrow part opens the menu below the button; a press on the main
/// part runs the action and never opens it. The button is one Tab stop: Space and Enter run the action, Alt+Down and
/// Down open the menu with its first row active, Escape closes it with focus back on the button.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(SplitButton<'static, u8>))]
pub struct SplitButton<'a, T: Value + 'static> {
    #[common]
    common: Common,
    entries: &'a [MenuEntry<T>],
    /// The arrow's accessible name and tooltip ("Tracing Presets"); the label alone names it without one.
    #[prop(into)]
    arrow_label: Option<Label>,
    /// A leading icon on the main part.
    #[prop]
    icon: Option<IconId>,
    chosen: Option<T>,
}

impl<'a, T: Value + 'static> SplitButton<'a, T> {
    /// A split button showing `label` whose arrow opens `entries`.
    pub fn new(label: impl Into<Label>, entries: &'a [MenuEntry<T>]) -> Self {
        SplitButton { common: Common::new(), entries, arrow_label: None, icon: None, chosen: None }.label(label)
    }

    fn spec<'s>(&'s self, label: &'s str) -> FaceSpec<'s> {
        FaceSpec { label, role: None, lead: self.icon.as_ref(), trail: None, loading: false }
    }
}

impl<T: Value + 'static> Component for SplitButton<'_, T> {
    const ROLE: Role = Role::BUTTON;
    type Style = SplitButtonStyle;
    type Output = SplitOutput<T>;

    fn style(cx: &StyleCx<'_>, _: SplitButtonStyleKey<T>) -> SplitButtonStyle {
        let parts = cx.variant_look();
        let mut surface = face::surface(cx, &parts);
        let (fill, clear) = (parts.fill.off, cx.color(craft_theme::Rgba::TRANSPARENT));
        surface.fill = Stateful::new(fill.rest).off(States::new(fill.rest, fill.rest, fill.rest, fill.disabled));
        let divider = parts.edge.unwrap_or_else(|| Stateful::new(clear));
        let (text, icon) = face::ink(cx);
        SplitButtonStyle {
            surface,
            metrics: cx.control().metrics(),
            focus: face::focus(cx),
            text,
            icon,
            face: FaceLook::of(cx),
            part_fill: Stateful::new(clear).off(States::new(clear, fill.hover, fill.pressed, clear)),
            divider,
            divider_inset: cx.tokens.space.gap.inline,
            radius: parts.radius,
        }
    }

    /// The face and the arrow part, which sits in the end padding.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let face = face::measure(cx, &self.spec(cx.resolved.label));
        Vec2::new(face.x + cx.icon(), face.y)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let (arrow, _) = Flow::new(rect, cx.flow.is_rtl()).take_end(cx.icon());
        let part =
            cx.part(ARROW, arrow).sense(Sense::CLICK).priority(HitPriority::new(HitLevel::Handle, 0)).pointer_only("Alt+Down and Down open the menu");
        if let Some(label) = &self.arrow_label {
            part.tooltip(label.clone());
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.join_roving();
        let menu = cx.popup("menu");
        let open = menu.is_open(cx);
        if cx.part(ARROW).press.phase.is_click() {
            if open {
                menu.close(cx);
            } else {
                menu.open(cx);
            }
        } else if !open {
            let mut asked = cx.intents().take(Intent::Open);
            while let Some((step, _)) = cx.intents().take_move() {
                asked |= matches!(step, NavMove::Down);
            }
            if asked {
                menu.open(cx);
            }
        }
    }

    /// The menu: one row per entry. A pick comes back from the list as Enter or a click on a row.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        let labels: Vec<Label> = self.entries.iter().map(|entry| cx.env.resolve(&entry.label)).collect();
        let events = ui.show(popup, Overlay::menu(), |ui, _| {
            let rows = Entries { labels: &labels };
            OptionList::new(Bind::given(Selection::default()))
                .lead_on_open(LeadOnOpen::First)
                .lead_follows_hover(true)
                .show_items(ui, &rows, &mut |key, _| OptionRow::new(labels[*key].as_str()))
                .events
        });
        let mut picked = None;
        for event in events.into_iter().flatten() {
            if let CollectionEvent::Activate(at) = event {
                picked = Some(at);
            }
        }
        if let Some(at) = picked {
            self.chosen = Some(self.entries[at].value.clone());
            popup.close(cx);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let name = match &self.arrow_label {
            Some(label) => cx.env.resolve(label).as_str().to_owned(),
            None => cx.resolved.label.to_owned(),
        };
        access.node(ARROW, Role::BUTTON).name = Name::Text(name);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let (rtl, direction) = (cx.flow.is_rtl(), cx.env.direction());
        let (arrow, main) = Flow::new(cx.rect, rtl).take_end(cx.style.icon.side);
        let main = main.rect();
        let main_fill = cx.root_state().resolve(&cx.style.part_fill);
        let draw = ui.draw();
        draw.fill(&PaintShape::Rect(main, Corners::Start.apply(cx.style.radius, direction)), main_fill);
        draw.fill(&PaintShape::Rect(arrow, Corners::End.apply(cx.style.radius, direction)), cx.resolve_part(ARROW, &cx.style.part_fill));
        let area = Flow::new(main, rtl).inset(cx.style.metrics.padding);
        face::paint(ui, cx, (&cx.style.text, &cx.style.icon), &cx.style.face, &self.spec(cx.resolved.label), area);
        let draw = ui.draw();
        let side = cx.style.icon.side;
        let chevron = Rect::from_center_size(arrow.center(), Vec2::splat(side));
        draw.icon(chevron, &IconSpec::svg(IconId::new("chevron_down")).color(cx.resolve(&cx.style.icon.color)));
        let x = if rtl { arrow.right() } else { arrow.left() };
        let inset = cx.style.divider_inset;
        draw.stroke(
            &PaintShape::Path(vec![Pos2::new(x, cx.rect.top() + inset), Pos2::new(x, cx.rect.bottom() - inset)]),
            StrokeWidth::Thin,
            cx.resolve(&cx.style.divider),
        );
    }

    fn finish(self, cx: Cx<'_, Self>) -> SplitOutput<T> {
        SplitOutput { response: cx.response.clone(), chosen: self.chosen }
    }
}

/// The labels of a split button's entries as a collection source, keyed by position: the entries are fixed while the
/// menu is open.
struct Entries<'a> {
    labels: &'a [Label],
}

impl DisplayOrder<usize> for Entries<'_> {
    fn len(&self) -> usize {
        self.labels.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.labels.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.labels[at].as_str()))
    }
}

impl ItemSource<usize> for Entries<'_> {}
