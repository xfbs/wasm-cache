use crate::{CacheItem, CacheKey, RcValue};
use prokio::time::sleep;
use std::{any::Any, collections::BTreeMap, rc::Rc, sync::Mutex, time::Duration};
use yew::{
    functional::{UseStateHandle, UseStateSetter},
    prelude::*,
};

const DELAY_INITIAL: Duration = Duration::from_millis(100);
const DELAY_MULTIPLIER: f64 = 1.5;

#[derive(Clone, Default)]
pub struct Entry {
    /// Delay to use for next request
    pub delay: Option<Duration>,
    /// Fetch in-progress
    pub progress: bool,
    /// Current cached value.
    pub value: RcValue,
    /// List of subscribers to this value.
    pub subscriptions: Vec<UseStateSetter<RcValue>>,
}

impl Entry {
    /// Broadcast the current value of the cache entry to all subscribers.
    pub fn broadcast(&self) {
        for subscriber in &self.subscriptions {
            subscriber.set(self.value.clone());
        }
    }

    /// Subscribe for updates
    pub fn subscribe(&mut self, setter: &UseStateSetter<RcValue>) {
        if !self.subscriptions.iter().any(|i| i == setter) {
            self.subscriptions.push(setter.clone());
        }
    }

    /// Unsubscribe for updates
    pub fn unsubscribe(&mut self, setter: &UseStateSetter<RcValue>) {
        self.subscriptions.retain(|s| s != setter);
    }

    /// Get current delay and update.
    pub fn delay_update(&mut self) {
        self.delay = match self.delay {
            Some(current) => Some(Duration::from_secs_f64(
                current.as_secs_f64() * DELAY_MULTIPLIER,
            )),
            None => Some(DELAY_INITIAL),
        };
    }

    pub fn delay_reset(&mut self) {
        self.delay = None;
    }

    pub fn needs_fetch(&self) -> bool {
        !self.value.valid() && !self.progress
    }
}

pub struct BTreeCache<M: 'static = ()> {
    pub entries: BTreeMap<Box<dyn CacheKey<M>>, Entry>,
}

impl<M: 'static> Clone for BTreeCache<M> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<M: 'static> Default for BTreeCache<M> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
        }
    }
}

pub struct Cache<M: 'static = ()> {
    pub cache: Rc<Mutex<BTreeCache<M>>>,
}

impl<M: 'static> Clone for Cache<M> {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}

impl<M: 'static> Default for Cache<M> {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

impl<M: 'static> PartialEq for Cache<M> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cache, &other.cache)
    }
}

impl<M: 'static> BTreeCache<M> {
    /// Unsubscribe to the value of this data.
    pub fn mutate<T: CacheKey<M>, R, F: FnOnce(&mut Entry) -> R>(
        &mut self,
        data: &T,
        mutate: F,
    ) -> Option<R> {
        if let Some(entry) = self.entries.get_mut(data as &dyn CacheKey<M>) {
            Some(mutate(entry))
        } else {
            None
        }
    }

    /// Unsubscribe to the value of this data.
    pub fn mutate_all<F: Fn(&Box<dyn CacheKey<M>>, &mut Entry)>(&mut self, mutate: F) {
        for (key, entry) in &mut self.entries {
            mutate(key, entry);
        }
    }

    /// Unsubscribe to the value of this data.
    pub fn insert<T: CacheKey<M>>(&mut self, data: T, entry: Entry) {
        let key = Box::new(data);
        self.entries.insert(key, entry);
    }

    pub fn get<T: CacheKey<M>>(&self, data: &T) -> Option<&Entry> {
        self.entries.get(data as &dyn CacheKey<M>)
    }
}

impl<M: 'static> Cache<M> {
    fn subscribe<R: CacheItem<M>>(&self, request: &R, handle: UseStateHandle<RcValue>)
    where
        R::Value: PartialEq,
    {
        let setter = handle.setter();
        let mut cache = self.cache.lock().expect("Failure to lock cache");

        // add self as subscriber to cache value, if exists.
        let mutated = cache.mutate(request, |entry| {
            entry.subscribe(&setter);

            // only set it if it is different
            let value = entry.value.clone().downcast::<R::Value>().unwrap();
            let current = (*handle).clone().downcast::<R::Value>().unwrap();
            if value != current {
                setter.set(entry.value.clone());
            }

            entry.clone()
        });

        match mutated {
            None => {
                cache.insert(
                    request.clone(),
                    Entry {
                        progress: true,
                        subscriptions: vec![setter.clone()],
                        ..Default::default()
                    },
                );
                drop(cache);
                self.fetch(request, None);
            }
            Some(entry) if entry.needs_fetch() => {
                let delay = entry.delay;
                drop(cache);
                self.fetch(request, delay);
            }
            _ => {}
        }
    }

    /// Trigger a fetch of this data.
    fn fetch<T: CacheItem<M>>(&self, data: &T, delay: Option<Duration>) {
        let data = data.clone();
        let cache = self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(delay) = delay {
                sleep(delay).await;
            }
            match data.send().await {
                Ok(result) => cache.cache(&data, Rc::new(result)),
                Err(error) => cache.failure(&data, error),
            }
        });
    }

    /// Handle failure.
    pub fn failure<T: CacheItem<M>>(&self, data: &T, error: T::Error) {
        #[cfg(feature = "log")]
        log::error!("error fetching {data:?}: {error}");
        self.cache
            .lock()
            .expect("Failure to lock cache")
            .mutate(data, move |entry| {
                entry.delay_update();
                entry.progress = false;
                entry.broadcast();
            });
    }

    /// Cache this data.
    pub fn cache<T: CacheItem<M>>(&self, data: &T, value: Rc<T::Value>) {
        self.cache
            .lock()
            .expect("Failure to lock cache")
            .mutate(data, move |entry| {
                entry.delay_reset();
                entry.value = RcValue::new(value as Rc<dyn Any>);
                entry.progress = false;
                entry.broadcast();
            });
    }

    /// Unsubscribe to the value of this data.
    pub fn unsubscribe<T: CacheItem<M>>(&self, data: &T, setter: &UseStateSetter<RcValue>) {
        self.cache
            .lock()
            .expect("Failure to lock cache")
            .mutate(data, |entry| {
                entry.unsubscribe(setter);
            });
    }

    /// Invalidate this invalidation.
    pub fn invalidate(&self, mutation: &M) {
        self.cache
            .lock()
            .expect("Failure to lock cache")
            .mutate_all(|key, entry| {
                if key.invalidated_by(mutation) {
                    entry.value.invalidate();
                    entry.broadcast();
                }
            });
    }

    /// Invalidate this key.
    pub fn invalidate_key<T: CacheItem<M>>(&self, data: &T) {
        self.cache
            .lock()
            .expect("Failure to lock cache")
            .mutate(data, |entry| {
                entry.value.invalidate();
                entry.broadcast();
            });
    }

    /// Invalidates entire cache.
    pub fn invalidate_all(&self) {
        let mut cache = self.cache.lock().expect("Failure to lock cache");
        cache.mutate_all(|_key, entry| {
            entry.value.invalidate();
            entry.broadcast();
        });
    }
}

#[derive(Properties)]
pub struct CacheProviderProps<M: 'static = ()> {
    pub children: Children,
    #[prop_or_default]
    pub cache: Cache<M>,
}

impl<M: 'static> PartialEq<Self> for CacheProviderProps<M> {
    fn eq(&self, other: &Self) -> bool {
        self.children.eq(&other.children) && self.cache.eq(&other.cache)
    }
}

#[function_component]
pub fn CacheProvider<M: 'static = ()>(props: &CacheProviderProps<M>) -> Html {
    html! {
        <ContextProvider<Cache<M>> context={props.cache.clone()}>
        { for props.children.iter() }
        </ContextProvider<Cache<M>>>
    }
}

#[hook]
pub fn use_cached<M: 'static, R: CacheItem<M>>(data: R) -> RcValue<R::Value>
where
    R::Value: PartialEq,
{
    #[cfg(feature = "log")]
    log::debug!("use_data({data:?})");
    let cache = use_context::<Cache<M>>().expect("Cache not present");
    let state = use_state(|| RcValue::default());
    let state_clone = state.clone();
    use_effect(move || {
        cache.subscribe(&data, state_clone.clone());
        move || {
            cache.unsubscribe(&data, &state_clone.setter());
        }
    });
    let value = (*state).clone();
    value.downcast().expect("Value is of wrong type")
}
