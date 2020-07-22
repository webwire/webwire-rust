//! Atomic weak reference

use std::sync::{Arc, Mutex, Weak};

/// This is essentially a wrapper around `Mutex<Option<Weak<T>>>` which
/// is used for bidirectional communication between the various layers.
///
/// e.g. Engine owns Transport and passes a Weak pointer to itself when
/// calling the `Transport::start` method. That weak pointer is then stored
/// inside the Transport to notify the engine about inbound frames.
pub struct AtomicWeak<T: Sync + Send + ?Sized>(Mutex<Option<Weak<T>>>)
where
    Self: Sync + Send;

impl<T: Sync + Send + ?Sized> AtomicWeak<T> {
    /// Create new atomic weak reference
    pub fn new(value: Weak<T>) -> Self {
        Self(Mutex::new(Some(value)))
    }
    /// Replace internal weak reference by a new one
    pub fn replace(&self, value: Weak<T>) -> Option<Weak<T>> {
        (*self.0.lock().unwrap()).replace(value)
    }
    /// Upgrade this weak reference to an `Arc`
    pub fn upgrade(&self) -> Option<Arc<T>> {
        (*self.0.lock().unwrap())
            .as_ref()
            .and_then(|w| Weak::upgrade(&w))
    }
}

impl<T: Sync + Send + ?Sized> Default for AtomicWeak<T> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}
