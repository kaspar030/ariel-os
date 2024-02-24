pub(crate) use embassy_executor::InterruptExecutor as Executor;

use esp32c3_hal::{
    clock::ClockControl,
    embassy::{
        self,
        executor::{FromCpu1, InterruptExecutor},
    },
    interrupt::Priority,
    peripherals::Peripherals,
    prelude::*,
};

pub type Executor = InterruptExecutor<FromCpu1>;

#[interrupt]
fn FROM_CPU_INTR1() {
    unsafe { crate::EXECUTOR.on_interrupt() }
}

pub fn init(config: Config) -> Peripherals {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    embassy::init(
        &clocks,
        esp32c3_hal::systimer::SystemTimer::new(peripherals.SYSTIMER),
    );

    peripherals
}

// #[cfg(feature = "usb")]
// use embassy_rp::{bind_interrupts, peripherals::USB, usb::InterruptHandler};

// // rp2040 usb start
// #[cfg(feature = "usb")]
// bind_interrupts!(struct Irqs {
//     USBCTRL_IRQ => InterruptHandler<USB>;
// });

// #[cfg(feature = "usb")]
// pub mod usb {
//     use embassy_rp::peripherals;
//     use embassy_rp::usb::Driver;

//     use crate::arch;

//     pub type UsbDriver = Driver<'static, peripherals::USB>;

//     pub fn driver(peripherals: &mut arch::OptionalPeripherals) -> UsbDriver {
//         let usb = peripherals.USB.take().unwrap();
//         Driver::new(usb, super::Irqs)
//     }
// }
