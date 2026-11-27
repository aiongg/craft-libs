//! Component keys of value types this crate hands to its components.

/// Plain value types with no `Hash`, keyed by their `Debug` text.
macro_rules! debug_key {
    ($($t:ty),* $(,)?) => {
        $(impl craft_ui::contract::MemoProps for $t {
            fn key(&self, key: &mut craft_ui::contract::MemoKey) {
                key.feed_hash(format!("{self:?}").as_str());
            }
        })*
    };
}
