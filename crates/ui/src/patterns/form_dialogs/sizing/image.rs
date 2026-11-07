//! `ImageSize`: the Image Size dialog.

use super::{Kit, LinkedSize, Resample, SizeForm, SizingOutput, edited, show_form};
use crate::choice::Checkbox;
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use crate::form::Field;
use crate::pickers::Select;
use crate::runtime::env::UnitKind;
use craft_behavior::PixelSize;
use egui::Ui;

/// What Image Size answers with at OK.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImageSizeResult {
    /// The new size in pixels.
    pub size: PixelSize,
    /// The new resolution, in pixels per inch.
    pub pixels_per_inch: f64,
    /// The unit the width and height showed at OK.
    pub unit: UnitKind,
    /// The unit the resolution showed at OK.
    pub resolution_unit: UnitKind,
    /// The method the pixels are made with; `None` when Resample is off and the pixel count stays.
    pub resample: Option<Resample>,
    /// Whether layer effects scale with the image.
    pub scale_styles: bool,
}

/// What Image Size reports: its response, how it closed and the [`ImageSizeResult`] at OK.
pub type ImageSizeOutput = SizingOutput<ImageSizeResult>;

/// The values Image Size edits.
#[derive(Clone, PartialEq)]
struct ImageState {
    size: LinkedSize,
    method: Resample,
    scale_styles: bool,
}

impl SizeForm for ImageState {
    fn pixels(&self) -> PixelSize {
        self.size.pixels()
    }
}

/// Image Size: the width, height and resolution with a unit each, the chain that keeps the proportions, Resample with
/// its method list and Scale Styles, over a readout of the file size and the dimensions.
///
/// - Keys: Tab walks the fields, Enter is OK unless a field owns it, Escape cancels. Holding Alt turns Cancel into
///   Reset, which restores the values from when the dialog opened and keeps the dialog open. Ctrl/Cmd+Z steps the
///   dialog's own settings.
/// - With Resample off the pixel count is fixed: the width and height in pixels or percent refuse input, and a physical
///   size changes the resolution.
/// - OK is refused while a side is under 1 or over `MAX_DOCUMENT_SIDE` pixels.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct ImageSize<'a> {
    size: PixelSize,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// The resolution the document opens with, in pixels per inch.
    #[prop(quantity = "pixels per inch")]
    pixels_per_inch: f64,
    /// The unit the width and height open in.
    #[prop]
    unit: UnitKind,
    /// The unit the resolution opens in.
    #[prop]
    resolution_unit: UnitKind,
    /// Whether Resample opens on; the default is on.
    #[prop]
    resampling: bool,
    /// The method Resample opens with, and returns to when it is turned on again.
    #[prop]
    method: Resample,
    /// The methods the list offers, in the order of [`Resample::ALL`]; all of them when empty.
    #[prop]
    methods: Vec<Resample>,
    /// Whether Scale Styles opens on.
    #[prop]
    scale_styles: bool,
    /// The bytes one pixel takes, for the file size readout.
    #[prop(quantity = "bytes per pixel")]
    bytes_per_pixel: f64,
    /// The points in an inch of the Points and Picas units: 72, or 72.27 for traditional points.
    #[prop(quantity = "points in an inch")]
    points_per_inch: f64,
}

impl<'a> ImageSize<'a> {
    /// The dialog for a document of `size` pixels, at 72 pixels per inch, in pixels, resampling automatically.
    pub fn new(size: PixelSize) -> Self {
        ImageSize {
            size,
            open: None,
            pixels_per_inch: 72.0,
            unit: UnitKind::Pixels,
            resolution_unit: UnitKind::PixelsPerInch,
            resampling: true,
            method: Resample::Automatic,
            methods: Vec::new(),
            scale_styles: true,
            bytes_per_pixel: 4.0,
            points_per_inch: UnitKind::POINTS_PER_INCH,
        }
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> ImageSizeOutput {
        let ImageSize {
            size,
            open,
            pixels_per_inch,
            unit,
            resolution_unit,
            resampling,
            method,
            methods,
            scale_styles,
            bytes_per_pixel,
            points_per_inch,
        } = self;
        let kit = Kit::new(ui);
        let entries = Resample::entries(kit.words(), &methods);
        let method = if entries.iter().any(|entry| entry.value() == Some(&method)) {
            method
        } else {
            entries.first().and_then(|entry| entry.value().copied()).unwrap_or(method)
        };

        let mut linked = LinkedSize::new(size).opening_resolution(pixels_per_inch).points_per_inch(points_per_inch);
        linked.set_unit(unit);
        linked.set_resolution_unit(resolution_unit);
        linked.resampling(resampling);
        let opening = ImageState { size: linked, method, scale_styles };

        let words = kit.words();
        let title = words.image_size_title.clone();
        show_form(
            ui,
            title,
            open,
            &opening,
            |ui, state| {
                let was = kit.file_size(size, bytes_per_pixel);
                let now = state.size.pixels();
                let tone = kit.tone_of(now);
                let shown = words.size_was.fill(&[("size", kit.file_size(now, bytes_per_pixel).as_str()), ("old", was.as_str())]);
                kit.readout(ui, &words.image_size, shown, tone);
                kit.readout(ui, &words.dimensions, kit.dimensions(now), tone);
                kit.refusal(ui, now);
                kit.sides(ui, &mut state.size);
                kit.resolution(ui, &mut state.size);

                let on = Checkbox::new(Bind::given(state.size.is_resampling())).label(words.resample.clone()).id_salt("resample").show(ui);
                if let Some(on) = edited(&on) {
                    state.size.resampling(on);
                }
                if state.size.is_resampling() {
                    Field::new(words.resample_method.clone()).id_salt("method").show(ui, |ui, _| {
                        let picked = Select::new(words.resample_method.clone(), Bind::given(state.method), &entries).show(ui);
                        if let Some(method) = edited(&picked) {
                            state.method = method;
                        }
                    });
                }
                let styles = Checkbox::new(Bind::given(state.scale_styles))
                    .label(words.scale_styles.clone())
                    .id_salt("scale-styles")
                    .enabled(state.size.is_linked())
                    .show(ui);
                if let Some(on) = edited(&styles) {
                    state.scale_styles = on;
                }
            },
            |state| ImageSizeResult {
                size: state.size.pixels(),
                pixels_per_inch: state.size.pixels_per_inch(),
                unit: state.size.unit(),
                resolution_unit: state.size.resolution_unit(),
                resample: state.size.is_resampling().then_some(state.method),
                scale_styles: state.scale_styles,
            },
        )
    }
}
