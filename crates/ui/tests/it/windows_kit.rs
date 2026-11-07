//! A frame driver for trials with child viewports: the context opens OS viewports (not embedded), and a renderer
//! like eframe's runs each child's pass nested inside its parent's UI closure, with events and window facts
//! the trial gives it.

use craft_host::target::ShellInfo;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::frame::{FrameInput, set_input};
use craft_ui::testing::Frames;
use craft_ui::testing::windows::FakeSystem;
use craft_ui::testing::windows::{FullOutput, RawInput};
use egui::{Context, Event, Pos2, Rect, Ui, Vec2, ViewportCommand, ViewportId, ViewportInfo};
use std::cell::RefCell;
use std::sync::Arc;

const ROOT_SIZE: Vec2 = Vec2::new(800.0, 600.0);
const CHILD_SIZE: Vec2 = Vec2::new(400.0, 300.0);

thread_local! {
    /// The events the next child pass receives (the renderer runs on the trial's thread).
    static CHILD_EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    /// The window facts every child pass reports.
    static CHILD_INFO: RefCell<ViewportInfo> = RefCell::new(ViewportInfo::default());
    /// The window facts the root reports.
    static ROOT_INFO: RefCell<ViewportInfo> = RefCell::new(ViewportInfo::default());
    /// The children that ran a pass so far: like an integration, every pass's input lists all the windows.
    static CHILDREN: RefCell<Vec<ViewportId>> = const { RefCell::new(Vec::new()) };
    /// The window commands every pass's output carried since the last frame.
    static COMMANDS: RefCell<Vec<(ViewportId, ViewportCommand)>> = const { RefCell::new(Vec::new()) };
}

fn keep_commands(output: &FullOutput) {
    COMMANDS.with(|kept| {
        let mut kept = kept.borrow_mut();
        for (id, viewport) in &output.viewport_output {
            kept.extend(viewport.commands.iter().map(|command| (*id, command.clone())));
        }
    });
}

/// The window facts of every window the integration knows: the root and the children that ran, each with the
/// facts the trial gave it.
fn all_viewports(child: ViewportInfo) -> egui::ViewportIdMap<ViewportInfo> {
    let mut all = egui::ViewportIdMap::default();
    all.insert(ViewportId::ROOT, ROOT_INFO.with(|info| info.borrow().clone()));
    CHILDREN.with(|children| {
        for id in children.borrow().iter() {
            all.insert(*id, child.clone());
        }
    });
    all
}

fn render_child(ctx: &Context, viewport: egui::ImmediateViewport<'_>) {
    let mut callback = viewport.viewport_ui_cb;
    let events = CHILD_EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()));
    let mut info = CHILD_INFO.with(|info| info.borrow().clone());
    info.parent = Some(viewport.ids.parent);
    let mut input =
        RawInput { viewport_id: viewport.ids.this, events, screen_rect: Some(Rect::from_min_size(Pos2::ZERO, CHILD_SIZE)), ..RawInput::default() };
    CHILDREN.with(|children| {
        let mut children = children.borrow_mut();
        if !children.contains(&viewport.ids.this) {
            children.push(viewport.ids.this);
        }
    });
    input.viewports = all_viewports(info);
    let mut output = ctx.run_ui(input, |ui| callback(ui));
    output.textures_delta.clear();
    keep_commands(&output);
}

/// What a frame sent out.
#[non_exhaustive]
pub struct Out {
    /// The title each viewport asked for.
    pub titles: Vec<(ViewportId, String)>,
    /// The window commands of every pass of the frame.
    pub commands: Vec<(ViewportId, ViewportCommand)>,
}

/// A context with the runtime installed whose child viewports are OS windows.
#[non_exhaustive]
pub struct Windows {
    pub ctx: Context,
    root_info: RefCell<ViewportInfo>,
}

impl Windows {
    /// The running build's shell and a scripted system.
    pub fn new() -> Self {
        Self::with(InstallSpec::new(ShellInfo::default(), Arc::new(FakeSystem::new())))
    }

    /// A context installed with `spec`.
    pub fn with(spec: InstallSpec) -> Self {
        let ctx = Frames::uninstalled().ctx;
        ctx.set_embed_viewports(false);
        assert!(craft_ui::install(&ctx, spec).is_ok(), "the context takes the runtime once");
        Context::set_immediate_viewport_renderer(render_child);
        CHILD_EVENTS.with(|events| events.borrow_mut().clear());
        CHILD_INFO.with(|info| *info.borrow_mut() = ViewportInfo::default());
        CHILDREN.with(|children| children.borrow_mut().clear());
        COMMANDS.with(|kept| kept.borrow_mut().clear());
        Windows { ctx, root_info: RefCell::new(ViewportInfo::default()) }
    }

    /// The window facts child passes report from now on.
    pub fn child_info(&self, info: ViewportInfo) {
        CHILD_INFO.with(|kept| *kept.borrow_mut() = info);
    }

    /// The window facts the root viewport reports from now on.
    pub fn root_info(&self, info: ViewportInfo) {
        *self.root_info.borrow_mut() = info;
    }

    /// One frame: `root` events go to the root viewport, `child` events to the child's next pass. Returns the
    /// titles and the window commands of the frame.
    pub fn frame(&self, root: Vec<Event>, child: Vec<Event>, ui: impl FnMut(&mut Ui)) -> Out {
        CHILD_EVENTS.with(|events| *events.borrow_mut() = child);
        set_input(&self.ctx, FrameInput::new(RootEnv::default()));
        let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, ROOT_SIZE)), events: root, ..RawInput::default() };
        ROOT_INFO.with(|info| *info.borrow_mut() = self.root_info.borrow().clone());
        input.viewports = all_viewports(CHILD_INFO.with(|info| info.borrow().clone()));
        let mut output = self.ctx.run_ui(input, ui);
        output.textures_delta.clear();
        keep_commands(&output);
        let titles = output.viewport_output.iter().map(|(id, viewport)| (*id, viewport.builder.title.clone().unwrap_or_default())).collect();
        Out { titles, commands: COMMANDS.with(|kept| std::mem::take(&mut *kept.borrow_mut())) }
    }
}
