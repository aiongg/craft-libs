//! The registrations `#[derive(Component)]` writes, so the suite finds every type with no hand-kept list.
//! Each registration is one `macro_rules!` of craft-ui, expanded in the user's crate and backed by `linkme`;
//! the macro's body depends on craft-ui's own `testing` feature, so a derive carries no `cfg`.

use super::{
    BodyCx, Collection, Component, Container, ItemFn, Meta, Outcome, ShapeKind, Show as _, ShowCollection as _, ShowContainer as _, ShowShape,
    ValueOf, shape,
};
use crate::runtime::access::Role;
use crate::runtime::focus::KeyUse;
use craft_behavior::{Input, ItemSource};
use egui::{Rect, Response, Ui};
use std::any::{Any, TypeId};

/// Shows a leaf at a rect: the component, boxed. `None` when the box holds another type.
pub type LeafEntry = fn(&mut Ui, Rect, Box<dyn Any>) -> Option<Response>;

/// Shows a container at a rect with a body: the component, boxed.
pub type ContainerEntry = fn(&mut Ui, Rect, Box<dyn Any>, &mut dyn FnMut(&mut Ui, &mut BodyCx<'_>)) -> Option<Response>;

/// Shows a collection's items: the component, boxed; the source, a `Box<dyn ItemSource<Key>>`; and the item
/// closure, a `Box<ItemFn<'static, C>>`.
pub type CollectionEntry = fn(&mut Ui, Box<dyn Any>, &dyn Any, &mut dyn Any) -> Option<Response>;

/// How the suite shows an instance of a registered type, per shape.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Entry {
    /// `show_at`.
    Leaf(LeafEntry),
    /// `show_at` with a body.
    Container(ContainerEntry),
    /// `show_items`.
    Collection(CollectionEntry),
}

fn show_leaf<T: Component + Meta<Shape = shape::Leaf> + 'static>(ui: &mut Ui, rect: Rect, component: Box<dyn Any>) -> Option<Response> {
    let component = component.downcast::<T>().ok()?;
    Some(component.show_at(ui, rect).response().clone())
}

fn show_container<T: Container + 'static>(
    ui: &mut Ui,
    rect: Rect,
    component: Box<dyn Any>,
    body: &mut dyn FnMut(&mut Ui, &mut BodyCx<'_>),
) -> Option<Response> {
    let component = component.downcast::<T>().ok()?;
    Some(component.show_at(ui, rect, |ui, cx| body(ui, cx)).response().clone())
}

fn show_collection<T: Collection + 'static>(ui: &mut Ui, component: Box<dyn Any>, source: &dyn Any, item: &mut dyn Any) -> Option<Response> {
    let component = component.downcast::<T>().ok()?;
    let source = source.downcast_ref::<Box<dyn ItemSource<T::Key>>>()?;
    let item = item.downcast_mut::<Box<ItemFn<'static, T>>>()?;
    Some(component.show_items(ui, &**source, &mut **item).response().clone())
}

/// A named shared-case exemption, as `#[component(exempt(case = .., reason = ..))]` declares it.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Exemption {
    /// The shared case the type is exempt from.
    pub case: &'static str,
    /// Why the type cannot meet it.
    pub reason: &'static str,
}

impl Exemption {
    /// An exemption from `case`, for `reason`.
    pub const fn new(case: &'static str, reason: &'static str) -> Self {
        Exemption { case, reason }
    }
}

/// What the suite knows about one registered component type, without an instance: the facts the contract
/// types state and the declarations of `#[component(..)]`.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Probe {
    /// The package that declares the type, as `CARGO_PKG_NAME` spells it.
    pub crate_name: &'static str,
    /// The crate target that declares the type, as `CARGO_CRATE_NAME` spells it: the package's library crate, or
    /// an integration binary.
    pub crate_target: &'static str,
    /// The module path of the declaration.
    pub module: &'static str,
    /// The shape.
    pub shape: ShapeKind,
    /// The AccessKit role.
    pub role: Role,
    /// How input reaches the component.
    pub input: Input,
    /// The keys the focused component keeps.
    pub keys: KeyUse,
    /// How to show an instance.
    pub entry: Entry,
    /// A leaf the `Canvas` body shows.
    pub canvas_control: bool,
    /// The type's `SHOWS_LABEL`: whether it paints its label as visible text.
    pub shows_label: bool,
    /// The type declares `#[component(overlay)]`.
    pub overlay: bool,
    /// The shared cases the type is exempt from.
    pub exempt: &'static [Exemption],
    /// The first sentence of the type's doc comment.
    pub purpose: &'static str,
    name: fn() -> &'static str,
    value: fn() -> bool,
}

impl Probe {
    /// The probe of the leaf `T`, declared in `module` of the crate target `crate_target` of package `crate_name`.
    pub const fn leaf<T: Component + Meta<Shape = shape::Leaf> + 'static>(
        crate_name: &'static str,
        crate_target: &'static str,
        module: &'static str,
        overlay: bool,
        exempt: &'static [Exemption],
    ) -> Self {
        Self::of::<T>(crate_name, crate_target, module, overlay, exempt, Entry::Leaf(show_leaf::<T>))
    }

    /// The probe of the container `T`.
    pub const fn container<T: Container + 'static>(
        crate_name: &'static str,
        crate_target: &'static str,
        module: &'static str,
        overlay: bool,
        exempt: &'static [Exemption],
    ) -> Self {
        Self::of::<T>(crate_name, crate_target, module, overlay, exempt, Entry::Container(show_container::<T>))
    }

    /// The probe of the collection `T`.
    pub const fn collection<T: Collection + 'static>(
        crate_name: &'static str,
        crate_target: &'static str,
        module: &'static str,
        overlay: bool,
        exempt: &'static [Exemption],
    ) -> Self {
        Self::of::<T>(crate_name, crate_target, module, overlay, exempt, Entry::Collection(show_collection::<T>))
    }

    const fn of<T: Component + 'static>(
        crate_name: &'static str,
        crate_target: &'static str,
        module: &'static str,
        overlay: bool,
        exempt: &'static [Exemption],
        entry: Entry,
    ) -> Self {
        Probe {
            crate_name,
            crate_target,
            module,
            shape: <T::Shape as ShowShape>::KIND,
            role: T::ROLE,
            input: T::INPUT,
            keys: T::KEYS,
            entry,
            canvas_control: T::CANVAS_CONTROL,
            shows_label: T::SHOWS_LABEL,
            overlay,
            exempt,
            purpose: "",
            name: std::any::type_name::<T>,
            value: carries_value::<T>,
        }
    }

    /// The probe with the type's `purpose`.
    #[must_use]
    pub const fn with_purpose(mut self, purpose: &'static str) -> Self {
        self.purpose = purpose;
        self
    }

    /// The type's name, with its type arguments.
    pub fn type_name(&self) -> &'static str {
        (self.name)()
    }

    /// Whether the component's output carries a value, so the mixed and invalid states apply to it.
    pub fn carries_value(&self) -> bool {
        (self.value)()
    }
}

fn carries_value<T: Component + 'static>() -> bool {
    TypeId::of::<ValueOf<T>>() != TypeId::of::<super::NoValue>()
}

/// One entry of the suite's registration list.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Registration {
    /// A component type, as `#[derive(Component)]` or `register_contract!` registers it.
    Component(Probe),
}

impl Registration {
    /// The registration of a component type.
    pub const fn component(probe: Probe) -> Self {
        Registration::Component(probe)
    }

    /// The package the registered item belongs to.
    pub fn crate_name(&self) -> &'static str {
        match self {
            Registration::Component(probe) => probe.crate_name,
        }
    }

    /// The crate target the registered item belongs to.
    pub fn crate_target(&self) -> &'static str {
        match self {
            Registration::Component(probe) => probe.crate_target,
        }
    }

    /// The probe, for a component registration.
    pub fn probe(&self) -> Option<&Probe> {
        match self {
            Registration::Component(probe) => Some(probe),
        }
    }
}

/// Every registration in the running binary.
#[doc(hidden)]
#[cfg(feature = "testing")]
#[linkme::distributed_slice]
pub static REGISTRATIONS: [Registration];

/// Registers the component type `$ty` of shape `$shape` (`leaf`, `container` or `collection`) with its declarations:
/// `$overlay`, the `($case, $reason)` exemptions and the `$purpose` line of its doc comment.
#[doc(hidden)]
#[macro_export]
#[cfg(feature = "testing")]
macro_rules! __register_contract {
    ($ty:ty, $shape:ident, $overlay:expr, [$(($case:expr, $reason:expr)),*], $purpose:expr) => {
        const _: () = {
            const EXEMPT: &[$crate::contract::Exemption] = &[$($crate::contract::Exemption::new($case, $reason)),*];
            #[$crate::__private::linkme::distributed_slice($crate::contract::REGISTRATIONS)]
            #[linkme(crate = $crate::__private::linkme)]
            static REGISTRATION: $crate::contract::Registration = $crate::contract::Registration::component($crate::contract::Probe::$shape::<$ty>(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_CRATE_NAME"),
                module_path!(),
                $overlay,
                EXEMPT,
            ).with_purpose($purpose));
        };
    };
}

/// Registers the component type `$ty`; without the `testing` feature there is nothing to register.
#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "testing"))]
macro_rules! __register_contract {
    ($ty:ty, $shape:ident, $overlay:expr, [$(($case:expr, $reason:expr)),*], $purpose:expr) => {};
}
