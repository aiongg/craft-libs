//! The clipboard handle `Commands::clipboard` returns, over the window's clipboard: a backend that hands copies to the
//! platform through the `Context`'s requests.

use crate::contract::Context;
use crate::primitives::Id;
use crate::runtime::pending::Pending;
use craft_platform::clipboard::{
    Clipboard, ClipboardBackend, ClipboardError, ClipboardImage, Flavor, HTML, MATHML, RGBA8, RTF, TEXT, Written,
};
use egui::ColorImage;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

/// One clipboard item.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ClipItem {
    /// Plain text.
    Text(Arc<str>),
    /// HTML.
    Html(Arc<str>),
    /// An image.
    Image(Arc<ColorImage>),
    /// Rich text.
    Rtf(Arc<str>),
    /// MathML.
    MathMl(Arc<str>),
    /// An item of an app-defined type.
    #[non_exhaustive]
    Custom {
        /// The MIME type.
        mime: Arc<str>,
        /// The content.
        bytes: Arc<[u8]>,
    },
}

impl ClipItem {
    /// An item of an app-defined type: `bytes` under the MIME type `mime`.
    pub fn custom(mime: impl Into<Arc<str>>, bytes: impl Into<Arc<[u8]>>) -> ClipItem {
        ClipItem::Custom { mime: mime.into(), bytes: bytes.into() }
    }

    /// The flavour the clipboard carries this item as; an image goes as straight-alpha `RGBA8`.
    fn flavor(&self) -> Result<Flavor, ClipboardError> {
        Ok(match self {
            ClipItem::Text(text) => Flavor::text(text),
            ClipItem::Html(html) => Flavor::new(HTML, html.as_bytes().to_vec()),
            ClipItem::Rtf(rtf) => Flavor::new(RTF, rtf.as_bytes().to_vec()),
            ClipItem::MathMl(mathml) => Flavor::new(MATHML, mathml.as_bytes().to_vec()),
            ClipItem::Custom { mime, bytes } => Flavor::new(mime.to_string(), bytes.to_vec()),
            ClipItem::Image(image) => {
                let image = clipboard_image(image);
                if !image.is_whole() {
                    return Err(ClipboardError::BadImage);
                }
                Flavor::rgba8(&image)
            }
        })
    }
}

/// What a paste delivers.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ClipContent {
    /// The clipboard's text, when it holds any.
    pub text: Option<Arc<str>>,
    /// The clipboard's image, when it holds one.
    pub image: Option<Arc<ColorImage>>,
}

/// Why a paste has no content.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ClipError(pub String);

/// The component's paste slot over the window's clipboard. Every call goes to the clipboard of the context's window;
/// the handle holds nothing else.
pub struct ClipboardHandle {
    ctx: Context,
    slot: Pending<ClipContent, ClipError>,
    /// The handle of a measuring run: it writes nothing.
    fenced: bool,
}

impl ClipboardHandle {
    /// The handle of the component `owner` over `ctx`'s clipboard; `pasted` is the paste its run takes, if any.
    /// `fenced`: the handle of a measuring run, which writes nothing.
    pub(crate) fn new(ctx: &Context, owner: Option<Id>, pasted: Option<ClipContent>, fenced: bool) -> Self {
        let slot = match pasted.filter(|_| owner.is_some()) {
            Some(content) => Pending::Ready(content),
            None => Pending::Loading { progress: None },
        };
        ClipboardHandle { ctx: ctx.clone(), slot, fenced }
    }

    /// The clipboard handle of `ctx`, over the installed clipboard, for code outside a component; its paste slot
    /// stays loading, as no component takes the paste.
    pub fn of(ctx: &Context) -> ClipboardHandle {
        ClipboardHandle::new(ctx, None, None, false)
    }

    /// Offers `items` in the caller's order, as one offer, and says which flavour the clipboard kept. A measuring run
    /// writes nothing and answers `Written::Fenced`.
    pub fn offer(&self, items: &[ClipItem]) -> Result<Written, ClipboardError> {
        if self.fenced {
            return Ok(Written::Fenced);
        }
        let flavors = items.iter().map(ClipItem::flavor).collect::<Result<Vec<_>, _>>()?;
        with(&self.ctx, |clipboard| clipboard.write(&flavors))
    }

    /// The first of `mimes` the installed clipboard holds.
    pub fn read(&self, mimes: &[&str]) -> Result<Option<Flavor>, ClipboardError> {
        with(&self.ctx, |clipboard| clipboard.read(mimes))
    }

    /// The clipboard's text.
    pub fn read_text(&self) -> Result<Option<String>, ClipboardError> {
        with(&self.ctx, |clipboard| clipboard.read_text())
    }

    /// The clipboard's image as straight-alpha RGBA8 pixels; `ColorImage::from_rgba_unmultiplied` turns them back.
    pub fn read_image(&self) -> Result<Option<ClipboardImage>, ClipboardError> {
        with(&self.ctx, |clipboard| clipboard.read_image())
    }

    /// The files the clipboard holds.
    pub fn read_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        with(&self.ctx, |clipboard| clipboard.read_files())
    }

    /// Whether the clipboard holds one of `mimes`.
    pub fn has(&self, mimes: &[&str]) -> bool {
        with(&self.ctx, |clipboard| clipboard.has(mimes))
    }

    /// Whether the clipboard still holds what this app last offered.
    pub fn holds_ours(&self) -> bool {
        with(&self.ctx, |clipboard| clipboard.holds_ours())
    }

    /// The MIME names of the formats on the clipboard.
    pub fn formats(&self) -> Vec<String> {
        with(&self.ctx, |clipboard| clipboard.formats())
    }

    /// Whether a late paste offer arrived since the clipboard was last asked. The answer is the clipboard's
    /// (`Clipboard::take_arrived_paste`); it is not a tracked read.
    pub fn arrived(&self) -> bool {
        with(&self.ctx, Clipboard::take_arrived_paste)
    }

    /// Offers `items` as one offer ([`offer`](Self::offer)), for a caller that does not ask what was kept.
    pub fn put(&self, items: &[ClipItem]) {
        let _ = self.offer(items);
    }

    /// The pasted content: ready in the run that delivers the paste to the focused component that lists Paste.
    pub fn paste(&self) -> &Pending<ClipContent, ClipError> {
        &self.slot
    }
}

/// The clipboard of a window's context, over the platform backend, made on first use. Shared so `with`
/// can lend it after the context's service borrow ends.
#[derive(Default)]
pub(crate) struct Installed {
    clipboard: Option<Arc<Mutex<Clipboard>>>,
    /// What the platform backend wrote, until `with` hands it to the context's requests.
    copies: Copies,
}

/// Runs `f` on the clipboard of `ctx`'s window, made here once.
fn with<R>(ctx: &Context, f: impl FnOnce(&mut Clipboard) -> R) -> R {
    let (shared, copies) = ctx.service::<Installed, _>(|installed| {
        let copies = Arc::clone(&installed.copies);
        let clipboard = installed.clipboard.get_or_insert_with(|| Arc::new(Mutex::new(Clipboard::new(PlatformClipboard { copies: Arc::clone(&copies) }))));
        (Arc::clone(clipboard), copies)
    });
    let result = {
        let mut clipboard = shared.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut clipboard)
    };
    let copy = copies.lock().unwrap_or_else(PoisonError::into_inner).take();
    match copy {
        Some(PlatformCopy::Text(text)) => ctx.copy_text(text),
        Some(PlatformCopy::Image(image)) => ctx.copy_image(image),
        None => {}
    }
    result
}

/// The backend of a window's clipboard: the platform's clipboard through the context's
/// requests (`PlatformRequest::{CopyText, CopyImage}`). The platform carries one value of text or of an image, so a
/// write acts on the first flavour it carries and skips the rest; it reads asynchronously (a paste arrives as an
/// event), so a read finds nothing. It holds no `Context` (a backend is `Send`): a write is left in a slot that
/// `with` hands to the context.
struct PlatformClipboard {
    copies: Copies,
}

/// The copy a `PlatformClipboard` write leaves for `with`.
enum PlatformCopy {
    Text(String),
    Image(craft_ui_rt::RgbaImage),
}

/// Where a `PlatformClipboard` leaves its write until `with` hands it to the context.
type Copies = Arc<Mutex<Option<PlatformCopy>>>;

impl PlatformClipboard {
    fn leave(&self, copy: PlatformCopy) {
        *self.copies.lock().unwrap_or_else(PoisonError::into_inner) = Some(copy);
    }
}

impl ClipboardBackend for PlatformClipboard {
    fn write(&mut self, flavors: &[Flavor]) -> Result<usize, ClipboardError> {
        if flavors.is_empty() {
            self.leave(PlatformCopy::Text(String::new()));
            return Ok(0);
        }
        for (at, flavor) in flavors.iter().enumerate() {
            if flavor.mime == TEXT
                && let Ok(text) = std::str::from_utf8(&flavor.data)
            {
                self.leave(PlatformCopy::Text(text.to_owned()));
                return Ok(at);
            }
            if flavor.mime == RGBA8
                && let Some(image) = flavor.rgba8_image().filter(ClipboardImage::is_whole)
            {
                let size = craft_ui_rt::SizeU::new(image.width, image.height);
                self.leave(PlatformCopy::Image(craft_ui_rt::RgbaImage { size, pixels: image.rgba }));
                return Ok(at);
            }
        }
        Err(ClipboardError::NoFormat)
    }

    fn read(&mut self, _mimes: &[&str], _limit: u64) -> Result<Option<Flavor>, ClipboardError> {
        Ok(None)
    }

    fn read_image(&mut self, _limit: u64) -> Result<Option<ClipboardImage>, ClipboardError> {
        Ok(None)
    }

    fn read_files(&mut self, _limit: usize) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        Ok(None)
    }

    fn has(&mut self, _mimes: &[&str]) -> bool {
        false
    }
}

/// `image` as straight-alpha RGBA8 pixels, the form every clipboard flavour of a bitmap carries. A side too long for
/// a `u32` gives an image that is not whole, which a write refuses.
fn clipboard_image(image: &ColorImage) -> ClipboardImage {
    let [width, height] = image.size;
    ClipboardImage {
        width: u32::try_from(width).unwrap_or(0),
        height: u32::try_from(height).unwrap_or(0),
        rgba: image.pixels.iter().flat_map(|pixel| pixel.to_srgba_unmultiplied()).collect(),
    }
}
