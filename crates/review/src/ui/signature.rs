//! The signature pad dialog and saved signatures (N5.04).

use std::cell::Cell;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use craft_host::{Host, PickId, PickRequest, Picked};
use craft_ui::controls::{Button, Segmented, TextField};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::theme::{ToEgui as _, UiTokens, corner_radius, current};
use egui::epaint::{ColorImage, TextureHandle};
use egui::{Context, Id, Key, Modifiers, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureOptions, Ui, Vec2, pos2, vec2};

use super::comments::text;
use super::style::ReviewStyle;
use crate::signature::{MAX_IMAGE_BYTES, MAX_IMAGE_SIDE, MAX_SIGNATURE_CHARS, SavedSignature, SignatureDraft, SignatureImage, SignatureTab};

/// The pad's width over its height (PdfCraft's 460 × 150 pad). Strokes are kept in units of the
/// pad's box, so painting them elsewhere restores this shape.
const PAD_ASPECT: f32 = 460.0 / 150.0;
/// Pad points closer than this (in pad units, both axes together) to the last one are dropped.
const MIN_STEP: f32 = 0.002;
/// The share of a box's height that strokes fill (PdfCraft `fill_sign.rs:317`, `h * 0.9`).
const FILL: f32 = 0.9;
/// The share of a box's height that typed text's font size takes (PdfCraft `fill_sign.rs:317`).
const TYPED_HEIGHT_SHARE: f32 = 0.9;
/// The share of a box's width that typed text may take (PdfCraft `fill_sign.rs:318`, `w * 0.95`).
const TYPED_WIDTH_SHARE: f32 = 0.95;
/// Textures kept for painted images.
const TEXTURES_KEPT: usize = 8;

craft_ui::words! {
    /// The pad's words, English by default (PdfCraft's, Acrobat's).
    #[non_exhaustive]
    pub struct SignatureLabels {
        create: "signature.create" = "Create {what}";
        change: "signature.change" = "Change {what}";
        signature: "signature.signature" = "signature";
        initials: "signature.initials" = "initials";
        type_tab: "signature.typeTab" = "Type";
        draw_tab: "signature.drawTab" = "Draw";
        image_tab: "signature.imageTab" = "Image";
        scan_tab: "signature.scanTab" = "Scan";
        type_hint: "signature.typeHint" = "Type text";
        image_hint: "signature.imageHint" = "Choose a PNG or JPEG image (up to 4 MiB). Transparency is preserved.";
        browse: "signature.browse" = "Browse…";
        undo: "signature.undo" = "Undo";
        clear: "signature.clear" = "Clear";
        apply: "signature.apply" = "Apply";
        cancel: "signature.cancel" = "Cancel";
        image_failed: "signature.imageFailed" = "That image could not be read.";
        image_too_large: "signature.imageTooLarge" = "That image is too large: up to 4 MiB and 8192 × 8192 pixels.";
    }
}

/// The saved signature and initials, one settings section shared by every document.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SignatureSettings {
    /// The saved signature, if any.
    pub signature: Option<SavedSignature>,
    /// The saved initials, if any.
    pub initials: Option<SavedSignature>,
}

impl craft_host::Section for SignatureSettings {
    const KEY: &'static str = "signatures";
}

/// Draws a saved signature into `rect` in `color`: strokes scaled from pad units, typed text in
/// the proportional face (the library has no script face) fitted to the rect, an image fitted
/// keeping its aspect. An image's texture is cached by the painter's context under the image's
/// [`key`](SignatureImage::key), computed once at construction.
// pdfcraft: crates/ui-egui/src/tools/fill_sign.rs:406 `paint_saved`, `:309` `script_preview`.
pub fn paint_signature(painter: &Painter, rect: Rect, signature: &SavedSignature, color: egui::Color32) {
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    match signature {
        SavedSignature::Drawn(strokes) => {
            let width = craft_theme::Stroke::W1_5.width(painter.ctx().pixels_per_point());
            paint_strokes(&painter, rect, strokes, Stroke::new(width, color), true);
        }
        SavedSignature::Typed(label) => paint_typed(&painter, rect, label, color),
        SavedSignature::Image(image) => paint_image(&painter, rect, image),
    }
}

/// Strokes in pad units: with `fit`, scaled to fill `rect` keeping the pad's shape; else laid
/// over `rect` as the pad's own box.
fn paint_strokes(painter: &Painter, rect: Rect, strokes: &[Vec<[f32; 2]>], stroke: Stroke, fit: bool) {
    let points = || strokes.iter().flatten().filter(|p| p.iter().all(|v| v.is_finite()));
    let (mut min, mut max) = (vec2(f32::MAX, f32::MAX), vec2(f32::MIN, f32::MIN));
    for p in points() {
        let at = vec2(p[0] * PAD_ASPECT, p[1]);
        min = min.min(at);
        max = max.max(at);
    }
    if min.x > max.x {
        return;
    }
    let extent = (max - min).max(Vec2::splat(f32::EPSILON));
    let scale = (rect.width() / extent.x).min(rect.height() / extent.y) * FILL;
    let middle = (min + max) / 2.0;
    let place = |p: &[f32; 2]| {
        if fit {
            rect.center() + (vec2(p[0] * PAD_ASPECT, p[1]) - middle) * scale
        } else {
            rect.min + vec2(p[0] * rect.width(), p[1] * rect.height())
        }
    };
    for line in strokes {
        let shown: Vec<Pos2> = line.iter().filter(|p| p.iter().all(|v| v.is_finite())).map(place).collect();
        match shown.as_slice() {
            [] => {}
            [dot] => {
                painter.circle_filled(*dot, stroke.width / 2.0, stroke.color);
            }
            _ => {
                painter.add(Shape::line(shown, stroke));
            }
        }
    }
}

/// `label` in the proportional face, as large as fits `rect` and centred.
fn paint_typed(painter: &Painter, rect: Rect, label: &str, color: egui::Color32) {
    if label.trim().is_empty() {
        return;
    }
    let font = |size: f32| egui::FontId::proportional(size);
    let size = rect.height() * TYPED_HEIGHT_SHARE;
    let galley = painter.layout_no_wrap(label.to_owned(), font(size), color);
    let fit = (rect.width() * TYPED_WIDTH_SHARE / galley.size().x.max(f32::EPSILON)).min(1.0);
    let galley = if fit < 1.0 { painter.layout_no_wrap(label.to_owned(), font(size * fit), color) } else { galley };
    painter.galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// `image` fitted into `rect` keeping its aspect.
fn paint_image(painter: &Painter, rect: Rect, image: &SignatureImage) {
    let (w, h) = (image.width() as f32, image.height() as f32);
    if w < 1.0 || h < 1.0 {
        return;
    }
    let Some(texture) = image_texture(painter.ctx(), image) else { return };
    paint_texture(painter, rect, &texture, [image.width(), image.height()]);
}

/// `texture` of `size` pixels fitted into `rect` keeping its aspect.
fn paint_texture(painter: &Painter, rect: Rect, texture: &TextureHandle, size: [u32; 2]) {
    let (w, h) = (size[0] as f32, size[1] as f32);
    if w < 1.0 || h < 1.0 {
        return;
    }
    let size = vec2(w, h) * (rect.width() / w).min(rect.height() / h);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(texture.id(), Rect::from_center_size(rect.center(), size), uv, egui::Color32::WHITE);
}

/// The texture of `image`, cached by the context under the image's hash.
fn image_texture(ctx: &Context, image: &SignatureImage) -> Option<TextureHandle> {
    let (width, height) = (usize::try_from(image.width()).ok()?, usize::try_from(image.height()).ok()?);
    if width.checked_mul(height)?.checked_mul(4)? != image.rgba().len() {
        return None;
    }
    let key = image.key();
    let slot = Id::NULL.with("craft-review-signature-textures");
    let mut cache: Vec<(u64, TextureHandle)> = ctx.data(|data| data.get_temp(slot)).unwrap_or_default();
    if let Some((_, texture)) = cache.iter().find(|(k, _)| *k == key) {
        return Some(texture.clone());
    }
    let pixels = ColorImage::from_rgba_unmultiplied([width, height], image.rgba());
    let texture = ctx.load_texture("signature-image", pixels, TextureOptions::LINEAR);
    cache.push((key, texture.clone()));
    if cache.len() > TEXTURES_KEPT {
        drop(cache.remove(0));
    }
    ctx.data_mut(|data| data.insert_temp(slot, cache));
    Some(texture)
}

/// Why a picked image was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickError {
    /// Unreadable, or not a PNG or JPEG.
    Unreadable,
    /// More than [`MAX_IMAGE_BYTES`] of file or [`MAX_IMAGE_SIDE`] pixels on a side.
    TooLarge,
}

/// What a decode job reports.
type Decoded = Result<SignatureImage, PickError>;

/// Where a decode job gets its bytes.
enum Source {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

/// Decodes a PNG or JPEG of at most [`MAX_IMAGE_BYTES`] and [`MAX_IMAGE_SIDE`] pixels a side into
/// straight RGBA. The size limits apply while decoding, so an oversize image is refused before its
/// pixels are allocated.
fn decode_image(bytes: &[u8]) -> Decoded {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(PickError::TooLarge);
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|_| PickError::Unreadable)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| if matches!(error, image::ImageError::Limits(_)) { PickError::TooLarge } else { PickError::Unreadable })?
        .into_rgba8();
    Ok(SignatureImage::new(decoded.width(), decoded.height(), decoded.into_raw()))
}

/// The pad's image on the GPU, made once when the image arrives (or first shows) and painted
/// every pass without touching the pixels.
struct ImageTexture {
    texture: TextureHandle,
    size: [u32; 2],
}

impl ImageTexture {
    fn new(ctx: &Context, image: &SignatureImage) -> Option<Self> {
        let (width, height) = (usize::try_from(image.width()).ok()?, usize::try_from(image.height()).ok()?);
        if width.checked_mul(height)?.checked_mul(4)? != image.rgba().len() {
            return None;
        }
        let pixels = ColorImage::from_rgba_unmultiplied([width, height], image.rgba());
        Some(Self { texture: ctx.load_texture("signature-pad-image", pixels, TextureOptions::LINEAR), size: [image.width(), image.height()] })
    }
}

/// The Scan tab's content, the app's.
pub type ScanTab = Box<dyn FnMut(&mut egui::Ui, &mut SignatureDraft) + Send>;

/// Acrobat's signature pad on the dialog host: "Create signature" / "Change initials", the tabs
/// Type (a field, previewed in the proportional face), Draw (a pad: strokes with the pointer or pen,
/// Undo), Image (Browse… a PNG or JPEG up to [`MAX_IMAGE_BYTES`]
/// through `host.files()`) and Scan (shown when the app gives a scan hook); Clear; Apply (enabled
/// when [`SignatureDraft::ready`]) and Cancel. Tabs are a segmented control (arrow keys move);
/// Tab walks the controls; Enter is Apply; Esc is Cancel. Browse… reads and decodes the file on
/// `host.jobs()` (at most [`MAX_IMAGE_SIDE`] pixels a side) and the pad polls the result; a pick
/// that fails or is too large shows an error word under the button.
///
/// Undo and Clear sit in the dialog body: the footer is drawn after the body, so a footer button
/// would change the draft after the pass had painted it.
pub struct SignaturePadDialog {
    draft: SignatureDraft,
    scan: Option<ScanTab>,
    labels: Option<SignatureLabels>,
    state: DialogState<()>,
    /// The Browse… pick waiting for its answer.
    pick: Option<PickId>,
    /// The decode job of a pick, until its result arrives.
    decoding: Option<Receiver<Decoded>>,
    /// The last pick's failure, until a pick succeeds or Clear.
    error: Option<PickError>,
    /// `draft.image` on the GPU.
    texture: Option<ImageTexture>,
}

/// What [`SignaturePadDialog`] answers with.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SignaturePadResult {
    /// What the user drew, typed or picked.
    pub signature: SavedSignature,
    /// Whether it is initials (else the signature).
    pub initials: bool,
}

impl SignaturePadDialog {
    /// A pad that starts from `draft`.
    pub fn new(draft: SignatureDraft) -> Self {
        Self { draft, scan: None, labels: None, state: DialogState::default(), pick: None, decoding: None, error: None, texture: None }
    }

    /// The Scan tab's content (PdfCraft's camera and clean-up): it fills `draft.image`.
    pub fn scan(mut self, tab: impl FnMut(&mut egui::Ui, &mut SignatureDraft) + Send + 'static) -> Self {
        self.scan = Some(Box::new(tab));
        self
    }

    /// Uses these label texts instead of the ones read from the active locale's catalog.
    pub fn labels(mut self, labels: SignatureLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl StackDialog for SignaturePadDialog {
    type Result = SignaturePadResult;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("signature-pad")
    }

    // pdfcraft: crates/ui-egui/src/tools/fill_sign.rs:598 `signature_pad`, `:732` `pad_buttons`,
    // `fill_sign/scan.rs:204` `scan_tab`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>> {
        let labels = self.labels.clone().unwrap_or_else(|| SignatureLabels::of_ctx(ctx));
        self.take_picked(ctx);
        self.sync_texture(ctx);
        let tokens = current(ctx);
        let what = if self.draft.initials { &labels.initials } else { &labels.signature };
        let title = fmt(if self.draft.editing { &labels.change } else { &labels.create }, &[("what", what)]);
        let id = self.id();
        let heading = title.clone();
        let ready = Cell::new(self.draft.ready());
        let mut browse = false;
        let Self { draft, scan, state, texture, error, decoding, .. } = self;
        let decoding = decoding.is_some();
        let footer = Footer::standard().ok(labels.apply.as_str()).cancel(labels.cancel.as_str()).ok_enabled_with(|| ready.get());
        let shown = Dialog::with_id(id, title).stack(stack).footer(footer).show(ctx, state, &mut (), |ui, ()| {
            let salt = Id::NULL.with(id);
            let has_scan = scan.is_some();
            let mut segmented = Segmented::new(salt.with("tabs"), &heading, &mut draft.tab)
                .item(SignatureTab::Type, labels.type_tab.as_str())
                .item(SignatureTab::Draw, labels.draw_tab.as_str())
                .item(SignatureTab::Image, labels.image_tab.as_str());
            if has_scan {
                segmented = segmented.item(SignatureTab::Scan, labels.scan_tab.as_str());
            }
            segmented.show(ui);
            ui.add_space(tokens.space.gap.stack);
            match draft.tab {
                SignatureTab::Type => type_tab(ui, &tokens, &labels, salt, draft),
                SignatureTab::Draw => draw_tab(ui, &tokens, &labels, draft),
                SignatureTab::Image => browse = image_tab(ui, &tokens, &labels, draft, texture.as_ref(), *error, decoding),
                SignatureTab::Scan => {
                    if let Some(tab) = scan.as_mut() {
                        tab(ui, draft);
                    }
                }
            }
            ui.add_space(tokens.space.gap.stack);
            ui.horizontal(|ui| {
                if draft.tab == SignatureTab::Draw
                    && ui.add_enabled(!draft.strokes.is_empty(), Button::new(&labels.undo).id_salt(salt.with("undo"))).clicked()
                {
                    draft.undo_stroke();
                }
                if ui.add(Button::new(&labels.clear).id_salt(salt.with("clear"))).clicked() {
                    draft.clear();
                    *error = None;
                }
            });
            ready.set(draft.ready());
        });
        if browse {
            self.browse(ctx);
        }
        match shown.outcome {
            Some(Outcome::Ok) => match self.draft.saved() {
                Some(saved) => {
                    self.forget_pick(ctx);
                    Some(DialogResult::Ok(SignaturePadResult { signature: saved, initials: self.draft.initials }))
                }
                None => None,
            },
            Some(outcome) if outcome.closes() => {
                self.forget_pick(ctx);
                Some(DialogResult::Cancel)
            }
            _ => None,
        }
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<Self::Result> {
        self.draft.saved().map(|signature| SignaturePadResult { signature, initials: self.draft.initials })
    }

    fn cancelled(&mut self, ctx: &Context) {
        self.forget_pick(ctx);
    }
}

impl SignaturePadDialog {
    /// Asks the host's file service for a PNG or JPEG.
    fn browse(&mut self, ctx: &Context) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        self.forget_pick(ctx);
        let request = PickRequest::open("signature-image").filter("Image", &["png", "jpg", "jpeg"]);
        self.pick = Some(host.files().pick(request));
    }

    /// Starts the decode of a Browse… answer when it has come, and takes a finished decode.
    fn take_picked(&mut self, ctx: &Context) {
        let Some(host) = Host::try_of_ctx(ctx) else { return };
        if let Some(ticket) = self.pick
            && let Some(picked) = host.files().take_answer(ticket)
        {
            self.pick = None;
            let source = match picked {
                Picked::Cancelled => None,
                Picked::Contents(files) => files.into_iter().next().map(|(_, bytes)| Source::Bytes(bytes)),
                Picked::Paths(paths) => paths.into_iter().next().map(Source::Path),
            };
            if let Some(source) = source {
                let (send, receive) = channel();
                let (files, ctx) = (host.clone(), ctx.clone());
                host.jobs().spawn(
                    "signature-image",
                    Box::new(move || {
                        let decoded = match source {
                            Source::Bytes(bytes) => decode_image(&bytes),
                            Source::Path(path) => files.files().read(&path).map_err(|_| PickError::Unreadable).and_then(|bytes| decode_image(&bytes)),
                        };
                        // The dialog may have closed; then nobody reads the answer.
                        let _ = send.send(decoded);
                        ctx.request_repaint();
                    }),
                );
                self.decoding = Some(receive);
            }
        }
        let Some(receive) = &self.decoding else { return };
        match receive.try_recv() {
            Ok(Ok(image)) => {
                self.texture = ImageTexture::new(ctx, &image);
                self.draft.image = Some(image);
                self.error = None;
                self.decoding = None;
            }
            Ok(Err(error)) => {
                self.error = Some(error);
                self.decoding = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.error = Some(PickError::Unreadable);
                self.decoding = None;
            }
        }
    }

    /// Keeps the texture in step with `draft.image` without touching the pixels: dropped when the
    /// image is gone or the Scan tab (which may replace the image) is showing, made once when an
    /// image the draft came with first shows.
    fn sync_texture(&mut self, ctx: &Context) {
        if self.draft.image.is_none() || self.draft.tab == SignatureTab::Scan {
            self.texture = None;
        } else if self.texture.is_none()
            && let Some(image) = &self.draft.image
        {
            self.texture = ImageTexture::new(ctx, image);
        }
    }

    fn forget_pick(&mut self, ctx: &Context) {
        self.decoding = None;
        if let (Some(ticket), Some(host)) = (self.pick.take(), Host::try_of_ctx(ctx)) {
            host.files().forget(ticket);
        }
    }
}

/// The paper the Type, Draw and Image tabs show their content on.
fn pad_box(ui: &mut Ui, tokens: &UiTokens, sense: Sense) -> (Rect, egui::Response) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, width / PAD_ASPECT), sense);
    let radius = corner_radius(tokens.shape.radius.control);
    let edge = Stroke::new(craft_theme::Stroke::W1.width(ui.pixels_per_point()), tokens.color.line.border.to_egui());
    ui.painter().rect_filled(rect, radius, tokens.color.surface.canvas.to_egui());
    ui.painter().rect_stroke(rect, radius, edge, StrokeKind::Inside);
    (rect, response)
}

fn type_tab(ui: &mut Ui, tokens: &UiTokens, labels: &SignatureLabels, salt: Id, draft: &mut SignatureDraft) {
    let field = TextField::new(salt.with("text"), &labels.type_hint, &draft.text)
        .placeholder(&labels.type_hint)
        .char_limit(MAX_SIGNATURE_CHARS)
        .width(ui.available_width())
        .show(ui);
    if let Some(value) = field.event.as_ref().and_then(craft_ui::edit::EditEvent::value) {
        draft.text.clone_from(value);
    }
    let (rect, response) = pad_box(ui, tokens, Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Image, true, &draft.text));
    paint_signature(
        &ui.painter_at(rect),
        rect.shrink(tokens.space.pad.control),
        &SavedSignature::Typed(draft.text.trim().to_owned()),
        tokens.color.text.normal.to_egui(),
    );
}

/// The Image tab: the hint, Browse… (enabled while no decode runs), the error of the last pick,
/// and the picked image painted from its texture. Returns whether Browse… was pressed.
fn image_tab(
    ui: &mut Ui,
    tokens: &UiTokens,
    labels: &SignatureLabels,
    draft: &SignatureDraft,
    texture: Option<&ImageTexture>,
    error: Option<PickError>,
    decoding: bool,
) -> bool {
    let style = ReviewStyle::of(tokens);
    text(ui, tokens, style.body_text, tokens.color.text.muted.to_egui(), &labels.image_hint);
    let browse = ui.add_enabled(!decoding, Button::new(&labels.browse).icon(Icon::Folder).id_salt("browse")).clicked();
    if let Some(error) = error {
        let word = match error {
            PickError::Unreadable => &labels.image_failed,
            PickError::TooLarge => &labels.image_too_large,
        };
        text(ui, tokens, style.body_text, tokens.color.status.bad.fg.to_egui(), word);
    }
    let (rect, response) = pad_box(ui, tokens, Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Image, true, &labels.image_tab));
    if draft.image.is_some()
        && let Some(image) = texture
    {
        paint_texture(&ui.painter_at(rect), rect.shrink(tokens.space.pad.control), &image.texture, image.size);
    }
    browse
}

/// The Draw tab: a pad that records strokes in pad units, from the pointer or a pen.
fn draw_tab(ui: &mut Ui, tokens: &UiTokens, labels: &SignatureLabels, draft: &mut SignatureDraft) {
    // The dialog is modal, so Undo's shortcut can only mean the last stroke.
    if ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Z)) {
        draft.undo_stroke();
    }
    let (rect, response) = pad_box(ui, tokens, Sense::drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Image, true, &labels.draw_tab));
    let normalise = |p: Pos2| {
        let p = p.clamp(rect.min, rect.max);
        [(p.x - rect.left()) / rect.width(), (p.y - rect.top()) / rect.height()]
    };
    if response.drag_started() {
        draft.strokes.push(Vec::new());
    }
    if response.dragged()
        && let (Some(at), Some(stroke)) = (response.interact_pointer_pos(), draft.strokes.last_mut())
    {
        let point = normalise(at);
        if stroke.last().is_none_or(|last| (last[0] - point[0]).abs() + (last[1] - point[1]).abs() > MIN_STEP) {
            stroke.push(point);
        }
    }
    let width = craft_theme::Stroke::W2.width(ui.pixels_per_point());
    paint_strokes(&ui.painter_at(rect), rect, &draft.strokes, Stroke::new(width, tokens.color.text.normal.to_egui()), false);
}
