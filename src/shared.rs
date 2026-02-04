use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct SharedMut<T>(Arc<RwLock<T>>);

impl<T> SharedMut<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.0.read().expect("Lock poisoned");
        f(&*guard)
    }

    pub fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.0.write().expect("Lock poisoned");
        f(&mut *guard)
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for SharedMut<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
