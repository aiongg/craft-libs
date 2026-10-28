//! Reading and validating a theme file.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::{EDGE, ErrorKind, FormatError, FormatErrors, LENGTH, MAX_THEME_BYTES, SCHEMA, SCHEMA_VERSION, ThemeFile};
use crate::color::Rgba;
use crate::components::{ComponentField, FieldKind, OverrideValue};
use crate::scale::{Elevation, Radius};
use crate::theme::{AppOverrides, ContrastIntent, EdgeSeed, ModePins, Pins, Seeds, ShapeSeed, ThemeDef, Variants};
use crate::tokens::{ColorToken, RadiusSet, Status, TokenRef};
use crate::{AppId, ThemeId};

/// Parses and validates a theme file. Unknown keys are allowed anywhere except in colour and
/// component maps, and kept for [`write`](super::write). Every field error is reported, not
/// only the first.
pub fn parse(bytes: &[u8]) -> Result<ThemeFile, FormatErrors> {
    let stop = |path: &[&str], kind| Err(FormatErrors(vec![FormatError { path: path.iter().map(|s| (*s).to_owned()).collect(), kind }]));
    if bytes.len() > MAX_THEME_BYTES {
        return stop(&[], ErrorKind::TooLarge { len: bytes.len() });
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(e) => return stop(&[], ErrorKind::Json(e.to_string())),
    };
    let Value::Object(raw) = value else { return stop(&[], ErrorKind::NotObject) };
    match raw.get("schema") {
        Some(Value::String(s)) if s == SCHEMA => {}
        Some(Value::String(s)) => return stop(&["schema"], ErrorKind::OtherSchema { schema: s.clone() }),
        Some(_) => return stop(&["schema"], ErrorKind::WrongType { expected: "a string" }),
        None => return stop(&["schema"], ErrorKind::Missing),
    }
    match raw.get("schema_version").and_then(Value::as_u64) {
        Some(SCHEMA_VERSION) => {}
        Some(v) if v > SCHEMA_VERSION => return stop(&["schema_version"], ErrorKind::TooNew { schema_version: v }),
        _ => return stop(&["schema_version"], ErrorKind::BadSchemaVersion),
    }

    let mut p = Parser::new();
    let theme = p.theme(&raw);
    match theme {
        Some(theme) if p.errors.is_empty() => Ok(ThemeFile { theme, raw }),
        _ => Err(FormatErrors(p.errors)),
    }
}

/// Collects errors while walking the file; `path` is the position being read.
pub(super) struct Parser {
    pub(super) errors: Vec<FormatError>,
    path: Vec<String>,
}

impl Parser {
    pub(super) fn new() -> Parser {
        Parser { errors: Vec::new(), path: Vec::new() }
    }

    fn fail(&mut self, kind: ErrorKind) {
        self.errors.push(FormatError { path: self.path.clone(), kind });
    }

    /// Runs `f` with `key` pushed onto the path.
    fn at<T>(&mut self, key: &str, f: impl FnOnce(&mut Self) -> T) -> T {
        self.path.push(key.to_owned());
        let out = f(self);
        self.path.pop();
        out
    }

    /// The object under `key`: `None` when absent or null; an error when anything else.
    fn optional_object<'a>(&mut self, obj: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
        match obj.get(key) {
            None | Some(Value::Null) => None,
            Some(Value::Object(o)) => Some(o),
            Some(_) => {
                self.at(key, |p| p.fail(ErrorKind::WrongType { expected: "an object" }));
                None
            }
        }
    }

    /// The object under `key`; missing is an error too.
    fn required_object<'a>(&mut self, obj: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
        if obj.get(key).is_none_or(Value::is_null) {
            self.at(key, |p| p.fail(ErrorKind::Missing));
            return None;
        }
        self.optional_object(obj, key)
    }

    fn colour_at(&mut self, obj: &Map<String, Value>, key: &str) -> Option<Rgba> {
        match obj.get(key) {
            None | Some(Value::Null) => {
                self.at(key, |p| p.fail(ErrorKind::Missing));
                None
            }
            Some(v) => self.at(key, |p| p.colour(v)),
        }
    }

    fn colour(&mut self, v: &Value) -> Option<Rgba> {
        let c = v.as_str().and_then(Rgba::from_hex);
        if c.is_none() {
            self.fail(ErrorKind::BadColour);
        }
        c
    }

    fn theme(&mut self, raw: &Map<String, Value>) -> Option<ThemeDef> {
        let id = self.at("id", |p| match raw.get("id") {
            None => {
                p.fail(ErrorKind::Missing);
                None
            }
            Some(Value::String(s)) => ThemeId::parse(s).map_err(|e| p.fail(ErrorKind::BadId(e))).ok(),
            Some(_) => {
                p.fail(ErrorKind::WrongType { expected: "a string" });
                None
            }
        });
        let name = self.at("name", |p| match raw.get("name") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                p.fail(ErrorKind::WrongType { expected: "a string" });
                None
            }
        });
        let version = self.at("version", |p| match raw.get("version") {
            None | Some(Value::Null) => 1,
            Some(v) => v.as_u64().and_then(|n| u32::try_from(n).ok()).unwrap_or_else(|| {
                p.fail(ErrorKind::BadValue { expected: "a non-negative integer" });
                1
            }),
        });
        let variants =
            self.choice(raw, "variants", Variants::Both, &[("both", Variants::Both), ("light", Variants::LightOnly), ("dark", Variants::DarkOnly)]);
        let contrast_intent =
            self.choice(raw, "contrast_intent", ContrastIntent::Standard, &[("standard", ContrastIntent::Standard), ("low", ContrastIntent::Low)]);
        let seeds = self.seeds(raw);
        let shape = self.shape(raw);
        let pins = self.mode_pins(raw, variants);
        let high_contrast = match self.optional_object(raw, "high_contrast") {
            Some(hc) => self.at("high_contrast", |p| p.mode_pins(hc, variants)),
            None => Some(ModePins::default()),
        };
        let components = self.components(raw);
        let apps = self.apps(raw, variants);
        let id = id?;
        Some(ThemeDef {
            name: name.unwrap_or_else(|| id.as_str().to_owned()),
            id,
            version,
            seeds: seeds?,
            shape: shape?,
            variants,
            pins: pins?,
            high_contrast: high_contrast?,
            components: components?,
            contrast_intent,
            apps: apps?,
        })
    }

    /// A string-valued key restricted to `options`; `default` when absent.
    fn choice<T: Copy>(&mut self, obj: &Map<String, Value>, key: &str, default: T, options: &[(&str, T)]) -> T {
        match obj.get(key) {
            None | Some(Value::Null) => default,
            Some(Value::String(s)) => match options.iter().find(|(name, _)| name == s) {
                Some(&(_, v)) => v,
                None => {
                    self.at(key, |p| p.fail(ErrorKind::BadValue { expected: "one of the listed names" }));
                    default
                }
            },
            Some(_) => {
                self.at(key, |p| p.fail(ErrorKind::WrongType { expected: "a string" }));
                default
            }
        }
    }

    fn seeds(&mut self, raw: &Map<String, Value>) -> Option<Seeds> {
        let seeds = self.required_object(raw, "seeds")?;
        self.at("seeds", |p| p.seed_values(seeds))
    }

    fn seed_values(&mut self, seeds: &Map<String, Value>) -> Option<Seeds> {
        let neutral = self.colour_at(seeds, "neutral");
        let accent = self.colour_at(seeds, "accent");
        let status = self.required_object(seeds, "status").and_then(|st| {
            self.at("status", |p| {
                let good = p.colour_at(st, "good");
                let warn = p.colour_at(st, "warn");
                let bad = p.colour_at(st, "bad");
                let info = p.colour_at(st, "info");
                Some(Status { good: good?, warn: warn?, bad: bad?, info: info? })
            })
        });
        Some(Seeds { neutral: neutral?, accent: accent?, status: status? })
    }

    /// `Some(None)` is a theme without its own shape.
    fn shape(&mut self, raw: &Map<String, Value>) -> Option<Option<ShapeSeed>> {
        let Some(shape) = self.optional_object(raw, "shape") else { return Some(None) };
        self.at("shape", |p| {
            let radius = match p.optional_object(shape, "radius") {
                Some(r) => p.at("radius", |p| p.radius_set(r)).map(Some),
                None => Some(None),
            };
            let edge = p.at("edge", |p| match shape.get("edge") {
                None | Some(Value::Null) => Some(None),
                Some(v) => p.edge(v).map(Some),
            });
            Some(Some(ShapeSeed { radius: radius?, edge: edge? }))
        })
    }

    /// The five radii; all are required.
    fn radius_set(&mut self, r: &Map<String, Value>) -> Option<RadiusSet> {
        let mut get = |key: &str| {
            self.at(key, |p| match r.get(key) {
                None | Some(Value::Null) => {
                    p.fail(ErrorKind::Missing);
                    None
                }
                Some(v) => p.length(v),
            })
        };
        let (control, popup, window, panel, button) = (get("control"), get("popup"), get("window"), get("panel"), get("button"));
        Some(RadiusSet { control: control?, popup: popup?, window: window?, panel: panel?, button: button? })
    }

    /// A length in logical pixels: a number of 0 or more, or `"full"`.
    fn length(&mut self, v: &Value) -> Option<f32> {
        let out = match v {
            Value::String(s) if s == "full" => Some(Radius::Full.px()),
            Value::Number(n) => n.as_f64().filter(|x| x.is_finite() && *x >= 0.0).map(|x| x as f32),
            _ => None,
        };
        if out.is_none() {
            self.fail(ErrorKind::BadValue { expected: LENGTH });
        }
        out
    }

    /// `"flat"` or `{ "bevel": { "width": n } }`.
    fn edge(&mut self, v: &Value) -> Option<EdgeSeed> {
        let width = match v {
            Value::String(s) if s == "flat" => return Some(EdgeSeed::Flat),
            Value::Object(o) => o.get("bevel").and_then(|b| b.get("width")).and_then(Value::as_f64).filter(|w| w.is_finite() && *w > 0.0),
            _ => None,
        };
        if width.is_none() {
            self.fail(ErrorKind::BadValue { expected: EDGE });
        }
        width.map(|w| EdgeSeed::Bevel { width: w as f32 })
    }

    /// The `light` and `dark` maps of `obj`. A map for a mode `variants` leaves out is an error
    /// unless it is empty.
    fn mode_pins(&mut self, obj: &Map<String, Value>, variants: Variants) -> Option<ModePins> {
        let map = |p: &mut Self, key: &str, present: bool| {
            let m = p.optional_object(obj, key);
            p.at(key, |p| match m {
                Some(m) if !present => {
                    if !m.is_empty() {
                        p.fail(ErrorKind::NoSuchMode);
                    }
                    Some(Pins::default())
                }
                Some(m) => p.pins(m),
                None => Some(Pins::default()),
            })
        };
        let light = map(self, "light", variants != Variants::DarkOnly);
        let dark = map(self, "dark", variants != Variants::LightOnly);
        Some(ModePins { light: light?, dark: dark? })
    }

    /// A map of colour paths to colours. Surfaces must be opaque.
    fn pins(&mut self, m: &Map<String, Value>) -> Option<Pins> {
        let mut pins = Pins::default();
        let mut ok = true;
        for (key, v) in m {
            ok &= self.at(key, |p| {
                let Some(token) = TokenRef::from_path(key) else {
                    p.fail(ErrorKind::UnknownToken);
                    return false;
                };
                if let TokenRef::Component(f) = token
                    && f.kind() != FieldKind::Color
                {
                    p.fail(ErrorKind::PinNotColor);
                    return false;
                }
                match p.colour(v) {
                    Some(c) if c.a() < 255 && matches!(token, TokenRef::Color(t) if t.is_opaque_surface()) => {
                        p.fail(ErrorKind::NotOpaque);
                        false
                    }
                    Some(c) => {
                        pins.0.insert(token, c);
                        true
                    }
                    None => false,
                }
            });
        }
        ok.then_some(pins)
    }

    /// The `components` map of `obj`.
    fn components(&mut self, obj: &Map<String, Value>) -> Option<BTreeMap<ComponentField, OverrideValue>> {
        let Some(m) = self.optional_object(obj, "components") else { return Some(BTreeMap::new()) };
        self.at("components", |p| {
            let mut out = BTreeMap::new();
            let mut ok = true;
            for (key, v) in m {
                ok &= p.at(key, |p| match p.component_value(key, v) {
                    Some((field, value)) => {
                        out.insert(field, value);
                        true
                    }
                    None => false,
                });
            }
            ok.then_some(out)
        })
    }

    fn component_value(&mut self, key: &str, v: &Value) -> Option<(ComponentField, OverrideValue)> {
        if ColorToken::from_path(key).is_some() {
            self.fail(ErrorKind::ColorNotPerMode);
            return None;
        }
        let Some(field) = ComponentField::from_path(key) else {
            self.fail(ErrorKind::UnknownToken);
            return None;
        };
        let expected = field.kind();
        if expected == FieldKind::Color {
            self.fail(ErrorKind::ColorNotPerMode);
            return None;
        }
        let Some(found) = value_kind(v) else {
            self.fail(ErrorKind::BadValue { expected: kind_hint(expected) });
            return None;
        };
        if found != expected {
            self.fail(ErrorKind::WrongKind { expected, found });
            return None;
        }
        let value = match expected {
            FieldKind::Length => OverrideValue::Length(self.length(v)?),
            FieldKind::Edge => OverrideValue::Edge(self.edge(v)?),
            FieldKind::Elevation => OverrideValue::Elevation(self.elevation(v)?),
            FieldKind::Color => return None,
        };
        Some((field, value))
    }

    /// `{ "level": 0 to 3 }`.
    pub(super) fn elevation(&mut self, v: &Value) -> Option<Elevation> {
        let level = v.get("level").and_then(Value::as_u64).filter(|l| *l < Elevation::LEVELS.len() as u64);
        if level.is_none() {
            self.fail(ErrorKind::BadValue { expected: "{\"level\": 0 to 3}" });
        }
        level.map(|l| Elevation::level(l as usize))
    }

    fn apps(&mut self, raw: &Map<String, Value>, variants: Variants) -> Option<BTreeMap<AppId, AppOverrides>> {
        let Some(m) = self.optional_object(raw, "apps") else { return Some(BTreeMap::new()) };
        self.at("apps", |p| {
            let mut out = BTreeMap::new();
            let mut ok = true;
            for (key, v) in m {
                ok &= p.at(key, |p| {
                    let Ok(id) = AppId::parse(key) else {
                        p.fail(ErrorKind::BadAppId);
                        return false;
                    };
                    let Value::Object(app) = v else {
                        p.fail(ErrorKind::WrongType { expected: "an object" });
                        return false;
                    };
                    let pins = p.mode_pins(app, variants);
                    let high_contrast = match p.optional_object(app, "high_contrast") {
                        Some(hc) => p.at("high_contrast", |p| p.mode_pins(hc, variants)),
                        None => Some(ModePins::default()),
                    };
                    let components = p.components(app);
                    match (pins, high_contrast, components) {
                        (Some(pins), Some(high_contrast), Some(components)) => {
                            out.insert(id, AppOverrides { pins, high_contrast, components });
                            true
                        }
                        _ => false,
                    }
                });
            }
            ok.then_some(out)
        })
    }
}

/// The kind a JSON value looks like: a `#` colour, a number or `"full"`, an edge, an elevation.
fn value_kind(v: &Value) -> Option<FieldKind> {
    match v {
        Value::Number(_) => Some(FieldKind::Length),
        Value::String(s) if s == "full" => Some(FieldKind::Length),
        Value::String(s) if s == "flat" => Some(FieldKind::Edge),
        Value::String(s) if Rgba::from_hex(s).is_some() => Some(FieldKind::Color),
        Value::Object(o) if o.contains_key("bevel") => Some(FieldKind::Edge),
        Value::Object(o) if o.contains_key("level") => Some(FieldKind::Elevation),
        _ => None,
    }
}

fn kind_hint(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::Color => "a colour",
        FieldKind::Length => "a length: a number of 0 or more, or \"full\"",
        FieldKind::Edge => "\"flat\" or {\"bevel\": {\"width\": n}}",
        FieldKind::Elevation => "{\"level\": 0 to 3}",
    }
}
