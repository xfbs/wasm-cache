use crate::CacheKey;
use async_trait::async_trait;
use std::{error::Error, fmt::Debug};

/// Represents some action that can be cached.
///
/// The action has one associated type called [`Value`]. This is the value of data that this action
/// returns. That data is cached, so that future invocations of this item can reuse the previously
/// computed data.
///
/// The cached data here can be anything, but typically is a network request. The resulting value
/// is typically the response type of the network request.
#[async_trait(?Send)]
pub trait CacheItem<M = ()>: CacheKey<M> + Clone + Ord {
    type Value: Clone + Debug + PartialEq + 'static;
    type Error: Debug + Error + 'static;

    async fn send(&self) -> Result<Self::Value, Self::Error>;

    fn superset(&self) -> Vec<Self> {
        vec![]
    }
}
