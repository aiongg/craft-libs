//! The Image Size dialog's draft.

/// The unit of a size field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeUnit {
    #[default]
    Pixels,
    Percent,
}

impl SizeUnit {
    pub const ALL: [SizeUnit; 2] = [SizeUnit::Pixels, SizeUnit::Percent];

    pub fn label(self) -> &'static str {
        match self {
            SizeUnit::Pixels => "Pixels",
            SizeUnit::Percent => "Percent",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Resample {
    #[default]
    Automatic,
    Bicubic,
    Bilinear,
    NearestNeighbor,
}

impl Resample {
    pub const ALL: [Resample; 4] = [Resample::Automatic, Resample::Bicubic, Resample::Bilinear, Resample::NearestNeighbor];

    pub fn label(self) -> &'static str {
        match self {
            Resample::Automatic => "Automatic",
            Resample::Bicubic => "Bicubic",
            Resample::Bilinear => "Bilinear",
            Resample::NearestNeighbor => "Nearest Neighbor",
        }
    }
}

/// The dialog's fields while it is open. `width` and `height` are in their units; `original` is
/// the document's size in pixels when the dialog opened, the base of percent values and of the
/// linked ratio.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSizeDraft {
    pub width: f64,
    pub height: f64,
    pub unit_w: SizeUnit,
    pub unit_h: SizeUnit,
    pub resolution: f64,
    pub link: bool,
    pub resample: bool,
    pub method: Resample,
    pub original: [u32; 2],
}

impl ImageSizeDraft {
    pub fn new(size: [u32; 2]) -> Self {
        Self {
            width: f64::from(size[0]),
            height: f64::from(size[1]),
            unit_w: SizeUnit::Pixels,
            unit_h: SizeUnit::Pixels,
            resolution: 72.0,
            link: true,
            resample: true,
            method: Resample::Automatic,
            original: size,
        }
    }

    fn pixels(&self, value: f64, unit: SizeUnit, axis: usize) -> f64 {
        match unit {
            SizeUnit::Pixels => value,
            SizeUnit::Percent => value / 100.0 * f64::from(self.original[axis]),
        }
    }

    fn in_unit(&self, pixels: f64, unit: SizeUnit, axis: usize) -> f64 {
        match unit {
            SizeUnit::Pixels => pixels,
            SizeUnit::Percent => pixels / f64::from(self.original[axis]) * 100.0,
        }
    }

    /// The size the dialog would make the document, in whole pixels (at least 1). Without
    /// resampling the pixel count is the original.
    pub fn pixel_size(&self) -> [u32; 2] {
        if !self.resample {
            return self.original;
        }
        [self.pixels(self.width, self.unit_w, 0), self.pixels(self.height, self.unit_h, 1)].map(|p| p.round().max(1.0) as u32)
    }

    /// Sets the width; with the link on, the height keeps the original ratio.
    pub fn set_width(&mut self, width: f64) {
        if !self.resample {
            return;
        }
        self.width = width;
        if self.link {
            let pixels = self.pixels(width, self.unit_w, 0) * f64::from(self.original[1]) / f64::from(self.original[0]);
            self.height = self.in_unit(pixels, self.unit_h, 1);
        }
    }

    /// Sets the height; with the link on, the width keeps the original ratio.
    pub fn set_height(&mut self, height: f64) {
        if !self.resample {
            return;
        }
        self.height = height;
        if self.link {
            let pixels = self.pixels(height, self.unit_h, 1) * f64::from(self.original[0]) / f64::from(self.original[1]);
            self.width = self.in_unit(pixels, self.unit_w, 0);
        }
    }

    /// Turns resampling on or off. Off, the pixel count is fixed and the fields show it.
    pub fn set_resample(&mut self, on: bool) {
        self.resample = on;
        if !on {
            self.width = self.in_unit(f64::from(self.original[0]), self.unit_w, 0);
            self.height = self.in_unit(f64::from(self.original[1]), self.unit_h, 1);
        }
    }

    /// Changes a unit and converts the value so the size stays the same.
    pub fn set_unit(&mut self, axis: usize, unit: SizeUnit) {
        let (value, current) = if axis == 0 { (self.width, self.unit_w) } else { (self.height, self.unit_h) };
        let converted = self.in_unit(self.pixels(value, current, axis), unit, axis);
        if axis == 0 {
            (self.width, self.unit_w) = (converted, unit);
        } else {
            (self.height, self.unit_h) = (converted, unit);
        }
    }
}
