//! `contract::tooltip`: what a component's tooltip says (3.4, 15.5); the rt tip (`rt/tip.rs`) shows it.
//!
//! A tooltip shows after one delay while the pointer rests on the component, or on keyboard focus after the same
//! delay, and at once while a neighbour's tooltip was hiding a moment ago. A press, a menu that owns keys and a
//! running drag hide it. A press on the component keeps its tooltip hidden until the pointer leaves it, so what the
//! press opens (a popover under a swatch) is not covered.
//!
//! A component with no tooltip shows its own label as the tooltip when its type declares `Component::SHOWS_LABEL`
//! false (or an instance does, through `Component::shows_label`): no text paints that name, inside a field or not. A
//! component with no label of its own is named by its field label, which is on screen, and shows none.
//! A component that paints its label shows it whole as the tooltip when the paint cut it. A canvas control shows no
//! label as its tooltip: its rect is the canvas, and its parts carry their own tooltips.

use super::TooltipSpec;
use craft_behavior::PartRef;
use craft_ui_rt::Rect;

/// Seconds the pointer rests on a component before its tooltip shows.
pub const DELAY: f64 = 0.6;
/// Seconds after a tooltip hides during which the next one shows at once.
pub const GRACE: f64 = 0.3;

/// The text a tooltip shows for `spec`: its disabled text while disabled.
pub(crate) fn resolve_spec(env: &crate::runtime::env::EnvView<'_>, spec: &TooltipSpec, enabled: bool) -> String {
    let label = match (&spec.disabled_text, enabled) {
        (Some(disabled), false) => disabled,
        _ => &spec.text,
    };
    env.resolve(label).as_str().to_owned()
}

/// The text the component's own tooltip shows, or `None` when it has none: its validity message when no container
/// shows it (`message`); else a base text with the bound command's chord, and the bound command's hint on a line
/// after it; else, with no base text, the hint alone with the chord. The base is the tooltip's text, or its disabled
/// text while disabled; else the full label when the paint cut it, or its own label when the component declares it
/// paints no label (`shows_label`), its face dropped the label (`PaintRecord::label_dropped`) or its command has a
/// hint that is not shown alone (`CommandState::hint_alone`).
///
/// A canvas control (`canvas_control`) never shows its label: its rect is the canvas it is shown over, not a face of
/// its own, so the label would show wherever the pointer rests on the page ("Rulers margins" over a whole page). Its
/// parts name themselves with their own tooltips (a margin edge's "Left Margin").
pub(crate) fn text_of(core: &super::CxCore<'_>, shows_label: bool, canvas_control: bool, message: Option<&str>) -> Option<String> {
    if let Some(message) = message {
        return Some(message.to_owned());
    }
    let hint = core.command.as_deref().and_then(|command| command.hint.as_deref()).filter(|hint| !hint.is_empty());
    let base = match core.resolved.tooltip.map(|tooltip| tooltip.spec()) {
        Some(spec) => Some(resolve_spec(&core.env, spec, core.resolved.enabled)),
        None if canvas_control => return None,
        None => {
            let paint = core.paint.borrow();
            // A hint shown alone (`CommandState::hint_alone`) does not bring the label that shows whole with it.
            let alone = hint.is_some() && core.command.as_deref().is_some_and(|command| command.hint_alone);
            let unpainted = !shows_label || paint.label_dropped || (hint.is_some() && !alone);
            paint.elided.clone().or_else(|| (unpainted && !core.resolved.label.is_empty()).then(|| core.resolved.label.to_owned()))
        }
    };
    let command = core.command.as_deref();
    let platform = core.context().os();
    let chorded = |text: &str| match command.and_then(|command| command.chord.as_ref()) {
        Some(chord) => with_chord(text, chord, platform),
        None => text.to_owned(),
    };
    match (base, hint) {
        (Some(base), Some(hint)) => Some(format!("{}\n{hint}", chorded(&base))),
        (Some(base), None) => Some(chorded(&base)),
        (None, Some(hint)) => Some(chorded(hint)),
        (None, None) => None,
    }
}

/// A tooltip's text followed by the chord that does the same, as every tooltip shows a key: "Hide Layers (⌘,)".
pub fn with_chord(base: &str, chord: &craft_keymap::Chord, os: craft_keymap::Platform) -> String {
    format!("{base} ({})", chord.display(os))
}

/// The tooltip of a member of a collection's run (`Collection::MEMBERS`) the pointer rests on: shown by the
/// collection's step 16 as a part's, under the collection's timer, with the member's part as its subject.
pub(crate) struct MemberTip {
    pub part: PartRef,
    pub spec: TooltipSpec,
    pub rect: Rect,
    pub pressed: bool,
}
