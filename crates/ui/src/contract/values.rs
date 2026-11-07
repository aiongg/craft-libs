//! The value types the component traits name.
//!
//! F3-LATER: values (F4 owns `Bind`, `BindMut`, `Outcome`, `EditResponse`, validation, gestures and the apply
//! helper; F6 owns `ItemOutput`). These carry what the traits and the pipeline's read paths need.

use craft_behavior::Value;
use egui::Response;
use std::marker::PhantomData;

/// The value type of a component with no value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoValue;

/// What `show` returns: the component's response and the type of value it edits.
pub trait Outcome {
    /// The value type; `NoValue` for a component with none.
    type Value: Value;

    /// The component's response.
    fn response(&self) -> &Response;
}

impl Outcome for Response {
    type Value = NoValue;

    fn response(&self) -> &Response {
        self
    }
}

/// The value type of `C`'s output.
pub type ValueOf<C> = <<C as super::Component>::Output as Outcome>::Value;

/// A read-only view of a value the app owns.
#[derive(Debug)]
pub struct Bind<'a, T>(&'a T);

impl<'a, T> Bind<'a, T> {
    /// Binds `value`.
    pub fn new(value: &'a T) -> Self {
        Bind(value)
    }

    /// The value.
    pub fn get(&self) -> &T {
        self.0
    }
}

/// A binding the pipeline reads at step 4.
#[derive(Debug)]
pub struct BindMut<'a, T>(&'a mut T);

impl<'a, T> BindMut<'a, T> {
    /// Binds `value`.
    pub fn new(value: &'a mut T) -> Self {
        BindMut(value)
    }

    /// The value.
    pub fn get(&self) -> &T {
        self.0
    }
}

/// The output of a component that edits a value.
#[derive(Debug)]
#[non_exhaustive]
pub struct EditResponse<T> {
    /// The component's response.
    pub response: Response,
    value: PhantomData<fn() -> T>,
}

impl<T> EditResponse<T> {
    /// An edit response with no event; `cx.edit_response()` builds the real ones.
    #[doc(hidden)]
    pub fn new(response: Response) -> Self {
        EditResponse { response, value: PhantomData }
    }
}

impl<T: Value> Outcome for EditResponse<T> {
    type Value = T;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// What an item's output may report beyond its response.
pub trait ItemOutput<E>: Outcome {}

impl<E> ItemOutput<E> for Response {}
