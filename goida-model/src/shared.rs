use std::sync::{Arc, RwLock, Weak};

use string_interner::backend::StringBackend;
use string_interner::StringInterner;

#[derive(Debug)]
pub struct SharedMut<T>(Arc<RwLock<T>>);

impl<T> SharedMut<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self
            .0
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
    }

    pub fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self
            .0
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }

    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    #[must_use]
    pub fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }

    #[must_use]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    #[must_use]
    pub fn downgrade(&self) -> WeakSharedMut<T> {
        WeakSharedMut(Arc::downgrade(&self.0))
    }
}

impl<T: Default> Default for SharedMut<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for SharedMut<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[derive(Debug)]
pub struct WeakSharedMut<T>(Weak<RwLock<T>>);

impl<T> WeakSharedMut<T> {
    #[must_use]
    pub fn upgrade(&self) -> Option<SharedMut<T>> {
        self.0.upgrade().map(SharedMut)
    }
}

impl<T> Clone for WeakSharedMut<T> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}

pub type SharedInterner = SharedMut<StringInterner<StringBackend>>;

#[must_use]
pub fn new_interner() -> SharedInterner {
    SharedInterner::default()
}
