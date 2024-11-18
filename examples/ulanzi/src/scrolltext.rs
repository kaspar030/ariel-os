use ariel_os::{
    debug::log::info,
    thread::block_on,
    time::{Duration, Timer},
};

use super::drawer::MyDrawTarget;

#[ariel_os::thread(autostart, priority = 9)]
fn main() {
    info!("scrolltext thread started");

    use embedded_graphics::{
        mono_font::{MonoTextStyle, ascii::FONT_5X8},
        pixelcolor::Rgb888,
        prelude::*,
        primitives::{
            Circle, CornerRadii, Ellipse, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle,
            RoundedRectangle, Triangle,
        },
        text::Text,
    };

    let mut display = MyDrawTarget::new();

    let stroke = PrimitiveStyle::with_stroke(Rgb888::RED, 1);

    let text_style = MonoTextStyle::new(&FONT_5X8, Rgb888::YELLOW);

    let text = "ARIEL OS";

    loop {
        for count in 0i32..(text.len() as i32 * 5 + 32) {
            display.clear(Rgb888::BLACK).unwrap();
            display.flush();

            Text::new(text, Point::new(32 - (count), 6), text_style)
                .draw(&mut display)
                .unwrap();

            Rectangle::new(Point::new(0, 0), Size::new(32, 8))
                .into_styled(stroke)
                .draw(&mut display)
                .unwrap();

            display.flush();

            block_on(Timer::after(Duration::from_millis(128)));
        }
    }
}
