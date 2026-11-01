//! The system clipboard for copy and paste of text, rich text, images and files (N10.01).
//!
//! - [`Clipboard`]: the one write and read path every copy and paste command goes through. It
//!   applies the size limits ([`ClipboardLimits`]), keeps input injected by automation from
//!   replacing what the user copied ([`Clipboard::set_synthetic`]), and remembers what it wrote
//!   last so the app can tell its own copy from one made in another app
//!   ([`Clipboard::holds_ours`]).
//! - [`ClipboardBackend`]: where the bytes go. [`SystemClipboard`] is the OS clipboard on the
//!   desktop (arboard, opened on first use); [`EventClipboard`] is the browser's, fed from paste
//!   events; tests plug in their own.
//! - [`ClipboardProbe`]: asks a second clipboard handle on a worker thread whether Paste has
//!   something to take, so a slow clipboard owner never stalls a frame.
//!
//! No egui, except the two event helpers behind the `ui` feature
//! ([`flavors_from_event`], [`copy_through_egui`]) that the web path uses.

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::{Duration, Instant};

use craft_host::Wake;

use crate::PlatformText;

/// `text/plain`, UTF-8.
pub const TEXT: &str = "text/plain";
/// `text/html`, a rich-text fragment.
pub const HTML: &str = "text/html";
/// `text/rtf`.
pub const RTF: &str = "text/rtf";
/// `image/png`.
pub const PNG: &str = "image/png";
/// `image/svg+xml`.
pub const SVG: &str = "image/svg+xml";
/// `application/pdf`.
pub const PDF: &str = "application/pdf";
/// `text/uri-list`: copied files, one `file://` URI per line (CRLF line ends, RFC 2483).
pub const URI_LIST: &str = "text/uri-list";
/// Any bitmap the clipboard holds (`image/png`, `image/bmp`, a TIFF on macOS…). A read returns it
/// as an [`RGBA8`] flavor (never PNG: this crate has no PNG encoder); [`ClipboardBackend::read_image`]
/// returns it as [`ClipboardImage`]. Also for [`ClipboardBackend::has`] and [`Clipboard::has`].
pub const BITMAP: &str = "image/*";
/// A raw bitmap as [`EventClipboard`] carries it: width and height as little-endian `u32`s, then
/// the straight RGBA8 pixels. What [`flavors_from_event`] makes of egui's `Event::PasteImage`.
pub const RGBA8: &str = "image/x-rgba8";

/// One representation of the clipboard's contents: its MIME type and bytes. A copy offers
/// several, best first (VectorCraft: SVG, PDF, PNG, then text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flavor {
    pub mime: Cow<'static, str>,
    pub data: Vec<u8>,
}

impl Flavor {
    pub fn new(mime: impl Into<Cow<'static, str>>, data: Vec<u8>) -> Self {
        Self { mime: mime.into(), data }
    }

    /// `text/plain` with `text`.
    pub fn text(text: &str) -> Self {
        Self::new(TEXT, text.as_bytes().to_vec())
    }
}

/// A bitmap on the clipboard: straight (not premultiplied) RGBA8, rows top to bottom,
/// `rgba.len() == width * height * 4`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ClipboardImage {
    /// True when the size is not zero and `rgba` holds exactly `width * height * 4` bytes.
    /// [`Clipboard::read_image`] drops images that fail this (PhotoCraft
    /// `import_os_clipboard`'s filter).
    // pc: crates/ui-egui/src/app/clipboard.rs:60 image.filter
    pub fn is_whole(&self) -> bool {
        let bytes = u64::from(self.width) * u64::from(self.height) * 4;
        bytes != 0 && self.rgba.len() as u64 == bytes
    }
}

/// The most a paste reads, so a broken or hostile clipboard source can't make the app allocate
/// without end. Over a limit, the read fails with [`ClipboardError::TooLarge`] and nothing is
/// pasted (owner question Q-S5S-3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipboardLimits {
    /// Bytes of one image's pixels or encoded data: 1 GiB, a 16K × 16K RGBA bitmap
    /// (PhotoCraft's `MAX_CLIPBOARD_IMAGE_BYTES = 4 * 16384 * 16384`), so no real screenshot is
    /// turned away.
    pub image_bytes: u64,
    /// Bytes of text, rich text and SVG markup: 128 MiB (PhotoCraft's `MAX_CLIPBOARD_SVG_BYTES`,
    /// the size an `.svgz` may inflate to).
    pub text_bytes: u64,
    /// Paths in one list of copied files: 10 000.
    pub files: usize,
}

impl ClipboardLimits {
    pub const DEFAULT: ClipboardLimits = ClipboardLimits { image_bytes: 4 * 16384 * 16384, text_bytes: 128 << 20, files: 10_000 };
}

impl Default for ClipboardLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Why a clipboard read or write failed. "Nothing in that format" is not an error: reads
/// answer `Ok(None)` then.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    /// No clipboard can be reached at all: a Flatpak on Wayland, where the sandbox hides the
    /// data-control protocol (PhotoCraft #5031), or a headless session. Paste explains it with
    /// [`CLIPBOARD_UNAVAILABLE`].
    Unavailable,
    /// Another read or write holds the clipboard handle (a re-entrant call).
    Busy,
    /// The contents are larger than the limit for their kind, in bytes.
    TooLarge { limit: u64 },
    /// None of the flavours written can be put on this clipboard (only SVG or PDF on the system
    /// clipboard, or an image the platform can't take).
    NoFormat,
    /// A bitmap whose pixels don't match its size ([`ClipboardImage::is_whole`]).
    BadImage,
    /// What the operating system said (a broken pipe on Wayland, a clipboard owner that never
    /// answered): free text, in the system's language, not a [`PlatformText`].
    Other(String),
}

impl ClipboardError {
    /// The library's own reason as a [`PlatformText`] the app looks up in its catalog, for the
    /// `{error}` of [`CLIPBOARD_FAILED`]. `None` for [`Other`](Self::Other), whose text is the
    /// system's. [`TooLarge`](Self::TooLarge) is [`CLIPBOARD_TOO_LARGE`] with its `{limit}`.
    pub fn text(&self) -> Option<PlatformText> {
        match self {
            Self::Unavailable => Some(CLIPBOARD_UNREACHABLE),
            Self::Busy => Some(CLIPBOARD_BUSY),
            Self::TooLarge { .. } => Some(CLIPBOARD_TOO_LARGE),
            Self::NoFormat => Some(CLIPBOARD_NO_FORMAT),
            Self::BadImage => Some(CLIPBOARD_BAD_IMAGE),
            Self::Other(_) => None,
        }
    }
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_str(message),
            Self::TooLarge { limit } => f.write_str(&CLIPBOARD_TOO_LARGE.english.replace("{limit}", &format!("{limit} bytes"))),
            other => f.write_str(other.text().map_or("", |text| text.english)),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// "The clipboard can't be read here", for [`ClipboardError::Unavailable`] on paste.
pub const CLIPBOARD_UNAVAILABLE: PlatformText =
    PlatformText { id: "clipboard.unavailable", english: "The system clipboard can't be reached. Paste works with what was copied in this app." };
/// A paste over [`ClipboardLimits`]; `{limit}` is the limit as a file size ("1 GB").
pub const CLIPBOARD_TOO_LARGE: PlatformText =
    PlatformText { id: "clipboard.tooLarge", english: "What's on the clipboard is too large to paste (more than {limit})." };
/// [`ClipboardError::Unavailable`] as the `{error}` of [`CLIPBOARD_FAILED`].
pub const CLIPBOARD_UNREACHABLE: PlatformText = PlatformText { id: "clipboard.unreachable", english: "the system clipboard can't be reached" };
/// [`ClipboardError::Busy`] as the `{error}` of [`CLIPBOARD_FAILED`].
pub const CLIPBOARD_BUSY: PlatformText = PlatformText { id: "clipboard.busy", english: "the clipboard is busy" };
/// [`ClipboardError::NoFormat`] as the `{error}` of [`CLIPBOARD_FAILED`].
pub const CLIPBOARD_NO_FORMAT: PlatformText = PlatformText { id: "clipboard.noFormat", english: "none of the formats can be put on the clipboard" };
/// [`ClipboardError::BadImage`] as the `{error}` of [`CLIPBOARD_FAILED`].
pub const CLIPBOARD_BAD_IMAGE: PlatformText = PlatformText { id: "clipboard.badImage", english: "the image's pixels don't match its size" };
/// Any other failure; `{error}` is the reason: [`ClipboardError::text`] looked up in the catalog,
/// or the system's own text of [`ClipboardError::Other`].
pub const CLIPBOARD_FAILED: PlatformText = PlatformText { id: "clipboard.failed", english: "Couldn't use the clipboard: {error}" };
/// Every [`PlatformText`] of this module, for the missing-translation report ([`crate::texts`]).
pub const TEXTS: &[PlatformText] =
    &[CLIPBOARD_UNAVAILABLE, CLIPBOARD_TOO_LARGE, CLIPBOARD_UNREACHABLE, CLIPBOARD_BUSY, CLIPBOARD_NO_FORMAT, CLIPBOARD_BAD_IMAGE, CLIPBOARD_FAILED];

/// What a write did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum Written {
    /// The clipboard holds the new contents.
    Written,
    /// Dropped: the write came from synthetic input ([`Clipboard::set_synthetic`]), and the
    /// clipboard keeps what the user copied.
    Fenced,
}

/// Where clipboard bytes go: the OS clipboard, the browser's, or a test's.
///
/// `Send` so a [`ClipboardProbe`] can own one on its worker thread. Every method may block on a
/// slow clipboard owner (an X11 client that never answers); the UI thread calls only
/// [`Clipboard`]'s reads and writes, on a user's copy or paste.
pub trait ClipboardBackend: Send {
    /// Replaces the clipboard's contents with `flavors`, best first; none empties it. A backend
    /// holds fewer flavours than an app offers, so it answers the index in `flavors` of the one
    /// it wrote (`0` when it emptied the clipboard); [`Clipboard::holds_ours`] compares that one.
    fn write(&mut self, flavors: &[Flavor]) -> Result<usize, ClipboardError>;
    /// The first of `mimes` the clipboard holds, at most `limit` bytes of it (more is
    /// [`ClipboardError::TooLarge`]). [`BITMAP`] matches any bitmap, returned as an [`RGBA8`] flavor.
    fn read(&mut self, mimes: &[&str], limit: u64) -> Result<Option<Flavor>, ClipboardError>;
    /// The clipboard's bitmap as RGBA8, at most `limit` bytes of pixels.
    fn read_image(&mut self, limit: u64) -> Result<Option<ClipboardImage>, ClipboardError>;
    /// Copied files (Finder, Explorer, GNOME Files), at most `limit` of them.
    fn read_files(&mut self, limit: usize) -> Result<Option<Vec<PathBuf>>, ClipboardError>;
    /// Does the clipboard hold one of `mimes`? Cheap enough to ask a few times a second.
    fn has(&mut self, mimes: &[&str]) -> bool;
    /// The flavours of a paste the host delivered as an event (the browser's Paste). Only
    /// [`EventClipboard`] keeps them; the system clipboard ignores them.
    fn offer(&mut self, _flavors: Vec<Flavor>) {}
}

/// The desktop's clipboard through arboard: X11 and Wayland, macOS, Windows. Not on wasm.
///
/// The handle opens on first use and stays open (on X11 the last handle alive hands what the app
/// copied to the clipboard manager at exit). When no backend opens, every call answers
/// [`ClipboardError::Unavailable`] and [`unreadable`](Self::unreadable) turns true. On Wayland a
/// bitmap arboard can't read (it asks for `image/png` only; a screenshot tool may offer only
/// `image/jpeg` or `image/bmp`, PhotoCraft #2546) reads as `None`.
///
/// arboard reaches text, HTML, one bitmap and copied files; other flavors ([`SVG`], [`PDF`],
/// [`RTF`]) are not written or read here.
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
pub struct SystemClipboard {
    handle: Option<arboard::Clipboard>,
    unreadable: bool,
    decode: Option<Decode>,
    presence: presence::Presence,
}

/// The app's decoder for encoded bitmaps ([`SystemClipboard::decoder`]).
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
pub(crate) type Decode = Box<dyn Fn(&[u8]) -> Option<ClipboardImage> + Send>;

#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
impl SystemClipboard {
    pub fn new() -> Self {
        Self { handle: None, unreadable: false, decode: None, presence: presence::Presence::new() }
    }

    /// Decodes a [`PNG`] flavor to pixels for [`ClipboardBackend::write`], which hands the
    /// platform pixels only. Without one, a write offers no PNG.
    pub fn decoder(mut self, decode: impl Fn(&[u8]) -> Option<ClipboardImage> + Send + 'static) -> Self {
        self.decode = Some(Box::new(decode));
        self
    }

    /// True when the last attempt to open the clipboard found no backend at all.
    pub fn unreadable(&self) -> bool {
        self.unreadable
    }

    fn open(&mut self) -> Result<&mut arboard::Clipboard, ClipboardError> {
        if self.handle.is_none() {
            match arboard::Clipboard::new() {
                Ok(handle) => {
                    self.unreadable = false;
                    self.handle = Some(handle);
                }
                Err(error) => {
                    log::debug!("no system clipboard: {error}");
                    self.unreadable = true;
                }
            }
        }
        self.handle.as_mut().ok_or(ClipboardError::Unavailable)
    }
}

#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
fn system_error(error: arboard::Error) -> ClipboardError {
    match error {
        arboard::Error::ClipboardOccupied => ClipboardError::Busy,
        arboard::Error::ClipboardNotSupported => ClipboardError::Unavailable,
        error => ClipboardError::Other(error.to_string()),
    }
}

/// A read's value, or `None` when the clipboard holds nothing in that format. `unconvertible`
/// also counts content the format can't be converted from (text that isn't UTF-8) as nothing.
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
fn present<T>(read: Result<T, arboard::Error>, unconvertible: bool) -> Result<Option<T>, ClipboardError> {
    match read {
        Ok(value) => Ok(Some(value)),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(arboard::Error::ConversionFailure) if unconvertible => Ok(None),
        Err(error) => Err(system_error(error)),
    }
}

/// Whether the system clipboard offers a format, asked of the system without reading or decoding
/// any content: Windows `IsClipboardFormatAvailable`, macOS the pasteboard's type list, Wayland
/// the offer's MIME types (`wl_clipboard_rs::paste::get_mime_types`), X11 a `TARGETS` request.
///
/// The macOS and Windows code is not built on the Linux box; it is a few lines each and needs a
/// check on those systems.
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
mod presence {
    #[cfg(any(windows, target_os = "macos"))]
    use super::{BITMAP, HTML, RGBA8, TEXT, URI_LIST};

    pub(super) struct Presence {
        #[cfg(any(windows, target_os = "macos"))]
        _system: (),
        #[cfg(not(any(windows, target_os = "macos")))]
        x11: Option<x11::Targets>,
    }

    #[cfg(windows)]
    impl Presence {
        pub(super) fn new() -> Self {
            Self { _system: () }
        }

        pub(super) fn any(&mut self, mimes: &[&str]) -> bool {
            use clipboard_win::{formats, is_format_avail, register_format};
            let registered = |name: &str| register_format(name).is_some_and(|format| is_format_avail(format.get()));
            mimes.iter().any(|&mime| match mime {
                TEXT => is_format_avail(formats::CF_UNICODETEXT),
                HTML => registered("HTML Format"),
                BITMAP | RGBA8 => registered("PNG") || is_format_avail(formats::CF_DIBV5) || is_format_avail(formats::CF_DIB),
                URI_LIST => is_format_avail(formats::CF_HDROP),
                _ => false,
            })
        }
    }

    #[cfg(target_os = "macos")]
    impl Presence {
        pub(super) fn new() -> Self {
            Self { _system: () }
        }

        pub(super) fn any(&mut self, mimes: &[&str]) -> bool {
            use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypeHTML, NSPasteboardTypeString, NSPasteboardTypeTIFF};
            // `types` lists the formats on offer; no item's data is fetched.
            let Some(types) = (unsafe { NSPasteboard::generalPasteboard().types() }) else { return false };
            mimes.iter().any(|&mime| {
                // Safety: the pasteboard type constants are valid for the life of the process.
                let wanted = unsafe {
                    match mime {
                        TEXT => NSPasteboardTypeString,
                        HTML => NSPasteboardTypeHTML,
                        BITMAP | RGBA8 => NSPasteboardTypeTIFF,
                        URI_LIST => NSPasteboardTypeFileURL,
                        _ => return false,
                    }
                };
                types.iter().any(|offered| offered.isEqualToString(wanted))
            })
        }
    }

    /// Linux and BSD. Wayland when `WAYLAND_DISPLAY` is set and the compositor has a data-control
    /// protocol, else X11: the order arboard picks its own backend in.
    #[cfg(not(any(windows, target_os = "macos")))]
    impl Presence {
        pub(super) fn new() -> Self {
            Self { x11: None }
        }

        pub(super) fn any(&mut self, mimes: &[&str]) -> bool {
            let offered = self.offered();
            mimes.iter().any(|&mime| super::offered_has(&offered, mime))
        }

        fn offered(&mut self) -> Vec<String> {
            use wl_clipboard_rs::paste::{ClipboardType, Error, Seat, get_mime_types};
            if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                match get_mime_types(ClipboardType::Regular, Seat::Unspecified) {
                    Ok(types) => return types.into_iter().collect(),
                    // Nothing is copied.
                    Err(Error::NoSeats | Error::ClipboardEmpty | Error::NoMimeType) => return Vec::new(),
                    Err(error) => log::debug!("no Wayland data control for the clipboard probe, trying X11: {error}"),
                }
            }
            if self.x11.is_none() {
                self.x11 = x11::Targets::connect();
            }
            self.x11.as_ref().map(x11::Targets::offered).unwrap_or_default()
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    mod x11 {
        use std::time::{Duration, Instant};

        use x11rb::connection::Connection;
        use x11rb::protocol::Event;
        use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, CreateWindowAux, EventMask, WindowClass};
        use x11rb::rust_connection::RustConnection;
        use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE};

        /// How long a `TARGETS` request waits for the owner. The probe's worker waits, not the UI.
        const ANSWER_WAIT: Duration = Duration::from_millis(250);

        /// A connection and a window that receive the clipboard owner's `TARGETS` answer.
        pub(super) struct Targets {
            conn: RustConnection,
            window: u32,
            clipboard: Atom,
            targets: Atom,
            property: Atom,
        }

        impl Targets {
            pub(super) fn connect() -> Option<Self> {
                let (conn, screen) = RustConnection::connect(None).ok()?;
                let root = conn.setup().roots.get(screen)?.root;
                let window = conn.generate_id().ok()?;
                conn.create_window(
                    COPY_DEPTH_FROM_PARENT,
                    window,
                    root,
                    0,
                    0,
                    1,
                    1,
                    0,
                    WindowClass::INPUT_OUTPUT,
                    0,
                    &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                )
                .ok()?;
                let atom = |name: &[u8]| conn.intern_atom(false, name).ok()?.reply().ok().map(|reply| reply.atom);
                let (clipboard, targets, property) = (atom(b"CLIPBOARD")?, atom(b"TARGETS")?, atom(b"CRAFT_CLIPBOARD_TARGETS")?);
                Some(Self { conn, window, clipboard, targets, property })
            }

            /// The target names the CLIPBOARD owner offers; none when nobody owns it or answers.
            pub(super) fn offered(&self) -> Vec<String> {
                self.ask().unwrap_or_default()
            }

            fn ask(&self) -> Option<Vec<String>> {
                self.conn.delete_property(self.window, self.property).ok()?;
                self.conn.convert_selection(self.window, self.clipboard, self.targets, self.property, CURRENT_TIME).ok()?;
                self.conn.flush().ok()?;
                let end = Instant::now() + ANSWER_WAIT;
                while Instant::now() < end {
                    match self.conn.poll_for_event().ok()? {
                        Some(Event::SelectionNotify(event)) if event.selection == self.clipboard => {
                            if event.property == NONE {
                                return None;
                            }
                            let reply =
                                self.conn.get_property(true, self.window, self.property, AtomEnum::ATOM, 0, u32::MAX / 4).ok()?.reply().ok()?;
                            let atoms: Vec<Atom> = reply.value32()?.collect();
                            return Some(
                                atoms
                                    .into_iter()
                                    .filter_map(|atom| {
                                        let name = self.conn.get_atom_name(atom).ok()?.reply().ok()?.name;
                                        String::from_utf8(name).ok()
                                    })
                                    .collect(),
                            );
                        }
                        Some(_) => {}
                        None => std::thread::sleep(Duration::from_millis(1)),
                    }
                }
                None
            }
        }
    }
}

/// Whether the target or MIME names a clipboard `offered` contain a format that [`SystemClipboard`]
/// can read as `mime`. Text includes the X11 text targets; a bitmap is `image/png`, the only
/// bitmap arboard reads on Linux.
#[allow(dead_code, reason = "only the Linux and BSD probe maps names")]
pub(crate) fn offered_has(offered: &[String], mime: &str) -> bool {
    let any = |names: &[&str]| offered.iter().any(|offer| names.iter().any(|name| offer.eq_ignore_ascii_case(name)));
    match mime {
        TEXT => any(&["text/plain", "text/plain;charset=utf-8", "UTF8_STRING", "STRING", "TEXT"]),
        HTML => any(&["text/html"]),
        BITMAP | RGBA8 => any(&[PNG]),
        URI_LIST => any(&[URI_LIST]),
        _ => false,
    }
}

/// What a write puts on the system clipboard.
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
pub(crate) enum Put {
    Clear,
    Image(ClipboardImage),
    Html { html: String, alt: Option<String> },
    Text(String),
}

/// The first flavor the system clipboard can hold, as its index in `flavors` and what to put
/// there.
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
pub(crate) fn plan_write(flavors: &[Flavor], decode: Option<&Decode>) -> Result<(usize, Put), ClipboardError> {
    if flavors.is_empty() {
        return Ok((0, Put::Clear));
    }
    let text_of = |f: &Flavor| String::from_utf8_lossy(&f.data).into_owned();
    for (index, flavor) in flavors.iter().enumerate() {
        match &*flavor.mime {
            RGBA8 => {
                if let Some(image) = rgba8_image(&flavor.data).filter(ClipboardImage::is_whole) {
                    return Ok((index, Put::Image(image)));
                }
            }
            PNG => {
                if let Some(image) = decode.and_then(|decode| decode(&flavor.data)).filter(ClipboardImage::is_whole) {
                    return Ok((index, Put::Image(image)));
                }
            }
            HTML => return Ok((index, Put::Html { html: text_of(flavor), alt: flavors.iter().find(|f| f.mime == TEXT).map(text_of) })),
            TEXT => return Ok((index, Put::Text(text_of(flavor)))),
            _ => {}
        }
    }
    Err(ClipboardError::NoFormat)
}

#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
impl ClipboardBackend for SystemClipboard {
    // pc: apps/photocraft/src/services.rs:702 clipboard_set_image (open on first use, "clipboard is busy")
    fn write(&mut self, flavors: &[Flavor]) -> Result<usize, ClipboardError> {
        let (index, put) = plan_write(flavors, self.decode.as_ref())?;
        let handle = self.open()?;
        match put {
            Put::Clear => handle.clear(),
            Put::Image(image) => {
                handle.set_image(arboard::ImageData { width: image.width as usize, height: image.height as usize, bytes: Cow::Borrowed(&image.rgba) })
            }
            Put::Html { html, alt } => handle.set_html(html, alt),
            Put::Text(text) => handle.set_text(text),
        }
        .map_err(system_error)?;
        Ok(index)
    }

    // vectorcraft: crates/ui-egui/src/sysclip.rs:27 SystemClipboard::read
    fn read(&mut self, mimes: &[&str], limit: u64) -> Result<Option<Flavor>, ClipboardError> {
        for &mime in mimes {
            let found = match mime {
                TEXT => present(self.open()?.get().text(), true)?.map(|text| Flavor::text(&text)),
                HTML => present(self.open()?.get().html(), true)?.map(|html| Flavor::new(HTML, html.into_bytes())),
                BITMAP | RGBA8 => self.read_image(limit)?.map(|image| rgba8_flavor(&image)),
                URI_LIST => {
                    let files = present(self.open()?.get().file_list(), false)?.filter(|files| !files.is_empty());
                    files.map(|files| Flavor::new(URI_LIST, uri_list(&files, cfg!(windows)).into_bytes()))
                }
                _ => None,
            };
            if let Some(flavor) = found {
                if flavor.data.len() as u64 > limit {
                    return Err(ClipboardError::TooLarge { limit });
                }
                return Ok(Some(flavor));
            }
        }
        Ok(None)
    }

    // pc: apps/photocraft/src/services.rs:714 clipboard_get_image, :536 wayland_clipboard_image, :601 clipboard_read
    fn read_image(&mut self, limit: u64) -> Result<Option<ClipboardImage>, ClipboardError> {
        let Some(image) = present(self.open()?.get_image(), false)? else { return Ok(None) };
        if image.bytes.len() as u64 > limit {
            return Err(ClipboardError::TooLarge { limit });
        }
        Ok(Some(ClipboardImage { width: image.width as u32, height: image.height as u32, rgba: image.bytes.into_owned() }))
    }

    // pc: apps/photocraft/src/services.rs:506 image_from_files (trailing '\r' of text/uri-list lines trimmed)
    fn read_files(&mut self, limit: usize) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        let Some(files) = present(self.open()?.get().file_list(), false)? else { return Ok(None) };
        // arboard splits a text/uri-list on LF only, so a path copied in GNOME Files keeps its CR.
        let files: Vec<PathBuf> = files.into_iter().map(trim_cr).collect();
        if files.len() > limit {
            return Err(ClipboardError::TooLarge { limit: limit as u64 });
        }
        Ok((!files.is_empty()).then_some(files))
    }

    // vectorcraft: crates/ui-egui/src/sysclip.rs:29 SystemClipboard::has
    fn has(&mut self, mimes: &[&str]) -> bool {
        self.presence.any(mimes)
    }
}

/// A clipboard fed by the app from paste events: the browser's (its Paste event carries text,
/// pictures and files; a page can't read the clipboard on its own) and any host that delivers
/// pastes as events. Writes go out through `out` (on the web, egui's copy commands:
/// [`copy_through_egui`]).
///
/// [`offer`](Self::offer) holds one paste's flavours for the pass it was offered in. The
/// clipboard counts passes itself (`pass`, egui's `cumulative_pass_nr` for
/// [`for_egui`](Self::for_egui)), so a menu Paste in a later pass finds nothing and
/// [`Clipboard::has`] is false: the browser can't be read outside a paste event. The app does
/// nothing at the end of a pass.
pub struct EventClipboard {
    pasted: Vec<Flavor>,
    offered_in: u64,
    pass: PassClock,
    out: CopyOut,
}

/// The number of the host's current pass; it changes when a pass ends.
type PassClock = Box<dyn Fn() -> u64 + Send>;

/// Where an [`EventClipboard`]'s writes go.
type CopyOut = Box<dyn FnMut(&[Flavor]) -> Result<usize, ClipboardError> + Send>;

impl EventClipboard {
    /// `pass` answers the host's pass number (any counter that changes when a pass ends). `out`
    /// puts the flavours where the host's copy goes and answers the index of the one it wrote
    /// ([`ClipboardBackend::write`]).
    pub fn new(pass: impl Fn() -> u64 + Send + 'static, out: impl FnMut(&[Flavor]) -> Result<usize, ClipboardError> + Send + 'static) -> Self {
        Self { pasted: Vec::new(), offered_in: 0, pass: Box::new(pass), out: Box::new(out) }
    }

    /// The browser's clipboard under egui: passes are `ctx.cumulative_pass_nr()` and copies go
    /// through [`copy_through_egui`]. With the `ui` feature.
    #[cfg(feature = "ui")]
    pub fn for_egui(ctx: &egui::Context) -> Self {
        let (clock, out) = (ctx.clone(), ctx.clone());
        Self::new(move || clock.cumulative_pass_nr(), move |flavors| copy_through_egui(&out, flavors))
    }

    /// The first of `mimes` this pass's paste offered; [`BITMAP`] is any offered `image/*`.
    fn offered(&self, mimes: &[&str]) -> Option<&Flavor> {
        if (self.pass)() != self.offered_in {
            return None;
        }
        mimes.iter().find_map(|&mime| self.pasted.iter().find(|f| if mime == BITMAP { f.mime.starts_with("image/") } else { f.mime == mime }))
    }
}

impl ClipboardBackend for EventClipboard {
    // new: the browser has no clipboard read; PhotoCraft's web shell maps Event::PasteImage (apps/photocraft-web/src/web.rs:344)
    fn write(&mut self, flavors: &[Flavor]) -> Result<usize, ClipboardError> {
        (self.out)(flavors)
    }

    // new: first offered flavour among `mimes`, limit checked
    fn read(&mut self, mimes: &[&str], limit: u64) -> Result<Option<Flavor>, ClipboardError> {
        let Some(flavor) = self.offered(mimes) else { return Ok(None) };
        if flavor.data.len() as u64 > limit {
            return Err(ClipboardError::TooLarge { limit });
        }
        Ok(Some(flavor.clone()))
    }

    // new: only an offered RGBA8 flavour reads (no codec here); an offered PNG is the app's to decode through `read`
    fn read_image(&mut self, limit: u64) -> Result<Option<ClipboardImage>, ClipboardError> {
        let Some(image) = self.offered(&[RGBA8]).and_then(|flavor| rgba8_image(&flavor.data)) else { return Ok(None) };
        if image.rgba.len() as u64 > limit {
            return Err(ClipboardError::TooLarge { limit });
        }
        Ok(Some(image))
    }

    // new: an offered text/uri-list, one file:// URI per line
    fn read_files(&mut self, limit: usize) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        let Some(flavor) = self.offered(&[URI_LIST]) else { return Ok(None) };
        let files = parse_uri_list(&String::from_utf8_lossy(&flavor.data));
        if files.len() > limit {
            return Err(ClipboardError::TooLarge { limit: limit as u64 });
        }
        Ok((!files.is_empty()).then_some(files))
    }

    fn has(&mut self, mimes: &[&str]) -> bool {
        self.offered(mimes).is_some()
    }

    /// Holds one paste's flavours, best first, until the pass it was offered in ends.
    fn offer(&mut self, flavors: Vec<Flavor>) {
        self.pasted = flavors;
        self.offered_in = (self.pass)();
    }
}

/// The app's clipboard: one per window, on the UI thread.
///
/// ```ignore
/// let mut clipboard = Clipboard::new(SystemClipboard::new().decoder(decode_png));
/// // Each frame, before running commands: input injected by automation must not copy.
/// clipboard.set_synthetic(app.automation_input);
/// // Copy
/// let _ = clipboard.write_image(&ClipboardImage { width, height, rgba })?;
/// // Paste: a new image from another app replaces the in-app copy.
/// if let Some(image) = clipboard.read_image()? && !clipboard.is_ours(&image) { session.clipboard = image.into(); }
/// ```
pub struct Clipboard {
    backend: Box<dyn ClipboardBackend>,
    limits: ClipboardLimits,
    synthetic: bool,
    ours: Option<Ours>,
}

/// What the last write put on the clipboard, as a fingerprint.
enum Ours {
    /// [`image_signature`] of a bitmap.
    Image(u64),
    /// The flavor the backend wrote: its type and a hash of its bytes.
    Flavor { mime: String, hash: u64 },
}

impl Clipboard {
    pub fn new(backend: impl ClipboardBackend + 'static) -> Self {
        Self { backend: Box::new(backend), limits: ClipboardLimits::DEFAULT, synthetic: false, ours: None }
    }

    pub fn limits(mut self, limits: ClipboardLimits) -> Self {
        self.limits = limits;
        self
    }

    /// While `true` (a frame that processes input injected by automation or a test's control
    /// channel), writes are dropped with [`Written::Fenced`] and reads still work. FigCraft's
    /// rule: "Input injected through the control channel never replaces what the user copied."
    /// The app sets it at the start of each frame.
    // figcraft: crates/ui-egui/src/app/clipboard.rs:6 clipboard_set_text
    pub fn set_synthetic(&mut self, synthetic: bool) {
        self.synthetic = synthetic;
    }

    pub fn write_text(&mut self, text: &str) -> Result<Written, ClipboardError> {
        self.write(&[Flavor::text(text)])
    }

    /// Puts a bitmap on the clipboard and remembers its [`image_signature`].
    // pc: crates/ui-egui/src/app/clipboard.rs:13 export_os_clipboard
    pub fn write_image(&mut self, image: &ClipboardImage) -> Result<Written, ClipboardError> {
        if !image.is_whole() {
            return Err(ClipboardError::BadImage);
        }
        let written = self.write(&[rgba8_flavor(image)])?;
        if written == Written::Written {
            self.ours = Some(Ours::Image(image_signature(image.width, image.height, &image.rgba)));
        }
        Ok(written)
    }

    /// Replaces the clipboard with `flavors`, best first, unless fenced. What
    /// [`holds_ours`](Self::holds_ours) compares is the flavour the backend reports it wrote.
    pub fn write(&mut self, flavors: &[Flavor]) -> Result<Written, ClipboardError> {
        if self.synthetic {
            return Ok(Written::Fenced);
        }
        let wrote = self.backend.write(flavors)?;
        self.ours = flavors.get(wrote).map(|f| Ours::Flavor { mime: f.mime.to_string(), hash: hash_bytes(&f.data) });
        Ok(Written::Written)
    }

    /// Clipboard text; text that isn't UTF-8 is `None` (PhotoCraft `clipboard_text`).
    // pc: apps/photocraft/src/services.rs:611 clipboard_text
    pub fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        Ok(self.read(&[TEXT])?.and_then(|flavor| String::from_utf8(flavor.data).ok()))
    }

    /// The first of `mimes` the clipboard holds, within [`ClipboardLimits::text_bytes`] for text
    /// types and [`ClipboardLimits::image_bytes`] for the rest.
    pub fn read(&mut self, mimes: &[&str]) -> Result<Option<Flavor>, ClipboardError> {
        let limit = if mimes.iter().all(|mime| is_text(mime)) { self.limits.text_bytes } else { self.limits.image_bytes };
        let Some(flavor) = self.backend.read(mimes, limit)? else { return Ok(None) };
        let cap = if is_text(&flavor.mime) { self.limits.text_bytes } else { self.limits.image_bytes };
        if flavor.data.len() as u64 > cap {
            return Err(ClipboardError::TooLarge { limit: cap });
        }
        Ok(Some(flavor))
    }

    /// The clipboard's bitmap. An image that isn't [`ClipboardImage::is_whole`] reads as `None`.
    pub fn read_image(&mut self) -> Result<Option<ClipboardImage>, ClipboardError> {
        let limit = self.limits.image_bytes;
        let Some(image) = self.backend.read_image(limit)? else { return Ok(None) };
        if image.rgba.len() as u64 > limit {
            return Err(ClipboardError::TooLarge { limit });
        }
        Ok(Some(image).filter(ClipboardImage::is_whole))
    }

    /// Copied files, in the order the file manager listed them.
    pub fn read_files(&mut self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        let limit = self.limits.files;
        let Some(files) = self.backend.read_files(limit)? else { return Ok(None) };
        if files.len() > limit {
            return Err(ClipboardError::TooLarge { limit: limit as u64 });
        }
        Ok(Some(files).filter(|files| !files.is_empty()))
    }

    /// Does the clipboard hold one of `mimes`? For Paste's enabled state on the web and in
    /// tests; on the desktop a [`ClipboardProbe`] asks off the UI thread.
    pub fn has(&mut self, mimes: &[&str]) -> bool {
        self.backend.has(mimes)
    }

    /// Does the clipboard still hold what this app wrote last (nothing copied since, here or in
    /// another app)? Reads the clipboard's flavour of the type the backend wrote last and compares
    /// it with what was written (an image by [`image_signature`]); for a pasted image the app
    /// already read, [`is_ours`](Self::is_ours) answers without another read.
    // vectorcraft: crates/ui-egui/src/sysclip.rs:25 SystemClipboard::holds_ours
    pub fn holds_ours(&mut self) -> bool {
        match &self.ours {
            None => false,
            Some(Ours::Image(signature)) => {
                let signature = *signature;
                let limit = self.limits.image_bytes;
                matches!(self.backend.read_image(limit), Ok(Some(image)) if image_signature(image.width, image.height, &image.rgba) == signature)
            }
            Some(Ours::Flavor { mime, hash }) => {
                let (mime, hash) = (mime.clone(), *hash);
                let limit = if is_text(&mime) { self.limits.text_bytes } else { self.limits.image_bytes };
                matches!(self.backend.read(&[mime.as_str()], limit), Ok(Some(flavor)) if hash_bytes(&flavor.data) == hash)
            }
        }
    }

    /// Whether `image`, already read, is the one this app wrote last: PhotoCraft's check that a
    /// pasted bitmap is its own copy (paste the in-app clipboard with its layers) or another
    /// app's, without a second read.
    // pc: crates/ui-egui/src/app/clipboard.rs:69 import_os_clipboard (os_clip_sig == sig)
    pub fn is_ours(&self, image: &ClipboardImage) -> bool {
        matches!(self.ours, Some(Ours::Image(signature)) if signature == image_signature(image.width, image.height, &image.rgba))
    }

    /// A paste event's flavours for the backend ([`ClipboardBackend::offer`]); the web shell
    /// calls it with [`flavors_from_event`] before the paste command runs.
    pub fn offer_paste(&mut self, flavors: Vec<Flavor>) {
        self.backend.offer(flavors);
    }

    /// Forgets the last write's signature (the in-app copy was cleared).
    pub fn forget_ours(&mut self) {
        self.ours = None;
    }

    pub fn backend_mut(&mut self) -> &mut dyn ClipboardBackend {
        &mut *self.backend
    }
}

/// A cheap fingerprint of an image: its size and every 997th byte, rotated in (PhotoCraft
/// `clip_signature`). Two images with the same signature are taken to be the same copy.
// pc: crates/ui-egui/src/app/clipboard.rs:3 clip_signature
pub fn image_signature(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let mut signature = u64::from(width) << 32 | u64::from(height);
    for byte in rgba.iter().step_by(997) {
        signature = signature.rotate_left(5) ^ u64::from(*byte);
    }
    signature
}

/// How often the probe's worker looks at the clipboard while the UI wants to know: 250 ms.
pub const PROBE_INTERVAL: Duration = Duration::from_millis(250);
/// How long dropping the probe waits for the worker to let go of its handle: 250 ms.
pub const PROBE_STOP_WAIT: Duration = Duration::from_millis(250);

/// Whether Paste has something to take from the system clipboard, asked on a worker thread.
///
/// `has()` can block for seconds when the clipboard owner never answers (an unresponsive X11
/// client), so it never runs on the thread that draws the window: the worker owns a second
/// clipboard handle, looks every [`PROBE_INTERVAL`] while wanted (not at all otherwise), sends
/// its verdict when it changes and calls [`Wake::wake`], so the app runs a frame that reads it
/// with [`pasteable`](Self::pasteable). Dropping the probe stops the worker and waits
/// up to [`PROBE_STOP_WAIT`] for it to drop its handle (on X11 the last handle hands the copy to
/// the clipboard manager before exit).
pub struct ClipboardProbe {
    wants: Option<Sender<bool>>,
    wanted: bool,
    verdicts: Receiver<bool>,
    last: bool,
}

impl ClipboardProbe {
    /// Starts the worker with the backend `make` builds there, asking for `mimes`. `None` on wasm
    /// or when the thread can't be spawned: the app asks [`Clipboard::has`] in line instead.
    // vectorcraft: crates/ui-egui/src/clipboard_probe.rs:33 Probe::start
    pub fn start(make: Box<dyn FnOnce() -> Box<dyn ClipboardBackend> + Send>, mimes: Vec<&'static str>, wake: Wake) -> Option<Self> {
        if cfg!(target_arch = "wasm32") {
            return None;
        }
        let (wants, wanted_rx) = std::sync::mpsc::channel();
        let (verdict_tx, verdicts) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("craft-clipboard-probe".into())
            .spawn(move || {
                let mut backend = make();
                let (mut wanted, mut sent) = (false, None);
                loop {
                    if wanted {
                        // This thread may wait here for a slow owner; the UI's never does.
                        let verdict = backend.has(&mimes);
                        if sent != Some(verdict) {
                            if verdict_tx.send(verdict).is_err() {
                                break;
                            }
                            sent = Some(verdict);
                            wake.wake();
                        }
                    }
                    // Sleep until the next look (none while unwanted) or the UI's next word.
                    let next =
                        if wanted { wanted_rx.recv_timeout(PROBE_INTERVAL) } else { wanted_rx.recv().map_err(|_| RecvTimeoutError::Disconnected) };
                    match next {
                        Ok(w) => wanted = w,
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
                // The handle goes before the probe's drop stops waiting: on X11 the last handle
                // alive hands the app's copy to the clipboard manager, which must finish first.
                drop(backend);
                drop(verdict_tx);
            })
            .ok()?;
        Some(Self { wants: Some(wants), wanted: false, verdicts, last: false })
    }

    /// Paste can take from the system clipboard: `wanted` (nothing copied in the app, a document
    /// open) and the worker's latest verdict says so. Never blocks. `None` once the worker is
    /// gone.
    // vectorcraft: crates/ui-egui/src/clipboard_probe.rs:76 Probe::pasteable
    pub fn pasteable(&mut self, wanted: bool) -> Option<bool> {
        if wanted != self.wanted {
            self.wants.as_ref()?.send(wanted).ok()?;
            self.wanted = wanted;
        }
        loop {
            match self.verdicts.try_recv() {
                Ok(verdict) => self.last = verdict,
                Err(TryRecvError::Empty) => return Some(wanted && self.last),
                Err(TryRecvError::Disconnected) => return None,
            }
        }
    }
}

impl Drop for ClipboardProbe {
    // vectorcraft: crates/ui-egui/src/clipboard_probe.rs:94 Probe::drop
    fn drop(&mut self) {
        self.wants = None;
        let end = Instant::now() + PROBE_STOP_WAIT;
        while self.verdicts.recv_timeout(end.saturating_duration_since(Instant::now())).is_ok() {}
    }
}

/// The flavours an egui paste event carries: `Event::Paste` as [`TEXT`], `Event::PasteImage`
/// (the fork's) as an [`RGBA8`] flavour. `None` for other
/// events. With the `ui` feature.
#[cfg(feature = "ui")]
// pc: crates/ui-egui/src/input/shortcuts.rs:231 Event::PasteImage
pub fn flavors_from_event(event: &egui::Event) -> Option<Vec<Flavor>> {
    match event {
        egui::Event::Paste(text) => Some(vec![Flavor::text(text)]),
        egui::Event::PasteImage(image) => {
            let [width, height] = image.size;
            let rgba = image.pixels.iter().flat_map(|pixel| pixel.to_srgba_unmultiplied()).collect();
            Some(vec![rgba8_flavor(&ClipboardImage { width: width as u32, height: height as u32, rgba })])
        }
        _ => None,
    }
}

/// Writes the best of `flavors` through egui's output (the web's clipboard) and answers its
/// index: the first text flavour with `ctx.copy_text`, or the first bitmap with
/// `OutputCommand::CopyImage`, whichever comes first. One only: eframe's web code makes each
/// command its own `navigator.clipboard` write, so a second command would replace the first.
/// With the `ui` feature.
#[cfg(feature = "ui")]
// new: eframe's web integration owns navigator.clipboard (app_runner.rs: CopyText and CopyImage each write on their own)
pub fn copy_through_egui(ctx: &egui::Context, flavors: &[Flavor]) -> Result<usize, ClipboardError> {
    if flavors.is_empty() {
        return Ok(0);
    }
    for (index, flavor) in flavors.iter().enumerate() {
        if flavor.mime == TEXT {
            ctx.copy_text(String::from_utf8_lossy(&flavor.data).into_owned());
            return Ok(index);
        }
        if flavor.mime == RGBA8
            && let Some(image) = rgba8_image(&flavor.data).filter(ClipboardImage::is_whole)
        {
            ctx.copy_image(egui::ColorImage::from_rgba_unmultiplied([image.width as usize, image.height as usize], &image.rgba));
            return Ok(index);
        }
    }
    Err(ClipboardError::NoFormat)
}

/// Text types read under [`ClipboardLimits::text_bytes`]: plain, HTML and RTF text and SVG markup.
fn is_text(mime: &str) -> bool {
    mime.starts_with("text/") || mime == SVG
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// `image` as an [`RGBA8`] flavor.
pub(crate) fn rgba8_flavor(image: &ClipboardImage) -> Flavor {
    let mut data = Vec::with_capacity(8 + image.rgba.len());
    data.extend_from_slice(&image.width.to_le_bytes());
    data.extend_from_slice(&image.height.to_le_bytes());
    data.extend_from_slice(&image.rgba);
    Flavor::new(RGBA8, data)
}

/// The bitmap in an [`RGBA8`] flavor's bytes; `None` without the two-word header.
pub(crate) fn rgba8_image(data: &[u8]) -> Option<ClipboardImage> {
    let (header, rgba) = data.split_first_chunk::<8>()?;
    let width = u32::from_le_bytes(header[..4].try_into().ok()?);
    let height = u32::from_le_bytes(header[4..].try_into().ok()?);
    Some(ClipboardImage { width, height, rgba: rgba.to_vec() })
}

/// Paths from a `text/uri-list`: `file:` URIs only, percent-decoded; comments and other schemes
/// are skipped.
fn parse_uri_list(list: &str) -> Vec<PathBuf> {
    list.lines()
        .filter_map(|line| {
            let line = line.trim();
            let path = line.strip_prefix("file://")?;
            let path = path.strip_prefix("localhost").unwrap_or(path);
            let decoded = percent_decode(path);
            // `file:///C:/Users/x` is the drive path `C:/Users/x` on Windows.
            let decoded = match decoded.strip_prefix('/') {
                Some(drive) if cfg!(windows) && drive.as_bytes().get(1) == Some(&b':') => drive.to_owned(),
                _ => decoded,
            };
            path.starts_with('/').then(|| PathBuf::from(decoded))
        })
        .collect()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16).map(|d| d as u8);
        match (bytes[i], bytes.get(i + 1).copied().and_then(hex), bytes.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(high), Some(low)) => {
                out.push(high << 4 | low);
                i += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `files` as a `text/uri-list` (CRLF line ends, RFC 2483). With `windows`, paths are drive paths
/// and become `file:///C:/Users/…` (the drive kept, separators turned to `/`).
#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
pub(crate) fn uri_list(files: &[PathBuf], windows: bool) -> String {
    let mut list = String::new();
    for file in files {
        let path = file.to_string_lossy();
        let path = if windows { std::borrow::Cow::Owned(format!("/{}", path.replace('\\', "/"))) } else { path };
        list.push_str("file://");
        for &byte in path.as_bytes() {
            if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) || (windows && byte == b':') {
                list.push(char::from(byte));
            } else {
                list.push_str(&format!("%{byte:02X}"));
            }
        }
        list.push_str("\r\n");
    }
    list
}

#[cfg(all(feature = "clipboard", not(target_arch = "wasm32")))]
fn trim_cr(path: PathBuf) -> PathBuf {
    match path.to_str() {
        Some(text) if text.ends_with('\r') => PathBuf::from(text.trim_end_matches('\r')),
        _ => path,
    }
}

// The probe's backend lives on its worker.
const _: () = {
    const fn sendable<T: Send>() {}
    sendable::<EventClipboard>();
};
