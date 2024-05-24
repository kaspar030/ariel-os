pub mod gpio;

#[cfg(feature = "hwrng")]
pub mod hwrng;

#[cfg(feature = "usb")]
pub mod usb;

pub(crate) use embassy_executor::InterruptExecutor as Executor;

#[cfg(context = "nrf52")]
crate::executor_swi!(SWI0_EGU0);

#[cfg(context = "nrf5340")]
crate::executor_swi!(EGU0);

use embassy_nrf::config::Config;

pub use embassy_nrf::{interrupt, peripherals, OptionalPeripherals};

pub fn init() -> OptionalPeripherals {
    let peripherals = embassy_nrf::init(Config::default());
    OptionalPeripherals::from(peripherals)
}
