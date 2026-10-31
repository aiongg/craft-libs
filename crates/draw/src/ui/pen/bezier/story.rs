//! The path overlay's story, and the host the story and the tests keep their paths in.

use craft_ui::localized::Labels as _;
use craft_ui::testing::canvas_story::ViewArgs;
use egui::{Color32, Ui};
use kurbo::{ParamCurve as _, Point};

use super::{
    BezierNames, BezierOverlay, BezierResponse, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef, PathEdit, PathSelection,
};
use craft_ui::canvas::RulerUnits;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::story::{Level, Story, StoryChoice, Surface};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Bezier paths"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ToolArg {
    Direct,
    Convert,
    Show,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum HandlesArg {
    Selected,
    All,
}

#[derive(craft_ui::story::StoryArgs, Debug, PartialEq)]
struct EditArgs {
    #[arg(flatten)]
    view: ViewArgs,
    tool: ToolArg,
    handles: HandlesArg,
}

impl Default for EditArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), tool: ToolArg::Direct, handles: HandlesArg::Selected }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("primitives/bezier-paths/edit", Level::Primitives, "Bezier paths", "Edit", EditArgs::default(), show_paths).on(Surface::Canvas)]
}

/// A host's paths and selection, as the story and the tests keep them: it applies each edit to
/// the paths it had at the press, as a host showing the updates does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PathHost {
    pub paths: Vec<EditPath>,
    pub selection: PathSelection,
    /// The paths when a drag began, to apply its updates to and restore on a cancel.
    pub before: Option<Vec<EditPath>>,
}

impl PathHost {
    pub(crate) fn new(paths: Vec<EditPath>) -> Self {
        Self { paths, selection: PathSelection::default(), before: None }
    }

    /// Applies a pass's selection, edit, insert and delete.
    pub(crate) fn apply(&mut self, r: &BezierResponse) {
        if let Some(select) = &r.select {
            self.selection = select.clone();
        }
        match &r.event {
            Some(EditEvent::Begin { .. }) => self.before = Some(self.paths.clone()),
            Some(EditEvent::Update { value, .. }) => {
                if let Some(before) = &self.before {
                    self.paths = before.clone();
                    apply(&mut self.paths, value);
                }
            }
            Some(EditEvent::Commit { value, .. }) => {
                if let Some(before) = self.before.take() {
                    self.paths = before;
                }
                apply(&mut self.paths, value);
            }
            Some(EditEvent::Set { value, .. }) => apply(&mut self.paths, value),
            Some(EditEvent::Cancel { .. }) => {
                if let Some(before) = self.before.take() {
                    self.paths = before;
                }
            }
            Some(_) | None => {}
        }
        if let Some((path, at, t)) = r.insert
            && let Some(knot) = insert(&mut self.paths, path, at, t)
        {
            self.selection = PathSelection { path: Some(path), anchors: vec![knot], segment: None };
        }
        if let Some((path, anchors)) = &r.delete {
            delete(&mut self.paths, *path, anchors);
            self.selection.anchors.clear();
        }
    }
}

fn find(paths: &mut [EditPath], key: u64) -> Option<&mut EditPath> {
    paths.iter_mut().find(|p| p.key == key)
}

fn knot_mut(paths: &mut [EditPath], key: u64, at: KnotRef) -> Option<&mut Knot> {
    find(paths, key)?.subpaths.get_mut(at.subpath)?.knots.get_mut(at.knot)
}

/// Applies an edit as PhotoCraft's `photocraft_vector::edit` does.
pub(crate) fn apply(paths: &mut [EditPath], edit: &PathEdit) {
    match edit {
        PathEdit::MoveAnchors { path, anchors, by } => {
            for &at in anchors {
                if let Some(k) = knot_mut(paths, *path, at) {
                    k.anchor += *by;
                    k.in_ctrl += *by;
                    k.out_ctrl += *by;
                }
            }
        }
        &PathEdit::MoveHandle { path, knot, side, to, alone } => {
            let Some(k) = knot_mut(paths, path, knot) else { return };
            let other = match side {
                HandleSide::In => HandleSide::Out,
                HandleSide::Out => HandleSide::In,
            };
            let length = (k.handle(other) - k.anchor).hypot();
            let away = k.anchor - to;
            // A smooth knot keeps its handles collinear, the other keeping its length.
            let opposite = (k.smooth && !alone && length > 0.0 && away.hypot() > 0.0).then(|| k.anchor + away.normalize() * length);
            match side {
                HandleSide::In => k.in_ctrl = to,
                HandleSide::Out => k.out_ctrl = to,
            }
            match (other, opposite) {
                (HandleSide::In, Some(p)) => k.in_ctrl = p,
                (HandleSide::Out, Some(p)) => k.out_ctrl = p,
                _ => {}
            }
            if alone {
                k.smooth = false;
            }
        }
        &PathEdit::BendSegment { path, segment, t, by } => {
            let Some(sub) = find(paths, path).and_then(|p| p.subpaths.get_mut(segment.subpath)) else { return };
            let Some((a, b)) = sub.segment_ends(segment.knot) else { return };
            // The point at t follows `by`; each handle moves by its weight, the nearer one more.
            let t = t.clamp(0.05, 0.95);
            let s = 1.0 - t;
            let k = 1.0 / (3.0 * s * t * (s * s + t * t));
            sub.knots[a].out_ctrl += by * (s * k);
            sub.knots[b].in_ctrl += by * (t * k);
        }
        &PathEdit::Convert { path, knot, out } => {
            let Some(k) = knot_mut(paths, path, knot) else { return };
            *k = match out {
                None => Knot::corner(k.anchor),
                Some(out) => Knot::smooth(k.anchor, out),
            };
        }
    }
}

/// Adds an anchor at `t` to the segment leaving `at`, keeping the curve's shape (de Casteljau);
/// the new knot.
pub(crate) fn insert(paths: &mut [EditPath], path: u64, at: KnotRef, t: f64) -> Option<KnotRef> {
    let sub = find(paths, path)?.subpaths.get_mut(at.subpath)?;
    let (a, b) = sub.segment_ends(at.knot)?;
    let c = sub.segment(at.knot)?;
    let (left, right) = (c.subsegment(0.0..t), c.subsegment(t..1.0));
    let straight = sub.is_straight(at.knot);
    sub.knots[a].out_ctrl = left.p1;
    sub.knots[b].in_ctrl = right.p2;
    let knot = Knot { anchor: left.p3, in_ctrl: left.p2, out_ctrl: right.p1, smooth: !straight };
    sub.knots.insert(a + 1, knot);
    Some(KnotRef::new(at.subpath, a + 1))
}

/// Removes anchors, joining their neighbours; an emptied subpath goes.
pub(crate) fn delete(paths: &mut [EditPath], path: u64, anchors: &[KnotRef]) {
    let Some(p) = find(paths, path) else { return };
    let mut gone = anchors.to_vec();
    gone.sort_unstable();
    for at in gone.into_iter().rev() {
        if let Some(sub) = p.subpaths.get_mut(at.subpath)
            && at.knot < sub.knots.len()
        {
            sub.knots.remove(at.knot);
        }
    }
    p.subpaths.retain(|s| !s.knots.is_empty());
}

/// The story's paths: a closed curved shape in the overlay's colour and an open path in a layer
/// colour.
pub(crate) fn sample_paths() -> Vec<EditPath> {
    let shape = EditSubpath {
        knots: vec![
            Knot::smooth(Point::new(160.0, 100.0), Point::new(220.0, 100.0)),
            Knot::corner(Point::new(300.0, 200.0)),
            Knot::smooth(Point::new(180.0, 300.0), Point::new(120.0, 300.0)),
            Knot { anchor: Point::new(80.0, 200.0), in_ctrl: Point::new(80.0, 260.0), out_ctrl: Point::new(80.0, 140.0), smooth: true },
        ],
        closed: true,
    };
    let wave = EditSubpath {
        knots: vec![
            Knot::corner(Point::new(360.0, 280.0)),
            Knot::smooth(Point::new(460.0, 160.0), Point::new(500.0, 120.0)),
            Knot::corner(Point::new(580.0, 120.0)),
        ],
        closed: false,
    };
    vec![
        EditPath { key: 1, subpaths: vec![shape], color: None },
        EditPath { key: 2, subpaths: vec![wave], color: Some(Color32::from_rgb(230, 110, 20)) },
    ]
}

/// Two paths on the story document. Direct: click an anchor (⇧ toggles, ⌥ its subpath), drag
/// anchors, handles (⌥ alone) and segments, drag a marquee on empty canvas; Convert: click an
/// anchor for a corner, drag from it for smooth handles. Tab to the paths: Page keys pick, the
/// arrows move, Insert and Delete add and remove anchors, Esc deselects.
fn show_paths(ui: &mut Ui, args: &mut EditArgs) {
    let key = ui.scope_id().with("bezier-story");
    let mut host: PathHost = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| PathHost::new(sample_paths()));
    let tool = match args.tool {
        ToolArg::Convert => BezierTool::Convert,
        ToolArg::Show => BezierTool::Show,
        ToolArg::Direct => BezierTool::Direct,
    };
    let handles = if args.handles == HandlesArg::All { HandlesShown::All } else { HandlesShown::Selected };
    let canvas = craft_ui::testing::canvas_story::document(ui, args.view, "Document");
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let names = BezierNames::of(ui);
    let response = BezierOverlay::new("story-bezier", &host.paths, &host.selection, &px, &px, names).tool(tool).handles(handles).show(
        ui,
        &canvas.response,
        &canvas.view,
    );
    host.apply(&response);
    ui.data_mut(|d| d.insert_temp(key, host));
}
