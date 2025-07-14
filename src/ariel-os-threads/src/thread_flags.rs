//! Thread flags.
use crate::{SCHEDULER, Scheduler, ThreadId, ThreadState};

/// Bitmask that represent the flags that are set for a thread.
pub type ThreadFlags = u16;

/// Possible waiting modes for [`ThreadFlags`].
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WaitMode {
    /// Wait for any of the specified thread flags.
    Any(ThreadFlags),
    /// Wait for all of the specified thread flags.
    All(ThreadFlags),
}

/// Sets flags for a thread.
///
/// If the thread was blocked on these flags it's unblocked and added
/// to the runqueue.
///
/// # Panics
///
/// Panics if `thread_id` is >= [`THREAD_COUNT`](crate::THREAD_COUNT).
pub fn set(thread_id: ThreadId, mask: ThreadFlags) {
    SCHEDULER.with(|scheduler| scheduler.flag_set(thread_id, mask));
}

/// Waits until all flags in `mask` are set for the current thread.
///
/// Returns the set flags for this mask and clears them for the thread.
///
/// # Panics
///
/// Panics if this is called outside of a thread context.
pub fn wait_all(mask: ThreadFlags) -> ThreadFlags {
    loop {
        if let Some(flags) = SCHEDULER.with(|scheduler| scheduler.flag_wait_all(mask)) {
            return flags;
        }
    }
}

/// Waits until any flag in `mask` is set for the current thread.
///
/// Returns all set flags for this mask and clears them for the thread.
///
/// # Panics
///
/// Panics if this is called outside of a thread context.
pub fn wait_any(mask: ThreadFlags) -> ThreadFlags {
    loop {
        if let Some(flags) = SCHEDULER.with(|scheduler| scheduler.flag_wait_any(mask)) {
            return flags;
        }
    }
}

/// Waits until any flag in `mask` is set for the current thread.
///
/// Compared to [`wait_any`], this returns and clears only one flag
/// from the mask.
///
/// # Panics
///
/// Panics if this is called outside of a thread context.
pub fn wait_one(mask: ThreadFlags) -> ThreadFlags {
    loop {
        if let Some(flags) = SCHEDULER.with(|scheduler| scheduler.flag_wait_one(mask)) {
            return flags;
        }
    }
}

/// Clears flags for the current thread.
///
/// # Panics
///
/// Panics if this is called outside of a thread context.
pub fn clear(mask: ThreadFlags) -> ThreadFlags {
    SCHEDULER.with(|scheduler| {
        let thread = scheduler.current().unwrap();
        let res = thread.flags & mask;
        thread.flags &= !mask;
        res
    })
}

/// Returns the flags set for the current thread.
///
/// # Panics
///
/// Panics if this is called outside of a thread context.
pub fn get() -> ThreadFlags {
    // TODO: current() requires us to use mutable `scheduler` here
    SCHEDULER.with(|scheduler| scheduler.current().unwrap().flags)
}

impl Scheduler {
    // thread flags implementation
    fn flag_set(&mut self, thread_id: ThreadId, mask: ThreadFlags) {
        let thread = self.get_unchecked_mut(thread_id);
        thread.flags |= mask;
        match thread.state {
            ThreadState::FlagBlocked(WaitMode::Any(bits)) if thread.flags & bits != 0 => {}
            ThreadState::FlagBlocked(WaitMode::All(bits)) if thread.flags & bits == bits => {}
            _ => return,
        }
        self.set_state(thread_id, ThreadState::Running);
    }

    /// # Panics
    ///
    /// Panics if called outside a thread context.
    fn flag_wait_all(&mut self, mask: ThreadFlags) -> Option<ThreadFlags> {
        let thread = self.current().unwrap();
        if thread.flags & mask == mask {
            thread.flags &= !mask;
            Some(mask)
        } else {
            let thread_id = thread.tid;
            self.set_state(thread_id, ThreadState::FlagBlocked(WaitMode::All(mask)));
            None
        }
    }

    /// # Panics
    ///
    /// Panics if called outside a thread context.
    fn flag_wait_any(&mut self, mask: ThreadFlags) -> Option<ThreadFlags> {
        let thread = self.current().unwrap();
        if thread.flags & mask != 0 {
            let res = thread.flags & mask;
            thread.flags &= !res;
            Some(res)
        } else {
            let thread_id = thread.tid;
            self.set_state(thread_id, ThreadState::FlagBlocked(WaitMode::Any(mask)));
            None
        }
    }

    /// # Panics
    ///
    /// Panics if called outside a thread context.
    fn flag_wait_one(&mut self, mask: ThreadFlags) -> Option<ThreadFlags> {
        let thread = self.current().unwrap();
        if thread.flags & mask != 0 {
            let mut res = thread.flags & mask;
            // clear all but least significant bit
            res &= !res + 1;
            thread.flags &= !res;
            Some(res)
        } else {
            let thread_id = thread.tid;
            self.set_state(thread_id, ThreadState::FlagBlocked(WaitMode::Any(mask)));
            None
        }
    }
}
