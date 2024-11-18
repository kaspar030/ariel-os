#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(used_with_arg)]

use ariel_os::debug::{exit, log, log::*};
use core::cell::Cell;

use ariel_os::gpio::{Level, Output};
use ariel_os::hal::peripherals;
use ariel_os::thread::block_on;
use ariel_os::time::{Duration, Timer};

use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::CriticalSectionRawMutex};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use esp_hal::time::Rate;

ariel_os::hal::define_peripherals!(UlanziPeripherals {
    buzzer: GPIO15,
    matrix: GPIO32,
    rmt: RMT,
});

use smart_leds::RGB8;
const N_LEDS: usize = 32 * 8;
static SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static PIXELS: BlockingMutex<CriticalSectionRawMutex, Cell<[RGB8; N_LEDS]>> =
    BlockingMutex::new(Cell::new([RGB8::new(0, 0, 0); N_LEDS]));

#[ariel_os::task(autostart, peripherals)]
async fn matrix_refresh(peripherals: UlanziPeripherals) {
    info!("matrix refresh started");
    let mut buzzer = Output::new(peripherals.buzzer, Level::Low);
    buzzer.set_low();

    use esp_hal::rmt::Rmt;
    use esp_hal_smartled::{SmartLedsAdapterAsync, smart_led_buffer};
    use smart_leds_trait::SmartLedsWriteAsync;

    cfg_if::cfg_if! {
        if #[cfg(feature = "esp32h2")] {
            let freq = Rate::from_mhz(32);
        } else {
            let freq = Rate::from_mhz(80);
        }
    };

    let rmt = Rmt::new(peripherals.rmt, freq).unwrap().into_async();
    let mut rmt_buffer = smart_led_buffer!(N_LEDS + 20);
    let mut led = SmartLedsAdapterAsync::new(rmt.channel0, peripherals.matrix, &mut rmt_buffer);

    loop {
        let pixels = PIXELS.lock(|pixels| pixels.get());
        if let Err(e) = led.write(pixels).await {
            log::error!("Driving LED: {:?}", e);
        }
        SIGNAL.wait().await;
    }
}

mod drawer {
    use embedded_graphics::{
        geometry::{OriginDimensions, Size},
        pixelcolor::Rgb888,
        prelude::*,
    };
    use smart_leds::RGB8;

    pub struct MyDrawTarget {
        framebuffer: [RGB8; 256],
    }

    impl MyDrawTarget {
        pub const fn new() -> Self {
            Self {
                framebuffer: [RGB8::new(0, 0, 0); 256],
            }
        }
        pub fn set(&mut self, n: usize) {
            if n > 255 {
                return;
            }
            self.framebuffer[n] = RGB8::new(200, 200, 200);
        }

        pub fn unset(&mut self, n: usize) {
            if n > 255 {
                return;
            }
            self.framebuffer[n] = RGB8::new(0, 0, 0);
        }

        /// Updates the display from the framebuffer.
        pub fn flush(&self) {
            crate::PIXELS.lock(|out| out.set(self.framebuffer));
            crate::SIGNAL.signal(());
        }
    }

    impl DrawTarget for MyDrawTarget {
        type Color = Rgb888;
        // `ExampleDisplay` uses a framebuffer and doesn't need to communicate with the display
        // controller to draw pixel, which means that drawing operations can never fail. To reflect
        // this the type `Infallible` was chosen as the `Error` type.
        type Error = core::convert::Infallible;

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            for Pixel(coord, color) in pixels.into_iter() {
                // Check if the pixel coordinates are out of bounds (negative or greater than
                // (32,8)). `DrawTarget` implementation are required to discard any out of bounds
                // pixels without returning an error or causing a panic.
                if let Ok((x @ 0..=31, y @ 0..=7)) = coord.try_into() {
                    // Calculate the index in the framebuffer.
                    let index: u32 = {
                        if y % 2 == 0 {
                            x + y * 32
                        } else {
                            (y) * 32 + 31 - x
                        }
                    };

                    self.framebuffer[index as usize] = RGB8::new(color.r(), color.g(), color.b());
                }
            }

            Ok(())
        }
    }

    impl OriginDimensions for MyDrawTarget {
        fn size(&self) -> Size {
            Size::new(32, 8)
        }
    }
}

//this is currently scrambling output
//mod lavalamp;
mod scrolltext;
