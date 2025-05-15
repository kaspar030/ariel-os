use esp_hal::ledc::LSGlobalClkSource;
use esp_hal::ledc::Ledc;
use esp_hal::ledc::LowSpeed;
use esp_hal::ledc::channel::{self, ChannelIFace};
use esp_hal::ledc::timer::{self, TimerIFace};

use ariel_os::debug::log::*;
use ariel_os::hal::peripherals;
ariel_os::hal::define_peripherals!(BacklightPeripherals {
    led: GPIO5,
    ledc: LEDC
});

#[ariel_os::task(autostart, peripherals)]
async fn backlight(peripherals: BacklightPeripherals) {
    info!("ledc init");
    let led = peripherals.led;

    let mut ledc = Ledc::new(peripherals.ledc);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut lstimer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    lstimer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty5Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: fugit::HertzU32::kHz(24),
        })
        .unwrap();

    let mut channel0 = ledc.channel(channel::Number::Channel0, led);
    channel0
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 10,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    info!("ledc entering loop");
    loop {
        // Set up a breathing LED: fade from off to on over a second, then
        // from on back off over the next second.  Then loop.
        info!("ledc fade up");
        channel0.start_duty_fade(0, 100, 1000).unwrap();
        while channel0.is_duty_fade_running() {}
        info!("ledc fade down");
        channel0.start_duty_fade(100, 0, 1000).unwrap();
        while channel0.is_duty_fade_running() {}
    }
}
