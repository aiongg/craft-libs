//! The tools and the values of their options bars.

use super::doc::Blend;

/// Every tool the rail lists. Only [`Tool::is_working`] ones can be chosen; the others fill the
/// flyouts and slots PhotoCraft has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tool {
    #[default]
    Move,
    Artboard,
    RectMarquee,
    EllipseMarquee,
    Lasso,
    PolygonLasso,
    MagneticLasso,
    Crop,
    PerspectiveCrop,
    Eyedropper,
    Ruler,
    Brush,
    Pencil,
    Eraser,
    BackgroundEraser,
    Gradient,
    PaintBucket,
    Pen,
    FreeformPen,
    Type,
    Hand,
    RotateView,
    Zoom,
}

impl Tool {
    /// The tools the mock app implements: those with an options bar and a key.
    pub const WORKING: [Tool; 8] =
        [Tool::Move, Tool::RectMarquee, Tool::Brush, Tool::Eraser, Tool::Gradient, Tool::Eyedropper, Tool::Hand, Tool::Zoom];

    pub fn is_working(self) -> bool {
        Self::WORKING.contains(&self)
    }

    pub fn label(self) -> &'static str {
        match self {
            Tool::Move => "Move",
            Tool::Artboard => "Artboard",
            Tool::RectMarquee => "Rectangular Marquee",
            Tool::EllipseMarquee => "Elliptical Marquee",
            Tool::Lasso => "Lasso",
            Tool::PolygonLasso => "Polygonal Lasso",
            Tool::MagneticLasso => "Magnetic Lasso",
            Tool::Crop => "Crop",
            Tool::PerspectiveCrop => "Perspective Crop",
            Tool::Eyedropper => "Eyedropper",
            Tool::Ruler => "Ruler",
            Tool::Brush => "Brush",
            Tool::Pencil => "Pencil",
            Tool::Eraser => "Eraser",
            Tool::BackgroundEraser => "Background Eraser",
            Tool::Gradient => "Gradient",
            Tool::PaintBucket => "Paint Bucket",
            Tool::Pen => "Pen",
            Tool::FreeformPen => "Freeform Pen",
            Tool::Type => "Type",
            Tool::Hand => "Hand",
            Tool::RotateView => "Rotate View",
            Tool::Zoom => "Zoom",
        }
    }

    /// The key that chooses the tool, as the rail's tooltips spell it.
    pub fn key(self) -> Option<char> {
        Some(match self {
            Tool::Move | Tool::Artboard => 'V',
            Tool::RectMarquee | Tool::EllipseMarquee => 'M',
            Tool::Lasso | Tool::PolygonLasso | Tool::MagneticLasso => 'L',
            Tool::Crop | Tool::PerspectiveCrop => 'C',
            Tool::Eyedropper | Tool::Ruler => 'I',
            Tool::Brush | Tool::Pencil => 'B',
            Tool::Eraser | Tool::BackgroundEraser => 'E',
            Tool::Gradient | Tool::PaintBucket => 'G',
            Tool::Pen | Tool::FreeformPen => 'P',
            Tool::Type => 'T',
            Tool::Hand | Tool::RotateView => 'H',
            Tool::Zoom => 'Z',
        })
    }

    /// The working tool a key chooses.
    pub fn from_key(key: char) -> Option<Tool> {
        Self::WORKING.into_iter().find(|t| t.key() == Some(key.to_ascii_uppercase()))
    }
}

/// What Move selects on a click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveTarget {
    #[default]
    Layer,
    Group,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveOptions {
    pub auto_select: bool,
    pub target: MoveTarget,
    pub transform: bool,
}

impl Default for MoveOptions {
    fn default() -> Self {
        Self { auto_select: false, target: MoveTarget::Layer, transform: false }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarqueeStyle {
    #[default]
    Normal,
    FixedRatio,
    FixedSize,
}

impl MarqueeStyle {
    pub const ALL: [MarqueeStyle; 3] = [MarqueeStyle::Normal, MarqueeStyle::FixedRatio, MarqueeStyle::FixedSize];

    pub fn label(self) -> &'static str {
        match self {
            MarqueeStyle::Normal => "Normal",
            MarqueeStyle::FixedRatio => "Fixed Ratio",
            MarqueeStyle::FixedSize => "Fixed Size",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarqueeOptions {
    pub style: MarqueeStyle,
    /// Pixels.
    pub feather: f64,
}

/// Percent values, as the options bar shows them.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushOptions {
    pub mode: Blend,
    pub opacity: f64,
    pub flow: f64,
    pub smoothing: f64,
}

impl Default for BrushOptions {
    fn default() -> Self {
        Self { mode: Blend::Normal, opacity: 100.0, flow: 100.0, smoothing: 10.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EraserOptions {
    pub opacity: f64,
    pub flow: f64,
}

impl Default for EraserOptions {
    fn default() -> Self {
        Self { opacity: 100.0, flow: 100.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientOptions {
    pub mode: Blend,
    pub opacity: f64,
    pub reverse: bool,
}

impl Default for GradientOptions {
    fn default() -> Self {
        Self { mode: Blend::Normal, opacity: 100.0, reverse: false }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SampleSize {
    #[default]
    Point,
    Average3,
    Average5,
}

impl SampleSize {
    pub const ALL: [SampleSize; 3] = [SampleSize::Point, SampleSize::Average3, SampleSize::Average5];

    pub fn label(self) -> &'static str {
        match self {
            SampleSize::Point => "Point Sample",
            SampleSize::Average3 => "3 by 3 Average",
            SampleSize::Average5 => "5 by 5 Average",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EyedropperOptions {
    pub sample: SampleSize,
}

/// The current tool and every tool's option values; they persist when the tool changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tools {
    pub current: Tool,
    pub move_: MoveOptions,
    pub marquee: MarqueeOptions,
    pub brush: BrushOptions,
    pub eraser: EraserOptions,
    pub gradient: GradientOptions,
    pub eyedropper: EyedropperOptions,
}
