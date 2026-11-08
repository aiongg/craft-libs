//! The component traits, context types, `PaintUi`, `Looks`, the pipeline and its step functions, the label, body and item steps, the mount and overlay hosts and `contract::tooltip`.

mod body;
mod collection;
mod container;
mod cx;
pub(crate) mod editor;
mod entry;
pub mod ghost;
mod group;
mod held;
mod items;
mod label;
mod looks;
mod mount;
pub mod overlay;
mod paint_ui;
mod props;
mod registry;
pub(crate) mod reorder;
mod run;
mod shell;
mod steps;
pub(crate) mod text;
pub mod toast;
pub mod tooltip;
mod values;

pub use crate::layout::{BodyLayout, RowAlign};
pub use crate::runtime::access::{Access, FieldLabel, GridPlace, Name, Numeric, Politeness, PopupKind, Role, SetPosition, SubNode, announce};
pub use crate::runtime::commands::{CommandSet, Commands};
pub use crate::runtime::keys::MenuKey;
pub use crate::runtime::memory::{Remember, Savable, SaveKey, Slot};
pub use crate::runtime::motion::{Animate, Interpolate, MotionSpec, Presence, Tween, spec};
pub use crate::runtime::pending::{Job, Pending, Progress};
pub(crate) use body::REORDER_BAND;
pub use collection::{
    ActivateOn, CollectionEvent, CollectionProps, CollectionResponse, ColumnAlign, ColumnEvent, ColumnKind, ColumnSort, ColumnSpec,
    ContextMenuRequest, DragPayload, HeaderMenuRequest, ItemKeys, LeadOnOpen, LeadSource, ListDrag, OpenChange, Rename, ReorderMove, ReorderRule,
    RowState, SelectMode, SpaceAction, ToggleColumn, ToggleEdit,
};
pub use container::{
    Collapse, ContainerProps, DragBand, EndRegion, FooterPlace, FooterRegion, Gap, HeaderRegion, Mount, Notch, Pad, PaneEdge, Scroll, ScrollAnchor,
    ScrollAxes, ShellPane, Sizing, ThemeScope,
};
pub use craft_ui_macros::{CommandSet, Component, Pattern, Props, register_contract};
pub(crate) use cx::SeenPress;
pub use cx::{Accessible, Activation, BodyCx, Cx, CxCore, Edit, Focus, ItemCx, MeasureCx, PartBuilder, PartsCx, Pointer, Styled, WordSetCx};
pub use entry::Entry;
pub use group::{GroupHandle, GroupKind, GroupState};
pub use held::Held;
pub use items::{ColumnCommands, request_rename};
pub(crate) use items::{lead_position, section_part};
pub use label::{CellWidth, FieldLabelRole, FieldLayout, GlyphLabel, LabelAlign, LabelKind, Scrub, SideLabel};
pub use looks::Looks;
pub use overlay::{Overlay, OverlayUi, Overlays, Popup, PopupHandle, Trigger};
pub use paint_ui::PaintUi;
pub(crate) use props::StopProps;
pub use props::{Common, Props, Resolved, Tooltip, TooltipAt, TooltipSpec, TooltipWhen};
#[doc(hidden)]
#[cfg(feature = "testing")]
pub use registry::REGISTRATIONS;
#[doc(hidden)]
pub use registry::{CollectionEntry, ContainerEntry, Exemption, LeafEntry, PatternProbe, Probe, Registration, ShowEntry};
pub(crate) use values::landed;
pub use values::{Bind, BindMut, EditResponse, ItemOutput, NoValue, Outcome, Validity, ValueOf, clicked};

use crate::runtime::overlay::CloseReason;
use crate::style::{ComponentStyle, Flags, StyleCx, StyleKey};
pub use craft_behavior::Input;
use craft_behavior::{ItemLayout, ItemSource, Proposal, ScrollPos, SelectionModel, Sense};
use egui::{Id, IdSalt, Rect, Ui, Vec2};
use run::ShowAt;
use std::hash::Hash;
use std::ops::Deref;

pub use crate::runtime::drag::{DragPreview, DropMark, DropTarget, OsFile, OsFiles, WindowDropTarget};
#[cfg(feature = "testing")]
pub(crate) use body::MOVE_BAND;
pub use craft_behavior::DropOperation;
pub(crate) use run::measure_under;
pub(crate) use steps::take_reveal;

/// Where an element's id comes from.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub enum IdSource {
    /// Derived from the parent scope and the order of its siblings.
    #[default]
    Auto,
    /// A salt added to the parent scope's id.
    Salt(IdSalt),
    /// The caller's id.
    Id(Id),
}

/// The shapes of component: which show trait applies.
pub mod shape {
    /// A component with no body and no items.
    #[derive(Clone, Copy, Debug)]
    pub struct Leaf;
    /// A component whose caller supplies a body closure.
    #[derive(Clone, Copy, Debug)]
    pub struct Body;
    /// A component that shows an item per visible entry of a source.
    #[derive(Clone, Copy, Debug)]
    pub struct Items;
}

mod sealed {
    pub trait ShowShape {}
    impl ShowShape for super::shape::Leaf {}
    impl ShowShape for super::shape::Body {}
    impl ShowShape for super::shape::Items {}

    pub trait Show {}
    pub trait ShowContainer {}
    pub trait ShowCollection {}
}

/// The shape of a component, as a value: what a registration records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShapeKind {
    /// No body and no items.
    Leaf,
    /// The caller supplies a body.
    Body,
    /// An item per visible entry of a source.
    Items,
}

/// Which show trait applies, and the shape's default sense.
pub trait ShowShape: sealed::ShowShape {
    /// The shape as a value.
    const KIND: ShapeKind;
    /// Leaf `CLICK | FOCUS`; body empty; items `CLICK | DRAG | FOCUS | WHEEL | TAKE_FOCUS`.
    const DEFAULT_SENSE: Sense;
}

impl ShowShape for shape::Leaf {
    const KIND: ShapeKind = ShapeKind::Leaf;
    const DEFAULT_SENSE: Sense = Sense::CLICK.union(Sense::FOCUS);
}

impl ShowShape for shape::Body {
    const KIND: ShapeKind = ShapeKind::Body;
    const DEFAULT_SENSE: Sense = Sense::empty();
}

impl ShowShape for shape::Items {
    const KIND: ShapeKind = ShapeKind::Items;
    const DEFAULT_SENSE: Sense = Sense::CLICK.union(Sense::DRAG).union(Sense::FOCUS).union(Sense::WHEEL).union(Sense::TAKE_FOCUS);
}

/// A token only craft-ui can build: the private field and `pub(crate)` constructor let derive output in any
/// crate implement the accessors below, while only craft-ui can call them.
#[derive(Debug)]
pub struct Internal(());

impl Internal {
    pub(crate) const fn new() -> Self {
        Internal(())
    }
}

/// Written by `#[derive(Component)]`; never by hand.
pub trait Meta {
    /// The shape: leaf, body or items.
    type Shape: ShowShape;
    /// The instance inputs the style depends on.
    type StyleKey: StyleKey;
    /// A leaf the `Canvas` body shows (`#[component(canvas_control)]`): its default sense is empty, so a press away
    /// from its parts reaches the tool, and its type is a canvas control.
    const CANVAS_CONTROL: bool = false;

    /// The style key of this instance.
    fn style_key(&self) -> Self::StyleKey;
    #[doc(hidden)]
    fn common(&self, _: Internal) -> &Common;
    #[doc(hidden)]
    fn common_mut(&mut self, _: Internal) -> &mut Common;
}

/// Written by the derive from a `#[container]` field, for containers and collections.
pub trait HasProps {
    #[doc(hidden)]
    fn props(&self, _: Internal) -> &ContainerProps;
}

/// The container show output: the component's own output and the body's result.
#[derive(Debug)]
#[non_exhaustive]
pub struct Inner<O, R> {
    /// The component's own output.
    pub output: O,
    /// The body's result; `None` when the body did not run.
    pub inner: Option<R>,
}

impl<O, R> Deref for Inner<O, R> {
    type Target = O;

    fn deref(&self) -> &O {
        &self.output
    }
}

/// The most a component may measure on either side, in points.
pub const MAX_EXTENT: f32 = 32_768.0;

/// A type that only assembles components: a composition with no contract shape of its own (24.4).
///
/// A pattern has no pipeline, no style, no sense and no props of the contract. It shows components, and the
/// AccessKit node of its root container is the pattern's node. `#[derive(Pattern)]` writes this impl and registers
/// the type, so the components index lists it and the suite runs the cases that apply to what it shows: the
/// container and collection cases of its contents, and the role and name checks of every node. A type is a
/// `Component` or a `Pattern`, never both: a type with its own sensing, painting, popups or focus is a component.
pub trait Pattern {}

/// The core trait: every UI type is a `Component`.
pub trait Component: Meta + Sized {
    /// The AccessKit role.
    const ROLE: Role;
    /// Roles `access` may pick per instance.
    const ALT_ROLES: &'static [Role] = &[];
    /// How input reaches the component.
    const INPUT: Input = Input::Pointer;
    /// Whether the component hosts popups: `false` skips the overlay bookkeeping of steps 9, 10, 12, 15 and 16.
    const OVERLAYS: bool = true;
    /// Whether the component declares parts, a text part or an editor part: `false` skips the parts step (the parts
    /// hook, the measure view it reads and the memory of the last state) and leaves the sense step with no part
    /// regions. A shape that declares parts of its own (a body's scroll parts, an item layout) still runs its hook.
    const PARTS: bool = true;
    /// Whether the component or its parts can take key focus: `false` skips the stop registration of step 7, except
    /// for the roving group of a body the shape declares.
    const STOPS: bool = true;
    /// Keys the focused component keeps from egui and the shortcut dispatcher.
    const KEYS: crate::runtime::focus::KeyUse = crate::runtime::focus::KeyUse::of(Self::INPUT);
    /// Whether the component paints its label as visible text. `false` for a face that shows only an icon or a
    /// glyph: with no tooltip set, its label is then its tooltip, since the label is the only visible name the
    /// pointer can reach. A component whose answer varies per instance overrides `shows_label`.
    const SHOWS_LABEL: bool = true;

    /// The style type.
    type Style: ComponentStyle;
    /// `Response`, `EditResponse<T>` or a richer type.
    type Output: Outcome;

    /// Whether this instance paints its label as visible text; `SHOWS_LABEL` unless an instance prop decides, as
    /// `icon_only` does. It reads props only, never the paint record.
    fn shows_label(&self) -> bool {
        Self::SHOWS_LABEL
    }

    /// How far the component paints outside its rect by design, in points (a graph node's ports sit on its edge).
    /// Steps 13 and 14 clip to the rect grown by this much; the paint-bounds suite case reads it.
    fn paint_outset(_tokens: &crate::style::UiTokens) -> f32 {
        0.0
    }

    /// Pure function of the resolved context and the key.
    fn style(cx: &StyleCx<'_>, key: Self::StyleKey) -> Self::Style;

    /// Content size only; the pipeline adds padding and minimum size.
    fn measure(&mut self, _cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    /// Whether the component's parts or size depend on its value: a text commit that changes the value then runs the
    /// pass again, so the parts show the new value.
    fn parts_follow_value(&self) -> bool {
        false
    }

    /// The axis along which the component takes all the room the flow offers when that is more than it measured,
    /// as a container with `Sizing::Fill` does: a vertical rule fills its row's height.
    fn fills(&self) -> Option<craft_behavior::Axis> {
        None
    }

    /// The root's sense; the shape's default unless a canvas control.
    fn sense(&self) -> Sense {
        if Self::CANVAS_CONTROL { Sense::empty() } else { <Self::Shape as ShowShape>::DEFAULT_SENSE }
    }

    /// Named sub-rects: thumbs, chevrons, handles, segments and the roving groups over them.
    fn parts(&mut self, _rect: Rect, _cx: &mut PartsCx<'_, Self>) {}

    /// Value components return their binding; the same binding on every call, with no side effects.
    fn bind(&mut self) -> Option<BindMut<'_, ValueOf<Self>>> {
        None
    }

    /// Lets a drag on the field's side label change the value; the pipeline runs the gesture (16.3, B2).
    fn scrub(&self) -> Option<Scrub<ValueOf<Self>>> {
        None
    }

    /// The scroll offset's owner when the app drives scrolling; `None` keeps it in session memory.
    fn scroll(&mut self) -> Option<BindMut<'_, ScrollPos>> {
        None
    }

    /// App values for content the component does not build: its body or its items. Runs once, at step 13.
    fn content_env(&self, _cx: &Cx<'_, Self>, _values: &mut crate::runtime::env::AppValues) {}

    /// After sensing, before paint: value maths, memory, popup triggers.
    fn update(&mut self, _cx: &mut Cx<'_, Self>) {}

    /// Anchored popups. Runs before flags and paint.
    fn overlay(&mut self, _ui: &mut OverlayUi<'_>, _cx: &mut Cx<'_, Self>) {}

    /// Value-driven flags (checked, open, invalid, busy).
    fn flags(&self) -> Flags {
        Flags::empty()
    }

    /// When a bound command fires; the default is a primary click or `Intent::Activate`.
    fn activated(&self, cx: &Cx<'_, Self>) -> bool {
        cx.activation().is_some()
    }

    /// Content only; surface, state fill and focus ring are painted by the pipeline.
    fn paint(&mut self, _ui: &mut PaintUi<'_>, _cx: &Cx<'_, Self>) {}

    /// Content above the body or items.
    fn paint_over(&mut self, _ui: &mut PaintUi<'_>, _cx: &Cx<'_, Self>) {}

    /// Value, range, relations, sub-nodes and custom actions beyond role, label and flags.
    fn access(&self, _access: &mut Access, _cx: &Cx<'_, Self>) {}

    /// Builds the output from the finished context.
    fn finish(self, cx: Cx<'_, Self>) -> Self::Output;
}

/// Implemented by the allowlisted components that paint content with a raw painter (a preview, a waveform).
/// `PaintUi::content_painter` takes no other component.
pub trait ContentPainter: Component {}

/// A component whose caller supplies a body.
pub trait Container: Component + HasProps + Meta<Shape = shape::Body> {
    /// The open state's owner: the caller's binding. `None`: a session slot under the id.
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        None
    }

    /// Fills the header region the body step laid out.
    fn header(&mut self, _ui: &mut Ui, _cx: &mut BodyCx<'_>) {}

    /// Fills the header's trailing region.
    fn controls(&mut self, _ui: &mut Ui, _cx: &mut BodyCx<'_>) {}

    /// Fills the footer region.
    fn footer(&mut self, _ui: &mut Ui, _cx: &mut BodyCx<'_>) {}

    /// Fills the end region, after the body.
    fn end(&mut self, _ui: &mut Ui, _cx: &mut BodyCx<'_>) {}

    /// The container closed, and why: a layer's dismissal rule, a close the body or footer asked for with
    /// `BodyCx::request_close_as`, `overlay::request_close`, or the OS closing its window. Called once for a close,
    /// at step 15 or, for a dismissal rule, right after the pass.
    fn closed(&mut self, _why: CloseReason) {}

    /// The user moved or resized the container's layer: by the move band, a grip or the keyboard Move and Size
    /// modes. Called for each step of the gesture, so a container whose layer the caller places (`Overlay::placed`)
    /// can take the rect it reports.
    fn placement_changed(&mut self, _change: &overlay::PlacementChange) {}
}

/// A component that shows an item per visible entry of a source.
pub trait Collection: Component + HasProps + Meta<Shape = shape::Items> {
    /// The item key.
    type Key: Clone + Eq + Hash + Send + Sync + 'static;
    /// Kept in session memory so step 8 can hit-test against last frame's layout.
    type ItemLayout: ItemLayout + Clone + Send + Sync + 'static;
    /// The selection model.
    type Selection: SelectionModel<Self::Key>;
    /// The item component.
    type Item<'r>: Component<Output: ItemOutput<Self::ItemEvent>> + Meta<Shape = shape::Leaf>;
    /// Events an item reports.
    type ItemEvent: 'static;

    /// Plain data: selection mode, reorder, rename, columns.
    fn items(&self) -> &CollectionProps;

    /// The selection binding; `None` keeps it in session memory.
    fn selection(&mut self) -> Option<BindMut<'_, Self::Selection>> {
        None
    }

    /// A one-shot request to scroll an item into view.
    fn reveal(&self) -> Option<&Self::Key> {
        None
    }

    /// The layout of `source` at `width`.
    fn item_layout(&self, source: &dyn ItemSource<Self::Key>, width: f32, cx: &StyleCx<'_>) -> Self::ItemLayout;

    /// The layout of `source` at `width`, given the layout of the last pass. An implementor may reuse `previous`
    /// when the width and the rows stamp are the same and the source's revision has not moved; the default builds
    /// the layout anew.
    fn item_layout_from(
        &self,
        _previous: Option<Self::ItemLayout>,
        source: &dyn ItemSource<Self::Key>,
        width: f32,
        cx: &StyleCx<'_>,
    ) -> Self::ItemLayout {
        self.item_layout(source, width, cx)
    }

    /// A view of `source` for this pass, such as the rows a filter keeps; `None` shows `source` as it is.
    fn filtered<'s>(&self, _source: &'s dyn ItemSource<Self::Key>, _env: &crate::runtime::env::Env) -> Option<Box<dyn ItemSource<Self::Key> + 's>> {
        None
    }

    /// Adjusts the item the closure built for a row, from the row's state.
    fn dress<'r>(&self, item: Self::Item<'r>, _row: &RowState) -> Self::Item<'r> {
        item
    }

    /// The rename editor at `label`: the label rect the renaming item reports (`ItemOutput::label_rect`), else its row.
    /// `name` is the item's name before the rename; the editor is named after it.
    fn rename_editor(&mut self, _ui: &mut Ui, _draft: Bind<'_, String>, _label: Rect, _name: &str) -> Option<EditResponse<String>> {
        None
    }

    /// Whether the item `key` may be renamed; a rename request on another item does nothing.
    fn can_rename(&self, _key: &Self::Key) -> bool {
        true
    }

    /// Whether the item `key` may be dragged (reordered, moved in a free layout, or dragged to another collection).
    fn can_drag(&self, _key: &Self::Key) -> bool {
        true
    }

    /// Whether a drag from the collection `from` may drop here.
    fn accept_from(&self, _from: Id) -> bool {
        true
    }

    /// Whether the dragged items may land before `insert` (a position of the display order, `over` is the item the
    /// pointer is on). A refused spot shows no drop mark.
    fn accepts(&self, _drag: &ListDrag<Self::Key>, _over: &Self::Key, _insert: usize) -> bool {
        true
    }

    /// Whether the collection takes files dragged in from the OS: a drop reports `CollectionEvent::FilesDropped` at the
    /// insert position under the pointer, with the same drop mark an internal drag gets.
    fn takes_files(&self) -> bool {
        false
    }

    /// The height of the empty state at `width`: a `Sizing::Content` collection with no rows is this tall.
    fn empty_height(&self, _width: f32, _cx: &StyleCx<'_>) -> f32 {
        0.0
    }

    /// The empty state at the rect the item step placed.
    fn empty(&mut self, _ui: &mut Ui, _cx: &mut BodyCx<'_>) {}
}

/// Shows a leaf.
pub trait Show: Component + Meta<Shape = shape::Leaf> + sealed::Show {
    /// Shows the component in the flow of `ui`.
    fn show(self, ui: &mut Ui) -> Self::Output {
        run::show_leaf(self, ui, ShowAt::Flow, None)
    }

    /// Shows the component at `rect`, with no allocation.
    fn show_at(self, ui: &mut Ui, rect: Rect) -> Self::Output {
        run::show_leaf(self, ui, ShowAt::Rect(rect), None)
    }

    /// Shows the component with rich tooltip content from the same call.
    fn show_with_tip(self, ui: &mut Ui, tip: impl FnOnce(&mut Ui)) -> Self::Output {
        run::show_leaf(self, ui, ShowAt::Flow, Some(Box::new(tip)))
    }

    /// Shows the component at `rect` with rich tooltip content.
    fn show_at_with_tip(self, ui: &mut Ui, rect: Rect, tip: impl FnOnce(&mut Ui)) -> Self::Output {
        run::show_leaf(self, ui, ShowAt::Rect(rect), Some(Box::new(tip)))
    }

    /// Measures without allocating: steps 1 to 5 under the id `show` would use next. It registers no id, opens no
    /// memory stage and reads memory through a peek that moves no slot.
    fn measure_in(&mut self, ui: &Ui, proposal: Proposal) -> Vec2 {
        run::measure_under(self, ui, None, proposal, None)
    }
}

/// Shows a container with its body.
pub trait ShowContainer: Container + sealed::ShowContainer {
    /// Shows the container in the flow of `ui`.
    fn show<R>(self, ui: &mut Ui, body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Inner<Self::Output, R> {
        run::show_container(self, ui, ShowAt::Flow, body, None)
    }

    /// Shows the container at `rect`, with no allocation: a frame over a split or a layer the parent places.
    fn show_at<R>(self, ui: &mut Ui, rect: Rect, body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Inner<Self::Output, R> {
        run::show_container(self, ui, ShowAt::Rect(rect), body, None)
    }

    /// Shows the container at `rect` and takes that rect's room in the flow of `ui`: what is shown after it lies
    /// past it, as after [`show`](Self::show).
    fn show_at_taking_room<R>(self, ui: &mut Ui, rect: Rect, body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Inner<Self::Output, R> {
        run::show_container_taking(self, ui, rect, body)
    }

    /// Shows the container with rich tooltip content.
    fn show_with_tip<R>(self, ui: &mut Ui, body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R, tip: impl FnOnce(&mut Ui)) -> Inner<Self::Output, R> {
        run::show_container(self, ui, ShowAt::Flow, body, Some(Box::new(tip)))
    }

    /// Shows the container at `rect` with rich tooltip content.
    fn show_at_with_tip<R>(
        self,
        ui: &mut Ui,
        rect: Rect,
        body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R,
        tip: impl FnOnce(&mut Ui),
    ) -> Inner<Self::Output, R> {
        run::show_container(self, ui, ShowAt::Rect(rect), body, Some(Box::new(tip)))
    }
}

/// Shows a collection's items.
pub trait ShowCollection: Collection + sealed::ShowCollection {
    /// Shows the visible items of `source`, one `item` call each.
    fn show_items<'r>(self, ui: &mut Ui, source: &dyn ItemSource<Self::Key>, item: &mut ItemFn<'r, Self>) -> Self::Output {
        run::show_items(self, ui, source, item)
    }
}

impl<C: Component + Meta<Shape = shape::Leaf>> sealed::Show for C {}
impl<C: Container> sealed::ShowContainer for C {}
impl<C: Collection> sealed::ShowCollection for C {}
impl<C: Component + Meta<Shape = shape::Leaf>> Show for C {}
impl<C: Container> ShowContainer for C {}
impl<C: Collection> ShowCollection for C {}

/// The item closure of `ShowCollection::show_items`: one call per visible key.
pub type ItemFn<'r, C> = dyn FnMut(&<C as Collection>::Key, &mut ItemCx<'_, 'r, <C as Collection>::Key>) -> <C as Collection>::Item<'r> + 'r;

/// A tooltip closure the pipeline runs at step 16.
pub(crate) type TipFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;
