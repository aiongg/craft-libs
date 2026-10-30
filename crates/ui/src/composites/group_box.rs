//! [`GroupBox`]: an outlined, titled group of rows, for a dialog or a preferences page.

use craft_theme::{Stroke as Hairline, TypeRole};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, InnerResponse, Label, Margin, Stroke, Ui, UiBuilder};

use crate::primitives::Content;
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};
use crate::typed_story;

/// A group box's outline, padding and title, from [`GroupBox::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GroupBoxStyle {
    /// The outline, one physical pixel: `group_box.edge`.
    pub edge: Stroke,
    /// Corner radius: `group_box.radius`.
    pub radius: CornerRadius,
    /// Between the outline and the content: `group_box.padding`.
    pub padding: f32,
    /// Between two boxes: `group_box.gap`. The host's stack applies it; the box adds none.
    pub gap: f32,
    /// Between the title and the content: `gap.stack`.
    pub title_gap: f32,
    /// The title's type role: `body_strong`.
    pub title_role: TypeRole,
    /// The title: `text.primary`.
    pub title: Color32,
}

/// An outlined group with a title in its first line, as a preferences page's "Appearance" group.
///
/// ```
/// use craft_ui::composites::GroupBox;
/// # egui::__run_test_ui(|ui| {
/// GroupBox::new("Appearance").show(ui, |ui| { ui.label("Theme"); });
/// # });
/// ```
///
/// - The box fills the available width. The content sits `group_box.padding` inside the outline,
///   under the title, `gap.stack` below it.
/// - Several boxes in a column are `group_box.gap` apart ([`GroupBox::gap`]).
/// - **AccessKit:** a `Group` named by the title.
#[must_use = "show the group box with `show`"]
pub struct GroupBox {
    title: String,
    id_salt: Option<IdSalt>,
}

impl GroupBox {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), id_salt: None }
    }

    /// The box's id, when two boxes of one parent have the same title.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> GroupBoxStyle {
        let g = &tokens.component.group_box;
        GroupBoxStyle {
            edge: Stroke::new(Hairline::Hairline.width(pixels_per_point), g.edge.to_egui()),
            radius: corner_radius(g.radius),
            padding: g.padding,
            gap: g.gap,
            title_gap: tokens.space.gap.stack,
            title_role: TypeRole::BodyStrong,
            title: Content::Normal.color(tokens).to_egui(),
        }
    }

    /// The gap between boxes in a column, `group_box.gap`.
    pub fn gap(ui: &Ui) -> f32 {
        Tokens::of(ui).component.group_box.gap
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let tokens = Tokens::of(ui);
        let style = GroupBox::style(&tokens, ui.pixels_per_point());
        let salt = self.id_salt.unwrap_or_else(|| IdSalt::new(self.title.as_str()));
        let builder = UiBuilder::new().id_salt(salt).accessibility_role(Role::Group).accessibility_label(self.title.as_str());
        ui.scope_builder(builder, |ui| {
            let width = ui.available_width();
            egui::Frame::new()
                .stroke(style.edge)
                .corner_radius(style.radius)
                .inner_margin(Margin::same(style.padding.round() as i8))
                .show(ui, |ui| {
                    ui.set_width(width - 2.0 * style.padding.round() - 2.0 * style.edge.width);
                    ui.add(
                        Label::new(LayoutJob::single_section(self.title.clone(), role_format(&tokens, style.title_role, style.title)))
                            .selectable(false),
                    );
                    ui.add_space(style.title_gap);
                    add_contents(ui)
                })
                .inner
        })
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["GroupBox"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/group-box/default",
        Level::Composites,
        "GroupBox",
        "Default",
        GroupArgs { second: true, width: 320.0 },
        show_default
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct GroupArgs {
    pub(super) second: bool,
    #[arg(range = 120.0..=480.0)]
    pub(super) width: f32,
}

/// One box, or two with `second`, in a column `width` wide.
pub(super) fn show_default(ui: &mut Ui, args: &mut GroupArgs) {
    let width = args.width.min(ui.available_width());
    let gap = GroupBox::gap(ui);
    ui.allocate_ui(egui::vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        for i in 0..(1 + usize::from(args.second)) {
            GroupBox::new(format!("Group {}", i + 1)).show(ui, |ui| {
                ui.label("A row of settings");
                ui.label("Another row");
            });
            ui.add_space(gap);
        }
    });
}
