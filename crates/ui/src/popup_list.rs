//! [`PopupList`]: the popup that holds a list under a field (a select's box, a combo box's text
//! field, a menu's button), as tall as its contents while they change.
//!
//! egui lays a popup out in the size it had on the previous pass, so a list that shrinks (a search
//! with no matches) and grows again would only scroll inside the small popup until it closed
//! (emilk/egui#5225). The popup list offers its contents the full height on every pass.

use std::ops::Range;
use std::sync::Arc;

use egui::{
    Align, Context, Frame, Id, LayerId, Margin, Order, Popup, PopupCloseBehavior, Rect, RectAlign, Response, Role, ScrollArea, Shape, Style,
    TextWrapMode, Ui, style::StyleModifier,
};

use crate::primitives::{SurfaceEdge, SurfaceStyle};

/// Keeps the popup `popup` in front of `host`, the layer of the widget it opens from.
///
/// A popup that opens in the pass its host's layer is raised in (a press in a modal raises it)
/// is raised with it, and egui's stable sort would leave it under the host. As a sublayer it
/// always sits directly above its host. Only a host in the popups' own order needs it: a window
/// in a lower order is under every popup already.
///
/// A host that is itself a sublayer (a popover in a modal, with a list opened from it) passes
/// the popup on to its own parent, so the layers stay one level deep: egui leaves deeper nesting
/// unspecified.
pub(crate) fn in_front_of(ctx: &Context, host: LayerId, popup: Id) {
    let layer = LayerId::new(Order::Foreground, popup);
    let root = ctx.memory(|mem| {
        let mut root = host;
        while let Some(parent) = mem.areas().parent_layer(root) {
            root = parent;
        }
        root
    });
    if root.order == Order::Foreground && root != layer {
        ctx.set_sublayer(root, layer);
    }
}

/// A list in a popup under a field, at least as wide as the field.
///
/// It opens below the field, or above it when there is more room there, and stays on that side
/// while it is open. It is never taller than the room on that side: a long list scrolls.
///
/// A header (a search field) and a footer (an "Add…" row) stay in place while only the list
/// scrolls. [`Self::show_rows_with_header`] is for rows of one height; rows of different heights,
/// or a tree, use [`Self::show_ui_with_header`].
#[must_use = "You should call .show*"]
pub struct PopupList<'a> {
    popup: Popup<'a>,
    ctx: Context,
    host: LayerId,
    field: Rect,
    /// The widgets whose keyboard focus is the field's: the field, and anything that takes the
    /// focus in its place (a search field in the popup).
    owners: Vec<Id>,
    height: f32,
    style: StyleModifier,
    /// The style of the theme scope the popup opens in, under `style`.
    scoped: Option<Arc<Style>>,
    close_behavior: PopupCloseBehavior,
    open: Option<&'a mut bool>,
    /// Drawn behind the list in place of egui's popup frame.
    surface: Option<SurfaceStyle>,
}

impl<'a> PopupList<'a> {
    /// The popup list under `field`, a widget in `ui`, opened and closed by clicks on it.
    pub(crate) fn new(ui: &Ui, field: &Response) -> Self {
        Self::with_popup(Popup::menu(field), ui, field)
    }

    /// The popup list in `popup`, under `field`.
    pub fn with_popup(popup: Popup<'a>, ui: &Ui, field: &Response) -> Self {
        // Below the field or above it, never beside it.
        let flipped: &'static [RectAlign] = if ui.ctx().global_style().right_to_left { &[RectAlign::TOP_END] } else { &[RectAlign::TOP_START] };
        Self {
            popup: popup.align_alternatives(flipped),
            ctx: field.ctx.clone(),
            host: field.layer_id,
            field: field.rect,
            owners: vec![field.id],
            height: ui.spacing().combo_height,
            style: StyleModifier::from(egui::containers::menu::menu_style),
            scoped: crate::theme::scoped_style(ui),
            close_behavior: PopupCloseBehavior::CloseOnClick,
            open: None,
            surface: None,
        }
    }

    /// The maximum height of the contents, inside the frame, kept to the room on the popup's side
    /// of the field. Default: [`egui::Spacing::combo_height`].
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// When the popup closes. Default: [`PopupCloseBehavior::CloseOnClick`].
    pub fn close_behavior(mut self, close_behavior: PopupCloseBehavior) -> Self {
        self.close_behavior = close_behavior;
        self
    }

    /// Shows the popup while `*open`. The caller sets `*open` to open it, and the popup clears it
    /// when it closes: on its close behavior, Escape, [`Ui::close`] inside it, or the keyboard
    /// focus leaving the [focus owners](Self::focus_owners) and the popup. Clicks on the field
    /// neither open nor close it. The popup doesn't take the focus.
    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// The widgets whose keyboard focus keeps an [`open`](Self::open) popup open; the field by
    /// default.
    pub fn focus_owners(mut self, owners: &[Id]) -> Self {
        self.owners = owners.to_vec();
        self
    }

    /// The popup's style. Default: egui's menu style.
    pub(crate) fn style(mut self, style: impl Into<StyleModifier>) -> Self {
        self.style = style.into();
        self
    }

    /// Draws the popup on `surface` (a [`Surface`](crate::primitives::Surface) style, which may
    /// bevel) instead of egui's popup frame, with the style's menu margin inside it.
    pub fn surface(mut self, surface: SurfaceStyle) -> Self {
        self.surface = Some(surface);
        self
    }

    /// Shows `add` in the popup while it is open, scrolling when it is taller than the popup.
    /// Returns what `add` returned, or `None` while the popup is closed.
    pub fn show_ui<R>(self, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        self.show(|_| (), |scroll, ui, ()| scroll.show(ui, add).inner, None::<fn(&mut Ui)>).map(|(r, _)| r)
    }

    /// Shows `header` at the top of the popup and `list` under it while the popup is open, and
    /// returns what `list` returned. `list` gets what `header` returned, such as the rows a search
    /// field's text matches. Only `list` scrolls; the header stays in place. The popup is as tall
    /// as both, up to its height. `list` lays out in a vertical scroll area: give it rows, not a
    /// scroll area of its own.
    pub(crate) fn show_ui_with_header<H, R>(self, header: impl FnOnce(&mut Ui) -> H, list: impl FnOnce(&mut Ui, H) -> R) -> Option<R> {
        self.show(header, |scroll, ui, h| scroll.show(ui, |ui| list(ui, h)).inner, None::<fn(&mut Ui)>).map(|(r, _)| r)
    }

    /// Like [`Self::show_ui_with_header`], with a list of `total_rows(&header's)` rows, each
    /// `row_height` tall without the item spacing, that only lays out the rows in view: `rows`
    /// gets the range to add, as in [`ScrollArea::show_rows`], and what `header` returned.
    pub(crate) fn show_rows_with_header<H, R>(
        self,
        header: impl FnOnce(&mut Ui) -> H,
        row_height: f32,
        total_rows: impl FnOnce(&H) -> usize,
        rows: impl FnOnce(&mut Ui, Range<usize>, H) -> R,
    ) -> Option<R> {
        self.show(header, rows_list(row_height, total_rows, rows), None::<fn(&mut Ui)>).map(|(r, _)| r)
    }

    /// Like [`Self::show_ui_with_header`], with `footer` fixed under the list. Returns what `list`
    /// and `footer` returned.
    pub(crate) fn show_ui_with_header_footer<H, R, F>(
        self,
        header: impl FnOnce(&mut Ui) -> H,
        list: impl FnOnce(&mut Ui, H) -> R,
        footer: impl FnOnce(&mut Ui) -> F,
    ) -> Option<(R, F)> {
        self.show(header, |scroll, ui, h| scroll.show(ui, |ui| list(ui, h)).inner, Some(footer)).and_then(|(r, f)| Some((r, f?)))
    }

    /// Like [`Self::show_rows_with_header`], with `footer` fixed under the list.
    pub(crate) fn show_rows_with_header_footer<H, R, F>(
        self,
        header: impl FnOnce(&mut Ui) -> H,
        row_height: f32,
        total_rows: impl FnOnce(&H) -> usize,
        rows: impl FnOnce(&mut Ui, Range<usize>, H) -> R,
        footer: impl FnOnce(&mut Ui) -> F,
    ) -> Option<(R, F)> {
        self.show(header, rows_list(row_height, total_rows, rows), Some(footer)).and_then(|(r, f)| Some((r, f?)))
    }

    fn show<H, R, F>(
        self,
        header: impl FnOnce(&mut Ui) -> H,
        list: impl FnOnce(ScrollArea, &mut Ui, H) -> R,
        footer: Option<impl FnOnce(&mut Ui) -> F>,
    ) -> Option<(R, Option<F>)> {
        let Self { popup, ctx, host, field, owners, height, style, scoped, mut close_behavior, open, surface } = self;
        let style = match scoped {
            Some(scoped) => StyleModifier::new(move |s: &mut Style| {
                *s = (*scoped).clone();
                style.apply(s);
            }),
            None => style,
        };
        let id = popup.get_id();
        in_front_of(&ctx, host, id);
        ctx.data_mut(|d| d.insert_temp(id.with(ANCHOR), field));
        let flagged = open.is_some();
        let popup = match open {
            Some(open) => {
                let focus_key = id.with("craft-field-focus");
                // The popup and the popups opened from inside it are one: focus and clicks in a
                // menu opened from a button in the popup are inside.
                let family = family(&ctx, id);
                if take_close_request(&ctx, id) {
                    *open = false;
                }
                let inside = focus_inside(&ctx, &owners, &family);
                // A press or click inside drops a text field's focus in this pass or the next (a
                // text field gives it up on the release of a click elsewhere): the user is still
                // inside, so the next pass doesn't count the focus as having been inside.
                let pressed = pressed_into(&ctx, &family);
                // A popup that wasn't shown last pass starts with what the focus is now.
                let shown = ctx.read_response(id).is_some();
                let had = ctx.data_mut(|d| std::mem::replace(d.get_temp_mut_or_default::<bool>(focus_key), inside && !pressed)) && shown;
                if had && !inside && !pressed {
                    *open = false;
                }
                // A click in the field (moving a text field's cursor) or in a popup opened from
                // inside this one leaves the popup open.
                let pointer = ctx.input(|i| i.pointer.interact_pos());
                if pointer.is_some_and(|pos| field.contains(pos) || ctx.layer_id_at(pos).is_some_and(|l| l.id != id && family.contains(&l.id))) {
                    close_behavior = PopupCloseBehavior::IgnoreClicks;
                }
                popup.open_bool(open)
            }
            None => popup,
        };
        let footer_id = id.with("craft-footer-height");
        let remeasure_id = id.with("craft-remeasure");
        let remeasure = ctx.data_mut(|d| d.remove_temp::<bool>(remeasure_id)).unwrap_or(false);
        let (align, settled) = side(&popup, remeasure);
        // At most two remeasures in a row, so a footer that changes height on every pass settles.
        // A pass where the footer keeps its height clears the count.
        let count_id = id.with("craft-remeasures");
        if ctx.read_response(id).is_none() {
            ctx.data_mut(|d| d.remove::<u8>(count_id));
        }
        let remeasures = ctx.data(|d| d.get_temp::<u8>(count_id)).unwrap_or(0);
        // Until the side is settled, the popup is measured against the whole screen.
        let room = match popup.get_anchor_rect() {
            Some(anchor) if settled => room(align, anchor, ctx.content_rect()),
            _ => ctx.content_rect().height(),
        };
        let mut popup_style = (*ctx.global_style()).clone();
        style.apply(&mut popup_style);
        // A surface's edge is drawn inside its rect, so the margin keeps the contents off it.
        let frame = match &surface {
            Some(surface) => Frame::NONE.inner_margin(popup_style.spacing.menu_margin + Margin::same(edge_width(surface).ceil() as i8)),
            None => Frame::popup(&popup_style),
        };
        let height = height.min(room - frame.total_margin().sum().y).max(0.0);
        let popup = if surface.is_some() { popup.frame(Frame::NONE) } else { popup };
        let drawing = Drawing::enter(&ctx, id);
        let inner = popup
            .align(align)
            .align_alternatives(&[])
            .width(field.width())
            .style(style)
            .close_behavior(close_behavior)
            .show(|ui| {
                // The pass that measures the popup before its side is settled: drawn at a side that
                // may change and at a height the room doesn't cap. Laid out and interactive (a
                // search field takes the focus), but not seen.
                if remeasure && !settled {
                    ui.multiply_opacity(0.0);
                }
                let contents = |ui: &mut Ui| {
                    ui.set_min_width(ui.available_width());
                    // The popup offers last pass's height. Offer the full height instead: the scroll
                    // area takes what the header and footer leave of it, then shrinks to the list.
                    ui.set_max_height(height);
                    let header = header(ui);
                    // The footer comes after the list; keep the room it took last pass.
                    let footer_height = ui.data(|d| d.get_temp::<f32>(footer_id));
                    let footer_room = if footer.is_some() { footer_height.unwrap_or(0.0) + ui.spacing().item_spacing.y } else { 0.0 };
                    // Labels widen a narrow popup rather than wrap early, as in egui's combo box. A
                    // list left with no room keeps ScrollArea's minimum scrolled height.
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    let list = list(ScrollArea::vertical().max_height((ui.available_height() - footer_room).max(0.0)), ui, header);
                    let footer = footer.map(|footer| {
                        let top = ui.cursor().top();
                        let f = footer(ui);
                        let height = ui.min_rect().bottom() - top;
                        let changed = footer_height.is_none_or(|h| (h - height).abs() > 0.5);
                        if !changed && remeasures > 0 {
                            ui.data_mut(|d| d.remove::<u8>(count_id));
                        }
                        if changed {
                            // Recorded even past the cap, so "unchanged" means unchanged since the last pass.
                            ui.data_mut(|d| d.insert_temp(footer_id, height));
                        }
                        if changed && remeasures < 2 {
                            // The list took the wrong room: lay the popup out again next pass. Not a
                            // discarded pass: the header's side effects (focusing a search field on
                            // open) would run twice.
                            ui.data_mut(|d| {
                                d.insert_temp(remeasure_id, true);
                                d.insert_temp(count_id, remeasures + 1);
                            });
                            ui.ctx().request_repaint();
                        }
                        f
                    });
                    (list, footer)
                };
                match surface {
                    Some(surface) => {
                        let slot = ui.painter().add(Shape::Noop);
                        let inner = frame.show(ui, contents);
                        ui.painter().set(slot, Shape::Vec(surface.shapes(inner.response.rect, ui.pixels_per_point())));
                        inner.inner
                    }
                    None => contents(ui),
                }
            })
            .map(|r| (r.inner, r.response.should_close()));
        // Recorded below as a child of the popup this one draws in, not of itself.
        drop(drawing);
        if flagged {
            record_open(&ctx, id, inner.as_ref().is_some_and(|(_, closing)| !closing));
        }
        let inner = inner.map(|(inner, _)| inner);
        if inner.is_some() {
            ctx.accesskit_node_builder(id, |node| node.set_role(Role::ListBox));
        }
        inner
    }
}

const ANCHOR: &str = "craft-anchor";
const CLOSE_REQUEST: &str = "craft-close-request";
const PARENT_CLOSE: &str = "craft-parent-close";

/// Whether [`close_children`] asked the popover `popup` to close since it last asked. It does not
/// expire: a popover drawn inside a closed parent reads it when the parent is drawn again.
pub(crate) fn take_parent_close(ctx: &Context, popup: Id) -> bool {
    ctx.data_mut(|d| d.remove_temp::<()>(popup.with(PARENT_CLOSE))).is_some()
}

/// Whether `popup` is recorded open (see [`record_open`]).
pub(crate) fn is_open_popup(ctx: &Context, popup: Id) -> bool {
    let (key, pass) = (open_lists_id(ctx), ctx.cumulative_pass_nr());
    ctx.data(|d| d.get_temp::<OpenLists>(key)).is_some_and(|l| l.0.iter().any(|l| l.popup == popup && l.pass + 1 >= pass))
}

/// Asks the open popups opened from inside `popup`, at any depth, to close: they do on their
/// next pass. A popover calls it when a press lands on its own surface, so a slider or grip that
/// never becomes a click still dismisses the menus it hides.
pub(crate) fn close_children(ctx: &Context, popup: Id) {
    let pass = ctx.cumulative_pass_nr();
    for child in family(ctx, popup).into_iter().skip(1) {
        ctx.data_mut(|d| {
            d.insert_temp(child.with(CLOSE_REQUEST), pass);
            d.insert_temp(child.with(PARENT_CLOSE), ());
        });
        Popup::close_id(ctx, child);
    }
}

/// Whether a close was asked of `popup` by [`close_children`] this pass or the last.
fn take_close_request(ctx: &Context, popup: Id) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.remove_temp::<u64>(popup.with(CLOSE_REQUEST))).is_some_and(|asked| pass <= asked + 1)
}

/// The rect of the field each open popup opened from inside `popup` hangs from (the button that
/// toggles it), as of its last pass.
pub(crate) fn child_anchors(ctx: &Context, popup: Id) -> Vec<Rect> {
    family(ctx, popup).into_iter().skip(1).filter_map(|child| ctx.data(|d| d.get_temp::<Rect>(child.with(ANCHOR)))).collect()
}

/// The popups open at the end of a pass, per viewport: those egui's popup memory doesn't know
/// (shown with a flag of their own, [`PopupList::open`], as Select and ComboBox open their lists)
/// and the menus. Each with the popup [drawing](Drawing) it, if any, and the pass it was last
/// recorded open in.
#[derive(Clone, Default)]
struct OpenLists(Vec<OpenList>);

#[derive(Clone, Copy)]
struct OpenList {
    popup: Id,
    /// The popup whose contents opened it: a menu opened from a button in a popover.
    parent: Option<Id>,
    pass: u64,
}

/// The [`PopupList`]s whose contents are being drawn, innermost last, per viewport. A popup
/// [recorded](record_open) open while one draws is that popup's child.
#[derive(Clone, Default)]
pub(crate) struct Drawing(Vec<Id>);

impl Drawing {
    fn key(ctx: &Context) -> Id {
        Id::unique("craft-ui-drawing-popups").with(ctx.viewport_id())
    }

    #[must_use = "the popup leaves the stack when the guard drops"]
    pub(crate) fn enter(ctx: &Context, popup: Id) -> DrawingGuard {
        let key = Self::key(ctx);
        ctx.data_mut(|d| d.get_temp_mut_or_default::<Drawing>(key).0.push(popup));
        DrawingGuard { ctx: ctx.clone(), popup }
    }

    fn innermost(ctx: &Context) -> Option<Id> {
        let key = Self::key(ctx);
        ctx.data(|d| d.get_temp::<Drawing>(key)).and_then(|d| d.0.last().copied())
    }
}

/// A popup's place in [`Drawing`] while its contents draw.
pub(crate) struct DrawingGuard {
    ctx: Context,
    popup: Id,
}

impl Drop for DrawingGuard {
    fn drop(&mut self) {
        let key = Drawing::key(&self.ctx);
        self.ctx.data_mut(|d| {
            let stack = &mut d.get_temp_mut_or_default::<Drawing>(key).0;
            if let Some(at) = stack.iter().rposition(|&p| p == self.popup) {
                stack.truncate(at);
            }
        });
    }
}

fn open_lists_id(ctx: &Context) -> Id {
    Id::unique("craft-ui-open-lists").with(ctx.viewport_id())
}

/// Records whether the list `popup` is open at the end of this pass. [`PopupList::show`] records
/// an [`open`](PopupList::open) popup; an owner that closes its list after showing it in a pass
/// (a click on an option, a commit read from the field) records it closed. A list no one records
/// any more counts as closed from the pass after its last record, so a list whose owner is no
/// longer drawn doesn't stay open.
///
/// A popup recorded open while a [`PopupList`]'s contents draw is that popup's child: the
/// parent stays open while the focus is in the child, and a click in the child is not outside it.
pub(crate) fn record_open(ctx: &Context, popup: Id, open: bool) {
    let (key, pass) = (open_lists_id(ctx), ctx.cumulative_pass_nr());
    let parent = Drawing::innermost(ctx).filter(|&p| p != popup);
    ctx.data_mut(|d| {
        let lists = &mut d.get_temp_mut_or_default::<OpenLists>(key).0;
        lists.retain(|l| l.popup != popup && l.pass + 1 >= pass);
        if open {
            lists.push(OpenList { popup, parent, pass });
        }
    });
}

/// `popup` and the open popups opened from inside it, at any depth.
pub(crate) fn family(ctx: &Context, popup: Id) -> Vec<Id> {
    let pass = ctx.cumulative_pass_nr();
    let key = open_lists_id(ctx);
    let lists = ctx.data(|d| d.get_temp::<OpenLists>(key)).unwrap_or_default().0;
    let mut family = vec![popup];
    let mut i = 0;
    while i < family.len() {
        let parent = family[i];
        for l in &lists {
            if l.pass + 1 >= pass && l.parent == Some(parent) && !family.contains(&l.popup) {
                family.push(l.popup);
            }
        }
        i += 1;
    }
    family
}

/// Whether a popup is open in this viewport: one in egui's popup memory ([`Popup::is_any_open`]:
/// menus, context menus) or a list [recorded](record_open) open at the end of the last pass or
/// earlier in this one. Keys a popup takes while it is open (Tab in a select's list) are read at
/// the start of a pass, before the popup's owner runs, so this is what a control reads to leave
/// them alone.
pub(crate) fn any_open(ctx: &Context) -> bool {
    let (key, pass) = (open_lists_id(ctx), ctx.cumulative_pass_nr());
    Popup::is_any_open(ctx) || ctx.data(|d| d.get_temp::<OpenLists>(key)).is_some_and(|l| l.0.iter().any(|l| l.pass + 1 >= pass))
}

fn rows_list<H, R>(
    row_height: f32,
    total_rows: impl FnOnce(&H) -> usize,
    rows: impl FnOnce(&mut Ui, Range<usize>, H) -> R,
) -> impl FnOnce(ScrollArea, &mut Ui, H) -> R {
    move |scroll, ui, h| {
        let total = total_rows(&h);
        scroll.show_rows(ui, row_height, total, |ui, range| rows(ui, range, h)).inner
    }
}

/// The width of a surface's edge, which it draws inside its rect.
pub(crate) fn edge_width(surface: &SurfaceStyle) -> f32 {
    match surface.edge {
        SurfaceEdge::Border(stroke) => stroke.width,
        SurfaceEdge::Bevel { width, .. } => width,
        _ => 0.0,
    }
}

/// Whether the keyboard focus is with one of `owners` or a widget in one of the popups `family`.
pub(crate) fn focus_inside(ctx: &Context, owners: &[Id], family: &[Id]) -> bool {
    let Some(focused) = ctx.memory(|m| m.focused()) else { return false };
    owners.contains(&focused) || ctx.read_response(focused).is_some_and(|r| family.contains(&r.layer_id.id))
}

/// Whether the pointer was pressed or clicked inside one of the popups `family` this pass: egui
/// then drops the focus, which says nothing about where the user is going.
pub(crate) fn pressed_into(ctx: &Context, family: &[Id]) -> bool {
    let Some(pos) = ctx.input(|i| (i.pointer.any_pressed() || i.pointer.any_click()).then(|| i.pointer.interact_pos())).flatten() else {
        return false;
    };
    ctx.memory(|m| family.iter().any(|&p| m.area_rect(p).is_some_and(|area| area.contains(pos))))
}

/// The side of its field `popup` shows on, and whether it is settled. The side is settled on the
/// first pass after the popup was shown and measured (`remeasure` when that measure is stale):
/// the side egui picks when the popup fits there, else the one with more room. It is kept while
/// the popup stays open, so a list that shrinks or grows while open doesn't jump to the other
/// side.
fn side(popup: &Popup<'_>, remeasure: bool) -> (RectAlign, bool) {
    let ctx = popup.ctx();
    let key = popup.get_id().with("craft-side");
    if !popup.is_open() || ctx.read_response(popup.get_id()).is_none() {
        ctx.data_mut(|d| d.remove::<RectAlign>(key));
        return (popup.get_best_align(), false);
    }
    if let Some(align) = ctx.data(|d| d.get_temp::<RectAlign>(key)) {
        return (align, true);
    }
    if remeasure {
        return (popup.get_best_align(), false);
    }
    // Outside the data lock: these read the popup's state from memory.
    let mut align = popup.get_best_align();
    if let (Some(size), Some(anchor)) = (popup.get_expected_size(), popup.get_anchor_rect()) {
        let screen = ctx.content_rect();
        let flipped = align.flipped_y();
        if room(align, anchor, screen) < size.y && room(flipped, anchor, screen) > room(align, anchor, screen) {
            align = flipped;
        }
    }
    ctx.data_mut(|d| d.insert_temp(key, align));
    (align, true)
}

/// The height a popup on the `align` side of `anchor` has inside `screen`.
fn room(align: RectAlign, anchor: Rect, screen: Rect) -> f32 {
    match (align.parent.y(), align.child.y()) {
        (Align::Max, Align::Min) => screen.bottom() - anchor.bottom(),
        (Align::Min, Align::Max) => anchor.top() - screen.top(),
        _ => screen.height(),
    }
}
