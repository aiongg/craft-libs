//! `Inspector`: the body of a Properties panel, with a header for what it shows, an empty state and
//! widget state keyed by the target. `InspectorHeader` is the target's row. The look is the
//! `inspector.*` tokens.
//!
//! The target keys the controls' scrub and text-edit state. A host that keeps section open states
//! across targets, as PhotoCraft does across layers, gives each `SectionHeader` its state with
//! `.open(&mut bool)` from its own state: a header's id is relative to the target, so an id alone
//! restarts with it.

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, Frame, IdSalt, InnerResponse, Label, Margin, Rect, Response, ScrollArea, Sense, Ui, UiBuilder, WidgetInfo, vec2};

use crate::composites::EmptyState;
use crate::direction;
use crate::docking::DockLook;
use crate::icons::{self, Icon, IconSize};
use crate::localized::Labels as _;
use crate::primitives::Content;
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

crate::words! {
    /// The text an inspector writes itself.
    pub(crate) struct InspectorLabels {
        /// The message shown when nothing is inspected.
        empty: "inspector.empty" = "No properties";
    }
}

/// An inspector's sizes and colours, from [`Inspector::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct InspectorStyle {
    /// Height of the target header: `inspector.header_height`.
    pub header_height: f32,
    /// Side of the kind icon: `inspector.icon`.
    pub icon: f32,
    /// Padding round the body: `inspector.pad`.
    pub pad: f32,
    /// Between the icon, the name and the kind text: `space.gap.inline`.
    pub gap: f32,
    /// The kind icon: `text.muted`.
    pub icon_color: Color32,
    /// The name's type role: `body_strong`.
    pub title_role: TypeRole,
    /// The name: `inspector.title`.
    pub title: Color32,
    /// The kind text's type role: `caption`.
    pub kind_role: TypeRole,
    /// The kind text: `inspector.kind`.
    pub kind: Color32,
}

/// The row that names what an [`Inspector`] shows: an icon, the name and the kind of the target.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::InspectorHeader;
/// # egui::__run_test_ui(|ui| {
/// InspectorHeader::new(Icon::Layers, "Layer 1").kind("Pixel layer").show(ui);
/// # });
/// ```
///
/// - The row is `inspector.header_height` high. The name is cut with an ellipsis and shows its full
///   text as a tooltip when it does not fit. The kind text is at the end of the row and is hidden
///   when the row is narrower than the kind text plus 60 points.
/// - **AccessKit:** one `Header` node named "{name} - {kind}" (the name alone without a kind).
#[must_use = "show the header with `show`"]
pub struct InspectorHeader {
    icon: Option<Icon>,
    name: String,
    kind: Option<String>,
}

impl InspectorHeader {
    /// A header with a kind icon and the target's name.
    pub fn new(icon: Icon, name: impl Into<String>) -> Self {
        Self { icon: Some(icon), name: name.into(), kind: None }
    }

    /// A header of the name alone, without icon or kind ("Document", "3 Objects").
    pub fn title_only(name: impl Into<String>) -> Self {
        Self { icon: None, name: name.into(), kind: None }
    }

    /// The kind of the target ("Pixel layer"), at the end of the row.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Shows the row across the available width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Inspector::style(&tokens);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), style.header_height));
        let id = ui.make_persistent_id("inspector-header");
        let response = ui.interact(rect, id, Sense::hover());
        let accessible = match &self.kind {
            Some(kind) => format!("{} - {kind}", self.name),
            None => self.name.clone(),
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Header, ui.is_enabled(), &accessible));

        let forward = direction::forward(ui);
        let (start, end) = (direction::start_x(ui, rect), direction::end_x(ui, rect));
        let band = |from: f32, to: f32| Rect::from_x_y_ranges(from.min(to)..=from.max(to), rect.y_range());
        let mut name_start = start;
        if let Some(icon) = self.icon {
            let icon_rect = band(start, start + forward * style.icon);
            if ui.is_rect_visible(icon_rect) {
                icons::paint_in(ui, icon_rect, icon, icons::active(ui.ctx()), style.icon, IconSize::Md.stroke(), style.icon_color);
            }
            name_start = start + forward * (style.icon + style.gap);
        }

        let mut name_end = end;
        let kind = self.kind.as_deref().and_then(|kind| {
            let width = ui.painter().layout_no_wrap(kind.to_owned(), tokens.font(style.kind_role), style.kind).size().x;
            (rect.width() >= width + tokens.component.inspector.kind_min_name).then_some((kind, width))
        });
        if let Some((kind, width)) = kind {
            let kind_start = end - forward * width;
            name_end = kind_start - forward * style.gap;
            let format = role_format(&tokens, style.kind_role, style.kind);
            ui.scope_builder(UiBuilder::new().max_rect(band(kind_start, end)).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(egui::text::LayoutJob::single_section(kind.to_owned(), format)).extend().selectable(false));
            });
        }
        if forward * (name_end - name_start) >= 1.0 {
            let format = role_format(&tokens, style.title_role, style.title);
            ui.scope_builder(UiBuilder::new().max_rect(band(name_start, name_end)).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(egui::text::LayoutJob::single_section(self.name.clone(), format)).truncate().selectable(false));
            });
        }
        response
    }
}

enum HeaderKind {
    None,
    Header(InspectorHeader),
}

/// The body of a Properties panel: a vertical scroll area with the target's header, then the
/// host's content (`PropertyGroup`, `SectionHeader` and the controls), or an empty state.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::{Inspector, InspectorHeader};
/// # egui::__run_test_ui(|ui| {
/// Inspector::new("properties")
///     .target("layer-1")
///     .header(InspectorHeader::new(Icon::Layers, "Layer 1").kind("Pixel layer"))
///     .show(ui, |ui| {
///         ui.label("sections go here");
///     });
/// # });
/// ```
///
/// - The area fills the panel; the host's dock draws the panel frame, tabs and close.
/// - **Target.** [`target`](Inspector::target) keys the content's id, so scrub state and text
///   editing restart when the selection changes. A gesture on a control is dropped with it: the
///   host cancels the edit (`CancelReason::DocumentSwitched`). Section open states restart too
///   unless the host holds them: give each `SectionHeader` `.open(&mut bool)` from host state to
///   keep them across targets.
/// - **Empty.** With [`show_empty`](Inspector::show_empty) the message of [`empty`](Inspector::empty)
///   replaces the content, as an inline [`EmptyState`] and a polite status region.
/// - The inspector adds no row layout and no edit lifecycle: the controls inside keep theirs.
/// - **AccessKit:** the area is a `GenericContainer`; the header is a `Header`.
#[must_use = "show the inspector with `show`"]
pub struct Inspector {
    id_salt: IdSalt,
    target: Option<IdSalt>,
    header: HeaderKind,
    /// `None`: the installed words' "No properties".
    empty: Option<String>,
    show_empty: bool,
    scroll_bar_label: Option<String>,
}

impl Inspector {
    /// An inspector with this id. Its message when empty is "No properties".
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), target: None, header: HeaderKind::None, empty: None, show_empty: false, scroll_bar_label: None }
    }

    /// What the inspector shows: a document, a layer, a mask. The controls' state is kept per
    /// target. A `SectionHeader`'s id is relative to the target, so a host that wants the open
    /// states to survive a target change gives the header `.open(&mut bool)` from its own state.
    pub fn target(mut self, target: impl AsIdSalt) -> Self {
        self.target = Some(IdSalt::new(target));
        self
    }

    /// The row that names the target, above the content.
    pub fn header(mut self, header: InspectorHeader) -> Self {
        self.header = HeaderKind::Header(header);
        self
    }

    /// A header of the name alone.
    pub fn title_only(self, name: impl Into<String>) -> Self {
        self.header(InspectorHeader::title_only(name))
    }

    /// The message shown when [`show_empty`](Inspector::show_empty) is on.
    pub fn empty(mut self, message: impl Into<String>) -> Self {
        self.empty = Some(message.into());
        self
    }

    /// Shows the empty message instead of the content.
    pub fn show_empty(mut self, show_empty: bool) -> Self {
        self.show_empty = show_empty;
        self
    }

    /// The accessible name of the vertical scroll bar.
    pub fn scroll_bar_label(mut self, label: impl Into<String>) -> Self {
        self.scroll_bar_label = Some(label.into());
        self
    }

    pub fn style(tokens: &UiTokens) -> InspectorStyle {
        let i = &tokens.component.inspector;
        InspectorStyle {
            header_height: i.header_height,
            icon: i.icon,
            pad: i.pad,
            gap: tokens.space.gap.inline,
            icon_color: Content::Muted.color(tokens).to_egui(),
            title_role: TypeRole::BodyStrong,
            title: i.title.to_egui(),
            kind_role: TypeRole::Caption,
            kind: i.kind.to_egui(),
        }
    }

    /// The empty message alone, for a host that lays out its own panel: an inline [`EmptyState`]
    /// and a polite status region.
    pub fn empty_state(ui: &mut Ui, message: impl Into<String>) -> Response {
        EmptyState::new(message).status().id_salt("inspector-empty").show(ui).response
    }

    /// Shows the inspector in the available space.
    pub fn show(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
        let tokens = Tokens::of(ui);
        let style = Inspector::style(&tokens);
        let body_gap = DockLook::of(ui).body_gap;
        let scroll_bar_label = self.scroll_bar_label.unwrap_or_else(|| crate::patterns::panel::ScrollBarLabels::name(ui));
        let builder = UiBuilder::new().id_salt(self.id_salt).accessibility_role(Role::GenericContainer);
        ui.scope_builder(builder, |ui| {
            let mut area = ScrollArea::vertical().id_salt("scroll").auto_shrink([false, false]);
            area = area.vertical_scroll_bar_label(scroll_bar_label);
            area.show(ui, |ui| {
                Frame::new().inner_margin(Margin::same(style.pad.round() as i8)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = body_gap;
                    ui.push_id(self.target, |ui| {
                        if let HeaderKind::Header(header) = self.header {
                            header.show(ui);
                        }
                        if self.show_empty {
                            Inspector::empty_state(ui, self.empty.unwrap_or_else(|| InspectorLabels::of(ui).empty));
                        } else {
                            add_contents(ui);
                        }
                    });
                });
            });
        })
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Inspector"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/inspector/properties",
        Level::Patterns,
        "Inspector",
        "Properties",
        InspectorArgs { target: TargetArg::Layer, empty: false },
        show_story
    )]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum TargetArg {
    Layer,
    Document,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct InspectorArgs {
    target: TargetArg,
    empty: bool,
}

/// The values of the story's two targets.
#[derive(Clone, Copy, Debug)]
struct Values {
    x: f64,
    y: f64,
    opacity: f64,
}

/// What a target keeps: its values, and the values a running drag or scrub started from.
#[derive(Clone, Copy, Debug)]
struct Kept {
    values: Values,
    before: Option<Values>,
}

/// The section open states; one set for every target, as PhotoCraft keeps them across layers.
#[derive(Clone, Copy, Debug)]
struct Open {
    transform: bool,
    appearance: bool,
}

/// Applies a control's event to the kept values; `set` writes the control's value. A cancel puts
/// back the values at the Begin.
fn keep(kept: &mut Kept, event: Option<crate::edit::EditEvent<f64>>, set: fn(&mut Values, f64)) {
    use crate::edit::EditEvent;
    match event {
        Some(EditEvent::Begin { .. }) => kept.before = Some(kept.values),
        Some(EditEvent::Update { value, .. }) => set(&mut kept.values, value),
        Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
            kept.before = None;
            set(&mut kept.values, value);
        }
        Some(EditEvent::Cancel { .. }) => {
            if let Some(before) = kept.before.take() {
                kept.values = before;
            }
        }
        _ => {}
    }
}

/// PhotoCraft's Properties body: the target header, a Transform section and an Appearance
/// section. Each target keeps its own values; the sections' open states are shared.
fn show_story(ui: &mut Ui, args: &mut InspectorArgs) {
    use crate::composites::{PropertyGroup, PropertyLabel, PropertyRow, SectionHeader};
    use crate::controls::{NumberField, Slider};
    use crate::localized::LocaleNumbers;

    let layer = args.target == TargetArg::Layer;
    let state_id = ui.scope_id().with("inspector-story").with(layer);
    let open_id = ui.scope_id().with("inspector-story-open");
    let mut kept = ui.data(|d| d.get_temp::<Kept>(state_id)).unwrap_or(Kept { values: Values { x: 12.0, y: 8.0, opacity: 100.0 }, before: None });
    let mut open = ui.data(|d| d.get_temp::<Open>(open_id)).unwrap_or(Open { transform: true, appearance: true });
    let values = kept.values;
    let numbers = LocaleNumbers::from_tag("en");
    let (header, target) = if layer {
        (InspectorHeader::new(Icon::Layers, "Layer 1").kind("Pixel layer"), "layer-1")
    } else {
        (InspectorHeader::title_only("Document"), "document")
    };
    ui.allocate_ui(vec2(280.0_f32.min(ui.available_width()), 360.0), |ui| {
        Inspector::new("properties").target(target).header(header).empty("No properties").show_empty(args.empty).show(ui, |ui| {
            SectionHeader::new("transform", "Transform").open(&mut open.transform).show(ui, |ui| {
                PropertyGroup::new("transform-rows").show(ui, |ui| {
                    let (x, y) = PropertyRow::pair(PropertyLabel::glyph("X"), PropertyLabel::glyph("Y"))
                        .show(
                            ui,
                            |ui, _| NumberField::new("x", "X", values.x, &numbers).show(ui),
                            |ui, _| NumberField::new("y", "Y", values.y, &numbers).show(ui),
                        )
                        .inner;
                    keep(&mut kept, x.event, |v, x| v.x = x);
                    keep(&mut kept, y.event, |v, y| v.y = y);
                });
            });
            if layer {
                SectionHeader::new("appearance", "Appearance").open(&mut open.appearance).show(ui, |ui| {
                    keep(&mut kept, Slider::new("opacity", "Opacity", values.opacity, 0.0..=100.0).show(ui).event, |v, o| v.opacity = o);
                });
            }
        });
    });
    ui.data_mut(|d| {
        d.insert_temp(state_id, kept);
        d.insert_temp(open_id, open);
    });
}
