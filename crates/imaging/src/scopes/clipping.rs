//! The clipping rule for one pixel ([`clipping`], [`ChannelMask`]) and [`ClippingMask`].

/// A set of the red, green and blue channels, as bits (bit 0 red, 1 green, 2 blue), PhotoCraft's
/// clipping masks. Serialised as its bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ChannelMask(pub u8);

impl ChannelMask {
    /// No channel.
    pub const NONE: ChannelMask = ChannelMask(0);
    /// Red.
    pub const R: ChannelMask = ChannelMask(1);
    /// Green.
    pub const G: ChannelMask = ChannelMask(2);
    /// Blue.
    pub const B: ChannelMask = ChannelMask(4);
    /// All three.
    pub const RGB: ChannelMask = ChannelMask(7);

    /// Whether every channel of `other` is in the mask.
    pub const fn contains(self, other: ChannelMask) -> bool {
        self.0 & other.0 == other.0
    }

    /// The bits.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether no channel is in the mask.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The channels in either mask.
    pub const fn union(self, other: ChannelMask) -> ChannelMask {
        ChannelMask(self.0 | other.0)
    }
}

/// Which channels of one pixel clip, at each end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClippedChannels {
    /// Channels at or below 0.
    pub shadows: ChannelMask,
    /// Channels at or above 1.
    pub highlights: ChannelMask,
}

/// The clipped channels of one straight (unmultiplied) RGBA pixel in 0..=1: a channel clips in the
/// shadows at a value `<= 0` and in the highlights at `>= 1`, the exact endpoints, not rounded
/// edge bins. A transparent pixel (alpha `<= 0`) or one with any component not finite clips
/// nothing.
// pc: crates/algo/src/histogram.rs:109 `clipping` (copied; the skeleton writes it, spec "Skeleton parts").
pub fn clipping(pixel: [f32; 4]) -> ClippedChannels {
    if pixel[3] <= 0.0 || !pixel.iter().all(|v| v.is_finite()) {
        return ClippedChannels::default();
    }
    let mut out = ClippedChannels::default();
    for (channel, value) in pixel.iter().take(3).enumerate() {
        let bit = ChannelMask(1 << channel);
        if *value <= 0.0 {
            out.shadows = out.shadows.union(bit);
        }
        if *value >= 1.0 {
            out.highlights = out.highlights.union(bit);
        }
    }
    out
}

/// The clipped channels of every pixel of an image, row by row: what the clipping overlay draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClippingMask {
    /// Width and height in pixels.
    pub size: [usize; 2],
    /// Each pixel's shadow channels, `size[0] × size[1]` long.
    pub shadows: Vec<ChannelMask>,
    /// Each pixel's highlight channels, the same length.
    pub highlights: Vec<ChannelMask>,
}

impl ClippingMask {
    /// A mask of `size` where nothing clips.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn new(size: [usize; 2]) -> Self {
        // new: two zeroed vectors of `size[0] * size[1]` (saturating).
        todo!()
    }

    /// [`clipping`] of each pixel of `pixels` (straight RGBA, row by row), which holds `size[0] ×
    /// size[1]` pixels; `None` when the length differs. Desktop builds may count rows in parallel
    /// later; the result is the same.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn from_rgba(size: [usize; 2], pixels: &[[f32; 4]]) -> Option<Self> {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:621-623 (`clipping(*p)` per proxy pixel).
        todo!()
    }

    /// The pixel at column `x`, row `y`; nothing clips outside the mask.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn at(&self, x: usize, y: usize) -> ClippedChannels {
        // new: index `y * size[0] + x`.
        todo!()
    }

    /// The channels that clip anywhere: what the warning triangles fill with.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn any(&self) -> ClippedChannels {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:402-403 (`sh`, `hi` folded from the per-channel counts).
        todo!()
    }
}
