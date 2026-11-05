//! Stories of the selection overlay.
//!
//! Every story draws `MockImage` (`ui::mock_image`) under the overlay of a fake `MaskSource`: the
//! picture's subject (the house, [`MockSubject`]), and for the large case a soft-edged disc in a
//! 40 000 × 30 000 document, which has no picture behind it (reading that many pixels is what the
//! story must not do). Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/selection-overlay/ants` | The subject at four zooms; the large mask at 12 %; a floating offset; a mapped (turned) outline; hidden |
//! | `composites/selection-overlay/preview` | The five preview modes; the Quick Mask colour, opacity and Color Indicates |

use craft_theme::Srgba;
use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, Surface};
use craft_ui::testing::canvas_story::{ViewArgs, document_over};
use craft_ui::typed_story;
use egui::{Response, Sense, Ui, vec2};

use super::{SelectionOverlay, SelectionOverlayLabels};
use crate::selection::{ColorIndicates, PreviewMode, QuickMaskLook};
use crate::trace::{MaskRect, MaskSource};
use crate::ui::mock_image::{MockImage, MockSubject};
use crate::ui::transform_preview::stories::grid;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SelectionOverlay"];

craft_ui::words! {
    /// The captions of the selection stories, English by default.
    #[non_exhaustive]
    pub(crate) struct SelectionStoryLabels {
        subject: "selectionStory.subject" = "Subject at {n}%";
        large: "selectionStory.large" = "Large mask at {n}%";
        floating: "selectionStory.floating" = "Floating piece, offset";
        mapped: "selectionStory.mapped" = "Mapped outline, turned";
        hidden: "selectionStory.hidden" = "Edges hidden";
        no_preview: "selectionStory.noPreview" = "No preview";
    }
}

/// The two stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/selection-overlay/ants", Level::Composites, "SelectionOverlay", "Ants", AntsArgs { turn: 20.0 }, show_ants)
            .on(Surface::Canvas),
        typed_story!(
            "composites/selection-overlay/preview",
            Level::Composites,
            "SelectionOverlay",
            "Preview",
            PreviewArgs { tint: Tint::Red, opacity: 50.0, indicates: Indicates::MaskedAreas },
            show_preview
        )
        .on(Surface::Canvas),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AntsArgs {
    /// The turn of the mapped outline, in degrees.
    #[arg(range = -90.0..=90.0)]
    turn: f64,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Tint {
    Red,
    Green,
    Blue,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Indicates {
    #[choice(name = "Masked Areas")]
    MaskedAreas,
    #[choice(name = "Selected Areas")]
    SelectedAreas,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PreviewArgs {
    /// The Quick Mask colour.
    tint: Tint,
    /// The Quick Mask opacity, in percent.
    #[arg(range = 0.0..=100.0)]
    opacity: f64,
    /// Color Indicates.
    indicates: Indicates,
}

/// The large case: a document of this size (document data, not a UI size) ...
const HUGE_SIZE: [u32; 2] = [40_000, 30_000];
/// ... with a disc of this radius and this edge width in its middle, in document pixels.
const HUGE_RADIUS: f64 = 1000.0;
const HUGE_EDGE: f64 = 24.0;
/// The view's zoom of the large case, in percent.
const HUGE_ZOOM: f64 = 12.0;
/// The subject cells: the id salt and the zoom in percent.
const ZOOMS: [(&str, f64); 4] = [("zoom-25", 25.0), ("zoom-50", 50.0), ("zoom-100", 100.0), ("zoom-200", 200.0)];
/// The floating piece's offset, in document pixels.
const FLOAT_OFFSET: [i32; 2] = [220, -140];

/// A soft-edged disc in the middle of a [`HUGE_SIZE`] document.
struct HugeDisc;

impl HugeDisc {
    fn centre() -> Point {
        Point::new(f64::from(HUGE_SIZE[0]) / 2.0, f64::from(HUGE_SIZE[1]) / 2.0)
    }
}

impl MaskSource for HugeDisc {
    fn size(&self) -> [u32; 2] {
        HUGE_SIZE
    }

    fn revision(&self) -> u64 {
        1
    }

    fn content_bounds(&self) -> MaskRect {
        let (c, reach) = (Self::centre(), HUGE_RADIUS + HUGE_EDGE);
        MaskRect::new((c.x - reach).floor() as i32, (c.y - reach).floor() as i32, (c.x + reach).ceil() as i32, (c.y + reach).ceil() as i32)
    }

    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]) {
        let c = Self::centre();
        for (i, value) in out.iter_mut().enumerate() {
            let x = f64::from(x0) + 0.5 + i as f64 * f64::from(step);
            let distance = (x - c.x).hypot(f64::from(y) + 0.5 - c.y) - HUGE_RADIUS;
            let t = (0.5 - distance / HUGE_EDGE).clamp(0.0, 1.0);
            *value = (t * t * (3.0 - 2.0 * t)) as f32;
        }
    }
}

/// The middle of the subject's bounds.
fn subject_centre(image: &MockImage) -> Point {
    let bounds = image.subject_bounds();
    Point::new(f64::from(bounds.x0 + bounds.x1) / 2.0, f64::from(bounds.y0 + bounds.y1) / 2.0)
}

/// A caption and the picture under it, with the overlay drawn by `overlay` over the canvas.
fn cell(
    ui: &mut Ui,
    caption: &str,
    salt: &'static str,
    image: &MockImage,
    zoom: Option<f64>,
    centre: Option<Point>,
    overlay: impl FnOnce(&mut Ui, &Response, &CanvasView),
) {
    ui.add(Text::new(caption).content(Content::Muted));
    let canvas = document_over(ui, ViewArgs::default(), salt, 0.0, image.view(zoom, centre));
    image.show(ui, &canvas);
    overlay(ui, &canvas.response, &canvas.view);
}

fn percent(numbers: &LocaleNumbers, n: f64) -> String {
    numbers.format(n, 0..=0)
}

fn caption(text: &str, n: &str) -> String {
    craft_i18n::fmt(text, &[("n", n)])
}

fn show_ants(ui: &mut Ui, args: &mut AntsArgs) {
    let labels = SelectionStoryLabels::of(ui);
    let overlay_labels = SelectionOverlayLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let image = MockImage::default();
    let subject = MockSubject(image);
    let centre = subject_centre(&image);
    let ants = |salt: &'static str| SelectionOverlay::new(("ants", salt), &subject).labels(overlay_labels.clone());

    // The subject at four zooms, centred on the house.
    for pair in ZOOMS.chunks(2) {
        ui.columns(2, |columns| {
            for (column, &(salt, zoom)) in columns.iter_mut().zip(pair) {
                let text = caption(&labels.subject, &percent(&numbers, zoom));
                cell(column, &text, salt, &image, Some(zoom), Some(centre), |ui, response, view| {
                    ants(salt).show(ui, response, view);
                });
            }
        });
    }

    ui.columns(2, |columns| {
        // The large mask: a flat document, since the case is about the mask's size.
        let text = caption(&labels.large, &percent(&numbers, HUGE_ZOOM));
        columns[0].add(Text::new(text.as_str()).content(Content::Muted));
        let width = columns[0].available_width();
        let (rect, response) = columns[0].allocate_exact_size(vec2(width, width * HUGE_SIZE[1] as f32 / HUGE_SIZE[0] as f32), Sense::hover());
        let ppp = columns[0].pixels_per_point();
        let view = CanvasView::new(rect, HugeDisc::centre(), CanvasView::points_per_unit(HUGE_ZOOM / 100.0, ppp));
        let document = Rect::new(0.0, 0.0, f64::from(HUGE_SIZE[0]), f64::from(HUGE_SIZE[1]));
        let fill = columns[0].visuals().faint_bg_color;
        columns[0].painter_at(rect).add(egui::Shape::convex_polygon(view.quad(document).to_vec(), fill, egui::Stroke::NONE));
        SelectionOverlay::new(("ants", "large"), &HugeDisc).labels(overlay_labels.clone()).show(&mut columns[0], &response, &view);

        // A floating piece: the selection at an offset.
        cell(&mut columns[1], &labels.floating, "floating", &image, None, None, |ui, response, view| {
            ants("floating").offset(FLOAT_OFFSET).show(ui, response, view);
        });
    });

    ui.columns(2, |columns| {
        // A Free Transform in progress: the outline through a point map, the picture as it was.
        let (sin, cos) = args.turn.to_radians().sin_cos();
        let turned = move |p: Point| {
            let (dx, dy) = (p.x - centre.x, p.y - centre.y);
            Point::new(centre.x + dx * cos - dy * sin, centre.y + dx * sin + dy * cos)
        };
        cell(&mut columns[0], &labels.mapped, "mapped", &image, None, None, |ui, response, view| {
            ants("mapped").mapped(&turned).show(ui, response, view);
        });
        cell(&mut columns[1], &labels.hidden, "hidden", &image, None, None, |ui, response, view| {
            ants("hidden").visible(false).show(ui, response, view);
        });
    });
}

fn mode_caption(mode: PreviewMode, labels: &SelectionStoryLabels, names: &SelectionOverlayLabels) -> String {
    match mode {
        PreviewMode::None => labels.no_preview.clone(),
        PreviewMode::Grayscale => names.grayscale.clone(),
        PreviewMode::BlackMatte => names.black_matte.clone(),
        PreviewMode::WhiteMatte => names.white_matte.clone(),
        PreviewMode::QuickMask => names.quick_mask.clone(),
    }
}

fn show_preview(ui: &mut Ui, args: &mut PreviewArgs) {
    let labels = SelectionStoryLabels::of(ui);
    let names = SelectionOverlayLabels::of(ui);
    let image = MockImage::default();
    let subject = MockSubject(image);
    let color = match args.tint {
        Tint::Red => Srgba::rgb(1.0, 0.0, 0.0),
        Tint::Green => Srgba::rgb(0.0, 0.8, 0.2),
        Tint::Blue => Srgba::rgb(0.1, 0.3, 1.0),
    };
    let indicates = match args.indicates {
        Indicates::MaskedAreas => ColorIndicates::MaskedAreas,
        Indicates::SelectedAreas => ColorIndicates::SelectedAreas,
    };
    let look = QuickMaskLook { color, opacity: (args.opacity / 100.0) as f32, indicates };
    // The look applies to the Quick Mask mode only.
    grid(ui, PreviewMode::ALL.to_vec(), |column, mode| {
        let text = mode_caption(mode, &labels, &names);
        cell(column, &text, mode.id(), &image, None, None, |ui, response, view| {
            SelectionOverlay::new(("preview", mode.id()), &subject).labels(names.clone()).preview(mode, look).show(ui, response, view);
        });
    });
}
