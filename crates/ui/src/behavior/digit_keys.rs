//! [`DigitKeys`]: Photoshop's number keys for opacity. With a painting tool (or another tool whose
//! options have an opacity) and no text field focused, 1 sets 10 % … 9 sets 90 %, 0 sets 100 %;
//! two digits typed within [`DigitKeys::TWO_DIGITS`] seconds set the exact value (4 then 5 is
//! 45 %, 0 then 0 is 0 %). Shift sends the digit to the second value (Flow, or Fill for a layer).
//! The app decides which value a press sets for the current tool.

use egui::{Context, Event, Key};

/// A number key entry: the percentage and whether Shift was held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DigitEntry {
    /// 0..=100.
    pub percent: f64,
    /// Shift was held: the second value (Flow, Fill).
    pub shift: bool,
}

/// The number keys' state between presses (the first digit of a pair). The app keeps one.
///
/// ```no_run
/// use craft_ui::behavior::DigitKeys;
/// # fn demo(ctx: &egui::Context, keys: &mut DigitKeys, painting_tool: bool) {
/// if let Some(entry) = keys.handle(ctx, painting_tool) {
///     // set the brush opacity (entry.shift: flow) to entry.percent
/// }
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DigitKeys {
    first: Option<(u8, bool, f64)>,
}

impl DigitKeys {
    /// Seconds within which a second digit makes an exact value.
    pub const TWO_DIGITS: f64 = 0.8;

    /// One digit press (`digit` 0..=9) at `now` (egui seconds): 10 × the digit (0: 100) on its
    /// own, or the two-digit value when it follows a first digit with the same Shift state in
    /// time. A digit over 9 is ignored (`None`).
    // pc: crates/ui-egui/src/input/opacity_keys.rs:90 `press`.
    pub fn press(&mut self, digit: u8, shift: bool, now: f64) -> Option<DigitEntry> {
        if digit > 9 {
            return None;
        }
        let percent = match self.first.take() {
            Some((first, s, at)) if s == shift && now - at < Self::TWO_DIGITS && now >= at => f64::from(first * 10 + digit),
            _ => {
                self.first = Some((digit, shift, now));
                if digit == 0 { 100.0 } else { f64::from(digit) * 10.0 }
            }
        };
        Some(DigitEntry { percent, shift })
    }

    /// Read this pass's digit key presses (main row and keypad, no Cmd/Ctrl/Alt, not repeats)
    /// when `enabled` (a tool with an opacity is current) and no widget has keyboard focus,
    /// consuming them. Returns the last entry of the pass.
    // pc: crates/ui-egui/src/input/opacity_keys.rs:61 `handle`, :44 `digit`.
    pub fn handle(&mut self, ctx: &Context, enabled: bool) -> Option<DigitEntry> {
        if !enabled || ctx.memory(|m| m.focused().is_some()) {
            return None;
        }
        let presses: Vec<(u8, bool)> = ctx.input_mut(|i| {
            let mut out = Vec::new();
            i.events.retain(|e| match e {
                // The physical key, so Shift+digit is still a digit whatever symbol the layout
                // puts there.
                Event::Key { key, physical_key, pressed: true, repeat, modifiers } if !modifiers.command && !modifiers.ctrl && !modifiers.alt => {
                    match physical_key.and_then(digit).or_else(|| digit(*key)) {
                        Some(d) => {
                            // A held key's auto-repeat is one digit, not a second one typed
                            // quickly; it is consumed all the same.
                            if !repeat {
                                out.push((d, modifiers.shift));
                            }
                            false
                        }
                        None => true,
                    }
                }
                _ => true,
            });
            out
        });
        let now = ctx.input(|i| i.time);
        presses.into_iter().filter_map(|(d, shift)| self.press(d, shift, now)).last()
    }
}

fn digit(key: Key) -> Option<u8> {
    Some(match key {
        Key::Num0 => 0,
        Key::Num1 => 1,
        Key::Num2 => 2,
        Key::Num3 => 3,
        Key::Num4 => 4,
        Key::Num5 => 5,
        Key::Num6 => 6,
        Key::Num7 => 7,
        Key::Num8 => 8,
        Key::Num9 => 9,
        _ => return None,
    })
}
