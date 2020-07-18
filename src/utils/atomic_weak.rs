use std::sync::{Arc, Mutex, Weak};

pub struct AtomicWeak<T: Sync + Send + ?Sized>(Mutex<Option<Weak<T>>>)
where
    Self: Sync + Send;

impl<T: Sync + Send + ?Sized> AtomicWeak<T> {
    pub fn new(value: Weak<T>) -> Self {
        Self(Mutex::new(Some(value)))
    }
    pub fn replace(&self, value: Weak<T>) -> Option<Weak<T>> {
        (*self.0.lock().unwrap()).replace(value)
    }
    pub fn upgrade(&self) -> Option<Arc<T>> {
        (*self.0.lock().unwrap()).as_ref().and_then(|w| Weak::upgrade(&w))
    }
}

impl<T: Sync + Send + ?Sized> Default for AtomicWeak<T> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}
