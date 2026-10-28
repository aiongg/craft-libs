//! Checking a [`ThemeDef`] against what [`parse`](super::parse) accepts, so that
//! [`write`](super::write) never produces a file that fails to parse.

use std::collections::BTreeMap;

use super::{EDGE, ErrorKind, FormatError, FormatErrors, LENGTH};
use crate::components::{ComponentField, FieldKind, OverrideValue};
use crate::ramp::Mode;
use crate::theme::{EdgeSeed, ModePins, Pins, ShapeSeed, ThemeDef, Variants};
use crate::tokens::TokenRef;

impl ThemeDef {
    /// Every value that [`parse`](super::parse) would reject in the file
    /// [`write`](super::write) makes of this theme, with its path there: pins for a mode the
    /// theme's `variants` leaves out, a pin on a non-colour field, a translucent surface, a
    /// colour or a value of the wrong kind in `components`, a negative or non-finite length, a
    /// bevel width that is not above 0.
    pub fn validate(&self) -> Result<(), FormatErrors> {
        let mut c = Check { errors: Vec::new(), path: Vec::new() };
        if let Some(shape) = &self.shape {
            c.at("shape", |c| c.shape(shape));
        }
        c.mode_pins(&self.pins, self.variants);
        for (key, mode) in [("light", Mode::Light), ("dark", Mode::Dark)] {
            if self.variants.mode_for(mode) != mode && !self.mode_seeds.get(mode).is_empty() {
                c.at(key, |c| c.at("seeds", |c| c.fail(ErrorKind::NoSuchMode)));
            }
        }
        c.at("high_contrast", |c| c.mode_pins(&self.high_contrast, self.variants));
        c.at("components", |c| c.components(&self.components));
        c.at("apps", |c| {
            for (id, app) in &self.apps {
                c.at(id.as_str(), |c| {
                    c.mode_pins(&app.pins, self.variants);
                    c.at("high_contrast", |c| c.mode_pins(&app.high_contrast, self.variants));
                    c.at("components", |c| c.components(&app.components));
                });
            }
        });
        if c.errors.is_empty() { Ok(()) } else { Err(FormatErrors(c.errors)) }
    }
}

struct Check {
    errors: Vec<FormatError>,
    path: Vec<String>,
}

impl Check {
    fn fail(&mut self, kind: ErrorKind) {
        self.errors.push(FormatError { path: self.path.clone(), kind });
    }

    fn at(&mut self, key: &str, f: impl FnOnce(&mut Self)) {
        self.path.push(key.to_owned());
        f(self);
        self.path.pop();
    }

    fn shape(&mut self, shape: &ShapeSeed) {
        if let Some(r) = shape.radius {
            self.at("radius", |c| {
                for (key, v) in
                    [("control", r.control), ("popup", r.popup), ("window", r.window), ("panel", r.panel), ("button", r.button), ("pill", r.pill)]
                {
                    c.at(key, |c| c.length(v));
                }
            });
        }
        if let Some(e) = shape.edge {
            self.at("edge", |c| c.edge(e));
        }
    }

    fn length(&mut self, v: f32) {
        if !(v.is_finite() && v >= 0.0) {
            self.fail(ErrorKind::BadValue { expected: LENGTH });
        }
    }

    fn edge(&mut self, e: EdgeSeed) {
        if let EdgeSeed::Bevel { width } = e
            && !(width.is_finite() && width > 0.0)
        {
            self.fail(ErrorKind::BadValue { expected: EDGE });
        }
    }

    fn mode_pins(&mut self, pins: &ModePins, variants: Variants) {
        for (key, mode) in [("light", Mode::Light), ("dark", Mode::Dark)] {
            let map = pins.get(mode);
            self.at(key, |c| {
                if variants.mode_for(mode) != mode && !map.0.is_empty() {
                    c.fail(ErrorKind::NoSuchMode);
                } else {
                    c.pins(map);
                }
            });
        }
    }

    fn pins(&mut self, pins: &Pins) {
        for (&token, colour) in &pins.0 {
            self.at(&token.path(), |c| match token {
                TokenRef::Component(f) if f.kind() != FieldKind::Color => c.fail(ErrorKind::PinNotColor),
                TokenRef::Color(t) if t.is_opaque_surface() && colour.a() < 255 => c.fail(ErrorKind::NotOpaque),
                _ => {}
            });
        }
    }

    fn components(&mut self, components: &BTreeMap<ComponentField, OverrideValue>) {
        for (&field, &value) in components {
            self.at(&field.path(), |c| {
                let (expected, found) = (field.kind(), value.kind());
                match value {
                    _ if expected == FieldKind::Color => c.fail(ErrorKind::ColorNotPerMode),
                    _ if expected != found => c.fail(ErrorKind::WrongKind { expected, found }),
                    OverrideValue::Length(l) => c.length(l),
                    OverrideValue::Color(_) | OverrideValue::Elevation(_) => {}
                }
            });
        }
    }
}
