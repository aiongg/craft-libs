//! Stories of the blend mode keys (task ST-AD2): not registered. Registering it (task RG2) adds
//! `blend_keys_stories` to the crate's `register!` list, [`COMPONENTS`] to the registry and
//! [`BlendKeysStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! A fake layer and a fake brush tool, each with its own entry list (the brush's with Clear after
//! Dissolve), a switch between them (the `target` arg), the current mode of each, and a log of what
//! the keys did. Both lists leave out most of the 26 positions, so a letter of the table that the
//! target lacks is consumed and logged as nothing (⇧⌥R on the layer, ⇧⌥F on the brush). A text
//! field keeps the keys. Every string goes through story words.
//!
//! Ids start with their level (the gallery requires it), so the spec's `imaging/blend-keys` is:
//!
//! | Story id | Component | Shows |
//! |---|---|---|
//! | `patterns/blend-keys/stepping` | `BlendKeys` | ⇧+ and ⇧− stepping with wrap, ⇧⌥ letters, a letter the target lacks (consumed, logged as nothing), and a text field that keeps the keys |

use craft_theme::TypeRole;
use craft_ui::focus::key_owner::{self, KeyOwner, OwnerGate};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::{Align2, CornerRadius, Event, Sense, TextEdit, Ui, vec2};

use super::blend_keys::{BlendKeyEntry, BlendKeyPick, BlendKeys, BlendShortcut};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["BlendKeys"];

craft_ui::words! {
    /// The captions of the blend key story, English by default.
    #[non_exhaustive]
    pub(crate) struct BlendKeysStoryLabels {
        normal: "blendKeysStory.normal" = "Normal";
        dissolve: "blendKeysStory.dissolve" = "Dissolve";
        clear: "blendKeysStory.clear" = "Clear";
        darken: "blendKeysStory.darken" = "Darken";
        multiply: "blendKeysStory.multiply" = "Multiply";
        screen: "blendKeysStory.screen" = "Screen";
        overlay: "blendKeysStory.overlay" = "Overlay";
        soft_light: "blendKeysStory.softLight" = "Soft Light";
        difference: "blendKeysStory.difference" = "Difference";
        hue: "blendKeysStory.hue" = "Hue";
        subtract: "blendKeysStory.subtract" = "Subtract";
        layer: "blendKeysStory.layer" = "Layer: {mode}";
        brush: "blendKeysStory.brush" = "Brush tool: {mode}";
        hint: "blendKeysStory.hint" = "Press ⇧+ or ⇧−, or ⇧⌥ with N, I, R, K, M, S, O, F, E or U. The brush lists Clear; the layer does not.";
        stepped: "blendKeysStory.stepped" = "Stepped to {mode}";
        lettered: "blendKeysStory.lettered" = "Letter picked {mode}";
        nothing: "blendKeysStory.nothing" = "Key consumed, no mode picked";
        no_keys: "blendKeysStory.noKeys" = "No blend key pressed yet.";
        text_hint: "blendKeysStory.textHint" = "Type here: the keys type text";
    }
}

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("patterns/blend-keys/stepping", Level::Patterns, "BlendKeys", "Stepping", Args { target: Target::Layer }, show)]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Target {
    Layer,
    Brush,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct Args {
    /// The target the keys act on.
    target: Target,
}

/// The story's key type: the app's own mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Normal,
    Dissolve,
    Clear,
    Darken,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    Difference,
    Hue,
    Subtract,
}

impl Mode {
    fn label(self, words: &BlendKeysStoryLabels) -> &str {
        match self {
            Mode::Normal => &words.normal,
            Mode::Dissolve => &words.dissolve,
            Mode::Clear => &words.clear,
            Mode::Darken => &words.darken,
            Mode::Multiply => &words.multiply,
            Mode::Screen => &words.screen,
            Mode::Overlay => &words.overlay,
            Mode::SoftLight => &words.soft_light,
            Mode::Difference => &words.difference,
            Mode::Hue => &words.hue,
            Mode::Subtract => &words.subtract,
        }
    }
}

/// The target's Mode menu, in menu order.
fn entries(target: Target) -> Vec<BlendKeyEntry<Mode>> {
    let lettered = |mode, shortcut| BlendKeyEntry::new(mode, shortcut);
    match target {
        Target::Layer => vec![
            lettered(Mode::Normal, BlendShortcut::Normal),
            lettered(Mode::Dissolve, BlendShortcut::Dissolve),
            lettered(Mode::Darken, BlendShortcut::Darken),
            lettered(Mode::Multiply, BlendShortcut::Multiply),
            lettered(Mode::Screen, BlendShortcut::Screen),
            lettered(Mode::Overlay, BlendShortcut::Overlay),
            lettered(Mode::SoftLight, BlendShortcut::SoftLight),
            lettered(Mode::Difference, BlendShortcut::Difference),
            lettered(Mode::Hue, BlendShortcut::Hue),
            BlendKeyEntry::unlettered(Mode::Subtract),
        ],
        Target::Brush => vec![
            lettered(Mode::Normal, BlendShortcut::Normal),
            lettered(Mode::Dissolve, BlendShortcut::Dissolve),
            BlendKeyEntry::clear(Mode::Clear),
            lettered(Mode::Darken, BlendShortcut::Darken),
            lettered(Mode::Multiply, BlendShortcut::Multiply),
            lettered(Mode::Screen, BlendShortcut::Screen),
            lettered(Mode::Overlay, BlendShortcut::Overlay),
            lettered(Mode::Hue, BlendShortcut::Hue),
        ],
    }
}

/// What the story keeps for one instance.
struct Rig {
    /// The layer's and the brush's current mode.
    modes: [Mode; 2],
    log: Vec<String>,
    text: String,
}

/// Lines of the log kept.
const LOG_LINES: usize = 6;

/// How many presses of this pass the blend keys will read: ⇧+ and ⇧− and ⇧⌥ with a letter of the
/// table (by layout or physical key), while the app owns the keyboard.
fn blend_presses(ctx: &egui::Context) -> usize {
    if key_owner::owner(ctx, &OwnerGate::default()) != KeyOwner::App {
        return 0;
    }
    ctx.input(|i| {
        i.events
            .iter()
            .filter(|event| match event {
                Event::Key { key, physical_key, pressed: true, modifiers, .. } if modifiers.shift && !modifiers.command => {
                    let step = matches!(key, egui::Key::Plus | egui::Key::Equals | egui::Key::Minus);
                    let letter = BlendShortcut::from_key(*key).is_some() || physical_key.and_then(BlendShortcut::from_key).is_some();
                    if modifiers.alt { letter } else { step }
                }
                _ => false,
            })
            .count()
    })
}

fn show(ui: &mut Ui, args: &mut Args) {
    let words = BlendKeysStoryLabels::of(ui);
    let state = shared(ui, "blend-keys", || Rig { modes: [Mode::Normal; 2], log: Vec::new(), text: String::new() });
    let mut guard = lock(&state);
    let rig = &mut *guard;
    let slot = usize::from(args.target == Target::Brush);

    // The keys run before anything else reads them, as an app does before its tool letters.
    let ctx = ui.ctx().clone();
    let pressed = blend_presses(&ctx);
    let list = entries(args.target);
    let pick = BlendKeys::new(&list).current(&rig.modes[slot]).consume(&ctx);
    let line = match &pick {
        Some(BlendKeyPick::Stepped(mode)) => Some(craft_i18n::fmt(&words.stepped, &[("mode", mode.label(&words))])),
        Some(BlendKeyPick::Lettered(mode)) => Some(craft_i18n::fmt(&words.lettered, &[("mode", mode.label(&words))])),
        None if pressed > 0 => Some(words.nothing.clone()),
        None => None,
    };
    if let Some(pick) = pick {
        rig.modes[slot] = pick.into_key();
    }
    if let Some(line) = line {
        rig.log.insert(0, line);
        rig.log.truncate(LOG_LINES);
    }

    // The fake layer or brush tool: the canvas of this story.
    let tokens = Tokens::of(ui);
    let caption = if slot == 0 { &words.layer } else { &words.brush };
    let text = craft_i18n::fmt(caption, &[("mode", rig.modes[slot].label(&words))]);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ui.spacing().interact_size.y * 3.0), Sense::click());
    key_owner::register_canvas(&ctx, response.id);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, tokens.color.surface.sunken.to_egui());
    painter.text(rect.center(), Align2::CENTER_CENTER, text, tokens.font(TypeRole::Body), tokens.color.text.normal.to_egui());

    ui.add(Text::new(words.hint.as_str()).content(Content::Muted));
    ui.add(TextEdit::singleline(&mut rig.text).hint_text(words.text_hint.as_str()));
    if rig.log.is_empty() {
        ui.add(Text::new(words.no_keys.as_str()).content(Content::Muted));
    }
    for line in &rig.log {
        ui.add(Text::new(line.as_str()));
    }
}
