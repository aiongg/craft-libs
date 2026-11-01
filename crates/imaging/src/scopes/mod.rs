//! The counting that scopes share, without egui: the clipping rule, clipping masks, the
//! vectorscope grid, hue and saturation, and histogram statistics (`clipping`, `ClippingMask`,
//! `Vectorscope`, `hue_saturation`, `histogram_statistics`). Colour management and the histogram
//! count stay in the app.

mod clipping;
mod statistics;
mod vectorscope;
