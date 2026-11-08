//! `Fixture`: one story or closure shown over [`Frames`], with the env a test varies and input by AccessKit query or part.

use super::Frames;
use super::story::{Story, StoryData, StoryOutput};
use crate::runtime::access::Role;
use crate::runtime::commands::{CommandInput, CommandRef, CommandState};
use crate::runtime::egui::access::{AccessTree, Declared, NodeKey};
use crate::runtime::egui::fixture::{self, AccessActionKind};
use crate::runtime::env::{Direction, Overrides, RootEnv, Scale, with_env};
use craft_behavior::{MotionMode, PartRef};
use craft_host::target::Os;
use craft_theme::{Contrast, Mode, Size, TextSize};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A node of the AccessKit tree, as input addresses it.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Target {
    /// The node.
    pub node: NodeKey,
    /// Its bounds in points.
    pub rect: Rect,
}

impl Target {
    /// The middle of the node's bounds.
    pub fn center(&self) -> Pos2 {
        self.rect.center()
    }
}

/// A closure or story shown frame by frame.
#[non_exhaustive]
pub struct Fixture {
    frames: Frames,
    show: Box<dyn FnMut(&mut Ui)>,
    root: RootEnv,
    commands: CommandInput,
    room: Vec2,
    tree: AccessTree,
    shown: Rect,
    data: Rc<RefCell<StoryData>>,
    outputs: Rc<RefCell<Vec<StoryOutput>>>,
    /// The id of the latest output, which stays when the outputs are taken.
    root_id: Rc<Cell<Option<Id>>>,
    passes: usize,
}

impl Fixture {
    /// A fixture that shows `show` in a room of 400 by 300 points.
    pub fn new(show: impl FnMut(&mut Ui) + 'static) -> Self {
        Fixture {
            frames: Frames::new(),
            show: Box::new(show),
            root: RootEnv::default(),
            commands: CommandInput::new(),
            room: Vec2::new(400.0, 300.0),
            tree: AccessTree::none(),
            shown: Rect::NOTHING,
            data: Rc::default(),
            outputs: Rc::default(),
            root_id: Rc::default(),
            passes: 0,
        }
    }

    /// A fixture that shows `story`; its data and outputs are read with [`Fixture::data`] and [`Fixture::take_outputs`].
    pub fn story(story: &Story) -> Self {
        Self::with_scope(story, None)
    }

    /// A fixture that shows `story` inside an env scope `scope` changes: disabled, read-only, or both.
    pub fn story_scoped(story: &Story, scope: fn(&mut Overrides)) -> Self {
        Self::with_scope(story, Some(scope))
    }

    fn with_scope(story: &Story, scope: Option<fn(&mut Overrides)>) -> Self {
        let data = Rc::new(RefCell::new(StoryData::of(story)));
        let outputs = Rc::new(RefCell::new(Vec::new()));
        let root_id = Rc::new(Cell::new(None));
        let (shared_data, shared_outputs, shared_root, show) = (data.clone(), outputs.clone(), root_id.clone(), story.show);
        let mut fixture = Fixture::new(move |ui| {
            let output = match scope {
                Some(scope) => with_env(ui, scope, |ui| show(ui, &mut shared_data.borrow_mut())),
                None => show(ui, &mut shared_data.borrow_mut()),
            };
            shared_root.set(output.id);
            shared_outputs.borrow_mut().push(output);
        });
        fixture.data = data;
        fixture.outputs = outputs;
        fixture.root_id = root_id;
        fixture
    }

    fn set_root(&mut self, change: impl FnOnce(&mut RootEnv)) -> &mut Self {
        change(&mut self.root);
        self.frames.root(self.root.clone());
        self
    }

    /// Gives app-scope `command` this state in every following pass, as an app's `FrameInput.commands` does; a later
    /// call for the same command replaces the earlier one.
    pub fn command_state(&mut self, command: impl Into<CommandRef>, state: CommandState) -> &mut Self {
        let command = command.into();
        self.commands.states.retain(|(held, _)| *held != command);
        self.commands.states.push((command, state));
        self.frames.commands(self.commands.clone());
        self
    }

    /// The room the story is shown in.
    pub fn room(&mut self, room: Vec2) -> &mut Self {
        self.room = room;
        self
    }

    /// The layout direction.
    pub fn direction(&mut self, direction: Direction) -> &mut Self {
        self.set_root(|root| root.direction = direction)
    }

    /// Light or dark, and standard or high contrast.
    pub fn theme(&mut self, mode: Mode, contrast: Contrast) -> &mut Self {
        self.set_root(|root| {
            root.mode = mode;
            root.contrast = contrast;
        })
    }

    /// The motion mode.
    pub fn motion(&mut self, motion: MotionMode) -> &mut Self {
        self.set_root(|root| root.motion = motion)
    }

    /// The text-size step.
    pub fn text_size(&mut self, text_size: TextSize) -> &mut Self {
        self.set_root(|root| root.text_size = text_size)
    }

    /// The density preset.
    pub fn density(&mut self, density: Size) -> &mut Self {
        self.set_root(|root| root.density = density)
    }

    /// The interface scale.
    pub fn scale(&mut self, scale: Scale) -> &mut Self {
        self.set_root(|root| root.scale = scale)
    }

    /// The host every following frame is given: its language, catalogs and services.
    pub fn host(&mut self, host: std::sync::Arc<craft_host::Host>) -> &mut Self {
        self.frames.host(host);
        self
    }

    /// The operating system the keys and chrome follow: the shell's OS and capabilities, and egui's own OS for its
    /// text editing.
    pub fn os(&mut self, os: Os) -> &mut Self {
        crate::runtime::windows::set_os(&self.frames.ctx, os);
        fixture::set_os(&self.frames.ctx, os);
        self
    }

    /// The manual clock, moved on by `seconds` before the next frame.
    pub fn advance(&mut self, seconds: f64) -> &mut Self {
        self.frames.advance(seconds);
        self
    }

    /// The frames' context.
    pub fn ctx(&self) -> &egui::Context {
        &self.frames.ctx
    }

    /// The story's data.
    pub fn data(&self) -> std::cell::RefMut<'_, StoryData> {
        self.data.borrow_mut()
    }

    /// The story's data, shared with the fixture's show closure.
    pub(crate) fn data_handle(&self) -> Rc<RefCell<StoryData>> {
        self.data.clone()
    }

    /// The outputs the story gave since the last call, one per pass.
    pub fn take_outputs(&mut self) -> Vec<StoryOutput> {
        std::mem::take(&mut *self.outputs.borrow_mut())
    }

    /// The component id of the latest shown output, kept when the outputs are taken.
    pub fn root_id(&self) -> Option<Id> {
        self.root_id.get()
    }

    /// The first part of the shown component that senses a drag.
    pub fn drag_part(&self) -> Option<PartRef> {
        crate::runtime::pointer::sensing_parts(self.ctx(), self.root_id()?, craft_behavior::Sense::DRAG).into_iter().next()
    }

    /// The first part of the shown component that senses a click: what a click activates when the component has
    /// parts of its own.
    pub fn click_part(&self) -> Option<PartRef> {
        crate::runtime::pointer::sensing_parts(self.ctx(), self.root_id()?, craft_behavior::Sense::CLICK).into_iter().next()
    }

    /// The styles the pipeline resolved for the component type `type_name` in this fixture.
    pub fn styles_of(&self, type_name: &str) -> Vec<std::sync::Arc<super::style::StyleSeen>> {
        super::style::styles_of(self.ctx(), type_name)
    }

    /// The text runs the latest frame painted.
    pub fn texts(&self) -> &[fixture::PaintedText] {
        self.frames.texts()
    }

    /// The visual bounds of the shapes the latest frame painted.
    pub fn shape_bounds(&self) -> &[Rect] {
        self.frames.shape_bounds()
    }

    /// Half the stroke width of the shape behind each of `shape_bounds`.
    pub fn shape_overshoot(&self) -> &[f32] {
        self.frames.shape_overshoot()
    }

    /// Whether anything asked for another pass since the latest frame ended.
    pub fn repaint_requested(&self) -> bool {
        self.frames.repaint_requested()
    }

    /// The IME output of the latest frame.
    pub fn ime(&self) -> Option<&fixture::ImeSeen> {
        self.frames.ime()
    }

    /// The area the latest pass allocated.
    pub fn shown(&self) -> Rect {
        self.shown
    }

    /// The passes the latest frame ran.
    pub fn passes(&self) -> usize {
        self.passes
    }

    /// One frame with `events`.
    pub fn frame(&mut self, events: Vec<Event>) -> &mut Self {
        let room = Rect::from_min_size(Pos2::ZERO, self.room);
        let (show, shown) = (&mut self.show, &mut self.shown);
        self.passes = self.frames.frame(events, |ui| {
            let used = super::layout::in_region(ui, room, |ui| {
                show(ui);
                fixture::used_rect(ui)
            });
            *shown = used;
        });
        self.tree = self.frames.tree();
        self
    }

    /// `n` frames with no events.
    pub fn idle(&mut self, n: usize) -> &mut Self {
        for _ in 0..n {
            self.frame(Vec::new());
        }
        self
    }

    /// Frames until ids, sizes and the AccessKit tree have settled.
    pub fn settle(&mut self) -> &mut Self {
        self.idle(4)
    }

    /// The AccessKit tree of the latest frame.
    pub fn tree(&self) -> &AccessTree {
        &self.tree
    }

    /// The node by key, with its bounds, when the latest tree has it.
    pub fn target(&self, node: NodeKey) -> Option<Target> {
        Some(Target { node, rect: self.tree.bounds_of(node)? })
    }

    /// The node named `label`.
    pub fn by_label(&self, label: &str) -> Option<Target> {
        let node = self.tree.keys().into_iter().find(|key| self.tree.name_of(*key) == Some(label))?;
        self.target(node)
    }

    /// The focusable node named `label`: the control, where a field's label and group carry the control's name too.
    pub fn control(&self, label: &str) -> Option<Target> {
        let tree = &self.tree;
        let node = tree.keys().into_iter().find(|key| tree.name_of(*key) == Some(label) && tree.declares_key(*key, Declared::Focus))?;
        self.target(node)
    }

    /// The nodes of `role`.
    pub fn by_role(&self, role: Role) -> Vec<Target> {
        self.tree.with_role(role).into_iter().filter_map(|node| self.target(node)).collect()
    }

    /// The sub-node of `part` below the component `component`.
    pub fn part(&self, component: Id, part: impl Into<PartRef>) -> Option<Target> {
        let id = crate::runtime::access::sub_node_id(component, part.into());
        self.target(NodeKey::of(id))
    }

    /// The node of the component `component`.
    pub fn node_of(&self, component: Id) -> Option<Target> {
        self.target(NodeKey::of(component))
    }

    /// Hover, press and release on `target`, in separate frames, then one idle frame.
    pub fn click(&mut self, target: &Target) -> &mut Self {
        self.click_at(target.center())
    }

    /// Hover, press and release at `pos`, in separate frames, then one idle frame.
    pub fn click_at(&mut self, pos: Pos2) -> &mut Self {
        self.frame(vec![super::pointer::moved(pos)]);
        self.frame(vec![super::pointer::press(pos)]);
        self.frame(vec![super::pointer::release(pos)]);
        self.frame(Vec::new())
    }

    /// The modifiers held in every following frame, until the next call.
    pub fn hold(&mut self, modifiers: Modifiers) -> &mut Self {
        self.frames.hold(modifiers);
        self
    }

    /// Press on `target`.
    pub fn press(&mut self, target: &Target) -> &mut Self {
        let at = target.center();
        self.frame(vec![super::pointer::moved(at)]);
        self.frame(vec![super::pointer::press(at)])
    }

    /// Release at the middle of `target`.
    pub fn release(&mut self, target: &Target) -> &mut Self {
        self.frame(vec![super::pointer::release(target.center())])
    }

    /// Press on `target`, move by `by` in four steps, release.
    pub fn drag(&mut self, target: &Target, by: Vec2) -> &mut Self {
        let from = target.center();
        self.frame(vec![super::pointer::moved(from)]);
        self.frame(vec![super::pointer::press(from)]);
        for step in 1..=4 {
            self.frame(vec![super::pointer::moved(from + by * (step as f32 / 4.0))]);
        }
        self.frame(vec![super::pointer::release(from + by)]);
        self.frame(Vec::new())
    }

    /// A key press and release with `modifiers`, in one frame.
    pub fn key(&mut self, key: Key, modifiers: Modifiers) -> &mut Self {
        self.frame(vec![fixture::key_press(key, modifiers), fixture::key_release(key, modifiers)])
    }

    /// Typed `text`.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        self.frame(vec![fixture::text_event(text)])
    }

    /// An AccessKit action on `target`.
    pub fn access(&mut self, kind: AccessActionKind, target: &Target) -> &mut Self {
        self.frame(vec![fixture::access_request_node(kind, target.node)])
    }

    /// The id of the widget egui has focused.
    pub fn focused(&self) -> Option<Id> {
        fixture::focused(&self.frames.ctx)
    }
}
