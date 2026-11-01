//! [`AlignBar`]: the Move tool's Align and Distribute controls in the options bar (F17.03).
//! Photoshop shows them whenever the Move tool is active; they enable once enough layers are
//! selected.

use egui::{AsIdSalt, IdSalt, Response, Sense, Ui, vec2};

use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::Divider;
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::ControlSize;

crate::words! {
    /// The align bar's words, English by default (Photoshop's Layer › Align and Distribute
    /// menus). A button's tooltip is "Align: Left Edges".
    #[non_exhaustive]
    pub struct AlignLabels {
        align: "align.align" = "Align";
        distribute: "align.distribute" = "Distribute";
        /// The ⋯ button's name and tooltip.
        more: "align.more" = "Align and distribute";
        /// `{group}: {action}`.
        tooltip: "align.tooltip" = "{group}: {action}";
        left_edges: "align.leftEdges" = "Left Edges";
        horizontal_centers: "align.horizontalCenters" = "Horizontal Centers";
        right_edges: "align.rightEdges" = "Right Edges";
        top_edges: "align.topEdges" = "Top Edges";
        vertical_centers: "align.verticalCenters" = "Vertical Centers";
        bottom_edges: "align.bottomEdges" = "Bottom Edges";
        horizontally: "align.horizontally" = "Horizontally";
        vertically: "align.vertically" = "Vertically";
    }
}

/// One align or distribute command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlignAction {
    AlignLeftEdges,
    AlignHorizontalCenters,
    AlignRightEdges,
    AlignTopEdges,
    AlignVerticalCenters,
    AlignBottomEdges,
    DistributeTopEdges,
    DistributeVerticalCenters,
    DistributeBottomEdges,
    DistributeLeftEdges,
    DistributeHorizontalCenters,
    DistributeRightEdges,
    /// Equal gaps between the layers, left to right.
    DistributeHorizontally,
    /// Equal gaps between the layers, top to bottom.
    DistributeVertically,
}

impl AlignAction {
    /// The six align buttons, in Photoshop's order.
    // pc: crates/ui-egui/src/tools/align_bar.rs:11 ALIGN
    pub const ALIGN: [AlignAction; 6] = [
        AlignAction::AlignLeftEdges,
        AlignAction::AlignHorizontalCenters,
        AlignAction::AlignRightEdges,
        AlignAction::AlignTopEdges,
        AlignAction::AlignVerticalCenters,
        AlignAction::AlignBottomEdges,
    ];

    /// The eight distribute commands, in PhotoCraft's menu order.
    // pc: crates/ui-egui/src/tools/align_bar.rs:20 DISTRIBUTE
    pub const DISTRIBUTE: [AlignAction; 8] = [
        AlignAction::DistributeTopEdges,
        AlignAction::DistributeVerticalCenters,
        AlignAction::DistributeBottomEdges,
        AlignAction::DistributeLeftEdges,
        AlignAction::DistributeHorizontalCenters,
        AlignAction::DistributeRightEdges,
        AlignAction::DistributeHorizontally,
        AlignAction::DistributeVertically,
    ];

    /// The action's icon: `AlignEdge*` for the six align actions, `Distribute*` for the eight
    /// distribute ones (`ui/icons/icons.toml`; the distribute icons are drawn in the base set only,
    /// as PhotoCraft draws them, and the packs fall back to it).
    pub fn icon(self) -> Icon {
        match self {
            AlignAction::AlignLeftEdges => Icon::AlignEdgeLeft,
            AlignAction::AlignHorizontalCenters => Icon::AlignEdgeHcenter,
            AlignAction::AlignRightEdges => Icon::AlignEdgeRight,
            AlignAction::AlignTopEdges => Icon::AlignEdgeTop,
            AlignAction::AlignVerticalCenters => Icon::AlignEdgeVcenter,
            AlignAction::AlignBottomEdges => Icon::AlignEdgeBottom,
            AlignAction::DistributeTopEdges => Icon::DistributeTop,
            AlignAction::DistributeVerticalCenters => Icon::DistributeVcenter,
            AlignAction::DistributeBottomEdges => Icon::DistributeBottom,
            AlignAction::DistributeLeftEdges => Icon::DistributeLeft,
            AlignAction::DistributeHorizontalCenters => Icon::DistributeHcenter,
            AlignAction::DistributeRightEdges => Icon::DistributeRight,
            AlignAction::DistributeHorizontally => Icon::DistributeHorizontal,
            AlignAction::DistributeVertically => Icon::DistributeVertical,
        }
    }

    /// The action's own name ("Left Edges"); its group is [`is_align`](Self::is_align).
    pub fn label(self, labels: &AlignLabels) -> &str {
        match self {
            AlignAction::AlignLeftEdges | AlignAction::DistributeLeftEdges => &labels.left_edges,
            AlignAction::AlignHorizontalCenters | AlignAction::DistributeHorizontalCenters => &labels.horizontal_centers,
            AlignAction::AlignRightEdges | AlignAction::DistributeRightEdges => &labels.right_edges,
            AlignAction::AlignTopEdges | AlignAction::DistributeTopEdges => &labels.top_edges,
            AlignAction::AlignVerticalCenters | AlignAction::DistributeVerticalCenters => &labels.vertical_centers,
            AlignAction::AlignBottomEdges | AlignAction::DistributeBottomEdges => &labels.bottom_edges,
            AlignAction::DistributeHorizontally => &labels.horizontally,
            AlignAction::DistributeVertically => &labels.vertically,
        }
    }

    pub fn is_align(self) -> bool {
        Self::ALIGN.contains(&self)
    }
}

/// Where the distribute commands sit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DistributeButtons {
    /// In the ⋯ menu only, with the align commands (PhotoCraft, Photoshop's narrow bar).
    #[default]
    Menu,
    /// Also as a second group of eight buttons after a divider (Photoshop's wide bar; PdfCraft's
    /// and FilmCraft's arrange bars).
    Row,
}

/// What the bar did this pass.
#[derive(Debug)]
pub struct AlignBarResponse {
    /// The command picked, from a button or the menu.
    pub picked: Option<AlignAction>,
    pub response: Response,
}

/// The six align icon buttons (24 pt, 16 pt icons, no gaps), then a ⋯ button whose menu lists
/// Align and Distribute as sections, each item with its icon and name; with
/// [`DistributeButtons::Row`] the distribute buttons follow too. The buttons are the library's
/// icon buttons and the menu its `PressMenu` (the closing press reaches nothing else, lead call
/// L21); a pick in the menu is reported in the pass it happens.
///
/// Each button's tooltip and AccessKit name is "Align: Left Edges"; a disabled command's button
/// and menu item are disabled. Every button is a Tab stop in the options bar's order; the menu
/// opens on click, Enter or Down and closes on a pick or Esc.
///
/// ```ignore
/// let bar = AlignBar::new("move-align").enabled(|a| session.is_enabled(command_of(a))).show(ui);
/// // Words other than the installed catalog's: AlignBar::new(..).labels(AlignLabels { more: "More".into(), ..AlignLabels::of(ui) })
/// if let Some(action) = bar.picked { app.run(command_of(action), json!({})); }
/// ```
#[must_use = "call show"]
pub struct AlignBar<'a> {
    id_salt: IdSalt,
    enabled: Box<dyn Fn(AlignAction) -> bool + 'a>,
    distribute: DistributeButtons,
    labels: Option<AlignLabels>,
}

impl<'a> AlignBar<'a> {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), enabled: Box::new(|_| true), distribute: DistributeButtons::Menu, labels: None }
    }

    /// The words; `AlignLabels::of(ui)` (the installed catalog's) when not called.
    pub fn labels(mut self, labels: AlignLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Which commands are enabled now; all of them by default.
    pub fn enabled(mut self, enabled: impl Fn(AlignAction) -> bool + 'a) -> Self {
        self.enabled = Box::new(enabled);
        self
    }

    pub fn distribute(mut self, distribute: DistributeButtons) -> Self {
        self.distribute = distribute;
        self
    }

    // pc: crates/ui-egui/src/tools/align_bar.rs:32 options
    pub fn show(self, ui: &mut Ui) -> AlignBarResponse {
        let labels = self.labels.unwrap_or_else(|| AlignLabels::of(ui));
        let name = |action: AlignAction| {
            let group = if action.is_align() { &labels.align } else { &labels.distribute };
            craft_i18n::fmt(&labels.tooltip, &[("group", group), ("action", action.label(&labels))])
        };
        let sizes = Tokens::of(ui).component.align;
        let mut picked = None;
        let inner = ui.scope_builder(egui::UiBuilder::new().id_salt(ui.make_persistent_id(self.id_salt)), |ui| {
            direction::horizontal(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let button = |ui: &mut Ui, action: AlignAction, picked: &mut Option<AlignAction>| {
                    let b = IconButton::new(action.icon(), name(action))
                        .side(sizes.button)
                        .icon_size(sizes.icon)
                        .enabled((self.enabled)(action))
                        .id_salt(action);
                    if ui.add(b).clicked() {
                        *picked = Some(action);
                    }
                };
                for action in AlignAction::ALIGN {
                    button(ui, action, &mut picked);
                }
                if self.distribute == DistributeButtons::Row {
                    let (rect, _) = ui.allocate_exact_size(vec2(1.0, sizes.button), Sense::hover());
                    Divider::paint(ui.painter(), rect.center_top(), rect.center_bottom(), &Tokens::of(ui));
                    for action in AlignAction::DISTRIBUTE {
                        button(ui, action, &mut picked);
                    }
                }
                let more = ui.add(IconButton::new(Icon::More, labels.more.clone()).side(sizes.button).icon_size(sizes.icon).id_salt("more"));
                let item = |action: AlignAction| {
                    MenuItem::new(action.label(&labels), action)
                        .icon(action.icon())
                        .accessible_name(name(action))
                        .enabled((self.enabled)(action))
                        .into()
                };
                let mut entries: Vec<MenuEntry<AlignAction>> = AlignAction::ALIGN.into_iter().map(item).collect();
                entries.push(MenuEntry::Separator);
                entries.extend(AlignAction::DISTRIBUTE.into_iter().map(item));
                if let Some(action) = PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &more).chosen {
                    picked = Some(action);
                }
            })
            .response
        });
        AlignBarResponse { picked, response: inner.response }
    }
}
