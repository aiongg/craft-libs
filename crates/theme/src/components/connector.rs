//! The Connector tool: the route, attach points, handles and the ghost. The same at every Size (they sit on the document).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Connector tool: the route, attach points, handles and the ghost. The same at every Size (they sit on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ConnectorTokens {
    /// The route and rings' width: `connector.line`.
    pub line: f32,
    /// An attach point's radius: `connector.attach`.
    pub attach: f32,
    /// The active attach point's radius: `connector.attach_active`.
    pub attach_active: f32,
    /// The active attach point's fill alpha (a ratio, not a size): `connector.attach_fill_alpha`.
    pub attach_fill_alpha: f32,
    /// The ghost's fill alpha (a ratio, not a size): `connector.ghost_alpha`.
    pub ghost_alpha: f32,
    /// The ghost outline's dash: `connector.ghost_dash`.
    pub ghost_dash: f32,
    /// The ghost outline's gap: `connector.ghost_gap`.
    pub ghost_gap: f32,
    /// The ghost outline's alpha (a ratio, not a size): `connector.ghost_line_alpha`.
    pub ghost_line_alpha: f32,
    /// A drag shorter than this many screen points makes nothing: `connector.click_slop`.
    pub click_slop: f32,
    /// An end handle's radius: `connector.end_handle`.
    pub end_handle: f32,
    /// A path handle's radius: `connector.path_handle`.
    pub path_handle: f32,
    /// An end within this many screen points of a site glues to it: `connector.attach_reach`.
    pub attach_reach: f32,
    /// An end or path handle within this many screen points of a press is grabbed: `connector.handle_reach`.
    pub handle_reach: f32,
    /// An elbow run shorter than this many screen points has no path handle: `connector.min_run`.
    pub min_run: f32,
    /// A target is hit within this many screen points of its bounds: `connector.target_tolerance`.
    pub target_tolerance: f32,
    /// A path handle's ring width: `connector.path_ring`.
    pub path_ring: f32,
    /// The hovered target's outline width: `connector.drop_outline`.
    pub drop_outline: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConnectorField {}

impl Family for ConnectorTokens {
    type Field = ConnectorField;
    const FIELDS: &'static [ConnectorField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ConnectorTokens;
        "line" 0.0..=16.0 => line;
        "attach" 0.0..=64.0 => attach;
        "attach_active" 0.0..=64.0 => attach_active;
        "attach_fill_alpha" 0.0..=1.0 => attach_fill_alpha;
        "ghost_alpha" 0.0..=1.0 => ghost_alpha;
        "ghost_dash" 0.0..=64.0 => ghost_dash;
        "ghost_gap" 0.0..=64.0 => ghost_gap;
        "ghost_line_alpha" 0.0..=1.0 => ghost_line_alpha;
        "click_slop" 0.0..=64.0 => click_slop;
        "end_handle" 0.0..=64.0 => end_handle;
        "path_handle" 0.0..=64.0 => path_handle;
        "attach_reach" 0.0..=128.0 => attach_reach;
        "handle_reach" 0.0..=128.0 => handle_reach;
        "min_run" 0.0..=128.0 => min_run;
        "target_tolerance" 0.0..=128.0 => target_tolerance;
        "path_ring" 0.0..=128.0 => path_ring;
        "drop_outline" 0.0..=128.0 => drop_outline;
    };

    fn defaults(_: &Semantic<'_>) -> ConnectorTokens {
        ConnectorTokens {
            line: values::CONNECTOR_LINE,
            attach: values::CONNECTOR_ATTACH,
            attach_active: values::CONNECTOR_ATTACH_ACTIVE,
            attach_fill_alpha: values::CONNECTOR_ATTACH_FILL_ALPHA,
            ghost_alpha: values::CONNECTOR_GHOST_ALPHA,
            ghost_dash: values::CONNECTOR_GHOST_DASH,
            ghost_gap: values::CONNECTOR_GHOST_GAP,
            ghost_line_alpha: values::CONNECTOR_GHOST_LINE_ALPHA,
            click_slop: values::CONNECTOR_CLICK_SLOP,
            end_handle: values::CONNECTOR_END_HANDLE,
            path_handle: values::CONNECTOR_PATH_HANDLE,
            attach_reach: values::CONNECTOR_ATTACH_REACH,
            handle_reach: values::CONNECTOR_HANDLE_REACH,
            min_run: values::CONNECTOR_MIN_RUN,
            target_tolerance: values::CONNECTOR_TARGET_TOLERANCE,
            path_ring: values::CONNECTOR_PATH_RING,
            drop_outline: values::CONNECTOR_DROP_OUTLINE,
        }
    }

    fn path(f: ConnectorField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ConnectorField> {
        None
    }

    fn kind(f: ConnectorField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ConnectorField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ConnectorField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
