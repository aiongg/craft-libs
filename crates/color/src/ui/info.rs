//! [`InfoPanel`]: a read-only readout grid of the colour, position and size under the pointer.

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use craft_ui::contract::Pattern;
use craft_ui::runtime::env::Label;
use egui::{Response, Ui};

use super::SamplerReadout;

craft_ui::words! {
    /// The words of the Info panel, in the installed language unless set.
    pub struct InfoWords {
        r: "info.r" = "R";
        g: "info.g" = "G";
        b: "info.b" = "B";
        c: "info.c" = "C";
        m: "info.m" = "M";
        y: "info.y" = "Y";
        k: "info.k" = "K";
        x: "info.x" = "X";
        pos_y: "info.posY" = "Y";
        w: "info.w" = "W";
        h: "info.h" = "H";
        /// The document size template.
        document: "info.document" = "Doc: {width} x {height}";
        no_document: "info.noDocument" = "No document";
    }
}

/// A colour in 8-bit sRGB and CMYK percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InfoColor {
    /// The colour in 8-bit sRGB.
    pub rgb: [u8; 3],
    /// The colour in CMYK percent.
    pub cmyk: [u8; 4],
}

impl InfoColor {
    /// `color` through the document's CMYK converter: the one sRGB-and-CMYK conversion, which `SamplerReadout::new` builds
    /// its `rgb` and `cmyk` through.
    pub fn new(color: Srgba, cmyk: &CmykConverter) -> Self {
        let _ = (color, cmyk);
        todo!("pn.C33")
    }
}

/// What the Info panel shows.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct InfoReadings<'a> {
    /// The colour under the pointer.
    pub color: Option<InfoColor>,
    /// The pointer's position.
    pub position: Option<[Label; 2]>,
    /// The selection's extent.
    pub extent: Option<[Label; 2]>,
    /// The document's size; `None` is the no-document state.
    pub document: Option<[u32; 2]>,
    /// The colour samplers' readouts with their position labels.
    pub samplers: &'a [(SamplerReadout, [Label; 2])],
}

impl<'a> InfoReadings<'a> {
    /// Readings with nothing in them.
    pub fn new() -> Self {
        todo!("pn.C33")
    }

    /// Sets the colour.
    #[must_use]
    pub fn color(mut self, color: Option<InfoColor>) -> Self {
        let _ = (&mut self, color);
        todo!("pn.C33")
    }

    /// Sets the position.
    #[must_use]
    pub fn position(mut self, position: Option<[Label; 2]>) -> Self {
        let _ = (&mut self, position);
        todo!("pn.C33")
    }

    /// Sets the extent.
    #[must_use]
    pub fn extent(mut self, extent: Option<[Label; 2]>) -> Self {
        let _ = (&mut self, extent);
        todo!("pn.C33")
    }

    /// Sets the document size.
    #[must_use]
    pub fn document(mut self, document: Option<[u32; 2]>) -> Self {
        let _ = (&mut self, document);
        todo!("pn.C33")
    }

    /// Sets the samplers.
    #[must_use]
    pub fn samplers(mut self, samplers: &'a [(SamplerReadout, [Label; 2])]) -> Self {
        let _ = (&mut self, samplers);
        todo!("pn.C33")
    }
}

/// A read-only grid: RGB beside CMYK, X/Y beside W/H, the document size, then one row group per sampler; a dash for a
/// missing value; `document: None` shows "No document" as a centred notice in place of the grid.
#[must_use = "show the panel with `show`"]
#[derive(Pattern)]
#[allow(dead_code)] // P3(panels-w7): pn.C33
pub struct InfoPanel<'a> {
    label: Label,
    readings: &'a InfoReadings<'a>,
}

impl<'a> InfoPanel<'a> {
    /// A panel named `label` over `readings`.
    pub fn new(label: impl Into<Label>, readings: &'a InfoReadings<'a>) -> Self {
        let _ = (label.into(), readings);
        todo!("pn.C33")
    }

    /// Draws the grid.
    pub fn show(self, ui: &mut Ui) -> Response {
        let _ = (self, ui);
        todo!("pn.C33")
    }
}
