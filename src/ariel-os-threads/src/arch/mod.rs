use crate::Thread;

/// Arch-specific implementations for the scheduler.
pub trait Arch {
    const DEFAULT_THREAD_DATA: Self::ThreadData;

    type ThreadData;

    /// Sets up the stack for newly created threads and returns the sp.
    ///
    /// After running this, the stack should look as if the thread was
    /// interrupted by an ISR.
    ///
    /// It sets up the stack so when the context is switched to this thread,
    /// it starts executing `func` with argument `arg`.
    /// Furthermore, it sets up the link-register with the [`crate::cleanup`] function that
    /// will be executed after the thread function returned.
    fn setup_stack(thread: &mut Thread, stack: &mut [u8], func: fn(), arg: Option<usize>);

    /// Trigger a context switch.
    fn schedule();

    /// Setup and initiate the first context switch.
    fn start_threading();

    /// Prompts the CPU to enter deep sleep until an interrupt occurs.
    #[allow(dead_code, reason = "used in scheduler implementation")]
    fn wfi();
}

cfg_if::cfg_if! {
    if #[cfg(context = "cortex-m")] {
        mod cortex_m;
        pub use cortex_m::Cpu;
    } else if #[cfg(context = "riscv")] {
        mod riscv;
        pub use riscv::Cpu;
    } else if #[cfg(context = "xtensa")] {
        mod xtensa;
        pub use xtensa::Cpu;
    } else {
        pub struct Cpu;
        impl Arch for Cpu {
            type ThreadData = ();
            const DEFAULT_THREAD_DATA: Self::ThreadData = ();

            fn setup_stack( _: &mut Thread, _: &mut [u8], _: fn(), _: Option<usize>) {
                unimplemented!()
            }
            fn start_threading() {
                unimplemented!()
            }
            fn schedule() {
                unimplemented!()
            }
            fn wfi() {
                unimplemented!()
            }
        }
    }
}

pub type ThreadData = <Cpu as Arch>::ThreadData;

pub fn schedule() {
    Cpu::schedule();
}
