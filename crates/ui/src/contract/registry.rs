//! The registrations `#[derive(Component)]` writes, so the suite finds every type with no hand-kept list.
//! Each registration is one `macro_rules!` of craft-ui, expanded in the user's crate and backed by `linkme`;
//! the macro's body depends on craft-ui's own `testing` feature, so a derive carries no `cfg`.

use super::{Component, ShapeKind, ShowShape, ValueOf};
use crate::runtime::access::Role;
use crate::runtime::focus::KeyUse;
use craft_behavior::Input;
use std::any::TypeId;

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
    /// A leaf the `Canvas` body shows.
    pub canvas_control: bool,
    /// The type declares `#[component(overlay)]`.
    pub overlay: bool,
    /// The shared cases the type is exempt from.
    pub exempt: &'static [Exemption],
    name: fn() -> &'static str,
    value: fn() -> bool,
}

impl Probe {
    /// The probe of `T`, declared in `module` of package `crate_name`.
    pub const fn of<T: Component + 'static>(crate_name: &'static str, module: &'static str, overlay: bool, exempt: &'static [Exemption]) -> Self {
        Probe {
            crate_name,
            module,
            shape: <T::Shape as ShowShape>::KIND,
            role: T::ROLE,
            input: T::INPUT,
            keys: T::KEYS,
            canvas_control: T::CANVAS_CONTROL,
            overlay,
            exempt,
            name: std::any::type_name::<T>,
            value: carries_value::<T>,
        }
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

/// Registers the component type `$ty` with its declarations: `$overlay`, and the `($case, $reason)` exemptions.
#[doc(hidden)]
#[macro_export]
#[cfg(feature = "testing")]
macro_rules! __register_contract {
    ($ty:ty, $overlay:expr, [$(($case:expr, $reason:expr)),*]) => {
        const _: () = {
            const EXEMPT: &[$crate::contract::Exemption] = &[$($crate::contract::Exemption::new($case, $reason)),*];
            #[$crate::__private::linkme::distributed_slice($crate::contract::REGISTRATIONS)]
            #[linkme(crate = $crate::__private::linkme)]
            static REGISTRATION: $crate::contract::Registration = $crate::contract::Registration::component($crate::contract::Probe::of::<$ty>(
                env!("CARGO_PKG_NAME"),
                module_path!(),
                $overlay,
                EXEMPT,
            ));
        };
    };
}

/// Registers the component type `$ty`; without the `testing` feature there is nothing to register.
#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "testing"))]
macro_rules! __register_contract {
    ($ty:ty, $overlay:expr, [$(($case:expr, $reason:expr)),*]) => {};
}

/// A canvas-control type, as `#[component(canvas_control)]` registers it, so the canvas-control cases run over it.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct CanvasControl {
    name: fn() -> &'static str,
}

impl CanvasControl {
    /// A registration for the type whose name `name` returns.
    pub const fn new(name: fn() -> &'static str) -> Self {
        CanvasControl { name }
    }

    /// The component's type name.
    pub fn type_name(&self) -> &'static str {
        (self.name)()
    }
}

/// Every registered canvas-control type in the running binary.
#[doc(hidden)]
#[cfg(feature = "testing")]
#[linkme::distributed_slice]
pub static CANVAS_CONTROLS: [CanvasControl];

/// Registers the canvas-control type `$ty`.
#[doc(hidden)]
#[macro_export]
#[cfg(feature = "testing")]
macro_rules! __register_canvas_control {
    ($ty:ty) => {
        const _: () = {
            #[$crate::__private::linkme::distributed_slice($crate::contract::CANVAS_CONTROLS)]
            #[linkme(crate = $crate::__private::linkme)]
            static REGISTRATION: $crate::contract::CanvasControl = $crate::contract::CanvasControl::new(::std::any::type_name::<$ty>);
        };
    };
}

/// Registers the canvas-control type `$ty`; without the `testing` feature there is nothing to register.
#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "testing"))]
macro_rules! __register_canvas_control {
    ($ty:ty) => {};
}
