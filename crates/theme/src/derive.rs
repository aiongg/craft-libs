//! Colour derivation: seeds to ramps, ramps to tokens, pins, then the contrast pass; and the
//! same for component colours: overrides, then their own contrast pass.

use std::collections::BTreeSet;

use super::color::{Oklab, Rgba};
use super::components::{ComponentField, ComponentTokens, FieldKind, OverrideValue};
use super::contrast::{contrast_ratio, ensure_contrast};
use super::ramp::{Mode, Ramp, RampKind};
use super::theme::{Contrast, Pins, ThemeDef};
use super::tokens::{ColorToken, ColorTokens, Status, TokenRef};
use super::values::{self, Bg, PairRule, RULES, Source};
use crate::AppId;

/// A contrast pair that is below its minimum after derivation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContrastWarning {
    /// The foreground token or field.
    pub fg: TokenRef,
    /// The background token or field. Overlay pairs name `overlay.halo` and report the lower of
    /// the ratios over black and over white.
    pub bg: TokenRef,
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
pub(super) fn colors(theme: &ThemeDef, app: Option<&AppId>, mode: Mode, contrast: Contrast) -> (ColorTokens, Vec<ContrastWarning>) {
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
    for (t, v) in pins(theme, app, mode, high) {
        if let TokenRef::Color(t) = t {
            c.set(t, v);
            pinned.insert(t);
        }
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

/// The mode's pins, then the app's, then, when `high`, the theme's and the app's high-contrast
/// pins; later ones win.
fn pins<'a>(theme: &'a ThemeDef, app: Option<&AppId>, mode: Mode, high: bool) -> impl Iterator<Item = (TokenRef, Rgba)> + 'a {
    let app = app.and_then(|a| theme.apps.get(a));
    let high_pins = high.then(|| [Some(&theme.high_contrast), app.map(|o| &o.high_contrast)]).into_iter().flatten().flatten();
    std::iter::once(theme.pins.get(mode))
        .chain(app.map(|o| o.pins.get(mode)))
        .chain(high_pins.map(move |p| p.get(mode)))
        .flat_map(|p: &Pins| p.0.iter().map(|(&t, &v)| (t, v)))
}

/// The value of a source that needs no other token, or `None` for one that does.
fn direct(r: &Ramps, src: Source) -> Option<Rgba> {
    Some(match src {
        Source::Neutral(n) => r.neutral.step(n),
        Source::NeutralAlpha(n, a) => r.neutral.step(n).with_alpha(values::alpha(a)),
        Source::Accent(n) => r.accent.step(n),
        Source::AccentMix { from, to, t } => mix(r.accent.step(from), r.accent.step(to), t),
        Source::Status(kind, n) => r.status.get(kind).step(n),
        Source::StatusMix { kind, from, to, t } => mix(r.status.get(kind).step(from), r.status.get(kind).step(to), t),
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
    let worst = |e: Rgba| solids.iter().map(|&s| contrast_ratio(e, bg_color(c, Bg::Token(s)))).fold(f64::INFINITY, f64::min);
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

/// The colour a background presents: a translucent token over `surface.panel`, as the
/// component pass measures it. Surfaces are opaque, so they present themselves.
fn bg_color(c: &ColorTokens, bg: Bg) -> Rgba {
    match bg {
        Bg::Token(t) => over(c.get(t), c.surface.panel),
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

/// The lowest ratio of `fg` over `bgs`; infinite when there are none.
fn worst_ratio(c: &ColorTokens, fg: Rgba, bgs: &[Bg]) -> f64 {
    worst(c, fg, bgs).map_or(f64::INFINITY, |(r, _)| r)
}

fn minimum(rule: &PairRule, high: bool) -> f64 {
    if high { rule.high } else { rule.standard }
}

/// Applies one rule. A pinned foreground is left alone in standard contrast and fixed in high.
fn fix_rule(c: &mut ColorTokens, pinned: &BTreeSet<ColorToken>, rule: &PairRule, done: &[PairRule], high: bool) {
    let min = minimum(rule, high);
    // A text rule may move a state fill in high contrast when the text alone can't reach `min`.
    let moves_fills = high && !rule.on_solid;
    for round in 0..2 {
        for &fg in rule.fgs {
            if pinned.contains(&fg) && !high {
                // Pinned on-solid text stays; the unpinned solids under it still move.
                if rule.on_solid {
                    move_solids(c, pinned, c.get(fg), rule.bgs, min, false);
                }
                continue;
            }
            if rule.on_solid {
                fix_on_solid(c, pinned, fg, rule.bgs, min, done, high);
            } else {
                fix_fg(c, fg, rule.bgs, min);
                if moves_fills {
                    relax_state_fills(c, fg, rule.bgs, min);
                }
            }
        }
        // The second round re-fixes foregrounds that a moved fill pushed below the minimum.
        if round == 0 && !moves_fills {
            break;
        }
    }
}

/// Moves each state fill among `bgs` that `fg` still fails on, away from `fg`, just far enough.
/// A translucent fill that moves becomes opaque.
fn relax_state_fills(c: &mut ColorTokens, fg: ColorToken, bgs: &[Bg], min: f64) {
    use ColorToken::{StateHover, StatePressed, StateSelected, StateSelectedHover};
    for &bg in bgs {
        let Bg::Token(t @ (StateHover | StatePressed | StateSelected | StateSelectedHover)) = bg else { continue };
        let (f, b) = (c.get(fg), bg_color(c, bg));
        if ratio(f, b) < min {
            c.set(t, ensure_contrast(b, f.with_alpha(255), min));
        }
    }
}

/// Moves each solid among `bgs` that `text` fails on away from it, just far enough; pinned
/// solids only when `high`. Moved solids are opaque.
fn move_solids(c: &mut ColorTokens, pinned: &BTreeSet<ColorToken>, text: Rgba, bgs: &[Bg], min: f64, high: bool) {
    for &b in bgs {
        let (solid, bg) = (bg_token(b), bg_color(c, b));
        if (high || !pinned.contains(&solid)) && ratio(text, bg) < min {
            c.set(solid, ensure_contrast(bg, text.with_alpha(255), min));
        }
    }
}

/// Moves `fg`'s lightness until it passes on every background, fixing against the hardest one
/// each round. A translucent `fg` that has to move becomes opaque so the move is exact.
///
/// When no colour passes everywhere, `fg` ends as the colour with the best lowest ratio among
/// its value before the pass, each round's value, white and black, preferring the value before
/// the pass on ties. A candidate with a lower ratio on the surfaces among `bgs` than the value
/// before the pass is never chosen: text that reads on the surfaces is not traded for a fill.
fn fix_fg(c: &mut ColorTokens, fg: ColorToken, bgs: &[Bg], min: f64) {
    let before = c.get(fg);
    let mut tried = vec![before];
    for _ in 0..MAX_ROUNDS {
        let f = c.get(fg);
        let Some((r, bg)) = worst(c, f, bgs) else { return };
        if r >= min {
            return;
        }
        let moved = ensure_contrast(f.with_alpha(255), bg, min);
        if moved == f {
            break;
        }
        c.set(fg, moved);
        tried.push(moved);
    }
    if worst_ratio(c, c.get(fg), bgs) >= min {
        return;
    }
    // Backgrounds on opposite sides of the foreground (the halo over black and over white) can
    // leave the hue-keeping move short; white or black then reaches the minimum when any colour does.
    let best = if worst_ratio(c, Rgba::WHITE, bgs) >= worst_ratio(c, Rgba::BLACK, bgs) { Rgba::WHITE } else { Rgba::BLACK };
    if worst_ratio(c, best, bgs) >= min {
        c.set(fg, best);
        return;
    }
    let surfaces: Vec<Bg> = bgs.iter().copied().filter(|&b| matches!(b, Bg::Token(t) if t.is_opaque_surface())).collect();
    let floor = worst_ratio(c, before, &surfaces);
    let mut keep = (before, worst_ratio(c, before, bgs));
    for &candidate in tried[1..].iter().chain(&[Rgba::WHITE, Rgba::BLACK]) {
        let r = worst_ratio(c, candidate, bgs);
        if r > keep.1 && worst_ratio(c, candidate, &surfaces) >= floor {
            keep = (candidate, r);
        }
    }
    c.set(fg, keep.0);
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
        move_solids(&mut trial, pinned, extreme, bgs, min, high);
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
                let bg = TokenRef::Color(bg_token(bg));
                match seen.iter_mut().find(|w| w.bg == bg) {
                    Some(w) => w.ratio = w.ratio.min(r),
                    None => seen.push(ContrastWarning { fg: TokenRef::Color(fg), bg, ratio: r, minimum }),
                }
            }
            out.extend(seen);
        }
    }
    out
}

/// Applies `theme`'s component overrides in `mode` for `app`: colour pins (in [`pins`] order),
/// then the mode-independent overrides, the app's last. Overrides that
/// [`ThemeDef::override_errors`] lists are skipped.
pub(super) fn component_overrides(comp: &mut ComponentTokens, theme: &ThemeDef, app: Option<&AppId>, mode: Mode, high: bool, color: &ColorTokens) {
    // `set` rejects pins on non-colour fields and values of the wrong kind; those keep the default.
    for (t, v) in pins(theme, app, mode, high) {
        if let TokenRef::Component(f) = t {
            let _ = comp.set(f, OverrideValue::Color(v), color);
        }
    }
    let app_components = app.and_then(|a| theme.apps.get(a)).map(|o| &o.components);
    for (&f, &v) in theme.components.iter().chain(app_components.into_iter().flatten()) {
        if f.kind() != FieldKind::Color {
            let _ = comp.set(f, v, color);
        }
    }
}

/// The component contrast pass. Defaults derive from semantic tokens that already passed or
/// were reported by the semantic pass. With standard contrast, failing pairs are kept and those
/// with a component override (a fg or bg that differs from `defaults`) are reported. With high
/// contrast every failing pair is fixed: the foreground moves first, then the background if the
/// foreground alone can't reach the minimum. Returns the pairs that still fail.
pub(super) fn component_colors(comp: &mut ComponentTokens, defaults: &ComponentTokens, color: &ColorTokens, high: bool) -> Vec<ContrastWarning> {
    let pairs = ComponentTokens::pairs();
    if high {
        for p in &pairs {
            fix_component_pair(comp, color, p.fg, p.bg, p.minimum.1);
        }
    }
    let mut out = Vec::new();
    for p in &pairs {
        let minimum = if high { p.minimum.1 } else { p.minimum.0 };
        let (Some(fg), Some(bg)) = (comp.color(p.fg), component_bg(comp, color, p.bg)) else { continue };
        let overridden = |f: ComponentField| comp.color(f) != defaults.color(f);
        let own = overridden(p.fg) || matches!(p.bg, TokenRef::Component(b) if overridden(b));
        let r = ratio(fg, bg);
        if r < minimum && (high || own) {
            out.push(ContrastWarning { fg: TokenRef::Component(p.fg), bg: p.bg, ratio: r, minimum });
        }
    }
    out
}

/// The colour a component pair's background presents: translucent fills over `surface.panel`.
fn component_bg(comp: &ComponentTokens, color: &ColorTokens, bg: TokenRef) -> Option<Rgba> {
    let c = match bg {
        TokenRef::Color(t) => color.get(t),
        TokenRef::Component(f) => comp.color(f)?,
    };
    Some(over(c, color.surface.panel))
}

/// Moves `fg`, then a component `bg`, until the pair reaches `min`. Moved colours are opaque.
fn fix_component_pair(comp: &mut ComponentTokens, color: &ColorTokens, fg: ComponentField, bg: TokenRef, min: f64) {
    let (Some(f), Some(b)) = (comp.color(fg), component_bg(comp, color, bg)) else { return };
    if ratio(f, b) >= min {
        return;
    }
    let moved = ensure_contrast(f.with_alpha(255), b, min);
    let _ = comp.set(fg, OverrideValue::Color(moved), color);
    if let TokenRef::Component(bg) = bg
        && ratio(moved, b) < min
    {
        let _ = comp.set(bg, OverrideValue::Color(ensure_contrast(b, moved, min)), color);
    }
}
