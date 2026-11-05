//! [`StrokeButton`]: the opener that shows the stroke's style and opens the editor in a popover.

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{AsIdSalt, IdSalt, Rect, Response, ScrollArea, Stroke, Ui, pos2, vec2};

use super::editor::{body_width, line_style_name};
use super::{StrokeEditor, StrokeEditorStyle, StrokeField, StrokeLabels, StrokePreview, StrokeSections, StrokeStyle};
use craft_ui::controls::{Select, TooltipExt as _};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose};
use craft_ui::primitives::{Corner, Interactive, SurfaceKind, Text};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};

/// What [`StrokeButton::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StrokeButtonResponse {
    /// The opener.
    pub response: Response,
    /// The editor's valid change, as [`StrokeEditorResponse::event`](super::StrokeEditorResponse::event):
    /// the host makes one undo step per gesture ([`field`](Self::field) names it).
    pub event: Option<EditEvent<StrokeStyle>>,
    pub field: Option<StrokeField>,
}

/// The stroke style's opener in an options bar or the shape Properties (PhotoCraft's
/// `shape_stroke_ui::button`) and the [`StrokeEditor`](super::StrokeEditor) in a craft-ui
/// `Popover` under it.
///
/// ```no_run
/// use craft_draw::ui::stroke::{StrokeButton, StrokeStyle};
/// use craft_ui::localized::LocaleNumbers;
/// # fn demo(ui: &mut egui::Ui, style: &mut StrokeStyle, numbers: &LocaleNumbers, open: &mut bool, layer: u64) {
/// let r = StrokeButton::new("shape-stroke", style, numbers).closed(true).target(layer).show(ui, open);
/// if let Some(value) = r.event.as_ref().and_then(|e| e.value()) {
///     *style = value.clone(); // the host runs its "shape.edit" with it
/// }
/// # }
/// ```
///
/// - The opener: `stroke_editor.opener_width` by `stroke_editor.opener_height`, the style's name
///   (Solid, Dashed, Dotted, Custom; "No Color" while [`enabled`](Self::enabled) is false), a
///   line sample (a faint line crossed by a red slash while off) and a chevron. AccessKit: a
///   button named "Stroke Options" with the style's name as its value; the tooltip says "Stroke
///   Options".
/// - A click, Enter or Space toggles the host's `open`. The popover opens below the opener, focus
///   moves to its first control; Esc closes it and returns focus to the opener; a press outside
///   closes it and reaches nothing under it (`PopoverClose::swallow_outside(true)`, lead call 22,
///   as menus); a click on the opener closes it. It also closes when the
///   [`target`](Self::target) changes or the value changes from outside (Undo, another control),
///   and when the opener is disabled.
/// - The editor in it opens from the value, or from [`StrokeStyle::default`] when the value fails
///   [`StrokeStyle::validate`].
pub struct StrokeButton<'a> {
    id_salt: IdSalt,
    value: &'a StrokeStyle,
    numbers: &'a LocaleNumbers,
    enabled: bool,
    closed: bool,
    corners: bool,
    target: Option<u64>,
    sections: StrokeSections,
    labels: Option<StrokeLabels>,
    look: Option<StrokeEditorStyle>,
}

impl<'a> StrokeButton<'a> {
    /// The opener of the host's `value`, its numbers formatted with `numbers`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a StrokeStyle, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            enabled: true,
            closed: true,
            corners: true,
            target: None,
            sections: StrokeSections::POPOVER,
            labels: None,
            look: None,
        }
    }

    /// Whether the shape has a stroke (PhotoCraft: a width above 0). Off shows "No Color"; picking
    /// a preset then still reports. True by default.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// As [`StrokeEditor::closed`](super::StrokeEditor::closed); true by default.
    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    /// As [`StrokeEditor::corners`](super::StrokeEditor::corners); true by default.
    pub fn corners(mut self, corners: bool) -> Self {
        self.corners = corners;
        self
    }

    /// The host's key of what the stroke belongs to (a layer); the popover closes when it
    /// changes. None by default (a tool's default stroke).
    pub fn target(mut self, target: u64) -> Self {
        self.target = Some(target);
        self
    }

    /// The editor's sections; [`StrokeSections::POPOVER`] by default.
    pub fn sections(mut self, sections: StrokeSections) -> Self {
        self.sections = sections;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: StrokeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// A look of the host's in place of [`StrokeEditor::style`](super::StrokeEditor::style)'s.
    pub fn look(mut self, style: StrokeEditorStyle) -> Self {
        self.look = Some(style);
        self
    }

    /// Adds the opener to `ui` and, while `open`, the popover. The host owns `open`.
    pub fn show(self, ui: &mut Ui, open: &mut bool) -> StrokeButtonResponse {
        // pc: crates/ui-egui/src/tools/shape_stroke_ui/editor.rs:81-110 `button` (label :91,
        // opener 138 wide :92, toggle :94-104, close on target or value change :105-108),
        // :249-258 (Esc returns focus, a press outside closes), :37-77 `style_button`;
        // crates/ui-egui/src/tools/shape_stroke_ui.rs:164-186 `dismiss_from_canvas` (the closing
        // press reaches nothing). new: craft-ui `Popover` with `PopoverClose::swallow_outside`.
        let tokens = Tokens::of(ui);
        let look = self.look.unwrap_or_else(|| StrokeEditor::style(&tokens));
        let labels = self.labels.unwrap_or_else(|| StrokeLabels::of(ui));
        let field = Select::style(&tokens, ControlSize::Md).field;
        let id = ui.make_persistent_id(self.id_salt);
        let name = if self.enabled { line_style_name(&labels, self.value).to_owned() } else { labels.no_color.clone() };

        let response = Interactive::salted(IdSalt::new((id, "opener")), Role::Button, labels.title.as_str())
            .value(name.as_str())
            .surface(SurfaceKind::Sunken)
            .corner(Corner::Control)
            .expanded(*open)
            .show(ui, vec2(look.opener_width, look.opener_height));
        let response = response.tooltip(labels.title.as_str());
        if response.clicked() {
            *open = !*open;
        }
        if ui.is_rect_visible(response.rect) {
            let enabled = ui.is_enabled();
            let color = if enabled { field.text } else { field.text_disabled };
            let rect = response.rect;
            let forward = direction::forward(ui);
            let chevron = Rect::from_center_size(
                pos2(direction::end_x(ui, rect) - forward * (field.pad_x + field.icon / 2.0), rect.center().y),
                vec2(field.icon, field.icon),
            );
            let job = LayoutJob::single_section(name, role_format(&tokens, field.role, color));
            let galley = ui.painter().layout_job(job);
            let text_start = direction::start_x(ui, rect) + forward * field.pad_x;
            let text_end = text_start + forward * galley.size().x;
            let at = if forward > 0.0 { text_start } else { text_start - galley.size().x };
            ui.painter().galley(pos2(at, rect.center().y - galley.size().y / 2.0), galley, color);
            let (from, to) = (text_end + forward * field.gap, chevron.center().x - forward * (field.icon / 2.0 + field.gap));
            let sample = Rect::from_x_y_ranges(from.min(to)..=from.max(to), rect.y_range()).shrink2(vec2(0.0, field.gap));
            if self.enabled {
                let preview = StrokePreview::new(self.value, look.sample_stroke, look.sample_color).margin(0.0);
                ui.painter().extend(preview.shapes(sample));
            } else {
                let painter = ui.painter();
                painter.line_segment([sample.left_center(), sample.right_center()], Stroke::new(look.off_stroke, look.off_line));
                let half = sample.height() / 2.0;
                painter.line_segment(
                    [sample.center() + vec2(-half / 2.0, half), sample.center() + vec2(half / 2.0, -half)],
                    Stroke::new(look.off_slash_stroke, look.off_slash),
                );
            }
            icons::paint(ui, chevron, Icon::ChevronDown, IconSize::Sm, color);
        }

        // The popover closes when the target or the value changes from outside: what it opened
        // with, or what the editor last reported, is what the host holds next. The width is the
        // host's own (the editor never edits it), so a width change leaves it open.
        let key = id.with("opened");
        let pending = id.with("pending");
        if !ui.is_enabled() {
            *open = false;
        }
        if *open {
            let seen = ui.data(|d| d.get_temp::<Opened>(key));
            match seen {
                Some(opened) if opened.target != self.target || (StrokeStyle { width: self.value.width, ..opened.value.clone() }) != *self.value => {
                    *open = false;
                }
                Some(_) => {}
                None => ui.data_mut(|d| {
                    d.insert_temp(key, Opened { target: self.target, value: self.value.clone() });
                }),
            }
        }
        if !*open {
            ui.data_mut(|d| {
                d.remove::<Opened>(key);
                d.remove::<super::editor::Pending>(pending);
            });
        }

        let value = if self.value.validate().is_ok() { self.value.clone() } else { StrokeStyle::default() };
        let width = body_width(ui, &look);
        let height = (ui.ctx().content_rect().height() - look.screen_margin_y).max(look.min_height);
        let popover = Popover::new((id, "popover"), PopoverAnchor::Control(response.clone()))
            .close(PopoverClose::default().swallow_outside(true))
            .toggle(response.id)
            .label(labels.title.as_str())
            .show(ui, open, |ui, _| {
                ui.set_width(width);
                ui.add(Text::new(labels.title.clone()).role(TypeRole::BodyStrong));
                let editor = StrokeEditor::new((id, "editor"), &value, self.numbers)
                    .sections(self.sections)
                    .closed(self.closed)
                    .corners(self.corners)
                    .labels(labels.clone())
                    .look(look)
                    .report_same_preset(!self.enabled)
                    .pending_key(pending);
                ScrollArea::vertical().max_height(height).auto_shrink([false, true]).show(ui, |ui| editor.show(ui)).inner
            });
        let (mut event, mut field) = (None, None);
        if let Some(popover) = popover {
            let reported = popover.inner;
            if let Some(next) = reported.event.as_ref().and_then(EditEvent::value) {
                let opened = Opened { target: self.target, value: next.clone() };
                ui.data_mut(|d| {
                    d.insert_temp(key, opened);
                });
            }
            (event, field) = (reported.event, reported.field);
        }
        StrokeButtonResponse { response, event, field }
    }
}

/// What the popover opened with, or last reported.
#[derive(Clone)]
struct Opened {
    target: Option<u64>,
    value: StrokeStyle,
}
