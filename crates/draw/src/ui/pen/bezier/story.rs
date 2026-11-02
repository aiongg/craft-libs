//! The path overlay's story, and the host the story and the tests keep their paths in.

use craft_ui::localized::Labels as _;
use craft_ui::testing::canvas_story::{PlainArgs, ViewArgs};
use egui::{Color32, Ui};
use kurbo::{ParamCurve as _, Point};

use super::{
    AnchorsShown, BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierTool, EditPath, EditSubpath, HandlesShown, Knot, KnotRef, PathEdit,
    PathSelection, SegmentMark,
};
use craft_ui::canvas::RulerUnits;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::story::{Level, Story, StoryChoice, Surface, lock, shared};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Bezier paths"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ToolArg {
    Direct,
    Convert,
    Show,
    #[choice(name = "Add anchor")]
    AddAnchor,
    #[choice(name = "Delete anchor")]
    DeleteAnchor,
    #[choice(name = "Auto add/delete")]
    AutoAddDelete,
    Curvature,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum HandlesArg {
    Selected,
    All,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SegmentArg {
    #[choice(name = "Handles only")]
    HandlesOnly,
    Line,
}

#[derive(craft_ui::story::StoryArgs, Debug, PartialEq)]
struct EditArgs {
    #[arg(flatten)]
    view: ViewArgs,
    tool: ToolArg,
    handles: HandlesArg,
    #[arg(name = "segment mark")]
    segment: SegmentArg,
    #[arg(name = "hide anchors")]
    hide_anchors: bool,
    #[arg(name = "preview anchor")]
    preview_anchor: bool,
}

impl Default for EditArgs {
    fn default() -> Self {
        Self {
            view: ViewArgs::default(),
            tool: ToolArg::Direct,
            handles: HandlesArg::Selected,
            segment: SegmentArg::HandlesOnly,
            hide_anchors: false,
            preview_anchor: false,
        }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/bezier-paths/edit", Level::Primitives, "Bezier paths", "Edit", EditArgs::default(), show_paths).on(Surface::Canvas),
        typed_story!("primitives/bezier-paths/curvature", Level::Primitives, "Bezier paths", "Curvature", PlainArgs::default(), show_curvature)
            .on(Surface::Canvas),
    ]
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
            Some(EditEvent::Begin { value, .. }) => {
                self.before = Some(self.paths.clone());
                if let Some(value) = value {
                    apply(&mut self.paths, value);
                }
            }
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

/// Applies an edit to the path it names.
fn apply(paths: &mut [EditPath], edit: &PathEdit) {
    for path in paths {
        if edit.apply(path) {
            break;
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

/// The story's curvature paths: a closed loop of five smooth anchors and an open path with a
/// corner, every handle the Curvature Pen's.
pub(crate) fn curvature_paths() -> Vec<EditPath> {
    let mut loop_ = EditSubpath {
        knots: [(160.0, 90.0), (290.0, 130.0), (270.0, 250.0), (160.0, 300.0), (70.0, 190.0)]
            .map(|(x, y)| Knot::smooth(Point::new(x, y), Point::new(x, y)))
            .to_vec(),
        closed: true,
    };
    loop_.reflow();
    let mut line = EditSubpath {
        knots: vec![
            Knot::smooth(Point::new(380.0, 290.0), Point::new(380.0, 290.0)),
            Knot::smooth(Point::new(450.0, 170.0), Point::new(450.0, 170.0)),
            Knot::corner(Point::new(520.0, 250.0)),
            Knot::smooth(Point::new(600.0, 120.0), Point::new(600.0, 120.0)),
        ],
        closed: false,
    };
    line.reflow();
    vec![
        EditPath { key: 1, subpaths: vec![loop_], color: None },
        EditPath { key: 2, subpaths: vec![line], color: Some(Color32::from_rgb(230, 110, 20)) },
    ]
}

/// Two paths on the story document. Direct: click an anchor (⇧ toggles, ⌥ its subpath), drag
/// anchors, handles (⌥ alone) and segments, drag a marquee on empty canvas; Convert: click an
/// anchor for a corner, drag from it for smooth handles; Add and Delete anchor, Auto add/delete
/// (⌥ swaps Add and Delete) and Curvature act on a click. Tab to the paths: Page keys pick, the
/// arrows move, Insert and Delete add and remove anchors, Esc deselects. The preview anchor shows
/// the highlight a host draws for a hit it chooses.
fn show_paths(ui: &mut Ui, args: &mut EditArgs) {
    let kept = shared(ui, "paths", || PathHost::new(sample_paths()));
    let mut host = lock(&kept);
    let tool = match args.tool {
        ToolArg::Convert => BezierTool::Convert,
        ToolArg::Show => BezierTool::Show,
        ToolArg::Direct => BezierTool::Direct,
        ToolArg::AddAnchor => BezierTool::AddAnchor,
        ToolArg::DeleteAnchor => BezierTool::DeleteAnchor,
        ToolArg::AutoAddDelete => BezierTool::AutoAddDelete,
        ToolArg::Curvature => BezierTool::Curvature,
    };
    let handles = if args.handles == HandlesArg::All { HandlesShown::All } else { HandlesShown::Selected };
    let mark = match args.segment {
        SegmentArg::HandlesOnly => SegmentMark::HandlesOnly,
        SegmentArg::Line => SegmentMark::Line,
    };
    let anchors = if args.hide_anchors { AnchorsShown::Hidden } else { AnchorsShown::All };
    let highlight = args.preview_anchor.then_some(BezierHit::Segment { path: 1, segment: KnotRef::new(0, 0), t: 0.5 });
    let document_name = <craft_ui::story::StoryLabels as craft_ui::localized::Labels>::of(ui).document;
    let canvas = craft_ui::testing::canvas_story::document(ui, args.view, &document_name);
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let names = BezierNames::of(ui);
    let response = BezierOverlay::new("story-bezier", &host.paths, &host.selection, &px, &px, names)
        .tool(tool)
        .handles(handles)
        .segment_mark(mark)
        .anchors(anchors)
        .highlight(highlight)
        .show(ui, &canvas.response, &canvas.view);
    host.apply(&response);
}

/// Two curvature paths under the Curvature tool: drag an anchor and the handles of every smooth
/// anchor follow, click a segment to add an anchor (drag to place it), double-click an anchor to
/// toggle it between smooth and corner; the arrows move the selected anchors and Enter toggles.
fn show_curvature(ui: &mut Ui, args: &mut PlainArgs) {
    let kept = shared(ui, "curvature", || PathHost::new(curvature_paths()));
    let mut host = lock(&kept);
    let document_name = <craft_ui::story::StoryLabels as craft_ui::localized::Labels>::of(ui).document;
    let canvas = craft_ui::testing::canvas_story::document(ui, args.view, &document_name);
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let names = BezierNames::of(ui);
    let response = BezierOverlay::new("story-curvature", &host.paths, &host.selection, &px, &px, names).tool(BezierTool::Curvature).show(
        ui,
        &canvas.response,
        &canvas.view,
    );
    host.apply(&response);
}
