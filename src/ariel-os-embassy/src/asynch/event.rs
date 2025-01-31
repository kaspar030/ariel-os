//! This module provides an event that can be waited for (async version).

#![deny(missing_docs)]
#![deny(clippy::pedantic)]

use core::cell::UnsafeCell;

use maitake_sync::WaitQueue;

/// An [`Event`], allowing to notify multiple threads that some event has happened.
///
/// An [`Event`] manages an internal flag that can be set to true with the [`Self::set()`] method and reset
/// to false with the [`Self::clear()`] method. The [`Self::wait()`] method blocks until the flag is set to true. The
/// flag is set to false initially.
pub struct Event {
    state: UnsafeCell<LockState>,
    wq: WaitQueue,
}

unsafe impl Sync for Event {}

#[derive(Debug, Default)]
enum LockState {
    #[default]
    Locked,
    Unlocked,
}

impl Event {
    /// Creates a new **unset** [`Event`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: UnsafeCell::new(LockState::Locked),
            wq: WaitQueue::new(),
        }
    }

    /// Creates a new **set** [`Event`].
    #[must_use]
    pub const fn new_set() -> Self {
        Self {
            state: UnsafeCell::new(LockState::Unlocked),
            wq: WaitQueue::new(),
        }
    }

    /// Returns whether the [`Event`] is set.
    pub fn is_set(&self) -> bool {
        critical_section::with(|_| {
            let state = unsafe { &*self.state.get() };
            matches!(state, LockState::Unlocked)
        })
    }

    /// Waits for this [`Event`] to be set (blocking).
    ///
    /// If the event was set, this function returns directly.
    /// If the event was unset, this function will block the current task until
    /// the event gets set elsewhere.
    pub async fn wait(&self) {
        // `WaitQueue::wait()` guarantees that a `wake()` is seen after the waiter has been
        // created. So do that first, then check condition, then actually await the future.
        // This way, if the event changes *after* creation of the event, and happens to be set
        // *when we check here*, we consider it has been set.
        let waiter = self.wq.wait();
        if !self.is_set() {
            waiter.await.unwrap();
        }
    }

    /// Clears the event (non-blocking).
    ///
    /// If the event was set, it will be cleared and the function returns true.
    /// If the event was unset, the function returns false.
    pub fn clear(&self) -> bool {
        critical_section::with(|_| {
            let state = unsafe { &mut *self.state.get() };
            match state {
                LockState::Unlocked => {
                    *state = LockState::Locked;
                    true
                }
                LockState::Locked => false,
            }
        })
    }

    /// Sets the event.
    ///
    /// If the event was unset, and there were waiters, all waiters will be
    /// woken up.
    /// If the event was already set, the function just returns.
    pub fn set(&self) {
        critical_section::with(|cs| {
            let state = unsafe { &mut *self.state.get() };
            match state {
                LockState::Unlocked => {}
                LockState::Locked => {
                    self.wq.wake_all();
                    *state = LockState::Unlocked;
                }
            }
        });
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}
