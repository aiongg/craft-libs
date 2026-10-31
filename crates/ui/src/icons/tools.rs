//! [`ToolIcon`]: the tool icons every app shares, by tool, drawn from the suite's icon set so a
//! theme's icon pack restyles them with the rest.
//!
//! Each tool maps to an [`Icon`]. Tools whose icon isn't in `icons.toml` yet get a row there with
//! its base-set (Phosphor) file, named by meaning (`marquee_row`, `pen_add_anchor`), in the fill
//! task; until then the mapping is `todo!()`.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use super::Icon;

/// A tool, for its icon. Photoshop's tool set, in PhotoCraft's order; other apps use the same
/// variant for the same tool (VectorCraft's Pen is [`ToolIcon::Pen`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToolIcon {
    /// PhotoCraft's `move` icon.
    Move,
    /// PhotoCraft's `artboard` icon.
    Artboard,
    /// PhotoCraft's `square-dashed` icon.
    RectangularMarquee,
    /// PhotoCraft's `ellipse-dashed` icon.
    EllipticalMarquee,
    /// PhotoCraft's `marquee-row` icon.
    SingleRowMarquee,
    /// PhotoCraft's `marquee-column` icon.
    SingleColumnMarquee,
    /// PhotoCraft's `brush` icon.
    Brush,
    /// PhotoCraft's `pencil` icon.
    Pencil,
    /// PhotoCraft's `palette` icon.
    MixerBrush,
    /// PhotoCraft's `brush-replace` icon.
    ColorReplacement,
    /// PhotoCraft's `eraser` icon.
    Eraser,
    /// PhotoCraft's `eraser-background` icon.
    BackgroundEraser,
    /// PhotoCraft's `eraser-magic` icon.
    MagicEraser,
    /// PhotoCraft's `pipette` icon.
    Eyedropper,
    /// PhotoCraft's `sampler` icon.
    ColorSampler,
    /// PhotoCraft's `ruler` icon.
    Ruler,
    /// PhotoCraft's `message-square` icon.
    Note,
    /// PhotoCraft's `circle-dot` icon.
    Count,
    /// PhotoCraft's `lasso` icon.
    Lasso,
    /// PhotoCraft's `pentagon` icon.
    PolygonalLasso,
    /// PhotoCraft's `lasso-magnetic` icon.
    MagneticLasso,
    /// PhotoCraft's `wand-sparkles` icon.
    MagicWand,
    /// PhotoCraft's `crop` icon.
    Crop,
    /// PhotoCraft's `perspective-crop` icon.
    PerspectiveCrop,
    /// PhotoCraft's `slice-knife` icon.
    Slice,
    /// PhotoCraft's `slice-select` icon.
    SliceSelect,
    /// PhotoCraft's `frame-placeholder` icon.
    Frame,
    /// PhotoCraft's `gradient` icon.
    Gradient,
    /// PhotoCraft's `paint-bucket` icon.
    PaintBucket,
    /// PhotoCraft's `type` icon.
    HorizontalType,
    /// PhotoCraft's `type` icon.
    VerticalType,
    /// PhotoCraft's `type-mask` icon.
    HorizontalTypeMask,
    /// PhotoCraft's `type-mask-vertical` icon.
    VerticalTypeMask,
    /// PhotoCraft's `hand` icon.
    Hand,
    /// PhotoCraft's `compass` icon.
    RotateView,
    /// PhotoCraft's `zoom-in` icon.
    Zoom,
    /// PhotoCraft's `bandage` icon.
    SpotHealing,
    /// PhotoCraft's `bandage` icon.
    Healing,
    /// PhotoCraft's `bandage-sparkle` icon.
    Remove,
    /// PhotoCraft's `lasso-select` icon.
    Patch,
    /// PhotoCraft's `arrow-left-right` icon.
    ContentAwareMove,
    /// PhotoCraft's `eye` icon.
    RedEye,
    /// PhotoCraft's `stamp` icon.
    CloneStamp,
    /// PhotoCraft's `grid-3x3` icon.
    PatternStamp,
    /// PhotoCraft's `clock` icon.
    HistoryBrush,
    /// PhotoCraft's `sparkles` icon.
    ArtHistoryBrush,
    /// PhotoCraft's `droplet` icon.
    Blur,
    /// PhotoCraft's `triangle` icon.
    Sharpen,
    /// PhotoCraft's `pointer` icon.
    Smudge,
    /// PhotoCraft's `lollipop` icon.
    Dodge,
    /// PhotoCraft's `flame` icon.
    Burn,
    /// PhotoCraft's `cloud` icon.
    Sponge,
    /// PhotoCraft's `quick-selection` icon.
    QuickSelection,
    /// PhotoCraft's `square-dashed-mouse-pointer` icon.
    ObjectSelection,
    /// PhotoCraft's `pen-tool` icon.
    Pen,
    /// PhotoCraft's `pen-freeform` icon.
    FreeformPen,
    /// PhotoCraft's `pen-curvature` icon.
    CurvaturePen,
    /// PhotoCraft's `pen-add-anchor` icon.
    AddAnchorPoint,
    /// PhotoCraft's `pen-delete-anchor` icon.
    DeleteAnchorPoint,
    /// PhotoCraft's `convert-point` icon.
    ConvertPoint,
    /// PhotoCraft's `mouse-pointer-2` icon.
    PathSelection,
    /// PhotoCraft's `direct-select` icon.
    DirectSelection,
    /// PhotoCraft's `rectangle-horizontal` icon.
    Rectangle,
    /// PhotoCraft's `circle` icon.
    Ellipse,
    /// PhotoCraft's `triangle` icon.
    Triangle,
    /// PhotoCraft's `pentagon` icon.
    Polygon,
    /// PhotoCraft's `slash` icon.
    Line,
    /// PhotoCraft's `cloud` icon.
    CustomShape,
}

impl ToolIcon {
    /// Every tool, in this enum's order.
    pub const ALL: [ToolIcon; 68] = [
        ToolIcon::Move,
        ToolIcon::Artboard,
        ToolIcon::RectangularMarquee,
        ToolIcon::EllipticalMarquee,
        ToolIcon::SingleRowMarquee,
        ToolIcon::SingleColumnMarquee,
        ToolIcon::Brush,
        ToolIcon::Pencil,
        ToolIcon::MixerBrush,
        ToolIcon::ColorReplacement,
        ToolIcon::Eraser,
        ToolIcon::BackgroundEraser,
        ToolIcon::MagicEraser,
        ToolIcon::Eyedropper,
        ToolIcon::ColorSampler,
        ToolIcon::Ruler,
        ToolIcon::Note,
        ToolIcon::Count,
        ToolIcon::Lasso,
        ToolIcon::PolygonalLasso,
        ToolIcon::MagneticLasso,
        ToolIcon::MagicWand,
        ToolIcon::Crop,
        ToolIcon::PerspectiveCrop,
        ToolIcon::Slice,
        ToolIcon::SliceSelect,
        ToolIcon::Frame,
        ToolIcon::Gradient,
        ToolIcon::PaintBucket,
        ToolIcon::HorizontalType,
        ToolIcon::VerticalType,
        ToolIcon::HorizontalTypeMask,
        ToolIcon::VerticalTypeMask,
        ToolIcon::Hand,
        ToolIcon::RotateView,
        ToolIcon::Zoom,
        ToolIcon::SpotHealing,
        ToolIcon::Healing,
        ToolIcon::Remove,
        ToolIcon::Patch,
        ToolIcon::ContentAwareMove,
        ToolIcon::RedEye,
        ToolIcon::CloneStamp,
        ToolIcon::PatternStamp,
        ToolIcon::HistoryBrush,
        ToolIcon::ArtHistoryBrush,
        ToolIcon::Blur,
        ToolIcon::Sharpen,
        ToolIcon::Smudge,
        ToolIcon::Dodge,
        ToolIcon::Burn,
        ToolIcon::Sponge,
        ToolIcon::QuickSelection,
        ToolIcon::ObjectSelection,
        ToolIcon::Pen,
        ToolIcon::FreeformPen,
        ToolIcon::CurvaturePen,
        ToolIcon::AddAnchorPoint,
        ToolIcon::DeleteAnchorPoint,
        ToolIcon::ConvertPoint,
        ToolIcon::PathSelection,
        ToolIcon::DirectSelection,
        ToolIcon::Rectangle,
        ToolIcon::Ellipse,
        ToolIcon::Triangle,
        ToolIcon::Polygon,
        ToolIcon::Line,
        ToolIcon::CustomShape,
    ];

    /// The icon that draws the tool.
    // pc: crates/ui-egui/src/icons.rs:85 `tool_icon` (Lucide-style names; the suite's rows are
    // snake_case by meaning, mapped onto existing icons where one fits: Brush → `brush`, Hand →
    // `hand`, Crop → `crop`).
    pub fn icon(self) -> Icon {
        todo!()
    }

    /// The tool's id as the apps' tool commands spell it (`tool.rectangularMarquee` without the
    /// `tool.` prefix), for automation and the keymap catalog.
    pub fn id(self) -> &'static str {
        todo!()
    }

    /// The tool for an id, `None` for an unknown one.
    pub fn from_id(id: &str) -> Option<ToolIcon> {
        todo!()
    }
}
