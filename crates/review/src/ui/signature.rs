//! The signature pad dialog and saved signatures (N5.04).

use std::cell::Cell;
use std::hash::{Hash as _, Hasher as _};

use craft_host::{Host, PickId, PickRequest, Picked};
use craft_theme::TypeRole;
use craft_ui::controls::{Button, Segmented, TextField};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::theme::{ToEgui as _, UiTokens, corner_radius, current};
use egui::epaint::{ColorImage, TextureHandle};
use egui::{Context, Id, Key, Modifiers, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureOptions, Ui, Vec2, pos2, vec2};

use super::comments::text;
use crate::signature::{MAX_IMAGE_BYTES, SavedSignature, SignatureDraft, SignatureImage, SignatureTab};

/// The pad's width over its height (PdfCraft's 460 × 150 pad). Strokes are kept in units of the
/// pad's box, so painting them elsewhere restores this shape.
const PAD_ASPECT: f32 = 460.0 / 150.0;
/// Pad points closer than this (in pad units, both axes together) to the last one are dropped.
const MIN_STEP: f32 = 0.002;
/// The share of a box that typed text and strokes fill.
const FILL: f32 = 0.9;
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
    }
}

/// The saved signature and initials, one settings section shared by every document.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SignatureSettings {
    pub signature: Option<SavedSignature>,
    pub initials: Option<SavedSignature>,
}

impl craft_host::Section for SignatureSettings {
    const KEY: &'static str = "signatures";
}

/// Draws a saved signature into `rect` in `color`: strokes scaled from pad units, typed text in
/// the script face fitted to the rect, an image fitted keeping its aspect. Textures for typed
/// and image signatures are cached by the painter's context under the signature's hash.
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
    let galley = painter.layout_no_wrap(label.to_owned(), font(rect.height() * FILL), color);
    let fit = (rect.width() * FILL / galley.size().x.max(f32::EPSILON)).min(1.0);
    let galley = if fit < 1.0 { painter.layout_no_wrap(label.to_owned(), font(rect.height() * FILL * fit), color) } else { galley };
    painter.galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// `image` fitted into `rect` keeping its aspect.
fn paint_image(painter: &Painter, rect: Rect, image: &SignatureImage) {
    let (w, h) = (image.width as f32, image.height as f32);
    if w < 1.0 || h < 1.0 {
        return;
    }
    let Some(texture) = image_texture(painter.ctx(), image) else { return };
    let size = vec2(w, h) * (rect.width() / w).min(rect.height() / h);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(texture.id(), Rect::from_center_size(rect.center(), size), uv, egui::Color32::WHITE);
}

/// The texture of `image`, cached by the context under the image's hash.
fn image_texture(ctx: &Context, image: &SignatureImage) -> Option<TextureHandle> {
    let (width, height) = (usize::try_from(image.width).ok()?, usize::try_from(image.height).ok()?);
    if width.checked_mul(height)?.checked_mul(4)? != image.rgba.len() {
        return None;
    }
    let mut hasher = std::hash::DefaultHasher::new();
    image.width.hash(&mut hasher);
    image.height.hash(&mut hasher);
    image.rgba.hash(&mut hasher);
    let key = hasher.finish();
    let slot = Id::NULL.with("craft-review-signature-textures");
    let mut cache: Vec<(u64, TextureHandle)> = ctx.data(|data| data.get_temp(slot)).unwrap_or_default();
    if let Some((_, texture)) = cache.iter().find(|(k, _)| *k == key) {
        return Some(texture.clone());
    }
    let pixels = ColorImage::from_rgba_unmultiplied([width, height], &image.rgba);
    let texture = ctx.load_texture("signature-image", pixels, TextureOptions::LINEAR);
    cache.push((key, texture.clone()));
    if cache.len() > TEXTURES_KEPT {
        drop(cache.remove(0));
    }
    ctx.data_mut(|data| data.insert_temp(slot, cache));
    Some(texture)
}

/// Decodes a PNG or JPEG of at most [`MAX_IMAGE_BYTES`] into straight RGBA.
fn decode_image(bytes: &[u8]) -> Option<SignatureImage> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let decoded = image::load_from_memory(bytes).ok()?.into_rgba8();
    Some(SignatureImage { width: decoded.width(), height: decoded.height(), rgba: decoded.into_raw() })
}

/// The Scan tab's content, the app's.
pub type ScanTab = Box<dyn FnMut(&mut egui::Ui, &mut SignatureDraft) + Send>;

/// Acrobat's signature pad on the dialog host: "Create signature" / "Change initials", the tabs
/// Type (a field, previewed in the script face), Draw (a pad: strokes with the pointer or pen,
/// Undo), Image (Browse… a PNG or JPEG up to [`MAX_IMAGE_BYTES`](crate::signature::MAX_IMAGE_BYTES)
/// through `host.files()`) and Scan (shown when the app gives a scan hook); Clear; Apply (enabled
/// when [`SignatureDraft::ready`]) and Cancel. Tabs are a segmented control (arrow keys move);
/// Tab walks the controls; Enter is Apply; Esc is Cancel.
pub struct SignaturePadDialog {
    draft: SignatureDraft,
    scan: Option<ScanTab>,
    labels: Option<SignatureLabels>,
    state: DialogState<()>,
    /// The Browse… pick waiting for its answer.
    pick: Option<PickId>,
}

impl SignaturePadDialog {
    pub fn new(draft: SignatureDraft) -> Self {
        Self { draft, scan: None, labels: None, state: DialogState::default(), pick: None }
    }

    /// The Scan tab's content (PdfCraft's camera and clean-up): it fills `draft.image`.
    pub fn scan(mut self, tab: impl FnMut(&mut egui::Ui, &mut SignatureDraft) + Send + 'static) -> Self {
        self.scan = Some(Box::new(tab));
        self
    }

    pub fn labels(mut self, labels: SignatureLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl StackDialog for SignaturePadDialog {
    /// The saved signature, and whether it is initials.
    type Result = (SavedSignature, bool);

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("signature-pad")
    }

    // pdfcraft: crates/ui-egui/src/tools/fill_sign.rs:598 `signature_pad`, `:732` `pad_buttons`,
    // `fill_sign/scan.rs:204` `scan_tab`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>> {
        let labels = self.labels.clone().unwrap_or_else(|| SignatureLabels::of_ctx(ctx));
        self.take_picked(ctx);
        let tokens = current(ctx);
        let what = if self.draft.initials { &labels.initials } else { &labels.signature };
        let title = fmt(if self.draft.editing { &labels.change } else { &labels.create }, &[("what", what)]);
        let id = self.id();
        let heading = title.clone();
        let ready = Cell::new(self.draft.ready());
        let mut browse = false;
        let Self { draft, scan, state, .. } = self;
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
                SignatureTab::Draw => draw_tab(ui, &tokens, draft),
                SignatureTab::Image => browse = image_tab(ui, &tokens, &labels, draft),
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
                    Some(DialogResult::Ok((saved, self.draft.initials)))
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
        self.draft.saved().map(|saved| (saved, self.draft.initials))
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

    /// Takes the Browse… answer when it has come.
    fn take_picked(&mut self, ctx: &Context) {
        let (Some(ticket), Some(host)) = (self.pick, Host::try_of_ctx(ctx)) else { return };
        let Some(picked) = host.files().take_answer(ticket) else { return };
        self.pick = None;
        let bytes = match picked {
            Picked::Cancelled => return,
            Picked::Contents(files) => files.into_iter().next().map(|(_, bytes)| bytes),
            Picked::Paths(paths) => paths.first().and_then(|path| host.files().read(path).ok()),
        };
        if let Some(image) = bytes.as_deref().and_then(decode_image) {
            self.draft.image = Some(image);
        }
    }

    fn forget_pick(&mut self, ctx: &Context) {
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
    let field = TextField::new(salt.with("text"), &labels.type_hint, &draft.text).placeholder(&labels.type_hint).width(ui.available_width()).show(ui);
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

fn image_tab(ui: &mut Ui, tokens: &UiTokens, labels: &SignatureLabels, draft: &SignatureDraft) -> bool {
    text(ui, tokens, TypeRole::Body, tokens.color.text.muted.to_egui(), &labels.image_hint);
    let browse = ui.add(Button::new(&labels.browse).icon(Icon::Folder).id_salt("browse")).clicked();
    let (rect, response) = pad_box(ui, tokens, Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Image, true, &labels.image_tab));
    if let Some(image) = &draft.image {
        paint_signature(&ui.painter_at(rect), rect.shrink(tokens.space.pad.control), &SavedSignature::Image(image.clone()), egui::Color32::WHITE);
    }
    browse
}

/// The Draw tab: a pad that records strokes in pad units, from the pointer or a pen.
fn draw_tab(ui: &mut Ui, tokens: &UiTokens, draft: &mut SignatureDraft) {
    // The dialog is modal, so Undo's shortcut can only mean the last stroke.
    if ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Z)) {
        draft.undo_stroke();
    }
    let (rect, response) = pad_box(ui, tokens, Sense::drag());
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
