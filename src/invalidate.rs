//! Traits to handle invalidations.

/// Something which can be invalidated.
pub trait Invalidatable<M> {
    fn invalidated_by(&self, _mutation: &M) -> bool {
        true
    }
}

/// Something which produces invalidations.
pub trait Invalidator<M> {
    fn mutations(&self) -> Vec<M>;
}
