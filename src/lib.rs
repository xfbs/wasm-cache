/// # WebAssembly Cache
///
/// This crate provides primitives to build a simple in-memory request cache for WebAssembly
/// applications. This cache relies on the type system to be able to store responses for any
/// request type.
///
/// It is intended to be used with the Yew framework, although more integrations may be added in
/// the future.
mod invalidate;
mod item;
mod key;
mod value;
#[cfg(feature = "yew")]
pub mod yew;

pub use crate::{invalidate::*, item::*, key::*, value::*};
