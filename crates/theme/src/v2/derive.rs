//! Colour derivation: seeds to ramps, ramps to tokens, pins, then the contrast pass.

use std::collections::BTreeSet;

use super::color::{Oklab, Rgba};
use super::contrast::{contrast_ratio, ensure_contrast};
use super::ramp::{Mode, Ramp, RampKind};
use super::theme::{Contrast, ThemeDef};
use super::tokens::{ColorToken, ColorTokens, Status};
use super::values::{self, Bg, PairRule, RULES, Source};

/// A contrast pair that is below its minimum after derivation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContrastWarning {
    /// The foreground token.
    pub fg: ColorToken,
    /// The background token. Overlay pairs name `overlay.halo` and report the lower of the
    /// ratios over black and over white.
    pub bg: ColorToken,
    /// The measured ratio.
    pub ratio: f64,
    /// The required ratio.
    pub minimum: f64,
}

/// Most times a foreground is moved while fixing it against several backgrounds.
const MAX_ROUNDS: usize = 8;

struct Ramps {
    neutral: Ramp,
    accent: Ramp,
    status: Status<Ramp>,
}

/// Derives every colour of `theme` in `mode`: ramp defaults, pins, then the contrast pass.
/// Returns the colours and the pairs that still fail.
pub(super) fn colors(theme: &ThemeDef, mode: Mode, contrast: Contrast) -> (ColorTokens, Vec<ContrastWarning>) {
    let seeds = &theme.seeds;
    let ramps = Ramps {
        neutral: Ramp::from_seed(seeds.neutral, mode, RampKind::Neutral),
        accent: Ramp::from_seed(seeds.accent, mode, RampKind::Chromatic),
        status: seeds.status.map(|&s| Ramp::from_seed(s, mode, RampKind::Chromatic)),
    };
    let high = contrast == Contrast::High;

    let mut pinned = BTreeSet::new();
    let mut c = ColorTokens::splat(Rgba::BLACK);
    for &t in ColorToken::ALL {
        c.set(t, direct(&ramps, values::source(t, mode)).unwrap_or(Rgba::BLACK));
    }
    let mut pins: Vec<_> = theme.pins.get(mode).0.iter().collect();
    if high {
        pins.extend(theme.high_contrast.get(mode).0.iter());
    }
    for (&t, &v) in pins {
        c.set(t, v);
        pinned.insert(t);
    }
    // Tokens defined by other tokens follow their pinned values; aliases follow the contrast
    // pass too. On-solid text is the pass's own choice, so it is not re-derived afterwards.
    follow(&mut c, &pinned, mode, true);
    for (i, rule) in RULES.iter().enumerate() {
        fix_rule(&mut c, &pinned, rule, &RULES[..=i], high);
    }
    follow(&mut c, &pinned, mode, false);
    let warnings = failures(&c, RULES, high);
    (c, warnings)
}

/// The value of a source that needs no other token, or `None` for one that does.
fn direct(r: &Ramps, src: Source) -> Option<Rgba> {
    Some(match src {
        Source::Neutral(n) => r.neutral.step(n),
        Source::NeutralAlpha(n, a) => r.neutral.step(n).with_alpha(values::alpha(a)),
        Source::Accent(n) => r.accent.step(n),
        Source::AccentMix { from, to, t } => mix(r.accent.step(from), r.accent.step(to), t),
        Source::Status(kind, n) => r.status.get(kind).step(n),
        Source::Fixed(c) => c,
        Source::Oklch(l, c, h) => super::color::Oklch { l, c, h }.to_rgba(),
        Source::Same(_) | Source::OnSolid(_) => return None,
    })
}

/// Sets every unpinned token whose source refers to other tokens; on-solid text only when
/// `on_solid`.
fn follow(c: &mut ColorTokens, pinned: &BTreeSet<ColorToken>, mode: Mode, on_solid: bool) {
    for &t in ColorToken::ALL {
        if pinned.contains(&t) {
            continue;
        }
        match values::source(t, mode) {
            Source::Same(other) => c.set(t, c.get(other)),
            Source::OnSolid(solids) if on_solid => c.set(t, best_extreme(c, solids)),
            _ => {}
        }
    }
}

/// White or black, whichever has the higher lowest ratio on `solids`.
fn best_extreme(c: &ColorTokens, solids: &[ColorToken]) -> Rgba {
    let worst = |e: Rgba| solids.iter().map(|&s| contrast_ratio(e, c.get(s))).fold(f64::INFINITY, f64::min);
    if worst(Rgba::WHITE) >= worst(Rgba::BLACK) { Rgba::WHITE } else { Rgba::BLACK }
}

/// Linear interpolation in OKLab, opaque.
fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let (a, b) = (a.to_oklab(), b.to_oklab());
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    Oklab { l: lerp(a.l, b.l), a: lerp(a.a, b.a), b: lerp(a.b, b.b) }.to_oklch().to_rgba()
}

/// `fg` alpha-composited over `bg` (sRGB channel blend, as renderers draw it); the result is
/// opaque when `bg` is.
fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f64::from(fg.a()) / 255.0;
    let ch = |i: usize| (f64::from(fg.0[i]) * a + f64::from(bg.0[i]) * (1.0 - a)).round() as u8;
    Rgba([ch(0), ch(1), ch(2), bg.a()])
}

/// The colour a background presents.
fn bg_color(c: &ColorTokens, bg: Bg) -> Rgba {
    match bg {
        Bg::Token(t) => c.get(t),
        Bg::HaloOver(backdrop) => over(c.get(ColorToken::OverlayHalo), backdrop),
    }
}

/// The token a background reports as.
fn bg_token(bg: Bg) -> ColorToken {
    match bg {
        Bg::Token(t) => t,
        Bg::HaloOver(_) => ColorToken::OverlayHalo,
    }
}

/// The ratio of `fg` on `bg`, compositing a translucent `fg` first.
fn ratio(fg: Rgba, bg: Rgba) -> f64 {
    contrast_ratio(over(fg, bg), bg)
}

/// The lowest ratio of `fg` over `bgs`, and the background that gives it.
fn worst(c: &ColorTokens, fg: Rgba, bgs: &[Bg]) -> Option<(f64, Rgba)> {
    bgs.iter().map(|&b| bg_color(c, b)).map(|bg| (ratio(fg, bg), bg)).min_by(|x, y| x.0.total_cmp(&y.0))
}

fn minimum(rule: &PairRule, high: bool) -> f64 {
    if high { rule.high } else { rule.standard }
}

/// Applies one rule. Pinned foregrounds are left alone unless `high`.
fn fix_rule(c: &mut ColorTokens, pinned: &BTreeSet<ColorToken>, rule: &PairRule, done: &[PairRule], high: bool) {
    let min = minimum(rule, high);
    for &fg in rule.fgs {
        if pinned.contains(&fg) && !high {
            continue;
        }
        if rule.on_solid {
            fix_on_solid(c, pinned, fg, rule.bgs, min, done, high);
        } else {
            fix_fg(c, fg, rule.bgs, min);
        }
    }
}

/// Moves `fg`'s lightness until it passes on every background, fixing against the hardest one
/// each round. A translucent `fg` that has to move becomes opaque so the move is exact.
fn fix_fg(c: &mut ColorTokens, fg: ColorToken, bgs: &[Bg], min: f64) {
    for _ in 0..MAX_ROUNDS {
        let f = c.get(fg);
        let Some((r, bg)) = worst(c, f, bgs) else { return };
        if r >= min {
            return;
        }
        let moved = ensure_contrast(f.with_alpha(255), bg, min);
        if moved == f {
            return;
        }
        c.set(fg, moved);
    }
}

/// Keeps text on solid fills that already passes; otherwise picks white or black. When an extreme can't reach `min` on a fill,
/// the fill moves away from it (if it may move). Of the two extremes, keeps the one that leaves
/// the fewest failing pairs among the rules fixed so far, preferring the derived default (the
/// one with more contrast on the step-9 solids).
fn fix_on_solid(c: &mut ColorTokens, pinned: &BTreeSet<ColorToken>, fg: ColorToken, bgs: &[Bg], min: f64, done: &[PairRule], high: bool) {
    let tokens: Vec<_> = bgs.iter().map(|&b| bg_token(b)).collect();
    let current = c.get(fg);
    if worst(c, current, bgs).is_none_or(|(r, _)| r >= min) {
        return;
    }
    let preferred = if current == Rgba::WHITE || current == Rgba::BLACK { current } else { best_extreme(c, &tokens) };
    let other = if preferred == Rgba::WHITE { Rgba::BLACK } else { Rgba::WHITE };
    let mut best: Option<(usize, ColorTokens)> = None;
    for extreme in [preferred, other] {
        let mut trial = *c;
        trial.set(fg, extreme);
        for &solid in &tokens {
            if (high || !pinned.contains(&solid)) && ratio(extreme, trial.get(solid)) < min {
                trial.set(solid, ensure_contrast(trial.get(solid), extreme, min));
            }
        }
        let fails = failures(&trial, done, high).len();
        if best.as_ref().is_none_or(|(f, _)| fails < *f) {
            best = Some((fails, trial));
        }
    }
    if let Some((_, trial)) = best {
        *c = trial;
    }
}

/// Every pair of `rules` below its minimum.
fn failures(c: &ColorTokens, rules: &[PairRule], high: bool) -> Vec<ContrastWarning> {
    let mut out = Vec::new();
    for rule in rules {
        let minimum = minimum(rule, high);
        for &fg in rule.fgs {
            let f = c.get(fg);
            // Both halo backdrops report as one pair, with the lower ratio.
            let mut seen: Vec<ContrastWarning> = Vec::new();
            for &bg in rule.bgs {
                let r = ratio(f, bg_color(c, bg));
                if r >= minimum {
                    continue;
                }
                let bg = bg_token(bg);
                match seen.iter_mut().find(|w| w.bg == bg) {
                    Some(w) => w.ratio = w.ratio.min(r),
                    None => seen.push(ContrastWarning { fg, bg, ratio: r, minimum }),
                }
            }
            out.extend(seen);
        }
    }
    out
}
