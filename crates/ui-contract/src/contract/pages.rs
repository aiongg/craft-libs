//! `Pages`: pages that take turns in one place, which keeps the size of the largest so a switch neither moves nor
//! resizes what is around them.

use crate::contract::Ui;
use crate::runtime::memory::sub;
use craft_ui_rt::Vec2;
use std::hash::Hash;

/// Pages that take turns in one place, such as the paint kinds of a paint chip's popover: one shows, and the place
/// is as wide and as tall as the largest of them, so switching the page moves and resizes nothing around it. A child
/// of the shown page that fills its width takes the width of the largest page.
///
/// The pages it does not show are measured only: each runs in a measuring run of the build that shows the pages,
/// where kept children answer from their size cache and nothing is reached, sensed, painted or written. The page
/// closure reads app data the framework does not track, so the hidden pages are measured in every build of the
/// enclosing node, never reused from an earlier one.
///
/// The page closure runs for the shown page and for every hidden page it measures, so it builds what it shows from its
/// page argument alone and changes nothing outside a component's value (a measuring run applies no edit).
#[must_use]
pub struct Pages<'a, K> {
    salt: u64,
    pages: &'a [K],
    current: &'a K,
}

impl<'a, K: Hash + PartialEq> Pages<'a, K> {
    /// The pages `pages`, with `current` shown. `salt` names this set among the others in the same `Ui`.
    pub fn new(salt: impl Hash, pages: &'a [K], current: &'a K) -> Self {
        Pages { salt: hash(&salt), pages, current }
    }

    /// Shows the current page through `page` in a flow of its own placed in flow in `ui`, at the size of the largest
    /// page, and returns what `page` returned for it. Each page runs in the env of `ui`.
    pub fn show<R>(self, ui: &mut Ui, mut page: impl FnMut(&mut Ui, &K) -> R) -> R {
        let base = sub(ui.scope_id(), ("pages", self.salt));
        let width = ui.available_width();
        let hidden = self.pages.iter().filter(|key| *key != self.current).fold(Vec2::ZERO, |largest, key| {
            largest.max(ui.measure_column(width, &mut |ui| {
                page(ui, key);
            }))
        });
        let mut out = None;
        ui.unit(sub(base, hash(self.current)), true, hidden, &mut |ui| out = Some(page(ui, self.current)));
        out.expect("the shown page ran")
    }
}

fn hash(value: &impl Hash) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
