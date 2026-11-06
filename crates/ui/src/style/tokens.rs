//! `UiTokens`: the runtime's egui-ready view of one resolved token set.

use craft_theme::{GroupValues, Tokens, TypeRole};
use egui::{CornerRadius, FontFamily, FontId};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

/// One resolved token set with the theme's token-group values and a key distinct per set. Each theme scope
/// (mode, contrast, theme, density, text size) resolves its own. It derefs to the craft-theme tokens, which only
/// `style` reads.
#[derive(Clone, Debug)]
pub struct UiTokens {
    tokens: Arc<Tokens>,
    groups: Arc<GroupValues>,
    key: u64,
}

impl UiTokens {
    /// A token set with no theme values for token groups.
    pub fn new(tokens: Arc<Tokens>) -> Self {
        UiTokens { tokens, groups: Arc::new(GroupValues::new()), key: NEXT_KEY.fetch_add(1, Ordering::Relaxed) }
    }

    /// The same tokens with the theme's values under token group paths. It is a different set: a new key.
    pub fn with_groups(&self, groups: GroupValues) -> Self {
        UiTokens { tokens: self.tokens.clone(), groups: Arc::new(groups), key: NEXT_KEY.fetch_add(1, Ordering::Relaxed) }
    }

    /// The source tokens.
    pub fn tokens(&self) -> &Arc<Tokens> {
        &self.tokens
    }

    /// The theme's token-group values.
    pub fn groups(&self) -> &GroupValues {
        &self.groups
    }

    /// Distinct per token set; the token-group and style caches key on it.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// The font of `role`, or the mono face for a mono run.
    pub(crate) fn font(&self, role: TypeRole, mono: bool) -> FontId {
        let style = if mono { self.typography.mono } else { self.typography.get(role) };
        let family = match style.font {
            craft_theme::FontSlot::Mono => FontFamily::Monospace,
            craft_theme::FontSlot::Ui | craft_theme::FontSlot::UiSemibold => FontFamily::Proportional,
        };
        FontId::new(style.size, family)
    }

    /// `px` as a corner radius, the same at every corner. A radius above 255 clamps; egui clamps a corner to half the shape.
    pub(crate) fn corner(px: f32) -> CornerRadius {
        CornerRadius::same(px.round().clamp(0.0, 255.0) as u8)
    }
}

impl Deref for UiTokens {
    type Target = Tokens;

    fn deref(&self) -> &Tokens {
        &self.tokens
    }
}
