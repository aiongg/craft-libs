//! [`LayerStyleDialog`]: the Layer Style dialog's body.

use craft_theme::Srgba;
use egui::accesskit::Role;
use std::borrow::Cow;
use std::collections::HashMap;

use egui::{Align, AsIdSalt, IdSalt, Layout, RichText, Sense, Ui, UiBuilder, vec2};

use craft_ui::composites::Sampler;
use craft_ui::controls::{BlendChoice, Checkbox};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::edit::parts::Edits;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Divider, Gap, Stack};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::blending::{BlendingInput, blending_page};
use super::contour::{ContourAction, ContourRequest};
use super::labels::{BlendIfBarLabels, ContourFieldLabels, LayerStyleLabels};
use super::list::{ListAction, ListOp, effect_list};
use super::page::{PageInput, PatternChoice, StyleOp, apply_op, edit_style, effect_page};
use super::styles::{StylePresets, styles_page};
use crate::layer_style::{Contour, EffectId, EffectKind, EffectSpec, LayerStyle, LayerStyleDraft, LayerStylePage, StyleParams, StyleValue};

/// What [`LayerStyleDialog::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit and answers the requests"]
pub struct LayerStyleOutput {
    /// The style after this pass's change: a slider drag is a gesture (`Begin`, `Update`…,
    /// `Commit`, Esc `Cancel`); a check, select or an instance added, removed or turned on or off
    /// is a `Set`. Only style changes are edits, so `Dialog::history` never records a page
    /// switch. The host keeps the value and previews it while the draft's `preview` is on;
    /// everything stays inside the dialog's outer gesture, and OK makes the one "Layer Style"
    /// history step.
    pub edit: EditResponse<LayerStyle>,
    /// The page the user chose this pass (a list row, an added or removed instance); the host
    /// keeps it in [`LayerStyleDraft::page`], and the dialog draws it from then on.
    pub page: Option<LayerStylePage>,
    /// The Preview check box changed to this; the host keeps it in [`LayerStyleDraft::preview`].
    pub preview: Option<bool>,
    /// The Blend If entry chosen this pass, an index into `blending.blend_if`; the host keeps it
    /// in [`LayerStyleDraft::blend_if_channel`].
    pub blend_if_channel: Option<usize>,
    /// A colour swatch was clicked: the host opens its colour picker and answers with
    /// [`LayerStyleDialog::set_color`].
    pub pick_color: Option<ColorRequest>,
    /// A style preset was picked on the Styles page (its key): the host looks it up and calls
    /// [`LayerStyleDraft::apply_preset`].
    pub apply_style: Option<String>,
    /// New Style… was pressed: the host asks for a name and saves the enabled instances and the
    /// blend mode and opacities as a preset.
    pub new_style: bool,
    /// Make Default on an effect page: the kind and the page's shown values, to keep as the
    /// user's default for new instances of the kind.
    pub make_default: Option<(EffectKind, StyleParams)>,
    /// Reset to Default on an effect page: the host writes the user's default (or the spec's
    /// defaults before the first Make Default) into the instance's parameters.
    pub reset_to_default: Option<EffectId>,
    /// A request for the host's contour library from a contour field.
    pub contour_action: Option<ContourAction>,
    /// A contour field's thumbnail was clicked: the host opens a
    /// [`ContourEditorDialog`](super::ContourEditorDialog) over the request as a child of the
    /// Layer Style dialog, previews from its [`current`](super::ContourEditorDialog::current)
    /// and answers with [`LayerStyleDialog::set_contour`].
    pub edit_contour: Option<ContourRequest>,
    /// A parameter value the page shows from a default the draft lacks (a Pattern Overlay with no
    /// pattern shows the first one): the instance, the parameter's key and the value. It is not an
    /// edit: [`apply_view`](LayerStyleOutput::apply_view) writes it into the draft without an undo
    /// step, and the report stops.
    pub fill_default: Option<(EffectId, String, StyleValue)>,
}

impl LayerStyleOutput {
    /// Writes the view state this pass reported (page, Preview, Blend If channel) and the
    /// [`fill_default`](LayerStyleOutput::fill_default) into `draft`. The style edit is the host's,
    /// as its history rules apply it.
    pub fn apply_view(&self, draft: &mut LayerStyleDraft) {
        if let Some(page) = self.page {
            draft.page = page;
        }
        if let Some(preview) = self.preview {
            draft.preview = preview;
        }
        if let Some(channel) = self.blend_if_channel {
            draft.blend_if_channel = channel;
        }
        if let Some((id, key, value)) = &self.fill_default {
            draft.set_param(*id, key, value.clone());
        }
    }
}

/// The Layer Style dialog's body: the effect list on the left, the selected page on the right
/// (`layer_style.page_width`, a vertical divider between), as Photoshop lays it out. The host owns
/// the frame (`Dialog` with OK, Cancel and Alt+Cancel Reset), the draft, the engine and the
/// preview.
///
/// - The list: Styles, Blending Options, the effects (one Tab stop, ↑ and ↓ show the page of a row
///   that is selectable, Space toggles, Enter shows the page; effects that are off muted, absent
///   ones dimmed, `+` and `−` for kinds that allow several).
/// - A page change is reported in [`LayerStyleOutput::page`] and drawn once the host has applied
///   it. The page drawn in the pass of a click is the one the click left, so its fields see their
///   focus loss and commit what was typed before the page changes.
/// - The page title is the effect's name (`type.title`) with the Preview check box at its end
///   (it flips [`LayerStyleDraft::preview`]; the Styles page has none).
/// - The Styles page ([`StylePresets`]), the Blending Options page, or an effect page drawn from
///   the kind's [`EffectSpec`].
/// - Use Global Light: on an effect that uses it, the Angle edits the shared light angle of
///   every such effect ([`LayerStyleDraft::set_param`]).
/// - Tab order: the list, then the page's controls; in the dialog Tab visits fields only (the
///   dialog rule). Esc is the frame's (Cancel).
///
/// ```no_run
/// use craft_imaging::layer_style::LayerStyleDraft;
/// use craft_imaging::ui::layer_style::LayerStyleDialog;
/// # fn demo(ui: &mut egui::Ui, draft: &mut LayerStyleDraft, channels: &[String], numbers: &craft_ui::localized::LocaleNumbers) {
/// let out = LayerStyleDialog::new("layer-style", draft, channels, numbers).show(ui);
/// out.apply_view(draft);
/// if let Some(value) = out.edit.event.and_then(|e| e.value().cloned()) {
///     draft.style = value; // and re-render the preview when `draft.render_hash()` changed
/// }
/// # }
/// ```
#[must_use = "show the dialog body with `show`"]
pub struct LayerStyleDialog<'a> {
    id_salt: IdSalt,
    draft: &'a LayerStyleDraft,
    channel_names: &'a [String],
    numbers: &'a LocaleNumbers,
    specs: Option<&'a [EffectSpec]>,
    blend_keys: Option<&'a HashMap<String, String>>,
    blend_modes: &'a [BlendChoice],
    patterns: &'a [PatternChoice],
    user_contours: &'a [Contour],
    sampler: Option<&'a Sampler>,
    style_presets: Option<StylePresets<'a>>,
    labels: Option<LayerStyleLabels>,
    policy: EditPolicy,
    enabled: bool,
}

impl<'a> LayerStyleDialog<'a> {
    /// The body over `draft`; `channel_names` are the document's colour channels ("R", "G", "B";
    /// "C", "M", "Y", "K"; "L", "a", "b"; "Gray") for the Channels checks and Blend If.
    pub fn new(id_salt: impl AsIdSalt, draft: &'a LayerStyleDraft, channel_names: &'a [String], numbers: &'a LocaleNumbers) -> Self {
        LayerStyleDialog {
            id_salt: IdSalt::new(id_salt),
            draft,
            channel_names,
            numbers,
            specs: None,
            blend_keys: None,
            blend_modes: &[],
            patterns: &[],
            user_contours: &[],
            sampler: None,
            style_presets: None,
            labels: None,
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// The effect tables in list order; Photoshop's ten ([`EffectSpec::photoshop`] for each of
    /// [`EffectKind::ALL`]) by default. A kind without a spec is not listed.
    pub fn specs(mut self, specs: &'a [EffectSpec]) -> Self {
        self.specs = Some(specs);
        self
    }

    /// Renames the blend defaults of every spec, the built-in ones included, from Photoshop's
    /// English names to the host's [`BlendChoice`] keys (`"Multiply"` to the host's key). Effects
    /// added in the dialog start from the renamed defaults, so their Blend selects show the
    /// host's entry; instances already in the draft are the host's to key.
    pub fn blend_keys(mut self, map: &'a HashMap<String, String>) -> Self {
        self.blend_keys = Some(map);
        self
    }

    /// The blend modes the Blend Mode selects offer (the host's list, with its separators).
    pub fn blend_modes(mut self, modes: &'a [BlendChoice]) -> Self {
        self.blend_modes = modes;
        self
    }

    /// The patterns Pattern Overlay offers.
    pub fn patterns(mut self, patterns: &'a [PatternChoice]) -> Self {
        self.patterns = patterns;
        self
    }

    /// The host's contour library, after the built-in presets in every contour picker.
    pub fn user_contours(mut self, contours: &'a [Contour]) -> Self {
        self.user_contours = contours;
        self
    }

    /// The host's interpolation of a contour's points, so the contour fields draw the curve the
    /// engine renders ([`ContourField::sampler`](super::ContourField::sampler)); a monotone cubic
    /// by default.
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// The Styles page's presets; without them the page shows only New Style….
    pub fn style_presets(mut self, presets: StylePresets<'a>) -> Self {
        self.style_presets = Some(presets);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: LayerStyleLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// How a drag or a typing run in a page reports; realtime by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Turns editing off with `false`. The editor is also off while `ui` is disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the list and the page and reports this pass's change. List actions and field edits
    /// are applied to a copy of the draft with the [`LayerStyleDraft`] rules; the copy's style is
    /// the edit's value and its page, Preview and Blend If channel are reported beside it.
    pub fn show(self, ui: &mut Ui) -> LayerStyleOutput {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:831-1185 `body` (list, divider at 330, title
        // with Preview, Styles page, Blending Options page, field pages, Make Default and Reset to
        // Default, the contour editor).
        // new: the body keeps no history and reads no undo key; the host's `Dialog` frame owns the
        // style's undo (the dialog is one outer gesture, OK one step), and the view state is not
        // part of the value it records.
        let LayerStyleDialog {
            id_salt,
            draft,
            channel_names,
            numbers,
            specs,
            blend_keys,
            blend_modes,
            patterns,
            user_contours,
            sampler,
            mut style_presets,
            labels,
            policy,
            enabled,
        } = self;
        let id = ui.make_persistent_id(id_salt);
        let labels = labels.unwrap_or_else(|| LayerStyleLabels::of(ui));
        let contour_labels = ContourFieldLabels::of(ui);
        let bar_labels = BlendIfBarLabels::of(ui);
        let base: Cow<'_, [EffectSpec]> = match specs {
            Some(specs) => Cow::Borrowed(specs),
            None => Cow::Owned(EffectKind::ALL.into_iter().map(EffectSpec::photoshop).collect()),
        };
        let keyed: Vec<EffectSpec>;
        let specs: &[EffectSpec] = match blend_keys {
            Some(map) => {
                keyed = base.iter().cloned().map(|spec| spec.with_blend_keys(map)).collect();
                &keyed
            }
            None => &base,
        };
        let tokens = Tokens::of(ui);
        let style = LayerStyleDialog::style(&tokens);

        let mut builder = UiBuilder::new().scope_id(id);
        if !enabled {
            builder = builder.disabled();
        }
        let mut work = Work { next: draft.clone(), shown: draft.page, extras: Extras::default() };
        let scope = ui.scope_builder(builder, |ui| {
            let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) };
            let ((), event) = edit_style(ui, id.with("style"), &draft.style, policy, enabled, |ui, edits| {
                ui.with_layout(layout, |ui| {
                    let list = ui.scope(|ui| effect_list(ui, draft, specs, &labels, numbers, enabled));
                    // The list's change goes first: the page is built on the style it leaves.
                    let op = list.inner.and_then(|action| work.list_action(action, specs));
                    let ops = op.clone().map(StyleOp::List);
                    let event = ops.as_ref().map(|_| EditEvent::set(draft.style.clone(), egui::Modifiers::NONE));
                    let change = EditResponse::new(list.response.clone(), event);
                    edits.push(&change, ops, apply_op, |_, base| match &op {
                        Some(op) => apply_op(&StyleOp::List(op.clone()), base).unwrap_or_else(|| base.clone()),
                        None => base.clone(),
                    });
                    let (slot, _) = ui.allocate_exact_size(vec2(ui.spacing().item_spacing.x, 0.0), Sense::hover());
                    let page = ui.scope(|ui| {
                        ui.set_width(style.page_width);
                        let page_id = ("page", work.shown);
                        let title = page_title(work.shown, &work.next, &labels);
                        let group = UiBuilder::new().id_salt(page_id).accessibility_role(Role::Group).accessibility_label(title);
                        ui.scope_builder(group, |ui| {
                            Stack::new(Gap::Row).show(ui, |ui| {
                                let env = PageEnv {
                                    specs,
                                    blend_modes,
                                    patterns,
                                    user_contours,
                                    sampler,
                                    channel_names,
                                    numbers,
                                    labels: &labels,
                                    contour_labels: &contour_labels,
                                    bar_labels: &bar_labels,
                                    enabled,
                                };
                                work.page(ui, &env, style_presets.as_mut(), edits);
                            });
                        });
                    });
                    let top = list.response.rect.top();
                    let bottom = list.response.rect.bottom().max(page.response.rect.bottom());
                    Divider::paint(ui.painter(), egui::pos2(slot.center().x, top), egui::pos2(slot.center().x, bottom), &tokens);
                });
            });
            event
        });
        let Work { next, extras, .. } = work;
        LayerStyleOutput {
            edit: EditResponse::new(scope.response, scope.inner),
            page: (next.page != draft.page).then_some(next.page),
            preview: (next.preview != draft.preview).then_some(next.preview),
            blend_if_channel: (next.blend_if_channel != draft.blend_if_channel).then_some(next.blend_if_channel),
            pick_color: extras.pick_color,
            apply_style: extras.apply_style,
            new_style: extras.new_style,
            make_default: extras.make_default,
            reset_to_default: extras.reset_to_default,
            contour_action: extras.contour_action,
            edit_contour: extras.edit_contour,
            fill_default: extras.fill_default,
        }
    }

    /// Answers a [`ColorRequest`] the dialog made: sets the colour parameter its key names
    /// (`"fx2/color"`; the instance id, a slash, the parameter key) to `color`. Returns false,
    /// changing nothing, for a key that names no instance of `draft`.
    pub fn set_color(draft: &mut LayerStyleDraft, key: &str, color: Srgba) -> bool {
        // pc: crates/ui-egui/src/dialogs/layer_style/color_picker.rs:128-136 `confirm` (the answer
        // written through `set_param` into the effect and field the request named).
        Self::set_param_by_key(draft, key, StyleValue::Color(color))
    }

    /// Answers a [`ContourRequest`] the dialog made: sets the contour parameter its key names to
    /// `contour` (the editor's live values and its OK; Cancel writes the editor's initial
    /// contour). Returns false, changing nothing, for a key that names no instance of `draft`.
    pub fn set_contour(draft: &mut LayerStyleDraft, key: &str, contour: Contour) -> bool {
        Self::set_param_by_key(draft, key, StyleValue::Contour(contour))
    }

    fn set_param_by_key(draft: &mut LayerStyleDraft, key: &str, value: StyleValue) -> bool {
        let Some((id, field)) = key.split_once('/') else { return false };
        let Some(id) = EffectId::from_key(id).filter(|id| draft.style.effect(*id).is_some()) else { return false };
        if field.is_empty() {
            return false;
        }
        draft.set_param(id, field, value);
        true
    }
}

/// What the page needs besides the draft.
struct PageEnv<'a> {
    specs: &'a [EffectSpec],
    blend_modes: &'a [BlendChoice],
    patterns: &'a [PatternChoice],
    user_contours: &'a [Contour],
    sampler: Option<&'a Sampler>,
    channel_names: &'a [String],
    numbers: &'a LocaleNumbers,
    labels: &'a LayerStyleLabels,
    contour_labels: &'a ContourFieldLabels,
    bar_labels: &'a BlendIfBarLabels,
    enabled: bool,
}

/// The requests of a pass besides the edit.
#[derive(Default)]
struct Extras {
    pick_color: Option<ColorRequest>,
    apply_style: Option<String>,
    new_style: bool,
    make_default: Option<(EffectKind, StyleParams)>,
    reset_to_default: Option<EffectId>,
    contour_action: Option<ContourAction>,
    edit_contour: Option<ContourRequest>,
    fill_default: Option<(EffectId, String, StyleValue)>,
}

/// The pass's working copy of the draft, for the view state the pass changes. The style is the
/// edit queue's.
struct Work {
    next: LayerStyleDraft,
    /// The page the pass draws: the draft's, whatever a list action in this pass chose.
    shown: LayerStylePage,
    extras: Extras,
}

impl Work {
    /// Applies a list action to the copy (its view state: the page an added effect opens on) and
    /// returns its change to the style, which the edit queue applies.
    fn list_action(&mut self, action: ListAction, specs: &[EffectSpec]) -> Option<ListOp> {
        match action {
            ListAction::Select(page) => {
                self.next.page = page;
                None
            }
            ListAction::Toggle(id) => {
                self.next.toggle(id);
                Some(ListOp::Toggle(id))
            }
            ListAction::Add(kind) => {
                let defaults = specs.iter().find(|spec| spec.kind == kind).map(|spec| spec.defaults.clone()).unwrap_or_default();
                self.next.add_instance(kind, &defaults);
                Some(ListOp::Add { kind, defaults })
            }
            ListAction::Remove(id) => {
                self.next.remove_instance(id);
                Some(ListOp::Remove(id))
            }
        }
    }

    /// Draws the page of the pass; its controls push their edits to `edits`.
    fn page(&mut self, ui: &mut Ui, env: &PageEnv<'_>, presets: Option<&mut StylePresets<'_>>, edits: &mut Edits<LayerStyle, StyleOp>) {
        let labels = env.labels;
        let page = self.shown;
        let instance = match page {
            LayerStylePage::Effect(id) => edits.value().effect(id).cloned(),
            _ => None,
        };
        match (page, instance) {
            (LayerStylePage::Blending, _) => {
                self.title_with_preview(ui, labels, &labels.blending_options);
                let input = BlendingInput {
                    blend_if_channel: self.next.blend_if_channel,
                    channel_names: env.channel_names,
                    blend_modes: env.blend_modes,
                    labels,
                    bar_labels: env.bar_labels,
                    numbers: env.numbers,
                    enabled: env.enabled,
                };
                let out = blending_page(ui, &input, edits);
                if let Some(channel) = out.blend_if_channel {
                    self.next.blend_if_channel = channel;
                }
            }
            (LayerStylePage::Effect(id), Some(instance)) => {
                self.title_with_preview(ui, labels, labels.effect(instance.kind));
                let Some(spec) = env.specs.iter().find(|spec| spec.kind == instance.kind) else { return };
                let input = PageInput {
                    spec,
                    instance: &instance,
                    blend_modes: env.blend_modes,
                    patterns: env.patterns,
                    user_contours: env.user_contours,
                    sampler: env.sampler,
                    labels,
                    contour_labels: env.contour_labels,
                    numbers: env.numbers,
                    enabled: env.enabled,
                };
                let out = effect_page(ui, &input, edits);
                self.extras.pick_color = out.pick_color;
                self.extras.contour_action = out.contour_action;
                self.extras.edit_contour = out.edit_contour;
                self.extras.make_default = out.make_default.map(|params| (instance.kind, params));
                self.extras.reset_to_default = out.reset_to_default.then_some(id);
                self.extras.fill_default = out.fill_default.map(|(key, value)| (id, key, value));
            }
            (LayerStylePage::Effect(_), None) => {}
            (LayerStylePage::Styles, _) => {
                let out = styles_page(ui, presets, labels);
                self.extras.apply_style = out.apply_style;
                self.extras.new_style = out.new_style;
            }
        }
    }

    /// The page title with the Preview check box at its end; a click flips the copy's `preview`.
    fn title_with_preview(&mut self, ui: &mut Ui, labels: &LayerStyleLabels, title: &str) {
        let mut preview = self.next.preview;
        let edit = title_row(ui, title, |ui| Checkbox::bool(labels.preview.as_str(), &mut preview).id_salt("preview").show(ui));
        if edit.event.is_some() {
            self.next.preview = preview;
        }
    }
}

/// The title of `page` of `draft`, for the page's AccessKit group.
fn page_title(page: LayerStylePage, draft: &LayerStyleDraft, labels: &LayerStyleLabels) -> String {
    match page {
        LayerStylePage::Blending => labels.blending_options.clone(),
        LayerStylePage::Effect(id) => match draft.style.effect(id) {
            Some(instance) => labels.effect(instance.kind).to_owned(),
            None => labels.styles.clone(),
        },
        LayerStylePage::Styles => labels.styles.clone(),
    }
}

/// A page title in `type.title` with `end` at the row's end.
pub(super) fn title_row<R>(ui: &mut Ui, title: &str, end: impl FnOnce(&mut Ui) -> R) -> R {
    let style = LayerStyleDialog::style(&Tokens::of(ui));
    direction::horizontal(ui, |ui| {
        ui.label(RichText::new(title).font(style.title_font.clone()).color(style.title_color));
        ui.with_layout(direction::trailing(ui), end).inner
    })
    .inner
}
