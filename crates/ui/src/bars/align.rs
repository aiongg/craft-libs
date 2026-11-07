//! `AlignBar`: the Align and Distribute controls of the Move tool's options bar. A pattern over the children of a
//! `Bar`: six align buttons, optionally eight distribute buttons after a divider, and a menu listing all fourteen.

use super::words::AlignWords;
use crate::buttons::IconButton;
use crate::contract::{BodyCx, Entry, Props as _, Show as _, ShowContainer as _, WordSetCx as _};
use crate::display::Divider;
use crate::menu::Menu;
use crate::runtime::env::Label;
use craft_behavior::IconId;
use egui::{Response, Ui};

#[cfg(feature = "testing")]
mod stories;

/// One align or distribute command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlignAction {
    /// Align the layers' left edges.
    AlignLeftEdges,
    /// Align the layers' horizontal centres.
    AlignHorizontalCenters,
    /// Align the layers' right edges.
    AlignRightEdges,
    /// Align the layers' top edges.
    AlignTopEdges,
    /// Align the layers' vertical centres.
    AlignVerticalCenters,
    /// Align the layers' bottom edges.
    AlignBottomEdges,
    /// Space the layers' top edges evenly.
    DistributeTopEdges,
    /// Space the layers' vertical centres evenly.
    DistributeVerticalCenters,
    /// Space the layers' bottom edges evenly.
    DistributeBottomEdges,
    /// Space the layers' left edges evenly.
    DistributeLeftEdges,
    /// Space the layers' horizontal centres evenly.
    DistributeHorizontalCenters,
    /// Space the layers' right edges evenly.
    DistributeRightEdges,
    /// Equal gaps between the layers, left to right.
    DistributeHorizontally,
    /// Equal gaps between the layers, top to bottom.
    DistributeVertically,
}

impl AlignAction {
    /// The six align commands, in the order of their buttons.
    pub const ALIGN: [AlignAction; 6] = [
        AlignAction::AlignLeftEdges,
        AlignAction::AlignHorizontalCenters,
        AlignAction::AlignRightEdges,
        AlignAction::AlignTopEdges,
        AlignAction::AlignVerticalCenters,
        AlignAction::AlignBottomEdges,
    ];

    /// The eight distribute commands, in the order of their buttons and menu rows.
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

    /// The command's icon.
    pub fn icon(self) -> IconId {
        IconId::new(match self {
            AlignAction::AlignLeftEdges => "align_edge_left",
            AlignAction::AlignHorizontalCenters => "align_edge_hcenter",
            AlignAction::AlignRightEdges => "align_edge_right",
            AlignAction::AlignTopEdges => "align_edge_top",
            AlignAction::AlignVerticalCenters => "align_edge_vcenter",
            AlignAction::AlignBottomEdges => "align_edge_bottom",
            AlignAction::DistributeTopEdges => "distribute_top",
            AlignAction::DistributeVerticalCenters => "distribute_vcenter",
            AlignAction::DistributeBottomEdges => "distribute_bottom",
            AlignAction::DistributeLeftEdges => "distribute_left",
            AlignAction::DistributeHorizontalCenters => "distribute_hcenter",
            AlignAction::DistributeRightEdges => "distribute_right",
            AlignAction::DistributeHorizontally => "distribute_horizontal",
            AlignAction::DistributeVertically => "distribute_vertical",
        })
    }

    /// Whether the command is one of the six align commands rather than a distribute command.
    pub fn is_align(self) -> bool {
        Self::ALIGN.contains(&self)
    }

    /// The command's own name, without its group.
    fn name(self, words: &AlignWords) -> &Label {
        match self {
            AlignAction::AlignLeftEdges | AlignAction::DistributeLeftEdges => &words.left_edges,
            AlignAction::AlignHorizontalCenters | AlignAction::DistributeHorizontalCenters => &words.horizontal_centers,
            AlignAction::AlignRightEdges | AlignAction::DistributeRightEdges => &words.right_edges,
            AlignAction::AlignTopEdges | AlignAction::DistributeTopEdges => &words.top_edges,
            AlignAction::AlignVerticalCenters | AlignAction::DistributeVerticalCenters => &words.vertical_centers,
            AlignAction::AlignBottomEdges | AlignAction::DistributeBottomEdges => &words.bottom_edges,
            AlignAction::DistributeHorizontally => &words.horizontally,
            AlignAction::DistributeVertically => &words.vertically,
        }
    }

    /// The command's name with its group, "Align: Left Edges": the tooltip and the accessible name.
    fn full_name(self, words: &AlignWords) -> Label {
        let group = if self.is_align() { &words.align } else { &words.distribute };
        Label::from(words.tooltip.fill(&[("group", group.as_str()), ("action", self.name(words).as_str())]))
    }
}

/// Where the distribute commands sit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DistributeButtons {
    /// In the menu only, with the align commands.
    #[default]
    Menu,
    /// Also as a second group of buttons after a divider.
    Row,
}

/// What the bar reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct AlignBarOutput {
    /// The command picked, from a button or from the menu.
    pub picked: Option<AlignAction>,
    /// The response of the menu button.
    pub more: Response,
}

/// The align buttons, the optional distribute buttons and the menu button, shown as children of a `Bar`.
#[derive(crate::contract::Pattern)]
pub struct AlignBar<'a> {
    enabled: Box<dyn Fn(AlignAction) -> bool + 'a>,
    distribute: DistributeButtons,
}

impl<'a> AlignBar<'a> {
    /// An align bar with every command enabled and the distribute commands in the menu.
    pub fn new() -> Self {
        AlignBar { enabled: Box::new(|_| true), distribute: DistributeButtons::Menu }
    }

    /// Which commands are enabled now. A disabled command's button and menu row are disabled.
    #[must_use]
    pub fn enabled(mut self, enabled: impl Fn(AlignAction) -> bool + 'a) -> Self {
        self.enabled = Box::new(enabled);
        self
    }

    /// Where the distribute commands sit.
    #[must_use]
    pub fn distribute(mut self, distribute: DistributeButtons) -> Self {
        self.distribute = distribute;
        self
    }

    /// Shows the bar as children of the bar body `cx` belongs to.
    pub fn show_in(self, ui: &mut Ui, cx: &mut BodyCx<'_>) -> AlignBarOutput {
        let words = cx.words::<AlignWords>();
        let mut picked = None;
        let mut button = |ui: &mut Ui, action: AlignAction| {
            let response =
                IconButton::new(action.icon()).label(action.full_name(&words)).enabled((self.enabled)(action)).id_salt(action).show(ui);
            if crate::contract::clicked(&response) {
                picked = Some(action);
            }
        };
        for action in AlignAction::ALIGN {
            button(ui, action);
        }
        if self.distribute == DistributeButtons::Row {
            Divider::vertical().show(ui);
            for action in AlignAction::DISTRIBUTE {
                button(ui, action);
            }
        }
        let more = IconButton::new(IconId::new("more")).label(words.more.clone()).id_salt("more").show(ui);
        let entry = |action: AlignAction| Entry::new(action, action.full_name(&words)).icon(action.icon()).enabled((self.enabled)(action));
        let mut entries: Vec<Entry<AlignAction>> = AlignAction::ALIGN.into_iter().map(entry).collect();
        entries.push(Entry::separator());
        entries.extend(AlignAction::DISTRIBUTE.into_iter().map(entry));
        let chosen = Menu::new(words.more.clone()).opened_by(&more).entries(&entries).show(ui, |_, _| {}).picked;
        AlignBarOutput { picked: chosen.or(picked), more }
    }
}

impl Default for AlignBar<'_> {
    fn default() -> Self {
        AlignBar::new()
    }
}
