//! [`BreadcrumbBar`]: a path as clickable segments, the current one last.

use craft_theme::TypeRole;
use egui::accesskit::{self, Role};
use egui::{AsIdSalt, CursorIcon, IdSalt, Rect, Response, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use super::TooltipExt as _;
use crate::direction;
use crate::icons::{self, Icon};
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::{Corner, Interactive};
use crate::text::{ElideAt, elided_galley};
use crate::theme::{ToEgui, Tokens, TokensOf as _, role_format};

/// The most segments shown before the rest go behind the ellipsis.
const MOST_SEGMENTS: usize = 4;

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
        let Self { id_salt, crumbs, labels } = self;
        let labels = labels.unwrap_or_else(|| BreadcrumbLabels::of(ui));
        let tokens = Tokens::of(ui);
        let bread = &tokens.component.breadcrumb;
        let (muted, strong) = (tokens.color.text.muted.to_egui(), tokens.color.text.normal.to_egui());
        let whole = crumbs.iter().map(|c| c.label.as_str()).collect::<Vec<_>>().join(" › ");
        let width = ui.available_width();
        let (_, strip) = ui.allocate_space(vec2(width, bread.height));
        let bar = ui.new_child(
            UiBuilder::new()
                .id_salt(id_salt)
                .max_rect(strip)
                .layout(direction::row(ui))
                .accessibility_role(Role::Navigation)
                .accessibility_label(labels.path.clone()),
        );
        let mut opened = None;
        let mut responses = Vec::new();
        let forward = direction::forward(&bar);
        // The span of the strip from `from` to `from + length`, along the reading direction.
        let span = |from: f32, length: f32| {
            let (a, b) = (direction::start_x(ui, strip) + forward * from, direction::start_x(ui, strip) + forward * (from + length));
            Rect::from_min_max(pos2(a.min(b), strip.top()), pos2(a.max(b), strip.bottom()))
        };

        let shown_count = ((width / bread.min_segment) as usize).clamp(1, MOST_SEGMENTS).min(crumbs.len());
        let hidden = crumbs.len() - shown_count;
        let format = |role, color| role_format(&tokens, role, color);
        let mut at = 0.0;

        if hidden > 0 {
            let galley = elided_galley(&bar, "…", format(TypeRole::Body, muted), f32::INFINITY, ElideAt::End).galley;
            let length = galley.size().x + 2.0 * bread.space;
            let rect = span(at, length);
            let sensed = Interactive::new("hidden", Role::Button, labels.hidden.as_str()).corner(Corner::Control).interact_at(&bar, rect);
            let response = sensed.finish(&bar).tooltip(&whole);
            bar.painter().galley(pos2(rect.center().x - galley.size().x / 2.0, rect.center().y - galley.size().y / 2.0), galley, muted);
            let entries: Vec<MenuEntry<usize>> =
                crumbs[..hidden].iter().enumerate().map(|(i, c)| MenuItem::new(c.label.as_str(), i).into()).collect();
            if let Some(i) = PressMenu::new(&entries).show(&bar, &response).chosen {
                opened = Some(crumbs[i].key.clone());
            }
            responses.push(response);
            at += length + bread.space;
            at += chevron(&bar, span(at, bread.separator), muted, bread.separator);
            at += bread.space;
        }

        // The shown segments share what the separators leave, each taking at most what it needs.
        let separators = shown_count.saturating_sub(1) as f32 * (bread.separator + 2.0 * bread.space);
        let room = (width - at - separators).max(0.0);
        let shown = &crumbs[hidden..];
        let natural: Vec<f32> = shown
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let role = if i + 1 == shown.len() { TypeRole::BodyStrong } else { TypeRole::Body };
                elided_galley(&bar, &c.label, format(role, strong), f32::INFINITY, ElideAt::Middle).galley.size().x
            })
            .collect();
        let widths = fair_widths(&natural, room);
        for (i, (crumb, &length)) in shown.iter().zip(&widths).enumerate() {
            let last = i + 1 == shown.len();
            let rect = span(at, length);
            let (role, color) = if last { (TypeRole::BodyStrong, strong) } else { (TypeRole::Body, muted) };
            let text = elided_galley(&bar, &crumb.label, format(role, color), length, ElideAt::Middle);
            let galley = text.galley;
            let id = Interactive::id_in(&bar, ("segment", hidden + i));
            let response = if last {
                let response = Interactive::with_id(id, Role::Label, crumb.label.as_str())
                    .sense(Sense::hover())
                    .focus_ring(false)
                    .interact_at(&bar, rect)
                    .finish(&bar);
                bar.ctx().accesskit_node_builder(id, |node| node.set_aria_current(accesskit::AriaCurrent::Page));
                response
            } else {
                let response = Interactive::with_id(id, Role::Link, crumb.label.as_str())
                    .corner(Corner::Control)
                    .state_layer(false)
                    .interact_at(&bar, rect)
                    .finish(&bar);
                if response.hovered() {
                    bar.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if response.clicked() {
                    opened = Some(crumb.key.clone());
                }
                response
            };
            let origin =
                pos2(if direction::is_rtl(&bar) { rect.right() - galley.size().x } else { rect.left() }, rect.center().y - galley.size().y / 2.0);
            if !last && response.hovered() {
                let y = origin.y + galley.size().y;
                let x = (origin.x, origin.x + galley.size().x);
                bar.painter().line_segment([pos2(x.0, y), pos2(x.1, y)], Stroke::new(1.0, strong));
            }
            bar.painter().galley(origin, galley, if !last && response.hovered() { strong } else { color });
            let response = if text.elided {
                response.tooltip(&crumb.label)
            } else if last {
                response
            } else {
                response.tooltip(&whole)
            };
            responses.push(response);
            at += length;
            if !last {
                at += bread.space;
                at += chevron(&bar, span(at, bread.separator), muted, bread.separator);
                at += bread.space;
            }
        }

        let response = match responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all.with_new_rect(strip),
            None => ui.interact(strip, bar.unique_id(), Sense::hover()),
        };
        BreadcrumbBarResponse { response, opened }
    }
}

/// Paints the chevron between segments in `rect` and returns its width.
fn chevron(ui: &Ui, rect: Rect, color: egui::Color32, size: f32) -> f32 {
    icons::paint_in(ui, rect, Icon::ChevronRight, icons::active(ui.ctx()), size, icons::IconSize::Md.stroke(), color);
    size
}

/// Widths for segments that want `natural` points each in `room` points: the shortest keep what
/// they want and the rest share what is left equally.
fn fair_widths(natural: &[f32], room: f32) -> Vec<f32> {
    let mut order: Vec<usize> = (0..natural.len()).collect();
    order.sort_by(|&a, &b| natural[a].total_cmp(&natural[b]));
    let mut widths = vec![0.0; natural.len()];
    let mut left = room;
    for (taken, &i) in order.iter().enumerate() {
        let share = left / (natural.len() - taken) as f32;
        widths[i] = natural[i].min(share).floor();
        left -= widths[i];
    }
    widths
}

/// The segments of a file path, each with the path that opens it, spelled as in `path`: its own
/// separators (`/` or `\`), a leading `/` or a UNC `\\server\share` kept in the keys, empty
/// parts skipped. A bare drive (`C:`) as the first segment gets its separator back in its key,
/// since on its own it means the current folder on that drive (LightCraft issue 538).
// lightcraft: crates/ui-egui/src/panels/grid.rs:1051 crumbs
pub fn path_crumbs(path: &str) -> Vec<Crumb<String>> {
    let mut out: Vec<Crumb<String>> = Vec::new();
    let mut start = 0;
    for (i, c) in path.char_indices().chain(std::iter::once((path.len(), '/'))) {
        if c != '/' && c != '\\' {
            continue;
        }
        if let Some(part) = path.get(start..i).filter(|p| !p.is_empty()) {
            let mut key = path.get(..i).unwrap_or(path).to_string();
            if out.is_empty() && part.ends_with(':') && key == part {
                key.push(if path.contains('\\') { '\\' } else { '/' });
            }
            out.push(Crumb { label: part.to_string(), key });
        }
        start = i + c.len_utf8();
    }
    out
}
