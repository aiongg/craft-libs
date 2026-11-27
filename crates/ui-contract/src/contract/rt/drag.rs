//! Drags on craft-ui-rt (docs/contract-rt.md section 10): the contract's drag sources and drop
//! targets on ui-rt's payload sessions (A2).
//!
//! A component names its payload and preview from its run ([`Component::drag_source`]); its node
//! keeps them, and when a press on it moves past slop the node starts ui-rt's session with them.
//! The preview is recorded once as a display list when the drag starts; ui-rt keeps it on the drag
//! layer and moves it with the pointer by translating it, so a move runs nothing. The source hears
//! the end in its next run ([`NodeInput::DragEnd`]); a target that takes the payload's type
//! ([`Component::drop_target`]) accepts it in ui-rt's routing and gets the payload in its next run
//! ([`NodeInput::Drop`]).
//!
//! [`Component::drag_source`]: super::Component::drag_source
//! [`Component::drop_target`]: super::Component::drop_target
//! [`NodeInput::DragEnd`]: super::NodeInput::DragEnd
//! [`NodeInput::Drop`]: super::NodeInput::Drop

use std::any::{Any, TypeId};
use std::sync::Arc;

use craft_theme::Tokens;
use craft_ui_rt::{DisplayList, DragData, DragGhost, DragStart, DropOperation, Item, Pos2, Rect, Stroke, Vec2};

use crate::runtime::drag::DragPreview;

/// A payload the contract drags: the value, and the extra values a drag carries beside it
/// (`Pointer::start_drag_also`). Shared so a target gets the payload as `Arc<P>`.
pub(crate) struct Payload {
    value: Arc<dyn Any + Send + Sync>,
    also: Vec<Arc<dyn Any + Send + Sync>>,
}

/// The value of type `payload` the drag `data` carries: its payload or one of the values beside it.
/// Drags from the OS carry no contract payload and answer `None`.
pub(crate) fn payload_of(data: &DragData, payload: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
    let carried = data.get::<Payload>()?;
    std::iter::once(&carried.value).chain(&carried.also).find(|value| (***value).type_id() == payload).cloned()
}

/// Whether the drag `data` carries a contract payload of type `payload`.
pub(crate) fn carries(data: &DragData, payload: TypeId) -> bool {
    payload_of(data, payload).is_some()
}

/// What a component drags: its payload and what shows beside the pointer.
#[derive(Clone)]
pub struct DragSource {
    /// What a target gets on the drop.
    pub payload: DragData,
    /// What shows beside the pointer.
    pub preview: DragPreview,
}

impl DragSource {
    /// Drags `payload` with `preview` beside the pointer.
    pub fn new<P: Send + Sync + 'static>(payload: P, preview: DragPreview) -> DragSource {
        DragSource::with_also(payload, Vec::new(), preview)
    }

    /// Drags `payload` and the values `also` beside it, which targets of their types take as well.
    pub fn with_also<P: Send + Sync + 'static>(payload: P, also: Vec<Arc<dyn Any + Send + Sync>>, preview: DragPreview) -> DragSource {
        DragSource { payload: DragData::new(Payload { value: Arc::new(payload), also }), preview }
    }

    /// The session ui-rt starts, with the preview recorded once against `tokens`.
    pub(crate) fn start(&self, tokens: &Tokens) -> DragStart {
        let start = DragStart { payload: self.payload.clone(), ghost: None, moving: None };
        match ghost(&self.preview, tokens) {
            Some(ghost) => start.ghost(ghost),
            None => start,
        }
    }
}

/// What a component takes as a drop: payloads of one type, with an operation.
#[derive(Clone, Copy, Debug)]
pub struct DropRule {
    accepts: fn(&DragData) -> bool,
    /// The operation the target offers.
    pub op: DropOperation,
}

fn is_type<P: 'static>(data: &DragData) -> bool {
    carries(data, TypeId::of::<P>()) || data.is::<P>()
}

impl DropRule {
    /// Takes payloads of type `P` with `op`.
    pub fn of<P: 'static>(op: DropOperation) -> DropRule {
        DropRule { accepts: is_type::<P>, op }
    }

    pub(crate) fn accepts(&self, data: &DragData) -> bool {
        (self.accepts)(data)
    }
}

/// The ghost of `preview`: its display list recorded once, replayed at each paint of the ghost
/// node (which ui-rt repaints only when it is new; moves translate it). `None` for previews that
/// show nothing beside the pointer.
pub(crate) fn ghost(preview: &DragPreview, tokens: &Tokens) -> Option<DragGhost> {
    let radius = tokens.size.button / 4.0;
    let (size, list) = match preview {
        DragPreview::Badge(label) => {
            let size =
                Vec2::new(label.as_str().chars().count() as f32 * tokens.size.icon.md * 0.55 + 2.0 * tokens.space.pad.control, tokens.size.button);
            let rect = Rect::from_min_size(Pos2::ZERO, size);
            (size, vec![Item::Rect { rect, radii: radius.into(), fill: tokens.color.surface.raised, stroke: None }])
        }
        DragPreview::Frame { size, .. } => {
            let rect = Rect::from_min_size(Pos2::ZERO, *size);
            let stroke = Stroke::new(1.0, tokens.color.accent.solid);
            (*size, vec![Item::Rect { rect, radii: radius.into(), fill: craft_ui_rt::Rgba::TRANSPARENT, stroke: Some(stroke) }])
        }
        _ => return None,
    };
    let list = DisplayList { items: list };
    let offset = match preview {
        DragPreview::Frame { .. } => -size / 2.0,
        _ => Vec2::splat(tokens.size.icon.md),
    };
    Some(DragGhost::new(size, offset, move |cx| cx.extend(&list)))
}
