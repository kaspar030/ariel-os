// Task switching based on upstream esp-rtos https://github.com/esp-rs/esp-hal/blob/210eac8d03/esp-rtos/src/task/riscv.rs
// (Apache 2.0/MIT)

#![expect(unsafe_code)]

use esp_hal::{
    interrupt::{self, software::SoftwareInterrupt},
    peripherals::Interrupt,
    riscv,
};

use crate::{Arch, SCHEDULER, Thread, cleanup};

const CONFIG_ISR_STACKSIZE: usize =
    ariel_os_utils::usize_from_env_or!("CONFIG_ISR_STACKSIZE", 2048, "ISR stack size (in bytes)");

pub struct Cpu;

/// Registers saved / restored by the context-switch trampoline.
///
/// Field order must match the offsets used in `swint_handler_trampoline`.
#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadData {
    ra: usize,
    t0: usize,
    t1: usize,
    t2: usize,
    t3: usize,
    t4: usize,
    t5: usize,
    t6: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
    gp: usize,
    tp: usize,
    sp: usize,
    pc: usize,
}

impl Arch for Cpu {
    type ThreadData = ThreadData;
    const DEFAULT_THREAD_DATA: Self::ThreadData = default_trap_frame();

    /// Stack size for the idle threads.
    /// On RISC-V (esp-hal), interrupts don't automatically change which stack they use,
    /// idle thread stack needs to be sized accordingly.
    #[cfg(feature = "idle-threads")]
    const IDLE_THREAD_STACK_SIZE: usize = CONFIG_ISR_STACKSIZE;

    /// Triggers software interrupt for the context switch.
    fn schedule() {
        // SAFETY: `steal().raise()` is safe on an initialized software interrupt
        unsafe { SoftwareInterrupt::<0>::steal().raise() }
    }

    fn setup_stack(thread: &mut Thread, stack: &mut [u8], func: fn(), arg: Option<usize>) {
        let stack_start = stack.as_ptr() as usize;
        // 16 byte alignment.
        let stack_pos = (stack_start + stack.len()) & 0xFFFF_FFE0;
        // Set up PC, SP, RA and first argument for function.
        thread.data.sp = stack_pos;
        thread.data.a0 = arg.unwrap_or_default();
        thread.data.ra = cleanup as *const () as usize;
        thread.data.pc = func as usize;
        // The trampoline restores `tp` from this field; it must point at this context.
        thread.data.tp = &raw mut thread.data as usize;

        thread.stack_lowest = stack_start;
        thread.stack_highest = stack_pos;

        // Safety: This is the place to initialize stack painting.
        unsafe { thread.stack_paint_init(stack_pos) };
    }

    /// Enable and trigger the appropriate software interrupt.
    fn start_threading() {
        // Bind the context-switch ISR directly to a CPU interrupt so it is not wrapped by
        // esp-hal's vectored handler (`riscv::interrupt::nested`). Nested handling would
        // restore `mepc` after the trampoline and prevent `mret` from reaching the new thread.
        //
        // SAFETY: This is the start of threading, so `FROM_CPU_INTR0` / CPU interrupt 0 are
        // unused. `esp_hal::init()` runs after threading starts, so `SoftwareInterruptControl`
        // cannot be constructed here.
        interrupt::enable_direct(
            Interrupt::FROM_CPU_INTR0,
            interrupt::Priority::min(),
            interrupt::DirectBindableCpuInterrupt::Interrupt0,
            swint_handler_trampoline,
        );

        Self::schedule();
    }

    fn wfi() {
        riscv::asm::wfi();
    }
}

const fn default_trap_frame() -> ThreadData {
    // SAFETY: `ThreadData` is a POD register save area.
    unsafe { core::mem::zeroed() }
}

/// Direct-bound ISR for `FROM_CPU_INTR0`.
///
/// Saves caller-saved registers into the context behind `tp`, runs the scheduler (which only
/// updates `tp`), then saves/restores the remaining context if `tp` changed.
///
/// `tp` is 0 before the first switch (and would be 0 for an idle hook with no TCB). The
/// trampoline skips saving those contexts.
#[unsafe(link_section = ".trap.rust")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
#[rustfmt::skip]
unsafe extern "C" fn swint_handler_trampoline() {
    core::arch::naked_asm! {"
        .cfi_startproc
        # https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/139d8d8e1d8ee8c0c3ee150de709ceaab5c08417/riscv-dwarf.adoc
        # .cfi_register ra, 0x1341 # Unwind with MEPC as return address, crashes probe-rs

        # Save registers
        addi sp, sp, -16 # allocate 16 bytes for saving regs (RISC-V requires 16-byte alignment)

        # Store the thread pointer on the stack. We'll use it to check what needs to be restored
        sw tp, 0*4(sp)

        # Skip storing context for the idle context or deleted tasks (no thread pointer)
        beqz tp, 1f # Skip to calling the interrupt handler

        sw ra, 0*4(tp)
        sw t0, 1*4(tp)
        sw t1, 2*4(tp)
        sw t2, 3*4(tp)
        sw t3, 4*4(tp)
        sw t4, 5*4(tp)
        sw t5, 6*4(tp)
        sw t6, 7*4(tp)
        sw a0, 8*4(tp)
        sw a1, 9*4(tp)
        sw a2, 10*4(tp)
        sw a3, 11*4(tp)
        sw a4, 12*4(tp)
        sw a5, 13*4(tp)
        sw a6, 14*4(tp)
        sw a7, 15*4(tp)

1:
        # Let's run the interrupt handler, which runs the scheduler. If the scheduler
        # decides we need to switch context, it will change the thread pointer to the new context.
        la t0, {scheduler_interrupt_handler}
        jalr ra, t0, 0

        # Load old thread pointer and free up stack. This way we store/reload the unmodified stack pointer.
        lw t0, 0*4(sp)
        addi sp, sp, 16

        # If the thread pointer has not changed, just restore caller-saved registers
        beq t0, tp, 3f # Skip to restoring caller-saved registers in the new context

        # Skip storing context for the idle context or deleted tasks (no thread pointer)
        beqz t0, 2f # Skip to loading registers for the new context

        # If the thread pointer has changed, switch context
        # First, save registers to the old context
        sw s0, 16*4(t0)
        sw s1, 17*4(t0)
        sw s2, 18*4(t0)
        sw s3, 19*4(t0)
        sw s4, 20*4(t0)
        sw s5, 21*4(t0)
        sw s6, 22*4(t0)
        sw s7, 23*4(t0)
        sw s8, 24*4(t0)
        sw s9, 25*4(t0)
        sw s10, 26*4(t0)
        sw s11, 27*4(t0)
        sw gp, 28*4(t0)
      # sw tp, 29*4(t0) # No need to save TP, it's set up when the task is created.
        sw sp, 30*4(t0)
        # mepc -> pc
        csrr t1, mepc
        sw t1, 31*4(t0)

2:
        # Next, load registers from the new context
        lw s0, 16*4(tp)
        lw s1, 17*4(tp)
        lw s2, 18*4(tp)
        lw s3, 19*4(tp)
        lw s4, 20*4(tp)
        lw s5, 21*4(tp)
        lw s6, 22*4(tp)
        lw s7, 23*4(tp)
        lw s8, 24*4(tp)
        lw s9, 25*4(tp)
        lw s10, 26*4(tp)
        lw s11, 27*4(tp)
        lw gp, 28*4(tp)
        # TP will be restored last.
        lw sp, 30*4(tp)

        lw t1, 31*4(tp)
        csrw mepc, t1

3:
        lw ra, 0*4(tp)
        lw t0, 1*4(tp)
        lw t1, 2*4(tp)
        lw t2, 3*4(tp)
        lw t3, 4*4(tp)
        lw t4, 5*4(tp)
        lw t5, 6*4(tp)
        lw t6, 7*4(tp)
        lw a0, 8*4(tp)
        lw a1, 9*4(tp)
        lw a2, 10*4(tp)
        lw a3, 11*4(tp)
        lw a4, 12*4(tp)
        lw a5, 13*4(tp)
        lw a6, 14*4(tp)
        lw a7, 15*4(tp)

        # Restore TP last. For the idle hook, this should write 0, which prevents saving its state.
        lw tp, 29*4(tp)

        mret
        .cfi_endproc
        ",
        scheduler_interrupt_handler = sym sched,
    }
}

/// Probes the runqueue for the next thread and switches context if needed.
///
/// The trampoline performs the actual register save/restore. This handler only clears the
/// software interrupt and, when the running thread changes, updates `tp` to the next
/// [`ThreadData`].
///
/// # Panics
///
/// Panics when the scheduler returned no task to switch to, this means idle threads are not enabled.
#[esp_hal::ram]
extern "C" fn sched() {
    critical_section::with(|cs| {
        // clear FROM_CPU_INTR0
        // SAFETY: `steal().reset()` is safe on an initialized software interrupt
        unsafe { SoftwareInterrupt::<0>::steal().reset() }

        SCHEDULER.with_mut_cs(cs, |mut scheduler| {
            #[cfg(feature = "multi-core")]
            scheduler.add_current_thread_to_rq();

            let next_tid = scheduler.get_next_tid().expect(
                "idle threads should be enabled, the scheduler should always have a thread ready",
            );

            if scheduler.current_tid() == Some(next_tid) {
                return;
            }

            *scheduler.current_tid_mut() = Some(next_tid);

            let next_ctx = &raw mut scheduler.get_unchecked_mut(next_tid).data;
            // SAFETY: `tp` is the RISC-V thread pointer used by `swint_handler_trampoline`.
            unsafe {
                core::arch::asm!("mv tp, {0}", in(reg) next_ctx, options(nostack));
            }
        });
    });
}
