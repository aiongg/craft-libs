//! [`SectionHeader`]: the header row of a section of a panel, collapsible or always open.

use craft_theme::{ControlSize, Stroke as Hairline, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{Color32, CursorIcon, IdSalt, InnerResponse, Label, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use super::{PropertyGroup, PropertyLabel, PropertyRow, measure};
use crate::controls::{IconButton, NumberField};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::icons::{self, Icon, IconSize};
use crate::localized::LocaleNumbers;
use crate::primitives::{Content, Interactive};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// The closure that adds a header's trailing controls.
type Controls<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A section header's row, chevron and title, from [`SectionHeader::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SectionStyle {
    /// Height of the row under the hairline: `section.row_height`.
    pub row_height: f32,
    /// Side of the chevron icon: `section.chevron`.
    pub chevron: f32,
    /// Side of the box the chevron is centred in: `section.chevron_box`.
    pub chevron_box: f32,
    /// From the row's start to the title: `section.title_offset`.
    pub title_offset: f32,
    /// Space above the hairline, and half of it under the row of an open section (PhotoCraft's
    /// `ROW_GAP`): `space.gap.row`.
    pub gap: f32,
    /// The hairline above the row: `line.divider`, one physical pixel.
    pub hairline: Stroke,
    /// The chevron: `text.muted`.
    pub chevron_color: Color32,
    /// The title's type role: `body_strong`.
    pub title_role: TypeRole,
    /// The title: `text.normal`.
    pub title: Color32,
}

/// The header of a section of a panel: a hairline above a row with a chevron and a title, as
/// PhotoCraft's property sections have it. Clicking anywhere on the row, or pressing Enter or Space
/// while it has focus, opens or closes the section; the row has no hover fill and draws the focus
/// ring when the keyboard focus is on it.
///
/// ```
/// use craft_ui::composites::SectionHeader;
/// # egui::__run_test_ui(|ui| {
/// SectionHeader::new("transform", "Transform").show(ui, |ui| {
///     ui.label("The section's rows.");
/// });
/// # });
/// ```
///
/// - **Open state** is kept in egui's memory, persisted with it, and the section is open at first.
///   [`open`](SectionHeader::open) hands the state to the host instead.
/// - **Body.** [`show`](SectionHeader::show) runs the body closure while the section is open and
///   returns its result. One closure form was chosen over a header-only call plus a flag: the body
///   cannot be drawn for a closed section by mistake, and the header and body share the state
///   without the caller threading it.
/// - [`collapsible(false)`](SectionHeader::collapsible) is the always-open header (DesignCraft): the
///   title alone, no chevron, not a Tab stop, the body always shown.
/// - [`controls`](SectionHeader::controls) puts controls at the trailing end of the title row, in
///   reading order (Tab and AccessKit visit them that way); the title is cut with an ellipsis (and
///   shows its full text as a tooltip) to leave them room, and is not drawn when they reach its
///   start.
/// - **AccessKit:** a collapsible header is a `DisclosureTriangle` named by the title and
///   reporting `expanded`.
#[must_use = "show the header with `show`"]
pub struct SectionHeader<'a> {
    title: String,
    id_salt: IdSalt,
    collapsible: bool,
    open: Option<&'a mut bool>,
    controls: Option<Controls<'a>>,
}

impl<'a> SectionHeader<'a> {
    /// A header titled `title`. `id_salt` is its id, under which the open state is kept: it
    /// stays when the title changes (a language switch), and two sections of one title keep
    /// separate states.
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), title: title.into(), collapsible: true, open: None, controls: None }
    }

    /// Whether the section can be closed; on by default. Off, the header is its title only and the
    /// body is always shown.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// The host owns the open state: the header shows `open` and a click or key flips it.
    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Controls at the trailing end of the title row (DesignCraft's `section_row`). The closure
    /// adds them in a row in reading order that ends at the row's trailing edge.
    pub fn controls(mut self, add_controls: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.controls = Some(Box::new(add_controls));
        self
    }

    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> SectionStyle {
        let s = &tokens.component.section;
        SectionStyle {
            row_height: s.row_height,
            chevron: s.chevron,
            chevron_box: s.chevron_box,
            title_offset: s.title_offset,
            gap: tokens.space.gap.row,
            hairline: Stroke::new(Hairline::Hairline.width(pixels_per_point), tokens.color.line.divider.to_egui()),
            chevron_color: Content::Muted.color(tokens).to_egui(),
            title_role: TypeRole::BodyStrong,
            title: Content::Normal.color(tokens).to_egui(),
        }
    }

    /// Shows the header and, while the section is open, `add_body` under it. The response is the
    /// header row's, marked changed on the frame the section opens or closes; the inner value is
    /// the body's result, `None` while closed.
    pub fn show<R>(self, ui: &mut Ui, add_body: impl FnOnce(&mut Ui) -> R) -> InnerResponse<Option<R>> {
        let tokens = Tokens::of(ui);
        let style = SectionHeader::style(&tokens, ui.pixels_per_point());
        let hairline = style.hairline.width;
        let rtl = direction::is_rtl(ui);
        ui.add_space(style.gap);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), hairline + style.row_height));
        let row = Rect::from_min_max(pos2(rect.left(), rect.top() + hairline), rect.max);
        ui.painter().hline(rect.x_range(), (rect.top() + hairline / 2.0).round_to_pixel_center(ui.pixels_per_point()), style.hairline);

        let id = ui.make_persistent_id(self.id_salt);
        let stored = || ui.ctx().data_mut(|d| d.get_persisted::<bool>(id)).unwrap_or(true);
        let mut open = match (self.collapsible, &self.open) {
            (false, _) => true,
            (true, Some(host)) => **host,
            (true, None) => stored(),
        };

        let mut response = if self.collapsible {
            let element = Interactive::salted(self.id_salt, Role::DisclosureTriangle, self.title.as_str())
                .sense(Sense::click())
                .expanded(open)
                .state_layer(false);
            let sensed = element.interact_at(ui, row);
            let clicked = sensed.response().clicked();
            let response = sensed.finish(ui).on_hover_cursor(CursorIcon::PointingHand);
            if clicked {
                open = !open;
                // The node was reported with the state before the click.
                ui.ctx().accesskit_node_builder(response.id, |node| node.set_expanded(open));
            }
            response
        } else {
            ui.interact(row, id, Sense::hover())
        };
        if self.collapsible && response.clicked() {
            response.mark_changed();
            match self.open {
                Some(host) => *host = open,
                None => ui.ctx().data_mut(|d| d.insert_persisted(id, open)),
            }
        }

        // The chevron's box starts a point in from the row's start; the title is
        // `section.title_offset` from it.
        let x = |offset: f32| if rtl { row.right() - offset } else { row.left() + offset };
        let mut title_end = direction::end_x(ui, row);
        let mut title_start = x(0.0);
        if self.collapsible {
            let centre = pos2(x(1.0 + style.chevron_box / 2.0), row.center().y);
            let icon = if open { Icon::ChevronDown } else { Icon::ChevronRight };
            icons::paint_in(
                ui,
                Rect::from_center_size(centre, vec2(style.chevron_box, style.chevron_box)),
                icon,
                icons::active(ui.ctx()),
                style.chevron,
                IconSize::Sm.stroke(),
                style.chevron_color,
            );
            title_start = x(style.title_offset);
        }
        // The title's `Ui` is made before the controls' so that AccessKit lists it first.
        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(row));
        if let Some(add_controls) = self.controls {
            let gap = tokens.space.gap.inline;
            let ((), used) = measure::trailing_row(ui, id.with("controls"), row, Sense::hover(), gap, 0.0, add_controls);
            if used.width() > 0.0 {
                title_end = direction::start_x(ui, used) - direction::forward(ui) * gap;
            }
        }
        if direction::forward(ui) * (title_end - title_start) >= 1.0 {
            let band = Rect::from_x_y_ranges(title_start.min(title_end)..=title_start.max(title_end), row.y_range());
            let format = role_format(&tokens, style.title_role, style.title);
            title_ui.scope_builder(UiBuilder::new().max_rect(band).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(egui::text::LayoutJob::single_section(self.title.clone(), format)).truncate().selectable(false));
            });
        }

        let inner = open.then(|| {
            ui.add_space(style.gap / 2.0);
            add_body(ui)
        });
        InnerResponse::new(inner, response)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SectionHeader"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/section-header/collapsible",
            Level::Composites,
            "SectionHeader",
            "Collapsible",
            WidthArgs::default(),
            show_collapsible
        ),
        typed_story!(
            "composites/section-header/always-open",
            Level::Composites,
            "SectionHeader",
            "Always open",
            WidthArgs::default(),
            show_always_open
        ),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct WidthArgs {
    #[arg(range = 60.0..=400.0)]
    pub(super) width: f32,
}

impl Default for WidthArgs {
    fn default() -> Self {
        Self { width: 260.0 }
    }
}

/// The numbers of the stories' inspectors.
#[derive(Clone, Copy, Debug)]
struct Values {
    w: f64,
    h: f64,
    x: f64,
    y: f64,
    opacity: f64,
    linked: bool,
}

fn values(ui: &Ui, id: &'static str) -> (egui::Id, Values) {
    let id = ui.scope_id().with(id);
    let v = ui.data(|d| d.get_temp::<Values>(id)).unwrap_or(Values { w: 640.0, h: 480.0, x: 12.0, y: 8.0, opacity: 100.0, linked: true });
    (id, v)
}

fn apply(edit: &EditResponse<f64>, value: &mut f64) {
    if let Some(EditEvent::Update { value: v, .. } | EditEvent::Set { value: v, .. } | EditEvent::Commit { value: v, .. }) = edit.event {
        *value = v;
    }
}

/// The rows of a Transform section: W and H with a link toggle, X and Y.
fn transform_rows(ui: &mut Ui, v: &mut Values, numbers: &LocaleNumbers) {
    PropertyGroup::new("section-transform").show(ui, |ui| {
        let linked = &mut v.linked;
        let (w, _, h) = PropertyRow::pair(PropertyLabel::glyph("W"), PropertyLabel::glyph("H").redundant())
            .show_linked(
                ui,
                |ui, _| NumberField::new("w", "Width", v.w, numbers).show(ui),
                |ui| {
                    ui.add(
                        IconButton::new(if *linked { Icon::Link } else { Icon::Unlink }, "Link width and height")
                            .size(ControlSize::Sm)
                            .toggle(linked),
                    )
                },
                |ui, _| NumberField::new("h", "Height", v.h, numbers).show(ui),
            )
            .inner;
        apply(&w, &mut v.w);
        apply(&h, &mut v.h);
        let (x, y) = PropertyRow::pair(PropertyLabel::glyph("X"), PropertyLabel::glyph("Y"))
            .show(ui, |ui, _| NumberField::new("x", "X", v.x, numbers).show(ui), |ui, _| NumberField::new("y", "Y", v.y, numbers).show(ui))
            .inner;
        apply(&x, &mut v.x);
        apply(&y, &mut v.y);
    });
}

fn opacity_row(ui: &mut Ui, v: &mut Values, numbers: &LocaleNumbers) {
    PropertyGroup::new("section-appearance").show(ui, |ui| {
        let edit = PropertyRow::word("Opacity")
            .scrubs()
            .show(ui, |ui, row| {
                let field = NumberField::new("opacity", "Opacity", v.opacity, numbers);
                match row.scrub() {
                    Some(label) => field.scrub_by(label),
                    None => field,
                }
                .show(ui)
            })
            .inner;
        apply(&edit, &mut v.opacity);
    });
}

/// An inspector of three collapsible sections, in a panel `width` wide: Transform and Appearance
/// are open at first and Effects is closed. The Effects section's state belongs to the story
/// (`open`); the others are kept by the headers.
pub(super) fn show_collapsible(ui: &mut Ui, args: &mut WidthArgs) {
    let width = args.width.min(ui.available_width());
    let (state, mut v) = values(ui, "section-collapsible");
    let numbers = LocaleNumbers::from_tag("en");
    let open_id = state.with("effects-open");
    let mut effects = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let _ = SectionHeader::new("transform", "Transform").show(ui, |ui| transform_rows(ui, &mut v, &numbers));
        let _ = SectionHeader::new("appearance", "Appearance").show(ui, |ui| opacity_row(ui, &mut v, &numbers));
        let _ = SectionHeader::new("effects", "Effects").open(&mut effects).show(ui, |ui| {
            ui.label("No effects on this layer.");
        });
    });
    ui.data_mut(|d| {
        d.insert_temp(state, v);
        d.insert_temp(open_id, effects);
    });
}

/// An always-open section header with controls at the trailing end of its title row, over the
/// same rows.
pub(super) fn show_always_open(ui: &mut Ui, args: &mut WidthArgs) {
    let width = args.width.min(ui.available_width());
    let (state, mut v) = values(ui, "section-always-open");
    let numbers = LocaleNumbers::from_tag("en");
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let _ = SectionHeader::new("transform", "Transform")
            .collapsible(false)
            .controls(|ui| {
                ui.add(IconButton::new(Icon::Settings, "Transform settings").size(ControlSize::Sm));
                ui.add(IconButton::new(Icon::Plus, "Add transform").size(ControlSize::Sm));
            })
            .show(ui, |ui| {
                transform_rows(ui, &mut v, &numbers);
                opacity_row(ui, &mut v, &numbers);
            });
    });
    ui.data_mut(|d| d.insert_temp(state, v));
}
