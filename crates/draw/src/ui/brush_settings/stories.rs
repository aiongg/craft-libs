//! Stories of the Brush Settings panel and its shared rows (stub; task ST-BR2 writes them, RG2
//! registers them and adds `BrushSettingsPanel`, `BrushSectionList`, `BrushStrokeStrip`,
//! `TipEllipse`, `BrushSizeField` and `DynamicControlRow` to the components). Fake section bodies
//! and a fake stroke image drawn from [`PreviewStroke`](super::PreviewStroke) with the painter, so
//! every state shows without an engine; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/brush-settings/sections` | The list with enable boxes, locks, switch rows and the link row; every section off |
//! | `composites/brush-settings/panel` | The whole panel with fake bodies for three sections and the strip; an off section greyed |
//! | `composites/brush-settings/ellipse` | The angle and roundness widget with its value readout |
//! | `composites/brush-settings/dynamics` | `DynamicControlRow` with and without Fade, the curve toggle lit and plain; `BrushSizeField` with and without restore |
//! | `composites/brush-settings/strip` | The strip with the S-curve of a fake stroke at three widths |
