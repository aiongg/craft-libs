//! [`Icon`]: a typed icon at a size class, in a content colour.

use egui::accesskit::Role;
use egui::{Color32, Response, Sense, Ui, Widget, WidgetInfo, vec2};

use super::{Content, ContentStoryArgs};
use crate::icons::{self, IconSize};
use crate::story::{Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// An icon's size and colour, from [`Icon::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconStyle {
    /// Side of the icon's square, in points: `size.icon.*`.
    pub points: f32,
    /// Stroke width of stroke sets, in points.
    pub stroke: f32,
    pub color: Color32,
}

/// An [`icons::Icon`] from the context's set at a size class ([`IconSize::Md`] by default),
/// tinted with a [`Content`] colour ([`Content::Normal`] by default). Directional icons mirror
/// in a right-to-left interface.
///
/// Decorative unless [`label`](Icon::label) names it: only a labelled icon is in the AccessKit
/// tree, as an image.
#[derive(Clone, Debug)]
pub struct Icon {
    icon: icons::Icon,
    size: IconSize,
    content: Content,
    label: Option<String>,
}

impl Icon {
    pub fn new(icon: icons::Icon) -> Self {
        Self { icon, size: IconSize::Md, content: Content::Normal, label: None }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn content(mut self, content: Content) -> Self {
        self.content = content;
        self
    }

    /// The accessible name, for an icon that means something on its own.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn style(tokens: &UiTokens, size: IconSize, content: Content) -> IconStyle {
        IconStyle { points: size.points(tokens), stroke: size.stroke(), color: content.color(tokens).to_egui() }
    }
}

impl Widget for Icon {
    fn ui(self, ui: &mut Ui) -> Response {
        let style = Icon::style(&Tokens::of(ui), self.size, self.content);
        let (rect, response) = ui.allocate_exact_size(vec2(style.points, style.points), Sense::hover());
        if ui.is_rect_visible(rect) {
            icons::paint(ui, rect, self.icon, self.size, style.color);
        }
        if let Some(label) = &self.label {
            response.widget_info(|| WidgetInfo::labeled(Role::Image, ui.is_enabled(), label));
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Icon"];

const SAMPLES: [icons::Icon; 6] =
    [icons::Icon::Close, icons::Icon::ChevronRight, icons::Icon::Undo, icons::Icon::Layers, icons::Icon::Eye, icons::Icon::Warning];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("primitives/icon/sizes", Level::Primitives, "Icon", "Sizes", ContentStoryArgs::default(), show)]
}

/// A row of icons per size class; chevron and undo mirror in right-to-left.
fn show(ui: &mut Ui, args: &mut ContentStoryArgs) {
    let content = args.content.content();
    crate::story::variant_list(ui, "icon-sizes", |form| {
        for (size, name) in IconSize::ALL.into_iter().zip(["Sm", "Md", "Lg"]) {
            crate::story::variant_cluster(form, name, |ui| {
                for icon in SAMPLES {
                    ui.add(Icon::new(icon).size(size).content(content));
                }
            });
        }
    });
}
