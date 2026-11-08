//! What the mock editor reads and edits: a tool, a layer list, a canvas view, a document title and a dirty flag.

use craft_behavior::CanvasView;
use craft_behavior::dock::{Column, DockEdge, Group, Layout, Node, SplitSize, Workspace};
use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{Axis, PixelSize};
use craft_ui::contract::CommandSet;
use craft_ui::patterns::form_dialogs::ImageSizeValues;
use craft_ui::rails::{LastUsed, RailTool, SlotId, ToolSlot};
use egui::Rect;

/// The width of the dock column at the end edge.
const SIDE_WIDTH: f32 = 220.0;

/// The width of the mock document, in document units.
const DOCUMENT_WIDTH: f64 = 1200.0;

/// The height of the mock document, in document units.
const DOCUMENT_HEIGHT: f64 = 800.0;

/// The share of the dock the canvas group takes.
const CANVAS_SHARE: f32 = 0.8;

/// The tools of the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CommandSet)]
#[command(prefix = "mockTool")]
#[non_exhaustive]
pub enum Tool {
    /// Move.
    #[command(label = "Move", icon = "move")]
    Move,
    /// Brush.
    #[command(label = "Brush", icon = "brush")]
    Brush,
    /// Pencil.
    #[command(label = "Pencil", icon = "pencil")]
    Pencil,
    /// Eraser.
    #[command(label = "Eraser", icon = "eraser")]
    Eraser,
    /// Clone Stamp.
    #[command(label = "Clone Stamp", icon = "stamp")]
    Stamp,
    /// Hand.
    #[command(label = "Hand", icon = "hand")]
    Hand,
    /// Zoom.
    #[command(label = "Zoom", icon = "zoom_in")]
    Zoom,
}

impl Tool {
    /// The name the status bar shows.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Move => "Move",
            Tool::Brush => "Brush",
            Tool::Pencil => "Pencil",
            Tool::Eraser => "Eraser",
            Tool::Stamp => "Clone Stamp",
            Tool::Hand => "Hand",
            Tool::Zoom => "Zoom",
        }
    }

    /// The rail's slots: Move alone, then the freehand tools that share a slot.
    pub fn slots() -> Vec<ToolSlot<Tool>> {
        let tool = |tool: Tool| RailTool::new(tool, tool);
        vec![
            ToolSlot::new(0, vec![tool(Tool::Move)]),
            ToolSlot::new(1, vec![tool(Tool::Brush), tool(Tool::Pencil)]),
            ToolSlot::new(2, vec![tool(Tool::Eraser)]),
            ToolSlot::new(3, vec![tool(Tool::Stamp)]),
            ToolSlot::new(4, vec![tool(Tool::Hand)]),
            ToolSlot::new(5, vec![tool(Tool::Zoom)]),
        ]
    }

    /// The slot that holds the tool.
    pub fn slot(self) -> SlotId {
        match self {
            Tool::Move => 0,
            Tool::Brush | Tool::Pencil => 1,
            Tool::Eraser => 2,
            Tool::Stamp => 3,
            Tool::Hand => 4,
            Tool::Zoom => 5,
        }
    }
}

/// The panels of the dock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Panel {
    /// The document tabs over the canvas.
    Canvas,
    /// The Navigator.
    Navigator,
    /// The layer list.
    Layers,
}

/// One layer of the document.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Layer {
    /// The name the layers panel shows; distinct within a document.
    pub name: String,
    /// Whether the layer shows on the canvas.
    pub visible: bool,
}

impl Layer {
    /// A visible layer.
    pub fn new(name: impl Into<String>) -> Self {
        Layer { name: name.into(), visible: true }
    }
}

/// A dialog the menus ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    /// The Gaussian Blur parameter dialog.
    GaussianBlur,
    /// The Image Size dialog.
    ImageSize,
    /// The Delete Layer question.
    DeleteLayer,
}

/// The open document.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Document {
    /// The title of the window and the tab.
    pub title: String,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
    /// The size in document units.
    pub size: Size,
    /// The layers, bottom first.
    pub layers: Vec<Layer>,
    /// The index of the current layer.
    pub current: usize,
    /// The canvas view.
    pub view: CanvasView,
    /// The ruler zero point.
    pub zero: Point,
    /// The part of the canvas inside the rulers, as the last pass showed it.
    pub content: Option<Rect>,
}

impl Default for Document {
    fn default() -> Self {
        let size = Size::new(DOCUMENT_WIDTH, DOCUMENT_HEIGHT);
        let view = CanvasView::new(Rect::NOTHING, Point::new(size.width / 2.0, size.height / 2.0), 1.0);
        let layers = vec![Layer::new("Background"), Layer::new("Sky"), Layer::new("Mountains")];
        Document { title: "Landscape.psd".to_owned(), dirty: false, size, current: layers.len() - 1, layers, view, zero: Point::ZERO, content: None }
    }
}

impl Document {
    /// The current layer.
    pub fn current_layer(&self) -> Option<&Layer> {
        self.layers.get(self.current)
    }

    /// Shows or hides the layer at `at`, which marks the document changed.
    pub fn set_visible(&mut self, at: usize, visible: bool) {
        if let Some(layer) = self.layers.get_mut(at).filter(|layer| layer.visible != visible) {
            layer.visible = visible;
            self.dirty = true;
        }
    }

    /// Deletes the current layer, which marks the document changed. The last layer stays.
    pub fn delete_current(&mut self) -> Option<Layer> {
        if self.layers.len() < 2 || self.current >= self.layers.len() {
            return None;
        }
        let removed = self.layers.remove(self.current);
        self.current = self.current.min(self.layers.len() - 1);
        self.dirty = true;
        Some(removed)
    }
}

/// Everything the mock edits.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MockState {
    /// The dock's panels and where they sit.
    pub workspace: Workspace<Panel>,
    /// The open document.
    pub document: Document,
    /// The current tool.
    pub tool: Tool,
    /// The tool each rail slot showed last.
    pub last_used: LastUsed<Tool>,
    /// The dialog a menu asked for, until the window opens it.
    pub request: Option<Request>,
    /// The values the Image Size dialog edits.
    pub image_size: ImageSizeValues,
    /// The Image Size dialog is open.
    pub image_size_open: bool,
}

impl Default for MockState {
    fn default() -> Self {
        let side = Column::new(vec![Group::new(vec![Panel::Navigator], 0), Group::new(vec![Panel::Layers], 0)], SIDE_WIDTH);
        let root = Node::split(Axis::X, SplitSize::Ratio(CANVAS_SHARE), Node::tabs(vec![Panel::Canvas], 0), Node::dock(DockEdge::End, vec![side]));
        MockState {
            workspace: Workspace::new(Layout::new(Some(root), Vec::new())),
            document: Document::default(),
            tool: Tool::Move,
            last_used: LastUsed::default(),
            request: None,
            image_size: ImageSizeValues::new(PixelSize::new(1200, 800)),
            image_size_open: false,
        }
    }
}
