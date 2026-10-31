//! [`GrayModel`]: the grey level of a colour.

use craft_theme::{Channel, ColorModel, DefaultOn, MAX_CHANNELS, ModelValues, Srgba};

/// Grayscale, as Photoshop's Color panel Grayscale Slider: one channel, K, the black ink in
/// percent (0 white, 100 black). A colour reads as the luma of its encoded sRGB values (Rec. 601
/// weights); K converts back to the equal grey. Offered in no picker unless the host asks.
#[derive(Clone, Copy, Debug)]
pub struct GrayModel;

const GRAY_CHANNELS: [Channel; 1] =
    [Channel { name: "Gray", qualified: "Grayscale gray", label: "K", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 }];

impl ColorModel for GrayModel {
    fn id(&self) -> &'static str {
        "gray"
    }
    fn name(&self) -> &'static str {
        "Grayscale"
    }
    fn channels(&self) -> &[Channel] {
        &GRAY_CHANNELS
    }
    fn from_srgb(&self, color: Srgba) -> ModelValues {
        let luma = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
        ModelValues::new(self.id(), &[(1.0 - luma) * 100.0])
    }
    fn to_srgb(&self, values: &[f32; MAX_CHANNELS]) -> Srgba {
        let level = 1.0 - values[0] / 100.0;
        Srgba::rgb(level, level, level)
    }
    fn default_on(&self) -> DefaultOn {
        DefaultOn { full: false, compact: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_is_no_ink_and_a_grey_converts_back_to_itself() {
        let white = GrayModel.from_srgb(Srgba::rgb(1.0, 1.0, 1.0));
        assert!(white.values[0].abs() < 1e-4);
        let half = GrayModel.from_srgb(Srgba::rgb(0.5, 0.5, 0.5));
        assert!((half.values[0] - 50.0).abs() < 1e-3);
        let back = GrayModel.to_srgb(&half.values);
        assert!((back.r - 0.5).abs() < 1e-4 && back.r == back.g && back.g == back.b);
    }
}
