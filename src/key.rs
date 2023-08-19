//! Arbitrary cache keys.
use super::Invalidatable;
use std::{any::Any, cmp::Ordering, fmt::Debug};

/// Trait for arbitrary cache keys.
///
/// Cache keys are arbitrary Rust types that are sortable and comparable.
pub trait CacheKey<M = ()>: Debug + Invalidatable<M> + 'static {
    fn any(&self) -> &(dyn Any + 'static);
    fn any_eq(&self, other: &dyn Any) -> bool;
    fn any_ord(&self, other: &dyn Any) -> Ordering;
    fn clone_boxed(&self) -> Box<dyn CacheKey<M>>;
}

impl<M: 'static> PartialOrd<Self> for dyn CacheKey<M> {
    fn partial_cmp(&self, other: &dyn CacheKey<M>) -> Option<Ordering> {
        Some(self.any_ord(other.any()))
    }
}

impl<M: 'static> PartialEq<Self> for dyn CacheKey<M> {
    fn eq(&self, other: &dyn CacheKey<M>) -> bool {
        self.any_eq(other.any())
    }
}

impl<M: 'static> Eq for dyn CacheKey<M> {}

impl<M: 'static> Ord for dyn CacheKey<M> {
    fn cmp(&self, other: &dyn CacheKey<M>) -> Ordering {
        self.any_ord(other.any())
    }
}

impl<M: 'static> Clone for Box<dyn CacheKey<M>> {
    fn clone(&self) -> Self {
        ((&**self) as &dyn CacheKey<M>).clone_boxed()
    }
}

impl<M, T: Debug + Eq + Ord + Any + Clone + Invalidatable<M> + 'static> CacheKey<M> for T {
    fn any_eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<T>() {
            Some(other) => {
                println!("Using eq: {}", std::any::type_name::<T>());
                self.eq(other)
            }
            None => false,
        }
    }

    fn any_ord(&self, other: &dyn Any) -> Ordering {
        match self.type_id().cmp(&other.type_id()) {
            Ordering::Equal => match <dyn Any>::downcast_ref::<T>(other) {
                Some(other) => self.cmp(other),
                None => unreachable!(),
            },
            ordering => ordering,
        }
    }

    fn any(&self) -> &(dyn Any + 'static) {
        self as &(dyn Any + 'static)
    }

    fn clone_boxed(&self) -> Box<dyn CacheKey<M>> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn cache_item_eq_identity() {
        let string: Box<dyn CacheKey> = Box::new(String::from("Hello"));
        assert_eq!(string.any_eq(&string), true);
        assert!(&string == &string);
    }

    #[test]
    fn cache_item_eq_equal() {
        println!("Running test");
        let string1: Box<dyn CacheKey> = Box::new(String::from("Hello"));
        let string2: Box<dyn CacheKey> = Box::new(String::from("Hello"));
        assert_eq!(string1.any_eq(&string2), true);
        assert!(&string1 == &string2);
    }

    #[test]
    fn cache_item_new_different() {
        let string_hello: Box<dyn CacheKey> = Box::new(String::from("Hello"));
        let string_world: Box<dyn CacheKey> = Box::new(String::from("World"));
        assert_eq!(string_hello.any_eq(&string_world), false);
    }

    #[test]
    fn cache_item_different_type() {
        let string_hello: Box<dyn CacheKey> = Box::new(String::from("Hello"));
        let array_empty: Box<dyn CacheKey> = Box::new(Vec::<usize>::new());
        assert_eq!(string_hello.any_eq(&array_empty), false);
    }

    #[test]
    fn test_cache_key() {
        let mut map: BTreeMap<Box<dyn CacheKey>, &str> = Default::default();
        map.insert(Box::new(String::from("Hello")), "String Hello");
        map.insert(Box::new(String::from("World")), "String World");
    }
}
