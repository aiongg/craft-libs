//! `RadioGroup` and `Radio`: the group form of 20.3. The group is a container and arena root; its members, shown
//! anywhere in its body, join its arena and register with its `GroupHandle`. A radio draws only its button; its text
//! is the label step's, as the check box's is.

use crate::contract::{
    Bind, BindMut, Common, Component, Container, ContainerProps, Cx, Edit as _, EditResponse, GroupHandle, GroupKind, GroupState, MeasureCx, PaintUi,
    Pointer as _, Props as _, Remember as _, Role,
};
use crate::primitives::{PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, Flags, FocusLook, Ink, StyleColor, StyleCx};
use craft_behavior::{Metrics, NavShape, Roving, Value};
use egui::{Id, Vec2};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, PoisonError};

/// The group kind of radios whose members stand for values of type `T`.
pub struct RadioKind<T>(PhantomData<fn() -> T>);

impl<T: Value + 'static> GroupKind for RadioKind<T> {
    type Member = T;
}

/// The members a group registered, as `GroupState` holds them.
type Members<T> = Vec<(T, Id, bool)>;

/// The group's registrations, kept in its shown memory so the members registered last pass are readable this pass.
/// `kept` holds the last non-empty registration: a pass that ran no body clears the live list, and the pick that
/// arrives in the next pass still finds its member.
struct GroupMemory<T: Value + 'static> {
    live: Arc<Mutex<GroupState<T>>>,
    kept: Arc<Mutex<Members<T>>>,
}

impl<T: Value + 'static> Clone for GroupMemory<T> {
    fn clone(&self) -> Self {
        GroupMemory { live: Arc::clone(&self.live), kept: Arc::clone(&self.kept) }
    }
}

impl<T: Value + 'static> Default for GroupMemory<T> {
    fn default() -> Self {
        GroupMemory { live: Arc::default(), kept: Arc::default() }
    }
}

impl<T: Value + 'static> GroupMemory<T> {
    /// The enabled member registered with `id`, in the live list or else the kept one.
    fn member(&self, id: Id) -> Option<T> {
        let find = |members: &Members<T>| members.iter().find(|(_, member, enabled)| *member == id && *enabled).map(|(value, ..)| value.clone());
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        find(&live.members).or_else(|| find(&self.kept.lock().unwrap_or_else(PoisonError::into_inner)))
    }

    /// The id of the enabled member registered with `value`, in the live list or else the kept one.
    fn member_id(&self, value: &T) -> Option<Id> {
        let find = |members: &Members<T>| members.iter().find(|(member, _, enabled)| member == value && *enabled).map(|(_, id, _)| *id);
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        find(&live.members).or_else(|| find(&self.kept.lock().unwrap_or_else(PoisonError::into_inner)))
    }

    /// Keeps the live registrations when there are any, then clears them for the pass's body to register again.
    fn begin_registration(&self) {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        if !live.members.is_empty() {
            *self.kept.lock().unwrap_or_else(PoisonError::into_inner) = std::mem::take(&mut live.members);
        }
    }
}

/// The style of the group: it has no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct GroupStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A group of radio buttons over one value: one Tab stop, arrows move focus, Space, Enter or a click selects.
#[derive(crate::contract::Component)]
#[component(container, probe(RadioGroup<'static, u8>))]
pub struct RadioGroup<'a, T: Value + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    value: Bind<'a, T>,
}

impl<'a, T: Value + 'static> RadioGroup<'a, T> {
    /// A group named `label` over `value`.
    pub fn new(label: impl Into<crate::runtime::env::Label>, value: Bind<'a, T>) -> Self {
        let props = ContainerProps::new().arena(true).roving(Roving::new(NavShape::Flat));
        RadioGroup { common: Common::new(), props, value }.label(label)
    }
}

impl<T: Value + 'static> Component for RadioGroup<'_, T> {
    const ROLE: Role = Role::RADIO_GROUP;
    type Style = GroupStyle;
    type Output = EditResponse<T>;

    fn style(_: &StyleCx<'_>, _: RadioGroupStyleKey<T>) -> GroupStyle {
        GroupStyle::default()
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::empty()
    }

    fn bind(&mut self) -> Option<BindMut<'_, T>> {
        Some(self.value.reborrow())
    }

    /// The pick (20.3 item 1): the member activated this pass, found in last pass's registrations, goes to the
    /// binding. Tab arrives at the selected member.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let memory = cx.memory::<GroupMemory<T>>().clone();
        let handle = GroupHandle::<RadioKind<T>>::new(Arc::clone(&memory.live));
        let activated = cx.activated_member();
        if let Some(picked) = activated.and_then(|id| memory.member(id)) {
            cx.propose(picked);
            cx.commit();
        }
        let selected = cx.value().cloned();
        if let Some(id) = selected.as_ref().and_then(|value| memory.member_id(value)) {
            cx.roving_arrival(id);
        }
        handle.select(selected);
        memory.begin_registration();
    }

    fn content_env(&self, cx: &Cx<'_, Self>, values: &mut crate::runtime::env::AppValues) {
        values.set(GroupHandle::<RadioKind<T>>::new(Arc::clone(&cx.recall::<GroupMemory<T>>().live)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<T> {
        cx.edit_response()
    }
}

impl<T: Value + 'static> Container for RadioGroup<'_, T> {}

/// The style of a radio button.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct RadioStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    icon: f32,
    ring: StyleColor,
    dot: StyleColor,
}

/// A member of a `RadioGroup`: selected when the group's pick is its value.
#[derive(crate::contract::Component)]
#[component(probe(Radio<u8>))]
pub struct Radio<T: Value + 'static> {
    #[common]
    common: Common,
    value: T,
    checked: bool,
}

impl<T: Value + 'static> Radio<T> {
    /// A radio button standing for `value`.
    pub fn new(value: T) -> Self {
        Radio { common: Common::new(), value, checked: false }
    }
}

impl<T: Value + 'static> Component for Radio<T> {
    const ROLE: Role = Role::RADIO_BUTTON;
    const OVERLAYS: bool = false;
    type Style = RadioStyle;
    type Output = egui::Response;

    fn style(cx: &StyleCx<'_>, _: RadioStyleKey<T>) -> RadioStyle {
        let control = cx.control();
        RadioStyle {
            metrics: control.metrics(),
            focus: cx.focus_look(),
            icon: control.icon,
            ring: cx.ink(Ink::OnPanel),
            dot: cx.tone(craft_theme::variant::Tone::Accent),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(cx.icon())
    }

    /// Reads "selected" from the group's pick and registers with the group.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let group = cx.env.get::<GroupHandle<RadioKind<T>>>();
        group.register(self.value.clone(), cx.id, cx.resolved.enabled);
        cx.join_roving();
        self.checked = group.is(&self.value);
    }

    fn flags(&self) -> Flags {
        if self.checked { Flags::CHECKED } else { Flags::empty() }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let center = cx.rect.center();
        let draw = ui.draw();
        draw.stroke(&PaintShape::Circle(center, cx.style.icon / 2.0), StrokeWidth::Thin, cx.style.ring);
        if self.checked {
            draw.fill(&PaintShape::Circle(center, cx.style.icon / 4.0), cx.style.dot);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}
