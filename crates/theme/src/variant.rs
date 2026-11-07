//! Variants: emphasis by tone, custom variants for one component's style key, and the look both resolve to.
//! Egui-free; a component's style turns a [`VariantLook`] into style parts.

use crate::color::Rgba;
use crate::parts::States;
use crate::tokens::{StatusColors, Tokens};
use std::any::TypeId;
use std::hash::{Hash, Hasher};

/// How strongly a variant stands out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Emphasis {
    /// A solid tone fill.
    Solid,
    /// A tinted fill with a tone edge and text.
    Soft,
    /// An outline, filled only by state.
    Outline,
    /// No fill or edge until a state fills it.
    Ghost,
}

/// What a variant says about its subject. The one tone type of the component layer: messages, notices, badges and
/// status ink all name it, and `StyleCx::tone` and `StyleCx::tone_fill` give its colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Tone {
    /// No status.
    #[default]
    Neutral,
    /// The accent.
    Accent,
    /// Information.
    Info,
    /// Success.
    Good,
    /// Needs attention.
    Warn,
    /// Failure.
    Bad,
}

/// Which radius of the theme's shape a look uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum RadiusStep {
    /// `radius.control`.
    #[default]
    Control,
    /// `radius.button`.
    Button,
    /// `radius.pill`.
    Pill,
}

/// The colours and shape of a variant. It holds no padding or width, so a variant cannot change layout.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct VariantLook {
    /// Background per state.
    pub fill: States<Rgba>,
    /// Label and icon per state.
    pub text: States<Rgba>,
    /// Edge per state; `None` draws no edge.
    pub edge: Option<States<Rgba>>,
    /// The corner radius step.
    pub radius: RadiusStep,
}

impl VariantLook {
    /// A look with no edge and control corners.
    pub const fn new(fill: States<Rgba>, text: States<Rgba>) -> Self {
        VariantLook { fill, text, edge: None, radius: RadiusStep::Control }
    }

    /// Sets the edge colours.
    pub const fn edge(mut self, edge: States<Rgba>) -> Self {
        self.edge = Some(edge);
        self
    }

    /// Sets the corner radius step.
    pub const fn radius(mut self, radius: RadiusStep) -> Self {
        self.radius = radius;
        self
    }

    /// The look of `emphasis` in `tone`, from the resolved tokens.
    pub fn of(emphasis: Emphasis, tone: Tone, tokens: &Tokens) -> VariantLook {
        let c = &tokens.color;
        let clear = Rgba::TRANSPARENT;
        let flat = |color: Rgba| States::new(clear, clear, clear, color);
        let text = |color: Rgba| States::new(color, color, color, c.text.disabled);
        let state_fill = States::new(clear, c.state.hover, c.state.pressed, clear);
        let status = |s: &StatusColors| (s.bg, s.fg, s.border, s.solid, s.solid_hover, s.solid_pressed, c.text.on_status);
        let (soft_fill, fg, edge, solid, solid_hover, solid_pressed, on_solid) = match tone {
            Tone::Neutral => (c.surface.sunken, c.text.normal, c.text.muted, c.text.muted, c.text.muted, c.text.normal, c.surface.panel),
            Tone::Accent => {
                (c.accent.soft, c.accent.text, c.accent.border, c.accent.strong, c.accent.strong_hover, c.accent.strong_pressed, c.text.on_strong)
            }
            Tone::Info => status(&c.status.info),
            Tone::Good => status(&c.status.good),
            Tone::Warn => status(&c.status.warn),
            Tone::Bad => status(&c.status.bad),
        };
        let button = RadiusStep::Button;
        match emphasis {
            Emphasis::Solid => VariantLook::new(States::new(solid, solid_hover, solid_pressed, c.state.disabled_fill), text(on_solid)).radius(button),
            Emphasis::Soft => VariantLook::new(States::new(soft_fill, soft_fill, soft_fill, c.state.disabled_fill), text(fg))
                .edge(States::new(edge, edge, edge, c.line.divider))
                .radius(button),
            Emphasis::Outline => VariantLook::new(state_fill, text(fg)).edge(States::new(edge, edge, edge, c.line.divider)).radius(button),
            Emphasis::Ghost => VariantLook::new(state_fill, text(fg)).radius(button).edge(flat(clear)),
        }
    }
}

/// A look for one component's style key, built in a `const`. It describes itself; nothing registers it.
#[derive(Clone, Copy)]
pub struct CustomVariant {
    id: &'static str,
    look: fn(&Tokens) -> VariantLook,
    for_key: fn() -> TypeId,
}

fn type_id_of<K: 'static>() -> TypeId {
    TypeId::of::<K>()
}

impl CustomVariant {
    /// A variant with id `"crate.name"` (a const panic without a crate prefix) whose look is `look`. `K` is the
    /// component's style key type.
    pub const fn new<K: 'static>(id: &'static str, look: fn(&Tokens) -> VariantLook) -> Self {
        let bytes = id.as_bytes();
        let mut i = 1;
        let mut prefixed = false;
        while i + 1 < bytes.len() {
            if bytes[i] == b'.' {
                prefixed = true;
            }
            i += 1;
        }
        assert!(prefixed, "a custom variant id is `crate.name`");
        CustomVariant { id, look, for_key: type_id_of::<K> }
    }

    /// The id, `"crate.name"`.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// The look in `tokens`.
    pub fn look(&self, tokens: &Tokens) -> VariantLook {
        (self.look)(tokens)
    }

    /// The `TypeId` of the style key the variant is for.
    pub fn key_type(&self) -> TypeId {
        (self.for_key)()
    }
}

impl PartialEq for CustomVariant {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CustomVariant {}

impl Hash for CustomVariant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl std::fmt::Debug for CustomVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomVariant({})", self.id)
    }
}

/// An emphasis in a tone, or a custom variant. Equal and hashed by id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Variant {
    /// A built-in look.
    #[non_exhaustive]
    Look {
        /// How strongly it stands out.
        emphasis: Emphasis,
        /// What it says.
        tone: Tone,
    },
    /// A look written for one component.
    Custom(CustomVariant),
}

impl Variant {
    /// Solid accent.
    pub const PRIMARY: Variant = Variant::look(Emphasis::Solid, Tone::Accent);
    /// Outline neutral.
    pub const SECONDARY: Variant = Variant::look(Emphasis::Outline, Tone::Neutral);
    /// Ghost neutral.
    pub const GHOST: Variant = Variant::look(Emphasis::Ghost, Tone::Neutral);
    /// Solid bad.
    pub const DANGER: Variant = Variant::look(Emphasis::Solid, Tone::Bad);

    /// A built-in variant; the constructor outside this crate, where `Look` cannot be built directly.
    pub const fn look(emphasis: Emphasis, tone: Tone) -> Variant {
        Variant::Look { emphasis, tone }
    }

    /// The look in `tokens`.
    pub fn resolve(&self, tokens: &Tokens) -> VariantLook {
        match self {
            Variant::Look { emphasis, tone } => VariantLook::of(*emphasis, *tone, tokens),
            Variant::Custom(custom) => custom.look(tokens),
        }
    }
}
