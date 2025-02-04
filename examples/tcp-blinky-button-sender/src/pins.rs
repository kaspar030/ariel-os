use ariel_os::hal::peripherals;

#[cfg(context = "nrf52840dk")]
ariel_os::hal::define_peripherals!(ButtonPeripherals { btn1: P0_11 });

#[cfg(context = "nrf5340dk")]
ariel_os::hal::define_peripherals!(ButtonPeripherals { btn1: P0_23 });

// ariel_os::hal::group_peripherals!(Peripherals { button: Button });
