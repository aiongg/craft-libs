//! The canvas controls: leaves a `Canvas` body shows (`#[component(canvas_control)]`).

mod words;

// Guides and smart guides.
mod guides;
mod smart_guides;

pub use guides::{GuideCommands, GuideSnap, GuideStyle, Guides, GuidesOutput};
pub use smart_guides::{Measure, SmartGuides, SmartGuidesStyle, SmartLine};
pub use words::GuideWords;
