#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`BreadcrumbBar`]: a path as clickable segments, the current one last.

use egui::{AsIdSalt, IdSalt, Response, Ui};

crate::words! {
    /// The bar's words, English by default.
    #[non_exhaustive]
    pub struct BreadcrumbLabels {
        /// The bar's accessible name.
        path: "breadcrumb.path" = "Path";
        /// The button standing for the segments that don't fit; its tooltip is the whole path.
        hidden: "breadcrumb.hidden" = "Show the whole path";
    }
}

/// One segment: what it shows and what opening it means to the app (a folder's path, a group's
/// id).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crumb<K> {
    pub label: String,
    pub key: K,
}

/// What the bar reported this pass.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BreadcrumbBarResponse<K> {
    pub response: Response,
    /// The segment clicked or activated: the app opens it. The last segment (where the user is)
    /// never reports.
    pub opened: Option<K>,
}

/// A breadcrumb bar, after LightCraft's folder header.
///
/// ```ignore
/// let crumbs = path_crumbs(&folder);
/// if let Some(path) = BreadcrumbBar::new("folder", &crumbs).show(ui).opened { app.browse(path) }
/// ```
///
/// - **Look.** Segments in the `Body` role, 24 points high, the ancestors in `text.muted`
///   and the last one in `text.normal` and `BodyStrong`, with a 16-point chevron right
///   ([`Icon::ChevronRight`](crate::icons::Icon::ChevronRight), mirrored in right-to-left) in
///   `text.muted` between them, 4 points either side. Ancestors show the pointing-hand cursor and
///   the link hover underline.
/// - **Room.** The bar shows the last `clamp(width / 80, 1, 4)` segments (LightCraft's); the
///   ones before them collapse into a "…" button whose tooltip is the whole path and which opens
///   a [`Menu`](crate::menu::Menu) of the hidden segments, nearest last. The shown segments share
///   the room left after the separators equally, each cut short in the middle with "…" when it
///   is longer than its share (services' `text::elided_galley` with `ElideAt::Middle`), its
///   whole label as the tooltip.
/// - **Keys.** Each ancestor (and the "…" button) is a Tab stop; Enter and Space open it. The
///   last segment is text, not a stop. The press that closes the "…" menu outside it does
///   nothing else (the menus' closing-press rule).
/// - **AccessKit.** A `Navigation` group named [`BreadcrumbLabels::path`]; ancestors are `Link`s
///   named by their labels; the last is `Label` text with `current` page.
pub struct BreadcrumbBar<'a, K> {
    id_salt: IdSalt,
    crumbs: &'a [Crumb<K>],
    labels: Option<BreadcrumbLabels>,
}

impl<'a, K: Clone> BreadcrumbBar<'a, K> {
    /// A bar of `crumbs`, the root first.
    pub fn new(id_salt: impl AsIdSalt, crumbs: &'a [Crumb<K>]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), crumbs, labels: None }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: BreadcrumbLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // lightcraft: crates/ui-egui/src/panels/grid.rs:1070 folder_breadcrumbs
    // needs: services craft-ui::text::elided_galley (N11.03) for the middle elision of a long segment.
    pub fn show(self, ui: &mut Ui) -> BreadcrumbBarResponse<K> {
        todo!()
    }
}

/// The segments of a file path, each with the path that opens it, spelled as in `path`: its own
/// separators (`/` or `\`), a leading `/` or a UNC `\\server\share` kept in the keys, empty
/// parts skipped. A bare drive (`C:`) as the first segment gets its separator back in its key,
/// since on its own it means the current folder on that drive (LightCraft issue 538).
// lightcraft: crates/ui-egui/src/panels/grid.rs:1051 crumbs
pub fn path_crumbs(path: &str) -> Vec<Crumb<String>> {
    todo!()
}
