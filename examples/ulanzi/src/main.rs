#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(used_with_arg)]

use ariel_os::debug::{exit, log, log::*};
use core::cell::Cell;

use ariel_os::gpio::{Level, Output};
use ariel_os::hal::peripherals;
use ariel_os::time::{Duration, Timer};
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex};
use embassy_sync::signal::Signal;
use esp_hal::time::RateExtU32;
use esp_hal::Async;

ariel_os::hal::define_peripherals!(UlanziPeripherals {
    buzzer: GPIO15,
    matrix: GPIO32,
    rmt: RMT,
});

use smart_leds::RGB8;
const N_LEDS: usize = 32 * 8;
static SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static PIXELS: Mutex<CriticalSectionRawMutex, Cell<[RGB8; N_LEDS]>> =
    Mutex::new(Cell::new([RGB8::new(0, 0, 0); N_LEDS]));

#[ariel_os::task(autostart, peripherals)]
async fn matrix_refresh(peripherals: UlanziPeripherals) {
    info!("Hello World!");
    let mut buzzer = Output::new(peripherals.buzzer, Level::Low);
    buzzer.set_low();

    use esp_hal::rmt::Rmt;
    use esp_hal_smartled::{asynch::SmartLedAdapterAsync, smart_led_buffer};

    let freq = 80u32.MHz();
    let rmt = Rmt::new(peripherals.rmt, freq).unwrap().into_async();
    let rmt_buffer = smart_led_buffer!(N_LEDS + 11);
    let mut led = SmartLedAdapterAsync::new(rmt.channel0, peripherals.matrix, rmt_buffer);

    loop {
        let pixels = PIXELS.lock(|pixels| pixels.get());
        if let Err(e) = led.write(pixels).await {
            log::error!("Driving LED: {:?}", e);
        }
        SIGNAL.wait().await;
    }
}

//mod thread_timer {
//    use embassy_sync::{
//        blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
//    };
//    use embassy_time::{Duration, Timer};
//
//    static TIMER_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
//    static TIMER_CHANNEL: Channel<CriticalSectionRawMutex, Duration, 1> = Channel::new();
//
//    #[ariel_os::task(autostart)]
//    async fn thread_timer_task() {
//        loop {
//            let duration = TIMER_CHANNEL.receive().await;
//            Timer::after(duration).await;
//            TIMER_SIGNAL.signal(());
//        }
//    }
//
//    pub fn after(duration: Duration) {
//        use ariel_os::asynch::blocker;
//        TIMER_SIGNAL.reset();
//        blocker::block_on(TIMER_CHANNEL.send(duration));
//        blocker::block_on(TIMER_SIGNAL.wait());
//    }
//}
//
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

//#[ariel_os::thread(autostart, priority = 9)]
//fn main() {
//    info!("Hello World from thread!");
//
//    use embedded_graphics::{
//        mono_font::{ascii::FONT_5X8, MonoTextStyle},
//        pixelcolor::Rgb888,
//        prelude::*,
//        primitives::{
//            Circle, CornerRadii, Ellipse, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle,
//            RoundedRectangle, Triangle,
//        },
//        text::Text,
//    };
//
//    let mut display = drawer::MyDrawTarget::new();
//    let mut count = 0;
//    let stroke = PrimitiveStyle::with_stroke(Rgb888::MAGENTA, 1);
//    let black = PrimitiveStyle::with_stroke(Rgb888::BLACK, 1);
//
//    //let stroke_off_fill_off = PrimitiveStyleBuilder::new()
//    //    .stroke_color(Rgb888::RED)
//    //    .stroke_width(1)
//    //    .fill_color(Rgb888::GREEN)
//    //    .build();
//    //
//    let fill_black = PrimitiveStyle::with_fill(Rgb888::BLACK);
//
//    //Pixel(Point::new(31, 3), Rgb888::MAGENTA)
//    //    .draw(&mut display)
//    //    .unwrap();
//    //display.flush();
//
//    let style = MonoTextStyle::new(&FONT_5X8, Rgb888::YELLOW);
//
//    let text = "ARIEL OS";
//
//    loop {
//        for count in 0i32..(text.len() as i32 * 5 + 32) {
//            display.clear(Rgb888::BLACK).unwrap();
//            display.flush();
//
//            Text::new(text, Point::new(32 - (count), 6), style)
//                .draw(&mut display)
//                .unwrap();
//
//            Rectangle::new(Point::new(0, 0), Size::new(32, 8))
//                .into_styled(stroke)
//                .draw(&mut display)
//                .unwrap();
//
//            display.flush();
//
//            thread_timer::after(Duration::from_millis(128));
//        }
//    }
//
//    //loop {
//    //    for x in 0..=31 {
//    //        for y in 0..=7 {
//    //            Pixel(Point::new(x, y), Rgb888::MAGENTA)
//    //                .draw(&mut display)
//    //                .unwrap();
//    //            display.flush();
//    //            thread_timer::after(Duration::from_millis(100));
//    //            Pixel(Point::new(x, y), Rgb888::BLACK)
//    //                .draw(&mut display)
//    //                .unwrap();
//    //            display.flush();
//    //            thread_timer::after(Duration::from_millis(100));
//    //        }
//    //    }
//    //}
//}
//
mod lavalamp;
