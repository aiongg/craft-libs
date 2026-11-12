//! The blend modes of every app, with stable ids shared by the CPU and the shaders.

use serde::{Deserialize, Serialize};

/// Writes [`BlendMode`] with explicit ids and, from the same list, the WGSL constants
/// `BLEND_<NAME>`, so the shader's mode numbers can't drift from the enum.
macro_rules! modes {
    ($( $(#[$meta:meta])* $name:ident = $id:literal => $wgsl:ident, )*) => {
        /// A blend mode. Saved documents store the variant name; shaders get [`BlendMode::id`].
        ///
        /// The ids never change once given: PhotoCraft's 34 modes keep the numbers its
        /// compositor used, After Effects' and Substance's extra modes follow.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[repr(u32)]
        pub enum BlendMode {
            $( $(#[$meta])* $name = $id, )*
        }

        impl BlendMode {
            /// Every mode, in id order.
            pub const ALL: &[BlendMode] = &[$(BlendMode::$name),*];
        }

        /// `const BLEND_<NAME>: u32 = <id>u;` for every mode.
        pub(crate) const MODE_IDS_WGSL: &str = concat!($("const BLEND_", stringify!($wgsl), ": u32 = ", stringify!($id), "u;\n",)*);
    };
}

modes! {
    /// Groups only: children blend straight into the group's backdrop.
    #[serde(alias = "Passthrough")]
    PassThrough = 0 => PASS_THROUGH,
    /// The source over the backdrop.
    #[default]
    Normal = 1 => NORMAL,
    /// Coverage decided per pixel by the caller's noise.
    Dissolve = 2 => DISSOLVE,
    /// Darken.
    Darken = 3 => DARKEN,
    /// Multiply.
    Multiply = 4 => MULTIPLY,
    /// Color Burn.
    ColorBurn = 5 => COLOR_BURN,
    /// Linear Burn.
    LinearBurn = 6 => LINEAR_BURN,
    /// Darker Color: the whole pixel with the lower luma.
    DarkerColor = 7 => DARKER_COLOR,
    /// Lighten.
    Lighten = 8 => LIGHTEN,
    /// Screen.
    Screen = 9 => SCREEN,
    /// Color Dodge.
    ColorDodge = 10 => COLOR_DODGE,
    /// Linear Dodge (Add).
    LinearDodge = 11 => LINEAR_DODGE,
    /// Lighter Color: the whole pixel with the higher luma.
    LighterColor = 12 => LIGHTER_COLOR,
    /// Overlay.
    Overlay = 13 => OVERLAY,
    /// Soft Light (the W3C curve).
    SoftLight = 14 => SOFT_LIGHT,
    /// Hard Light.
    HardLight = 15 => HARD_LIGHT,
    /// Vivid Light.
    VividLight = 16 => VIVID_LIGHT,
    /// Linear Light.
    LinearLight = 17 => LINEAR_LIGHT,
    /// Pin Light.
    PinLight = 18 => PIN_LIGHT,
    /// Hard Mix.
    HardMix = 19 => HARD_MIX,
    /// Difference.
    Difference = 20 => DIFFERENCE,
    /// Exclusion.
    Exclusion = 21 => EXCLUSION,
    /// Subtract.
    Subtract = 22 => SUBTRACT,
    /// Divide.
    Divide = 23 => DIVIDE,
    /// Hue.
    Hue = 24 => HUE,
    /// Saturation.
    Saturation = 25 => SATURATION,
    /// Color.
    Color = 26 => COLOR,
    /// Luminosity.
    Luminosity = 27 => LUMINOSITY,
    /// Paint.NET Reflect.
    Reflect = 28 => REFLECT,
    /// Paint.NET Glow.
    Glow = 29 => GLOW,
    /// Paint.NET Negation.
    Negation = 30 => NEGATION,
    /// Paint.NET XOR, on 8-bit channels.
    Xor = 31 => XOR,
    /// Paint.NET's Color Burn.
    PaintNetColorBurn = 32 => PAINT_NET_COLOR_BURN,
    /// Paint.NET's Color Dodge.
    PaintNetColorDodge = 33 => PAINT_NET_COLOR_DODGE,
    /// After Effects' Dancing Dissolve (the caller's noise changes per frame).
    DancingDissolve = 34 => DANCING_DISSOLVE,
    /// After Effects' Classic Color Burn: negative results kept, the source faded by coverage.
    ClassicColorBurn = 35 => CLASSIC_COLOR_BURN,
    /// After Effects' Add.
    Add = 36 => ADD,
    /// After Effects' Classic Color Dodge.
    ClassicColorDodge = 37 => CLASSIC_COLOR_DODGE,
    /// After Effects' Classic Difference.
    ClassicDifference = 38 => CLASSIC_DIFFERENCE,
    /// After Effects' Stencil Alpha: the backdrop kept where the source is opaque.
    StencilAlpha = 39 => STENCIL_ALPHA,
    /// After Effects' Stencil Luma.
    StencilLuma = 40 => STENCIL_LUMA,
    /// After Effects' Silhouette Alpha: the backdrop cut where the source is opaque.
    SilhouetteAlpha = 41 => SILHOUETTE_ALPHA,
    /// After Effects' Silhouette Luma.
    SilhouetteLuma = 42 => SILHOUETTE_LUMA,
    /// After Effects' Alpha Add: alphas summed, so shared edges close.
    AlphaAdd = 43 => ALPHA_ADD,
    /// After Effects' Luminescent Premul: the source's colour added over the backdrop.
    LuminescentPremul = 44 => LUMINESCENT_PREMUL,
    /// Substance's Disable: the backdrop unchanged.
    Disable = 45 => DISABLE,
    /// Substance's Replace: the source replaces the backdrop.
    Replace = 46 => REPLACE,
    /// Substance's Inverse Divide: source ÷ backdrop.
    InverseDivide = 47 => INVERSE_DIVIDE,
    /// Substance's Inverse Subtract: source − backdrop.
    InverseSubtract = 48 => INVERSE_SUBTRACT,
    /// Substance's Signed Addition: backdrop + 2·source − 1.
    SignedAddition = 49 => SIGNED_ADDITION,
    /// Substance's Tint: the source scaled by the backdrop's luma.
    Tint = 50 => TINT,
    /// Substance's Value: the backdrop scaled to the source's HSV value.
    Value = 51 => VALUE,
    /// Substance's Normal Map Combine (packed normals).
    NormalMapCombine = 52 => NORMAL_MAP_COMBINE,
    /// Substance's Normal Map Detail: reoriented normal mapping, source onto backdrop.
    NormalMapDetail = 53 => NORMAL_MAP_DETAIL,
    /// Substance's Normal Map Inverse Detail: backdrop onto source.
    NormalMapInverseDetail = 54 => NORMAL_MAP_INVERSE_DETAIL,
}

impl BlendMode {
    /// The mode's number in shaders (`BLEND_<NAME>` in WGSL).
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// The mode with number `id`.
    pub fn from_id(id: u32) -> Option<BlendMode> {
        BlendMode::ALL.iter().copied().find(|m| m.id() == id)
    }

    /// Whether the mode works on each channel alone. The others need the whole colour.
    pub fn is_separable(self) -> bool {
        !matches!(
            self,
            BlendMode::Hue
                | BlendMode::Saturation
                | BlendMode::Color
                | BlendMode::Luminosity
                | BlendMode::DarkerColor
                | BlendMode::LighterColor
                | BlendMode::Tint
                | BlendMode::Value
                | BlendMode::NormalMapCombine
                | BlendMode::NormalMapDetail
                | BlendMode::NormalMapInverseDetail
        )
    }

    /// The modes that change the backdrop by the source's alpha or luma instead of laying the
    /// source over it.
    pub fn is_stencil(self) -> bool {
        matches!(self, BlendMode::StencilAlpha | BlendMode::StencilLuma | BlendMode::SilhouetteAlpha | BlendMode::SilhouetteLuma)
    }

    /// Whether compositing needs the backdrop's colour: false for the modes that lay the source
    /// over it unchanged.
    pub fn reads_backdrop(self) -> bool {
        !matches!(self, BlendMode::PassThrough | BlendMode::Normal | BlendMode::Dissolve | BlendMode::DancingDissolve | BlendMode::Replace)
    }

    /// The modes After Effects keeps over-range (32 bpc) values for; the others clamp their
    /// inputs to 0..1 first, as After Effects documents. Overlay, Exclusion, Luminosity and the
    /// Color Burns keep their plain formula above 1 (#494).
    pub(crate) fn after_effects_keeps_over_range(self) -> bool {
        matches!(
            self,
            BlendMode::Normal
                | BlendMode::Dissolve
                | BlendMode::DancingDissolve
                | BlendMode::Darken
                | BlendMode::Multiply
                | BlendMode::ColorBurn
                | BlendMode::ClassicColorBurn
                | BlendMode::LinearBurn
                | BlendMode::DarkerColor
                | BlendMode::Add
                | BlendMode::Lighten
                | BlendMode::Screen
                | BlendMode::LinearDodge
                | BlendMode::LighterColor
                | BlendMode::Difference
                | BlendMode::ClassicDifference
                | BlendMode::Overlay
                | BlendMode::Exclusion
                | BlendMode::Luminosity
                | BlendMode::Subtract
                | BlendMode::Divide
                | BlendMode::StencilAlpha
                | BlendMode::StencilLuma
                | BlendMode::SilhouetteAlpha
                | BlendMode::SilhouetteLuma
                | BlendMode::AlphaAdd
                | BlendMode::LuminescentPremul
        )
    }

    /// The source colour a Classic mode leaves the backdrop unchanged with. After Effects fades
    /// the source toward it by the layer's coverage and then applies the formula at full
    /// strength, instead of mixing the full-strength result by coverage (#494).
    pub(crate) fn classic_neutral(self) -> Option<f32> {
        match self {
            BlendMode::ClassicColorDodge | BlendMode::ClassicDifference => Some(0.0),
            BlendMode::ClassicColorBurn => Some(1.0),
            _ => None,
        }
    }
}

/// Which app's formulas a mode follows where the apps differ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Variant {
    /// W3C Compositing where it defines a mode, Premiere's formulas elsewhere.
    #[default]
    Standard = 0,
    /// Photoshop, as measured on its rendered composites (psd-tools blend-modes corpus).
    Photoshop = 1,
    /// After Effects, including its 32 bpc rules (#494).
    AfterEffects = 2,
    /// Substance: signed results, no clipping, normal-map modes.
    Substance = 3,
}

impl Variant {
    /// The variant's number in shaders (`BLEND_VARIANT`).
    pub const fn id(self) -> u32 {
        self as u32
    }
}

/// The values a document holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Range {
    /// 0..1: integer depths and display-encoded colour. Add and Divide clip at 1, ClipColor
    /// keeps colours within 1.
    #[default]
    Unit,
    /// Values above 1 are light (Photoshop's 32-bit documents): Add and Divide stop at
    /// `f32::MAX` only, ClipColor pulls in negatives only, and overflow is recomputed in f64.
    Float,
}

impl Range {
    /// The brightest value a result keeps.
    pub(crate) fn top(self) -> f32 {
        match self {
            Range::Unit => 1.0,
            Range::Float => f32::MAX,
        }
    }
}
