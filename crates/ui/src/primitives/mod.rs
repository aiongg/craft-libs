//! Painters, `Draw`, the sensing core, the text adapter and `ImeSurface`.

mod content;
mod draw;
mod focus_ring;
mod handle;
mod icon;
mod record;
mod reset_mark;
mod rule;
mod scroll_bar;
pub(crate) mod sensing;
mod state_layer;
mod surface;
mod text;
pub mod textures;

pub use content::{ContentPainter, ContentPainterCx, DocRun, DocText};
pub use draw::{Draw, PaintShape, StrokeWidth};
pub use focus_ring::{FocusKind, FocusRing};
pub use handle::{Handle, HandleShape};
pub use icon::{Glyph, IconSpec, Mirror};
pub use reset_mark::ResetMark;
pub use rule::{Rule, RuleKind};
pub use scroll_bar::{BarAxis, ScrollBar};
pub use sensing::{PartResult, TextOutcome};
pub use state_layer::{StateLayer, StateLayerKind};
pub use surface::Surface;
pub use text::{ChordText, LayoutRun, Span, TextLayout, TextRun, Wrap};

#[cfg_attr(not(feature = "testing"), expect(unused_imports, reason = "used by the pipeline (F3)"))]
pub(crate) use draw::DrawEnv;
#[cfg_attr(not(feature = "testing"), expect(unused_imports, reason = "used by the pipeline (F3)"))]
pub(crate) use record::{PaintRecord, face_id};
