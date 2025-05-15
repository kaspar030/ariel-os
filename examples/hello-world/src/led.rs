use esp_hal::{delay::Delay, main, rmt::Rmt, time::Rate};
use esp_hal_smartled::{SmartLedsAdapter, smartLedBuffer};
use smart_leds::{
    SmartLedsWrite, brightness, gamma,
    hsv::{Hsv, hsv2rgb},
};

use ariel_os::debug::log::*;
use ariel_os::hal::peripherals;
ariel_os::hal::define_peripherals!(LedPeripherals { led: GPIO5 });

#[ariel_os::task(autostart, peripherals)]
async fn led(peripherals: LedPeripherals) {
    let led_pin = peripherals.LED;

    // Configure RMT peripheral globally
    let freq = Rate::from_mhz(80);

    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();

    // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can
    // be used directly with all `smart_led` implementations
    let rmt_buffer = smartLedBuffer!(1);
    let mut led = SmartLedsAdapter::new(rmt.channel0, led_pin, rmt_buffer);

    let delay = Delay::new();

    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;

    loop {
        // Iterate over the rainbow!
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [hsv2rgb(color)];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            led.write(brightness(gamma(data.iter().cloned()), 10))
                .unwrap();
            delay.delay_millis(20);
        }
    }
}
