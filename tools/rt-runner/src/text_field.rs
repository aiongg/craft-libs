//! A single-line text field node. It owns a [`TextEdit`] for its draft, takes key, text, IME,
//! paste and accessibility input while it has focus, gives the platform IME session its caret
//! rect after each layout, and reports Enter, Escape, Tab and focus loss to its parent. What
//! those mean (commit, cancel, the next rename) is the parent's.

use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use craft_behavior::{EditorInput, Sense};
use craft_host::target::Os;
use craft_theme::Rgba;
use craft_ui_rt::{Margin, Pos2, Rect, Vec2};

use crate::text_edit::{Applied, TextEdit, editor_input};
use craft_ui_rt::AccessCx;
use craft_ui_rt::BuildCx;
use craft_ui_rt::Constraints;
use craft_ui_rt::Element;
use craft_ui_rt::ImePurpose;
use craft_ui_rt::LayoutCx;
use craft_ui_rt::PaintCx;
use craft_ui_rt::{Event, EventCx, Gesture, Handled};
use craft_ui_rt::{FontSpec, Overflow, TextLayout, TextSpec};
use craft_ui_rt::{Stroke, TextHighlight};
use std::hash::DefaultHasher as StableHasher;

/// The field's look; the component that uses it fills it from the theme.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldStyle {
    pub font: FontSpec,
    pub line_height: Option<f32>,
    pub text: Rgba,
    pub fill: Rgba,
    pub stroke: Option<Stroke>,
    /// The stroke while the field has focus; `stroke` when `None`.
    pub focus_stroke: Option<Stroke>,
    pub radius: f32,
    pub padding: Margin,
    /// The height when the parent leaves it open.
    pub height: f32,
    pub selection: Rgba,
    pub caret: Rgba,
    pub caret_width: f32,
    /// How far the caret reaches above and below its line of text.
    pub caret_overhang: f32,
    /// The caret shows, then hides, this many seconds each from its last move; 0 keeps it shown.
    pub caret_blink: f64,
}

impl TextFieldStyle {
    fn key(&self, h: &mut StableHasher) {
        let stroke = |s: &Option<Stroke>| s.map(|s| (s.width.to_bits(), s.color, s.align as u8));
        (self.font.slot, self.font.size.to_bits(), self.line_height.map(f32::to_bits)).hash(h);
        (self.text, self.fill, self.selection, self.caret).hash(h);
        (stroke(&self.stroke), stroke(&self.focus_stroke)).hash(h);
        let p = self.padding;
        [self.radius, p.left, p.top, p.right, p.bottom, self.height, self.caret_width, self.caret_overhang, self.caret_blink as f32]
            .map(f32::to_bits)
            .hash(h);
    }
}

/// The sub-node key of the field's text run.
const TEXT_RUN: u64 = 0;

pub struct TextField {
    initial: Arc<str>,
    style: TextFieldStyle,
    select_all: bool,
    autofocus: bool,
    purpose: ImePurpose,
    label: Option<Arc<str>>,
}

/// What the field reported since its parent last built.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextFieldOutput {
    /// The draft after the last edit.
    pub edited: Option<String>,
    /// Enter, with the draft; `true` with Shift.
    pub submit: Option<(String, bool)>,
    /// Escape.
    pub cancel: bool,
    /// Tab; `true` for Shift+Tab.
    pub tab: Option<bool>,
    /// The field lost focus.
    pub blur: bool,
}

pub struct TextFieldLocal {
    edit: TextEdit,
    layout: Option<TextLayout>,
    /// Where the text's origin sits, local coordinates.
    origin: Pos2,
    inner: Rect,
    /// What the text, its selection and the caret show inside, local coordinates.
    clip: Rect,
    scroll: f32,
    /// The caret's line, from its top to its bottom, local coordinates.
    caret: (Pos2, Pos2),
    /// When the caret last moved, the start of its blink; set by the first paint after focus.
    blink_from: Cell<Option<f64>>,
    /// The selected bytes of the shown text, filled behind it.
    selection: Option<Range<usize>>,
    preedit: Option<(Pos2, Pos2)>,
    /// A press or drag to resolve against the text in the next layout: (local point, extend).
    click: Option<(Pos2, bool)>,
}

impl TextFieldLocal {
    pub fn edit(&self) -> &TextEdit {
        &self.edit
    }
}

impl TextField {
    /// A field whose draft starts as `initial`. Later changes to `initial` do not reset the
    /// draft: the field owns it.
    pub fn new(initial: impl Into<Arc<str>>, style: TextFieldStyle) -> TextField {
        TextField { initial: initial.into(), style, select_all: false, autofocus: false, purpose: ImePurpose::Normal, label: None }
    }

    /// The draft starts fully selected.
    pub fn select_all(mut self, on: bool) -> Self {
        self.select_all = on;
        self
    }

    /// The field takes key focus when it opens.
    pub fn autofocus(mut self, on: bool) -> Self {
        self.autofocus = on;
        self
    }

    pub fn purpose(mut self, purpose: ImePurpose) -> Self {
        self.purpose = purpose;
        self
    }

    /// The access node's name.
    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Whether the caret shows now: always without a blink, else in the even half periods since
    /// it last moved. The field repaints at the next blink edge only.
    fn caret_on(&self, local: &TextFieldLocal, cx: &mut PaintCx<'_>) -> bool {
        let half = self.style.caret_blink;
        if half <= 0.0 {
            return true;
        }
        let mut periods = 0.0_f64;
        cx.time_until(|now| {
            let from = local.blink_from.get().unwrap_or(now);
            local.blink_from.set(Some(from));
            // The edge the wake was set for counts as reached, whatever the division rounds to.
            let edge = |n: f64| from + (n + 1.0) * half;
            periods = ((now - from).max(0.0) / half).floor();
            if edge(periods) <= now {
                periods += 1.0;
            }
            Some(edge(periods))
        });
        periods % 2.0 == 0.0
    }

    fn apply(&self, local: &mut TextFieldLocal, cx: &mut EventCx<'_>, input: &EditorInput) {
        let applied: Applied = local.edit.apply(input);
        if applied.caret_moved || applied.text_changed {
            local.blink_from.set(Some(cx.time()));
            cx.request_layout();
            cx.request_paint();
            cx.request_access();
        }
        if applied.text_changed {
            cx.output::<TextFieldOutput>().edited = Some(local.edit.text().to_owned());
        }
        if let Some(text) = applied.copy {
            cx.set_clipboard(text);
        }
        if applied.paste {
            cx.request_paste();
        }
        if let Some(shift) = applied.submit {
            let text = local.edit.text().to_owned();
            cx.output::<TextFieldOutput>().submit = Some((text, shift));
        }
        if applied.cancel {
            cx.output::<TextFieldOutput>().cancel = true;
        }
        if let Some(back) = applied.tab {
            cx.output::<TextFieldOutput>().tab = Some(back);
        }
    }
}

impl Element for TextField {
    type Output = TextFieldOutput;
    type Local = TextFieldLocal;

    fn props_key(&self) -> u64 {
        let mut h = StableHasher::default();
        (&*self.initial, self.select_all, self.autofocus, self.purpose == ImePurpose::Password).hash(&mut h);
        self.label.as_deref().hash(&mut h);
        self.style.key(&mut h);
        h.finish()
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> TextFieldLocal {
        let mut edit = TextEdit::new(self.initial.as_ref());
        if self.select_all {
            edit.select_all();
        }
        if self.autofocus {
            cx.request_focus();
        }
        TextFieldLocal {
            edit,
            layout: None,
            origin: Pos2::ZERO,
            inner: Rect::ZERO,
            clip: Rect::ZERO,
            scroll: 0.0,
            caret: (Pos2::ZERO, Pos2::ZERO),
            blink_from: Cell::new(None),
            selection: None,
            preedit: None,
            click: None,
        }
    }

    fn layout(&self, local: &mut TextFieldLocal, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let st = &self.style;
        let width = if c.max.x.is_finite() { c.max.x } else { c.min.x };
        let size = c.constrain(Vec2::new(width, st.height));
        let p = st.padding;
        let inner = crate::geo::rect(p.left, p.top, size.x - p.right, size.y - p.bottom);
        let spec = |text: String| TextSpec {
            text: text.into(),
            font: st.font,
            line_height: st.line_height,
            color: st.text,
            max_width: None,
            overflow: Overflow::Wrap,
        };
        let (shown, _, _) = local.edit.shown();
        let layout = cx.text_layout(&spec(shown));
        let y = inner.min.y + (inner.height() - layout.size.y) / 2.0;
        if let Some((at, extend)) = local.click.take() {
            let text_x = inner.min.x - local.scroll;
            let index = cx.text_engine().hit_index(&layout, Pos2::new(at.x - text_x, at.y - y));
            let index = local.edit.from_shown(index);
            local.edit.place(index, extend);
        }
        let (_, caret_at, pre) = local.edit.shown();
        let engine = cx.text_engine();
        let caret = engine.caret_rect(&layout, caret_at);
        let view = inner.width();
        if layout.size.x <= view {
            local.scroll = 0.0;
        } else if caret.min.x - local.scroll > view - st.caret_width {
            local.scroll = caret.min.x - view + st.caret_width;
        } else if caret.min.x < local.scroll {
            local.scroll = caret.min.x;
        }
        let origin = Pos2::new(inner.min.x - local.scroll, y);
        local.caret = (
            Pos2::new(origin.x + caret.min.x, origin.y + caret.min.y - st.caret_overhang),
            Pos2::new(origin.x + caret.min.x, origin.y + caret.max.y + st.caret_overhang),
        );
        let sel = local.edit.selection();
        local.selection = (!sel.is_empty() && pre.is_none()).then_some(sel);
        local.preedit = pre.map(|r| {
            let a = engine.caret_rect(&layout, r.start);
            let b = engine.caret_rect(&layout, r.end);
            let y = origin.y + layout.baseline + 1.5;
            (Pos2::new(origin.x + a.min.x, y), Pos2::new(origin.x + b.min.x, y))
        });
        // The text row grown by a point, so the caret's ends are cut as egui's text edit cuts
        // them (it paints inside its row rect expanded by 1 pt).
        local.clip = crate::geo::rect(inner.min.x, origin.y - 1.0, inner.max.x, origin.y + layout.size.y + 1.0).intersect(inner);
        local.origin = origin;
        local.inner = inner;
        local.layout = Some(layout);
        size
    }

    fn paint(&self, local: &TextFieldLocal, cx: &mut PaintCx<'_>) {
        let st = &self.style;
        let focused = cx.interaction().focused;
        let bounds = cx.bounds();
        let stroke = if focused { st.focus_stroke.or(st.stroke) } else { st.stroke };
        // The fill, then the edge over it as a shape of its own, as craft's surfaces paint them.
        cx.rect(bounds, st.radius, st.fill, None);
        if stroke.is_some() {
            cx.rect(bounds, st.radius, Rgba::TRANSPARENT, stroke);
        }
        cx.hit(bounds, Sense::CLICK | Sense::DRAG, 0);
        cx.push_clip(local.clip);
        if let Some(layout) = &local.layout {
            match &local.selection {
                Some(range) => {
                    let highlight = TextHighlight { range: range.clone(), fill: st.selection };
                    cx.text_highlighted(layout, local.origin, Some(st.text), highlight);
                }
                None => cx.text(layout, local.origin, Some(st.text)),
            }
        }
        if let Some((a, b)) = local.preedit {
            cx.line(a, b, Stroke::new(1.0, st.text));
        }
        // The caret shows at the selection's moving end too, as craft's fields show it.
        if focused && self.caret_on(local, cx) {
            let (top, bottom) = local.caret;
            cx.line(top, bottom, Stroke::new(st.caret_width, st.caret));
        }
        cx.pop_clip();
    }

    fn event(&self, local: &mut TextFieldLocal, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => {
                let at = cx.to_local(p.pos);
                local.blink_from.set(Some(cx.time()));
                match p.gesture {
                    Gesture::Down { clicks } if clicks >= 2 => local.edit.select_all(),
                    Gesture::Down { .. } => local.click = Some((at, p.mods.shift)),
                    Gesture::DragStart { .. } | Gesture::Drag { .. } => {
                        local.click = Some((at, true));
                    }
                    _ => return Handled::Yes,
                }
                cx.request_layout();
                cx.request_paint();
                cx.request_access();
                Handled::Yes
            }
            Event::Key(k) if k.down => match editor_input(Os::current(), k.key, k.mods) {
                Some(input) => {
                    self.apply(local, cx, &input);
                    Handled::Yes
                }
                None => Handled::No,
            },
            Event::Text(text) | Event::Paste(text) => {
                self.apply(local, cx, &EditorInput::Text(text.as_str().into()));
                Handled::Yes
            }
            Event::ImePreedit { text, cursor } => {
                self.apply(local, cx, &EditorInput::preedit(text.as_str(), cursor.clone()));
                Handled::Yes
            }
            Event::ImeCommit(text) => {
                self.apply(local, cx, &EditorInput::ImeCommit);
                self.apply(local, cx, &EditorInput::Text(text.as_str().into()));
                Handled::Yes
            }
            Event::Focus { gained, .. } => {
                // The blink starts again at the next paint, which comes now.
                local.blink_from.set(None);
                cx.request_paint();
                if !gained {
                    cx.output::<TextFieldOutput>().blur = true;
                }
                Handled::Yes
            }
            Event::Access(a) if a.action == accesskit::Action::SetValue => {
                if let Some(accesskit::ActionData::Value(value)) = &a.data {
                    local.edit.set_text(value.as_ref());
                    cx.request_layout();
                    cx.request_paint();
                    cx.request_access();
                    cx.output::<TextFieldOutput>().edited = Some(local.edit.text().to_owned());
                }
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, local: &TextFieldLocal, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        let text = local.edit.text();
        node.set_value(text);
        if let Some(label) = &self.label {
            node.set_label(label.as_ref());
        }
        node.add_action(accesskit::Action::Focus);
        node.add_action(accesskit::Action::SetValue);
        // The text as one run, which the selection's positions name, as AccessKit's text model asks.
        let mut run = accesskit::Node::new(accesskit::Role::TextRun);
        run.set_value(text);
        let lengths: Vec<u8> = text.chars().map(|c| c.len_utf8() as u8).collect();
        run.set_character_lengths(lengths);
        if let Some(layout) = &local.layout {
            let shown = Rect::from_min_size(local.origin, layout.size).intersect(local.inner);
            run.set_bounds(accesskit::Rect::new(f64::from(shown.min.x), f64::from(shown.min.y), f64::from(shown.max.x), f64::from(shown.max.y)));
        }
        let run = cx.sub(TEXT_RUN, None, run);
        let at = |byte: usize| accesskit::TextPosition { node: run, character_index: text[..byte.min(text.len())].chars().count() };
        node.set_text_selection(accesskit::TextSelection { anchor: at(local.edit.anchor()), focus: at(local.edit.caret()) });
        Some(node)
    }

    fn focusable(&self) -> bool {
        true
    }

    fn text_input(&self) -> Option<ImePurpose> {
        Some(self.purpose)
    }

    fn ime_cursor(&self, local: &TextFieldLocal) -> Option<Rect> {
        let (top, bottom) = local.caret;
        Some(Rect::from_two_pos(top, bottom).expand2(Vec2::new(self.style.caret_width / 2.0, 0.0)))
    }
}
