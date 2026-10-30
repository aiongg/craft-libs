//! The choices of a Preferences row for the interface scale.

use super::{Fit, InterfaceScale};
use crate::localized::LocaleNumbers;

/// The words of the scale row, translated by the host.
#[derive(Clone, Debug)]
pub struct InterfaceScaleLabels {
    /// The follow-the-display choice: "Auto".
    pub auto: String,
    /// A percent, `{n}` standing for the number: "{n}%".
    pub percent: String,
}

impl Default for InterfaceScaleLabels {
    fn default() -> Self {
        Self { auto: "Auto".into(), percent: "{n}%".into() }
    }
}

impl InterfaceScale {
    /// Choices for a Preferences `RowKind::Choice` as `(code, label)`: `Auto`, then the steps that
    /// `fit` allows. `current` stays listed once, in its place, when it is off the steps or `fit`
    /// does not allow it, so a saved value can be seen and changed.
    pub fn options(current: InterfaceScale, labels: &InterfaceScaleLabels, numbers: &LocaleNumbers, fit: Fit) -> Vec<(String, String)> {
        let mut percents: Vec<u16> = Self::STEPS.iter().copied().filter(|&p| fit.allows(InterfaceScale::percent(p))).collect();
        if let InterfaceScale::Percent(p) = current {
            percents.push(p.get());
        }
        percents.sort_unstable();
        percents.dedup();
        let percent = |p: u16| (InterfaceScale::percent(p).code(), labels.percent.replace("{n}", &numbers.format(f64::from(p), 0..=0)));
        std::iter::once((InterfaceScale::Auto.code(), labels.auto.clone())).chain(percents.into_iter().map(percent)).collect()
    }
}
