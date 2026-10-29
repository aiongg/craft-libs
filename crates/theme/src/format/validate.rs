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
        c.references(self);
        if c.errors.is_empty() { Ok(()) } else { Err(FormatErrors(c.errors)) }
    }

    /// The reference cycles of this theme, each at the path of a reference on the cycle. Layers
    /// that can apply together are followed together: the theme's pins, its high-contrast pins,
    /// and each app's over them.
    pub(super) fn reference_errors(&self) -> Vec<FormatError> {
        let mut c = Check { errors: Vec::new(), path: Vec::new() };
        c.references(self);
        c.errors
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
                if variants.mode_for(mode) != mode && !map.is_empty() {
                    c.fail(ErrorKind::NoSuchMode);
                } else {
                    c.pins(map);
                }
            });
        }
    }

    fn pins(&mut self, pins: &Pins) {
        for &token in pins.refs.keys() {
            self.at(&token.path(), |c| {
                if matches!(token, TokenRef::Component(f) if f.kind() != FieldKind::Color) {
                    c.fail(ErrorKind::PinNotColor);
                }
            });
        }
        for (&token, colour) in &pins.colors {
            self.at(&token.path(), |c| match token {
                TokenRef::Component(f) if f.kind() != FieldKind::Color => c.fail(ErrorKind::PinNotColor),
                TokenRef::Color(t) if t.is_opaque_surface() && colour.a() < 255 => c.fail(ErrorKind::NotOpaque),
                _ => {}
            });
        }
    }

    /// Reports a reference cycle once, at the first reference on it in file order.
    fn references(&mut self, t: &ThemeDef) {
        type Layer<'a> = (Vec<&'a str>, &'a ModePins);
        let theme: Layer = (vec![], &t.pins);
        let theme_high: Layer = (vec!["high_contrast"], &t.high_contrast);
        let mut stacks: Vec<Vec<Layer>> = vec![vec![theme.clone()], vec![theme.clone(), theme_high.clone()]];
        for (id, app) in &t.apps {
            let id = id.as_str();
            let own: Layer = (vec!["apps", id], &app.pins);
            let own_high: Layer = (vec!["apps", id, "high_contrast"], &app.high_contrast);
            stacks.push(vec![theme.clone(), own.clone()]);
            stacks.push(vec![theme.clone(), own, theme_high.clone(), own_high]);
        }
        let mut reported: Vec<Vec<String>> = Vec::new();
        for stack in &stacks {
            for (mode_key, mode) in [("light", Mode::Light), ("dark", Mode::Dark)] {
                // The reference each token ends with, and where it is written.
                let mut effective: BTreeMap<TokenRef, Option<(TokenRef, Vec<String>)>> = BTreeMap::new();
                for (prefix, pins) in stack {
                    let pins = pins.get(mode);
                    for &k in pins.colors.keys() {
                        effective.insert(k, None);
                    }
                    for (&k, &target) in &pins.refs {
                        let path = prefix.iter().map(|s| (*s).to_owned()).chain([mode_key.to_owned(), k.path()]).collect();
                        effective.insert(k, Some((target, path)));
                    }
                }
                for (&start, entry) in &effective {
                    let Some((_, path)) = entry else { continue };
                    let mut chain = vec![start];
                    let mut at = start;
                    while let Some(Some((next, _))) = effective.get(&at) {
                        at = *next;
                        if at == start {
                            chain.push(at);
                            let names: Vec<String> = chain.iter().map(|t| t.path()).collect();
                            let mut key = names[..names.len() - 1].to_vec();
                            key.sort();
                            if !reported.contains(&key) {
                                reported.push(key);
                                self.errors.push(FormatError { path: path.clone(), kind: ErrorKind::ReferenceCycle { chain: names } });
                            }
                            break;
                        }
                        if chain.contains(&at) {
                            break;
                        }
                        chain.push(at);
                    }
                }
            }
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
