//! This module provides a Mutex-protected [`RefCell`] --- basically a way to ensure
//! at runtime that some reference is used only once.
use core::cell::{Cell, UnsafeCell};
use critical_section::CriticalSection;

pub(crate) struct EnsureOnce<T> {
    flag: Cell<bool>,
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for EnsureOnce<T> {}

impl<T> EnsureOnce<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            flag: Cell::new(false),
            inner: UnsafeCell::new(inner),
        }
    }

    #[inline]
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        critical_section::with(|cs| self.with_cs(cs, f))
    }

    #[inline]
    pub fn with_cs<F, R>(&self, _cs: CriticalSection<'_>, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        if self.flag.replace(true) {
            panic!("EnsureOnce check failed");
        }
        let inner = unsafe { &mut *self.inner.get() };
        let res = f(inner);
        self.flag.set(false);
        res
    }

    // SAFETY: caller must ensure that there is no other reference while
    // the returned `&mut T` is in use.
    pub unsafe fn get_unchecked(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}
