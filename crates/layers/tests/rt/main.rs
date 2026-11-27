//! Count trials of the Layers panel on craft-ui-rt, shown as an app shows it (the panel inside a memoized view over a
//! `Tracked` tree, as the experiment's glue and PhotoCraft's dock do): a hover repaints the rows it leaves and enters
//! and runs nothing, a select runs the same nodes at any list length and one row more when it moves the selection, a
//! footer click adds one layer at the same cost at any list length, and an idle window runs no frame once the
//! thumbnails have landed. Every count assertion is a relation between counts, never a time. A row's eye flips its
//! layer from a click and from an assistive technology's click on the toggle under the row's cell.

// The derives' output names the UI library as `craft_ui`.
extern crate craft_ui_panels as craft_ui;

use craft_behavior::{EditEvent, IconId, SelectionChange};
use craft_layers::ui::mock::FakeLayers;
use craft_layers::ui::options::LayersSettings;
use craft_layers::ui::panel::{LayerColumn, LayersEvent, LayersPanel};
use craft_layers::ui::thumbs::LayerThumb;
use craft_layers::ui::tree::{LayerPart, LayerRowKey, LayerSelection};
use craft_ui_panels::contract::{MemoProps, MemoView, Tracked, Ui};
use craft_ui_panels::list::ThumbSize;
use craft_ui_panels::primitives::IconSpec;
use craft_ui_panels::primitives::textures::{Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui_panels::sections::{FooterAction, FooterBar};
use craft_ui_panels::testing::{FRAME, Harness, Target};
use craft_ui_rt::{RgbaImage, SizeU, StepReport, Vec2};

/// The panel's size in the experiment (fixture::PANEL_WIDTH x PANEL_HEIGHT).
const PANEL: Vec2 = Vec2::new(280.0, 480.0);
const NEW_LAYER: usize = 1;

/// Every layer's picture: one grey square at the requested side, ready at once.
struct Grey;

impl ThumbSource<LayerThumb<u32>> for Grey {
    fn key_revision(&self, key: &LayerThumb<u32>) -> Revision {
        Revision::of(&key.layer)
    }

    fn render(&self, _key: &LayerThumb<u32>, request: &ThumbRequest) -> ThumbRender {
        let side = request.side.max(1);
        let mut image = RgbaImage::new(SizeU::new(side, side));
        image.pixels.chunks_exact_mut(4).for_each(|px| px.copy_from_slice(&[0x80, 0x80, 0x80, 0xff]));
        ThumbRender::Ready(image)
    }
}

struct App {
    tree: Tracked<FakeLayers>,
    settings: LayersSettings,
}

#[derive(MemoProps)]
struct Body<'a> {
    tree: &'a mut Tracked<FakeLayers>,
    #[memo_props(covered_by = "tree", reason = "the settings are fixed when the panel is made")]
    settings: &'a LayersSettings,
}

impl Body<'_> {
    fn show(&mut self, ui: &mut Ui) {
        let thumbs = Thumbnails::<LayerThumb<u32>>::in_context(ui.ctx(), "rt-layer-thumbs", ThumbnailOptions::new());
        let mut picked = None;
        let out = LayersPanel::new(&*self.tree)
            .settings(self.settings)
            .thumbnails(&thumbs, &Grey)
            .lock_column(true)
            .blend_rows(false)
            .filter_bar(false)
            .deletable(true)
            .footer(|ui| {
                let actions = [
                    FooterAction::new(IconSpec::svg(IconId::new("trash")), "Delete Layer"),
                    FooterAction::new(IconSpec::svg(IconId::new("square_plus")), "New Layer"),
                ];
                picked = FooterBar::actions(actions).show_footer(ui).picked;
            })
            .document(0u64)
            .show(ui);
        let mut selection = None;
        let mut shown = Vec::new();
        for event in out.events {
            match event {
                LayersEvent::Select(select) => {
                    if let SelectionChange::Set(ids) = select.change {
                        let active = ids.last().copied();
                        selection = Some(LayerSelection::new(ids, active, LayerPart::Pixels));
                    }
                }
                // As the experiment's glue applies them: a plain press is a `Set`, a sweep ends with a `Commit`.
                LayersEvent::Toggle(toggle) if toggle.column == LayerColumn::Visibility => {
                    if let EditEvent::Set { after, .. } | EditEvent::Commit { after, .. } = toggle.edit {
                        shown.extend(toggle.rows.iter().filter_map(|row| match row {
                            LayerRowKey::Layer(id) => Some((*id, after)),
                            _ => None,
                        }));
                    }
                }
                _ => {}
            }
        }
        if selection.is_some() || picked == Some(NEW_LAYER) || !shown.is_empty() {
            self.tree.edit(|tree| {
                for (id, on) in shown {
                    tree.layer_mut(id).info.visible = on;
                }
                if let Some(selection) = selection {
                    tree.selection = selection;
                }
                if picked == Some(NEW_LAYER) {
                    let id = u32::try_from(tree.layers.len() + 1).expect("fewer than 2^32 layers");
                    let info = craft_layers::ui::tree::LayerInfo::new(format!("Layer {id}"), craft_layers::ui::tree::LayerKind::Pixel);
                    tree.add(None, info, false);
                }
            });
        }
    }
}

fn view(ui: &mut Ui, app: &mut App) {
    let body = Body { tree: &mut app.tree, settings: &app.settings };
    let _ = MemoView::new("rt.layers", body, |body: &mut Body<'_>, ui: &mut Ui| body.show(ui)).show(ui);
}

/// The panel over `n` flat layers, settled: the band built and every visible thumbnail landed.
fn panel(n: usize) -> Harness<App> {
    let mut settings = LayersSettings::default();
    settings.options.thumbnail_size = ThumbSize::Medium;
    let app = App { tree: Tracked::new(FakeLayers::flat(n)), settings };
    let mut h = Harness::builder().with_size(PANEL).build_ui_state(view, app);
    h.settle();
    h
}

/// The row named `label`: its cells carry the row's name too, as a grid's cells do.
fn row(h: &Harness<App>, label: &str) -> Target {
    h.get_by_role_and_label(accesskit::Role::TreeItem, label)
}

fn hover(h: &mut Harness<App>, label: &str) -> StepReport {
    let row = row(h, label);
    h.hover(&row);
    h.last_report()
}

fn click(h: &mut Harness<App>, target: Target) -> StepReport {
    h.click(&target);
    h.last_report()
}

#[test]
fn a_hover_repaints_the_rows_it_leaves_and_enters_and_runs_nothing() {
    for n in [20, 600] {
        let mut h = panel(n);
        hover(&mut h, "Layer 2");
        let moved = hover(&mut h, "Layer 3");
        assert_eq!(moved.frame.painted, 2, "N={n}: only the row the pointer left and the hovered row repaint: {moved:?}");
        assert_eq!((moved.frame.runs, moved.frame.scopes, moved.frame.built), (0, 0, 0), "N={n}: a hover is a kept variant: {moved:?}");
    }
}

#[test]
fn a_select_runs_the_same_nodes_at_any_length_and_one_row_more_when_it_moves() {
    let mut first = Vec::new();
    for n in [20, 600] {
        let mut h = panel(n);
        let target = row(&h, "Layer 3");
        let selects = click(&mut h, target);
        assert_eq!(h.state().tree.selection.active, Some(3), "N={n}: the click selects Layer 3");
        let target = row(&h, "Layer 5");
        let moves = click(&mut h, target);
        assert_eq!(h.state().tree.selection.active, Some(5), "N={n}: the click moves the selection to Layer 5");
        assert_eq!(
            moves.frame.runs,
            selects.frame.runs + 1,
            "N={n}: moving the selection also runs the row it leaves, and no other row: {selects:?} then {moves:?}"
        );
        first.push(selects.frame.runs);
    }
    assert_eq!(first[0], first[1], "a select runs the same nodes at 20 and 600 layers (the band, not the list, is built)");
}

#[test]
fn a_footer_click_adds_one_layer_at_the_same_cost_at_any_length() {
    let mut runs = Vec::new();
    for n in [20, 600] {
        let mut h = panel(n);
        let target = h.get_by_label("New Layer");
        let added = click(&mut h, target);
        assert_eq!(h.state().tree.layers.len(), n + 1, "N={n}: New Layer adds one layer");
        runs.push(added.frame.runs);
    }
    assert_eq!(runs[0], runs[1], "a footer click runs the same nodes at 20 and 600 layers");
}

#[test]
fn an_idle_window_runs_no_frame_once_the_thumbnails_have_landed() {
    for n in [20, 600] {
        let mut h = panel(n);
        for step in 0..5 {
            let report = h.step_dt(FRAME);
            assert!(!report.ran && report.frame.runs == 0, "N={n}, idle step {step}: nothing runs: {report:?}");
        }
    }
}

#[test]
fn a_rows_eye_flips_its_layer_from_a_click_and_from_its_toggle_node() {
    let mut h = panel(20);
    let visible = |h: &Harness<App>| h.state().tree.layers.iter().find(|layer| layer.id == 3).map(|layer| layer.info.visible);
    assert_eq!(visible(&h), Some(true), "Layer 3 starts visible");
    let eye = h.get_by_label("Visibility Layer 3");
    click(&mut h, eye);
    assert_eq!(visible(&h), Some(false), "a click on the eye hides Layer 3");
    let eye = h.get_by_label("Visibility Layer 3");
    h.access(accesskit::Action::Click, &eye);
    h.step();
    assert_eq!(visible(&h), Some(true), "an assistive technology's click on the eye's node shows Layer 3 again");
}
