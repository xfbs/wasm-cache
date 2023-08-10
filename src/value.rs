//! Dynamic values.
//!
//! This module contains a dynamic value type that is agnostic over the storage container,
//! [`Value`]. It also contains aliases and implementations for [`RcValue`] and [`ArcValue`], which
//! use the [`Rc`] and [`Arc`] reference-counted containers, respectively.
use std::{any::Any, rc::Rc, sync::Arc};

/// Dynamic value.
///
/// Cache values are dynamic, but need to be able to be cast into a concrete type. The [`Value`]
/// type helps here. By default, it contains a `dyn Any`, so is able to store any type of data. It
/// is able to be cast into a concrete type. It also contains as state the validity of the type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value<T> {
    /// Valid means the data is usable.
    valid: bool,
    data: Option<T>,
}

pub type RcValue<V = dyn Any> = Value<Rc<V>>;
pub type ArcValue<V = dyn Any + Send + Sync> = Value<Arc<V>>;

impl Value<Rc<dyn Any>> {
    pub fn downcast<T: 'static>(self) -> Option<Value<Rc<T>>> {
        let value = Value {
            valid: self.valid,
            data: match self.data {
                None => None,
                Some(data) => Some(data.downcast::<T>().ok()?),
            },
        };
        Some(value)
    }
}

impl Value<Arc<dyn Any + Send + Sync>> {
    pub fn downcast<T: Any + Send + Sync>(self) -> Option<Value<Arc<T>>> {
        let value = Value {
            valid: self.valid,
            data: match self.data {
                None => None,
                Some(data) => Some(data.downcast::<T>().ok()?),
            },
        };
        Some(value)
    }
}

impl<T> Default for Value<T> {
    fn default() -> Self {
        Self {
            valid: false,
            data: None,
        }
    }
}

impl<T> Value<T> {
    /// Create new value with the given data.
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            valid: true,
        }
    }

    /// Return an option with a reference to the data.
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Determine if this data is valid.
    pub fn valid(&self) -> bool {
        self.valid
    }

    /// Invalidate this data.
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}
