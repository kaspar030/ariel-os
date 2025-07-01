use ariel_os::hal::peripherals;

#[cfg(context = "nrf52840dk")]
ariel_os::hal::define_peripherals!(Peripherals { btn1: P0_11 });

#[cfg(context = "nrf5340dk")]
ariel_os::hal::define_peripherals!(Peripherals { btn1: P0_23 });

#[cfg(context = "rp")]
ariel_os::hal::define_peripherals!(Peripherals { btn1: PIN_18 });

// ariel_os::hal::group_peripherals!(Peripherals { button: Button });
