//! `ToolIcon`: the tool icons every app shares, by tool, drawn from the SVG set so a theme's icon pack restyles
//! them with the rest.
//!
//! Each tool maps to an icon named by meaning (`marquee_row`, `pen_add_anchor`); the base set has a file for
//! every one.

use craft_behavior::IconId;

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
    pub fn icon(self) -> IconId {
        match self {
            ToolIcon::Move => IconId::new("move"),
            ToolIcon::Artboard => IconId::new("artboard"),
            ToolIcon::RectangularMarquee => IconId::new("marquee_rect"),
            ToolIcon::EllipticalMarquee => IconId::new("ellipse_dashed"),
            ToolIcon::SingleRowMarquee => IconId::new("marquee_row"),
            ToolIcon::SingleColumnMarquee => IconId::new("marquee_column"),
            ToolIcon::Brush => IconId::new("brush"),
            ToolIcon::Pencil => IconId::new("pencil"),
            ToolIcon::MixerBrush => IconId::new("palette"),
            ToolIcon::ColorReplacement => IconId::new("brush_replace"),
            ToolIcon::Eraser => IconId::new("eraser"),
            ToolIcon::BackgroundEraser => IconId::new("eraser_background"),
            ToolIcon::MagicEraser => IconId::new("eraser_magic"),
            ToolIcon::Eyedropper => IconId::new("eyedropper"),
            ToolIcon::ColorSampler => IconId::new("sampler"),
            ToolIcon::Ruler => IconId::new("ruler"),
            ToolIcon::Note => IconId::new("note"),
            ToolIcon::Count => IconId::new("target"),
            ToolIcon::Lasso => IconId::new("lasso"),
            ToolIcon::PolygonalLasso => IconId::new("pentagon"),
            ToolIcon::MagneticLasso => IconId::new("lasso_magnetic"),
            ToolIcon::MagicWand => IconId::new("wand_sparkles"),
            ToolIcon::Crop => IconId::new("crop"),
            ToolIcon::PerspectiveCrop => IconId::new("perspective_crop"),
            ToolIcon::Slice => IconId::new("slice_knife"),
            ToolIcon::SliceSelect => IconId::new("slice_select"),
            ToolIcon::Frame => IconId::new("frame_placeholder"),
            ToolIcon::Gradient => IconId::new("gradient"),
            ToolIcon::PaintBucket => IconId::new("paint_bucket"),
            ToolIcon::HorizontalType => IconId::new("text"),
            ToolIcon::VerticalType => IconId::new("text"),
            ToolIcon::HorizontalTypeMask => IconId::new("type_mask"),
            ToolIcon::VerticalTypeMask => IconId::new("type_mask_vertical"),
            ToolIcon::Hand => IconId::new("hand"),
            ToolIcon::RotateView => IconId::new("compass"),
            ToolIcon::Zoom => IconId::new("zoom_in"),
            ToolIcon::SpotHealing => IconId::new("bandage"),
            ToolIcon::Healing => IconId::new("bandage"),
            ToolIcon::Remove => IconId::new("bandage_sparkle"),
            ToolIcon::Patch => IconId::new("lasso_select"),
            ToolIcon::ContentAwareMove => IconId::new("swap_horizontal"),
            ToolIcon::RedEye => IconId::new("eye"),
            ToolIcon::CloneStamp => IconId::new("stamp"),
            ToolIcon::PatternStamp => IconId::new("grid"),
            ToolIcon::HistoryBrush => IconId::new("clock"),
            ToolIcon::ArtHistoryBrush => IconId::new("sparkles"),
            ToolIcon::Blur => IconId::new("droplet"),
            ToolIcon::Sharpen => IconId::new("triangle"),
            ToolIcon::Smudge => IconId::new("pointer"),
            ToolIcon::Dodge => IconId::new("lollipop"),
            ToolIcon::Burn => IconId::new("flame"),
            ToolIcon::Sponge => IconId::new("cloud"),
            ToolIcon::QuickSelection => IconId::new("quick_selection"),
            ToolIcon::ObjectSelection => IconId::new("object_select"),
            ToolIcon::Pen => IconId::new("pen_tool"),
            ToolIcon::FreeformPen => IconId::new("pen_freeform"),
            ToolIcon::CurvaturePen => IconId::new("pen_curvature"),
            ToolIcon::AddAnchorPoint => IconId::new("pen_add_anchor"),
            ToolIcon::DeleteAnchorPoint => IconId::new("pen_delete_anchor"),
            ToolIcon::ConvertPoint => IconId::new("convert_point"),
            ToolIcon::PathSelection => IconId::new("mouse_pointer_2"),
            ToolIcon::DirectSelection => IconId::new("direct_select"),
            ToolIcon::Rectangle => IconId::new("rectangle"),
            ToolIcon::Ellipse => IconId::new("circle"),
            ToolIcon::Triangle => IconId::new("triangle"),
            ToolIcon::Polygon => IconId::new("pentagon"),
            ToolIcon::Line => IconId::new("slash"),
            ToolIcon::CustomShape => IconId::new("cloud"),
        }
    }

    /// The tool's id as the apps' tool commands spell it (`tool.rectangularMarquee` without the
    /// `tool.` prefix), for automation and the keymap catalog.
    pub fn id(self) -> &'static str {
        match self {
            ToolIcon::Move => "move",
            ToolIcon::Artboard => "artboard",
            ToolIcon::RectangularMarquee => "rectangularMarquee",
            ToolIcon::EllipticalMarquee => "ellipticalMarquee",
            ToolIcon::SingleRowMarquee => "singleRowMarquee",
            ToolIcon::SingleColumnMarquee => "singleColumnMarquee",
            ToolIcon::Brush => "brush",
            ToolIcon::Pencil => "pencil",
            ToolIcon::MixerBrush => "mixerBrush",
            ToolIcon::ColorReplacement => "colorReplacement",
            ToolIcon::Eraser => "eraser",
            ToolIcon::BackgroundEraser => "backgroundEraser",
            ToolIcon::MagicEraser => "magicEraser",
            ToolIcon::Eyedropper => "eyedropper",
            ToolIcon::ColorSampler => "colorSampler",
            ToolIcon::Ruler => "ruler",
            ToolIcon::Note => "note",
            ToolIcon::Count => "count",
            ToolIcon::Lasso => "lasso",
            ToolIcon::PolygonalLasso => "polygonalLasso",
            ToolIcon::MagneticLasso => "magneticLasso",
            ToolIcon::MagicWand => "magicWand",
            ToolIcon::Crop => "crop",
            ToolIcon::PerspectiveCrop => "perspectiveCrop",
            ToolIcon::Slice => "slice",
            ToolIcon::SliceSelect => "sliceSelect",
            ToolIcon::Frame => "frame",
            ToolIcon::Gradient => "gradient",
            ToolIcon::PaintBucket => "paintBucket",
            ToolIcon::HorizontalType => "horizontalType",
            ToolIcon::VerticalType => "verticalType",
            ToolIcon::HorizontalTypeMask => "horizontalTypeMask",
            ToolIcon::VerticalTypeMask => "verticalTypeMask",
            ToolIcon::Hand => "hand",
            ToolIcon::RotateView => "rotateView",
            ToolIcon::Zoom => "zoom",
            ToolIcon::SpotHealing => "spotHealing",
            ToolIcon::Healing => "healing",
            ToolIcon::Remove => "remove",
            ToolIcon::Patch => "patch",
            ToolIcon::ContentAwareMove => "contentAwareMove",
            ToolIcon::RedEye => "redEye",
            ToolIcon::CloneStamp => "cloneStamp",
            ToolIcon::PatternStamp => "patternStamp",
            ToolIcon::HistoryBrush => "historyBrush",
            ToolIcon::ArtHistoryBrush => "artHistoryBrush",
            ToolIcon::Blur => "blur",
            ToolIcon::Sharpen => "sharpen",
            ToolIcon::Smudge => "smudge",
            ToolIcon::Dodge => "dodge",
            ToolIcon::Burn => "burn",
            ToolIcon::Sponge => "sponge",
            ToolIcon::QuickSelection => "quickSelection",
            ToolIcon::ObjectSelection => "objectSelection",
            ToolIcon::Pen => "pen",
            ToolIcon::FreeformPen => "freeformPen",
            ToolIcon::CurvaturePen => "curvaturePen",
            ToolIcon::AddAnchorPoint => "addAnchorPoint",
            ToolIcon::DeleteAnchorPoint => "deleteAnchorPoint",
            ToolIcon::ConvertPoint => "convertPoint",
            ToolIcon::PathSelection => "pathSelection",
            ToolIcon::DirectSelection => "directSelection",
            ToolIcon::Rectangle => "rectangle",
            ToolIcon::Ellipse => "ellipse",
            ToolIcon::Triangle => "triangle",
            ToolIcon::Polygon => "polygon",
            ToolIcon::Line => "line",
            ToolIcon::CustomShape => "customShape",
        }
    }

    /// The tool for an id, `None` for an unknown one.
    pub fn from_id(id: &str) -> Option<ToolIcon> {
        ToolIcon::ALL.into_iter().find(|tool| tool.id() == id)
    }
}
