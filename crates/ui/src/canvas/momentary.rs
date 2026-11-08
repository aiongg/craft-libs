//! Momentary tools, as in Photoshop: tools in effect only while a key is held.
//!
//! Space is the Hand while held, Cmd+Space (Ctrl+Space off the Mac) Zoom In, Alt+Space Zoom Out; Alt added to a held
//! Zoom In zooms out. The keys are the [`MomentaryCommand`] set (`Kind::Momentary`): the board fires a press when the
//! key goes down and a release when it comes up. [`Momentary`] folds those into the tool for the pass, and
//! [`ToolHost::momentary`](super::ToolHost::momentary) runs that tool instead of the selected one. A drag that began
//! with a tool finishes with it. Space during a marquee or shape drag is the
//! tool's own key ([`ToolContext::held`](super::ToolContext::held)), and the host withholds it from the shortcuts then.

use super::CanvasTool;
use super::navigate::NavigationOptions;
use super::tools::{HandTool, ZoomTool};
use crate::runtime::commands::FirePhase;
use craft_behavior::kurbo::Size;
use craft_keymap::Mods;

/// The commands that put a tool in effect while their key is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, craft_ui_macros::CommandSet)]
#[command(prefix = "canvas.momentary")]
#[non_exhaustive]
pub enum MomentaryCommand {
    /// The Hand tool while the key is held.
    #[command(label = "Hand (Hold)", kind = "momentary", default = "Space")]
    Hand,
    /// The Zoom tool, zooming in, while the keys are held.
    #[command(label = "Zoom In (Hold)", kind = "momentary", default = "Cmd+Space")]
    ZoomIn,
    /// The Zoom tool, zooming out, while the keys are held.
    #[command(label = "Zoom Out (Hold)", kind = "momentary", default = "Alt+Space")]
    ZoomOut,
}

/// A tool in effect only while its keys are held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MomentaryTool {
    /// The Hand tool.
    Hand,
    /// The Zoom tool, zooming in.
    ZoomIn,
    /// The Zoom tool, zooming out.
    ZoomOut,
}

impl MomentaryTool {
    /// Whether the Zoom tool zooms out.
    pub const fn zooms_out(self) -> bool {
        matches!(self, MomentaryTool::ZoomOut)
    }
}

impl From<MomentaryCommand> for MomentaryTool {
    fn from(command: MomentaryCommand) -> Self {
        match command {
            MomentaryCommand::Hand => MomentaryTool::Hand,
            MomentaryCommand::ZoomIn => MomentaryTool::ZoomIn,
            MomentaryCommand::ZoomOut => MomentaryTool::ZoomOut,
        }
    }
}

/// The momentary tools and the one in effect, kept by the app across passes. The app feeds it the fired
/// [`MomentaryCommand`]s and hands it to [`ToolHost::momentary`](super::ToolHost::momentary), which runs its Hand or
/// Zoom tool in place of the selected tool while a key is held.
#[derive(Clone, Debug)]
pub struct Momentary {
    /// The command whose key is down.
    held: Option<MomentaryCommand>,
    /// The tool the last pass without a gesture chose: the one a drag keeps.
    latched: Option<MomentaryTool>,
    hand: HandTool,
    zoom: ZoomTool,
}

impl Momentary {
    /// The momentary tools for a document of `size` under `options`.
    pub fn new(options: NavigationOptions, size: Size) -> Self {
        Momentary { held: None, latched: None, hand: HandTool::new(options), zoom: ZoomTool::new(size, options) }
    }

    /// Follows a change of the navigation options.
    pub fn set_options(&mut self, options: NavigationOptions) {
        self.hand.options = options;
        self.zoom.options = options;
    }

    /// Follows a change of the document's size.
    pub fn set_size(&mut self, size: Size) {
        self.zoom.size = size;
    }

    /// Ends a glide of the momentary Hand: the app's clamp of the view moved it, so the view hit a scroll limit.
    pub fn stop_glide(&mut self) {
        self.hand.stop_glide();
    }

    /// Takes a fired momentary command: a press puts its tool in effect, a release ends it.
    pub fn fired(&mut self, command: MomentaryCommand, phase: FirePhase) {
        match phase {
            FirePhase::Press | FirePhase::Repeat => self.held = Some(command),
            FirePhase::Release => self.held = None,
        }
    }

    /// The tool for this pass with `mods` held. Modifiers pressed after the key still count, in any order: Cmd turns
    /// the Hand into Zoom In, and Alt turns either into Zoom Out. While `in_gesture` (a drag runs), the tool of the
    /// pass before keeps the drag, whatever the keys do.
    pub fn tool(&mut self, mods: Mods, in_gesture: bool) -> Option<MomentaryTool> {
        if in_gesture {
            return self.latched;
        }
        let tool = self.held.map(|held| {
            let tool = MomentaryTool::from(held);
            if mods.alt {
                MomentaryTool::ZoomOut
            } else if tool == MomentaryTool::Hand && (mods.cmd || mods.ctrl) {
                MomentaryTool::ZoomIn
            } else {
                tool
            }
        });
        self.latched = tool;
        tool
    }

    /// The tool the host runs this pass: the momentary one in effect, else `selected`. The second answer is the
    /// momentary Hand when it is not the tool of the pass but still glides after a flick.
    pub(crate) fn substitute<'t>(
        &'t mut self,
        selected: &'t mut dyn CanvasTool,
        mods: Mods,
        in_gesture: bool,
    ) -> (&'t mut dyn CanvasTool, Option<&'t mut HandTool>) {
        match self.tool(mods, in_gesture) {
            Some(MomentaryTool::Hand) => (&mut self.hand, None),
            Some(tool) => {
                self.zoom.out = tool.zooms_out();
                (&mut self.zoom, self.hand.is_gliding().then_some(&mut self.hand))
            }
            None => (selected, self.hand.is_gliding().then_some(&mut self.hand)),
        }
    }
}
