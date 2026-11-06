//! [`PagePreview`] (N3.16): a page shown large over the window with previous and next, as
//! PdfCraft's Combine Files preview and VectorCraft's Place PDF page choice show it: the page as
//! large as fits, "Page 3 of 12", ← / → (and Page Up / Page Down) to step, Esc or the close
//! button to leave.

use craft_ui::canvas::textures::{MAX_TEXTURE, Priority, TextureCacheOptions, TextureState, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails};
use craft_ui::controls::{ControlSize, IconButton};
use craft_ui::direction;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::theme::{ToEgui as _, current};
use egui::accesskit::Role;
use egui::{Align, Context, Frame, Id, Key, Layout, Modal, Modifiers, Sense, Vec2};

use super::style::PagePreviewStyle;
use super::view::PageSource;

craft_ui::words! {
    /// The preview's words (PdfCraft's strings).
    #[non_exhaustive]
    pub struct PagePreviewLabels {
        page_of: "pagePreview.pageOf" = "Page {p} of {n}";
        previous: "pagePreview.previous" = "Previous page";
        next: "pagePreview.next" = "Next page";
        close: "pagePreview.close" = "Close";
        unavailable: "pagePreview.unavailable" = "Preview unavailable";
    }
}

/// What the preview did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PagePreviewResponse {
    /// The page shown changed (the app may sync its grid's selection).
    pub stepped: bool,
    /// Esc or the close button.
    pub closed: bool,
}

/// The preview over the window, shown each pass while the app keeps it open.
///
/// - A dimmed backdrop over the window (the dialog framework's backdrop); the page centred, as
///   large as fits, its picture through craft-ui's `Thumbnails<usize>` over the [`PageSource`]
///   (side from `ThumbRequest::for_cell`, at most `MAX_TEXTURE`; the last picture stays as the
///   stand-in until the new one lands). A page that fails to render shows the texture
///   contract's failure look with [`PagePreviewLabels::unavailable`] under it; the failure's
///   reason is in the picture's tooltip, as everywhere else.
/// - Below it, centred: previous, "Page {p} of {n}", next; the buttons are disabled at the ends.
///   The pages stepped through are `pages` (the pages a range selects), in order.
/// - Keys: ← and Page Up previous, → and Page Down next (not while a text field has the
///   keyboard), Esc closes; +, = , − and 0 are taken so the grid underneath doesn't zoom.
/// - Focus moves into the preview when the app says it [`opened`](Self::opened) and back when it closes; it is an AccessKit
///   Dialog named "Page {p} of {n}".
pub struct PagePreview<'a> {
    id_salt: egui::IdSalt,
    source: &'a dyn PageSource,
    pages: &'a [usize],
    at: &'a mut usize,
    opened: bool,
    labels: Option<PagePreviewLabels>,
}

impl<'a> PagePreview<'a> {
    /// The preview's measures, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> PagePreviewStyle {
        PagePreviewStyle { room: tokens.space.pad.dialog }
    }

    /// Preview `pages` (0-based), showing `pages[*at]`.
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a dyn PageSource, pages: &'a [usize], at: &'a mut usize) -> Self {
        PagePreview { id_salt: egui::IdSalt::new(id_salt), source, pages, at, opened: false, labels: None }
    }

    /// The app opened the preview this pass (set on the one pass that follows the command that
    /// opens it): the keyboard moves into the preview and the focus it had is given back on
    /// close.
    pub fn opened(mut self, opened: bool) -> Self {
        self.opened = opened;
        self
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: PagePreviewLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/workflows/combine_grid/card.rs:326 `preview`; vectorcraft:
    // crates/ui-egui/src/dialogs/import_pdf.rs (page choice with a preview).
    /// Shows the preview over the window and reports page changes and close requests.
    pub fn show(self, ctx: &Context) -> PagePreviewResponse {
        let PagePreview { id_salt, source, pages, at, opened, labels } = self;
        let mut out = PagePreviewResponse::default();
        if pages.is_empty() {
            out.closed = true;
            return out;
        }
        let labels = labels.unwrap_or_else(|| PagePreviewLabels::of_ctx(ctx));
        *at = (*at).min(pages.len() - 1);
        let id = Id::unique((id_salt, "page-preview"));

        // The focus from before the preview opened is kept until it closes: the preview is not
        // drawn in between, so only the preview can hold it.
        let before = id.with("focus-before");
        if opened {
            let focused = ctx.memory(|m| m.focused());
            ctx.data_mut(|d| d.insert_temp(before, focused));
        }

        let mut step = 0_isize;
        let mut close = false;
        if !ctx.text_edit_focused() {
            let (back, forward) = if direction::is_rtl(ctx) { (Key::ArrowRight, Key::ArrowLeft) } else { (Key::ArrowLeft, Key::ArrowRight) };
            ctx.input_mut(|input| {
                if input.consume_key(Modifiers::NONE, back) | input.consume_key(Modifiers::NONE, Key::PageUp) {
                    step = -1;
                } else if input.consume_key(Modifiers::NONE, forward) | input.consume_key(Modifiers::NONE, Key::PageDown) {
                    step = 1;
                }
                for key in [Key::Plus, Key::Equals, Key::Minus, Key::Num0] {
                    input.consume_key(Modifiers::NONE, key);
                    input.consume_key(Modifiers::COMMAND, key);
                }
            });
        }
        let last = pages.len() - 1;
        let target = (*at as isize + step).clamp(0, last as isize) as usize;
        if target != *at {
            *at = target;
            out.stepped = true;
        }
        let page = pages[*at];

        let tokens = current(ctx);
        let margin = Self::style(&tokens).room;
        let button = IconButton::style(&tokens, ControlSize::Md).side;
        let numbers = LocaleNumbers::of_ctx(ctx);
        let name = labels.page_of.replace("{p}", &source.page_label(page)).replace("{n}", &numbers.format(source.page_count() as f64, 0..=0));
        let area = Modal::default_area(id).accessible_name(name.clone()).role(Role::Dialog);
        let thumbs = Thumbnails::<usize>::in_context(
            ctx,
            (id_salt, "pictures"),
            ThumbnailOptions { max_side: MAX_TEXTURE, cache: TextureCacheOptions::pages(), ..ThumbnailOptions::default() },
        );
        let shown = Modal::new(id).area(area).backdrop_color(tokens.color.surface.scrim.to_egui()).frame(Frame::NONE).show(ctx, |ui| {
            let gap = ui.spacing().item_spacing.y;
            let room =
                Vec2::new((ctx.content_rect().width() - 2.0 * margin).max(0.0), (ctx.content_rect().height() - 2.0 * margin - button - gap).max(0.0));
            let size = source.page_size(page);
            let scale = if size.width > 0.0 && size.height > 0.0 {
                (f64::from(room.x) / size.width).min(f64::from(room.y) / size.height) as f32
            } else {
                0.0
            };
            let picture = if scale > 0.0 { Vec2::new(size.width as f32 * scale, size.height as f32 * scale) } else { Vec2::splat(room.min_elem()) };
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                let (rect, response) = ui.allocate_exact_size(picture, Sense::hover());
                let request = ThumbRequest::for_cell(picture.max_elem(), ctx.pixels_per_point());
                let state = thumbs.state(ctx, source, &page, &request, Priority::VISIBLE);
                thumbs.paint(ui.painter(), rect, &state, &ThumbPaint::default().checker(false));
                if let TextureState::Failed { message } = &state {
                    response.on_hover_text(message);
                    ui.label(labels.unavailable.as_str());
                }
                // Previous sits at the reading start and Next after the label, as the arrow keys
                // run (right to left: → is previous).
                let rtl = direction::is_rtl(ctx);
                ui.horizontal(|ui| {
                    let previous = |ui: &mut egui::Ui| {
                        ui.add_enabled(*at > 0, IconButton::new(Icon::ChevronLeft, labels.previous.as_str()).id_salt((id_salt, "previous")))
                    };
                    let next = |ui: &mut egui::Ui| {
                        ui.add_enabled(*at < last, IconButton::new(Icon::ChevronRight, labels.next.as_str()).id_salt((id_salt, "next")))
                    };
                    let (previous, next, closer) = if rtl {
                        let closer = ui.add(IconButton::new(Icon::Close, labels.close.as_str()).id_salt((id_salt, "close")));
                        let next = next(ui);
                        ui.label(name.as_str());
                        let previous = previous(ui);
                        (previous, next, closer)
                    } else {
                        let previous = previous(ui);
                        ui.label(name.as_str());
                        let next = next(ui);
                        let closer = ui.add(IconButton::new(Icon::Close, labels.close.as_str()).id_salt((id_salt, "close")));
                        (previous, next, closer)
                    };
                    if previous.clicked() && *at > 0 {
                        *at -= 1;
                        out.stepped = true;
                    }
                    if next.clicked() && *at < last {
                        *at += 1;
                        out.stepped = true;
                    }
                    close |= closer.clicked();
                    if opened {
                        closer.request_focus();
                    }
                });
            });
        });
        if shown.is_top_modal && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            close = true;
        }
        if close {
            out.closed = true;
            if let Some(Some(id)) = ctx.data(|d| d.get_temp::<Option<Id>>(before)) {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        }
        out
    }
}
