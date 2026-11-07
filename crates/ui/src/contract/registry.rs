//! The registrations `#[derive(Component)]` writes, so the suite finds every type with no hand-kept list.
//! Each registration is one `macro_rules!` of craft-ui, expanded in the user's crate and backed by `linkme`;
//! the macro's body depends on craft-ui's own `testing` feature, so a derive carries no `cfg`.

/// A canvas-control type, as `#[component(canvas_control)]` registers it, so the canvas-control cases run over it.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct CanvasControl {
    /// The component's type name.
    pub type_name: &'static str,
}

impl CanvasControl {
    /// A registration for the type named `type_name`.
    pub const fn new(type_name: &'static str) -> Self {
        CanvasControl { type_name }
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
            static REGISTRATION: $crate::contract::CanvasControl = $crate::contract::CanvasControl::new(::std::any::type_name::<$ty>());
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
