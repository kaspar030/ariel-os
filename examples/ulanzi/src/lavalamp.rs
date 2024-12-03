use ariel_os::debug::{log::info, println};
use ariel_os::random::RngCore;
use ariel_os::time::{Duration, Instant};
use embassy_time::Ticker;
use smart_leds::{
    colors, gamma,
    hsv::{hsv2rgb, Hsv},
    RGB8,
};

#[ariel_os::task(autostart)]
async fn lavalamp() {
    println!("lavalamp task started");
    const NODE: Node = Node::new();
    let mut nodes = [NODE; 256];
    let mut storage = [colors::BLACK; 256];
    let mut rng = ariel_os::random::fast_rng();

    let mut hue: u8 = rng.next_u32() as u8;
    let mut sat: u8 = rng.next_u32() as u8;
    let mut last_color = Instant::now();
    let mut last_draw = Ticker::every(Duration::from_micros(1_000_000 / 60));
    let mut idx = 0u32;
    for node in nodes.iter_mut() {
        node.idles = (rng.next_u32() as u16) & 0x3FF;
    }
    let mut dur = Duration::from_secs(15);
    let mut ticks = 0;
    let mut fps_tick = Instant::now();

    // Go forever
    loop {
        // Report FPS periodically
        if fps_tick.elapsed() >= Duration::from_secs(5) {
            println!("FPS: {:.02}", ticks as f64 / 5.0);
            ticks = 0;
            fps_tick = Instant::now();
        }

        // If it's time to pick a new color, do so!
        if last_color.elapsed() >= dur {
            // Modulate the time between new colors so that we get some variance
            // to the pattern
            if rng.next_u32() != 0 {
                dur = (dur + Duration::from_secs(1)).min(Duration::from_secs(60));
            } else {
                dur = Duration::from_millis(
                    (dur.as_millis()
                        .saturating_sub(Duration::from_secs(1).as_millis()))
                    .max(Duration::from_secs(5).as_millis()),
                );
            }
            println!("Duration: {dur:?}");
            hue = rng.next_u32() as u8;
            sat = SAT_LUT[rng.next_u32() as u8 as usize];
            last_color = Instant::now();
        }

        // For each LED, make it take one step, progressing through their individual
        // sine waves
        nodes.iter_mut().zip(storage.iter_mut()).for_each(|(n, s)| {
            *s = n.step(&mut rng, hue, sat);
        });

        // This is our rate-limiter (60fps)
        last_draw.next().await;

        crate::PIXELS.lock(|out| out.set(storage));
        crate::SIGNAL.signal(());

        ticks += 1;
    }
}

/// Each LED gets it's own "actor", all LEDs actually act independently
/// All math is 32-bit fixed point, and works well with no_std RngCore impls
struct Node {
    hue: u8,
    sat: u8,
    idles: u16,
    phase: u32,
    rate: u32,
}

impl Node {
    const fn new() -> Self {
        Self {
            hue: 0,
            sat: 0,
            phase: 0,
            rate: 0,
            idles: 0,
        }
    }

    fn step<R: RngCore>(&mut self, rng: &mut R, new_hue: u8, new_sat: u8) -> RGB8 {
        // If we're not currently fading a color...
        if self.rate == 0 {
            // Wait until the idle period has elapsed.
            if self.idles > 0 {
                self.idles -= 1;
            } else {
                // Idle period has elapsed, take the CURRENT new color
                self.hue = new_hue;
                self.sat = new_sat;
                self.phase = 0;
                // This is the rate that `phase` travels 0..u32::MAX. The higher the number,
                // the faster we go.
                self.rate = (rng.next_u32() & 0x01FF_FFFF) | 0x003F_FFFF;
            }
            colors::BLACK
        } else {
            // If we are fading a color, take one phase step forward
            match self.phase.checked_add(self.rate) {
                Some(val) => {
                    // Do some tricky fixed point interpolation for smoothed sine
                    // traversal. This modulates the "V" of the HSV color to get
                    // the single value that follows the sine curve.
                    self.phase = val;
                    let idx_now = (self.phase >> 24) as u8;
                    let idx_nxt = idx_now.wrapping_add(1);

                    let base_val = HALF_LUT[idx_now as usize] as u32;
                    let next_val = HALF_LUT[idx_nxt as usize] as u32;

                    // Distance to next value
                    let off = (self.phase >> 16) & 0xFF; // 0..=255
                    let cur_weight = base_val.wrapping_mul(256u32.wrapping_sub(off));
                    let nxt_weight = next_val.wrapping_mul(off);
                    let ttl_weight = cur_weight.wrapping_add(nxt_weight);
                    let ttl_val = ttl_weight >> 8;
                    let ttl_val = ttl_val as u8;

                    // Convert the current
                    hsv2rgb(Hsv {
                        hue: self.hue,
                        sat: self.sat,
                        val: ttl_val,
                    })
                }
                // We just overflowed the phase angle! This means we are done walking the up and down
                // of the sine wave. Pick a random number of frames to "stay idle", so we have some
                // contrast with other lit pixels
                None => {
                    self.rate = 0;
                    self.idles = ((rng.next_u32() as u16) & 0xFFF) | 0x3F;
                    colors::BLACK
                }
            }
        }
    }
}

/// 1/2 of a sine wave, just the positive up and down half
const HALF_LUT: [u8; 256] = [
    0, 3, 6, 9, 13, 16, 19, 22, 25, 28, 31, 34, 37, 41, 44, 47, 50, 53, 56, 59, 62, 65, 68, 71, 74,
    77, 80, 83, 86, 89, 92, 95, 98, 100, 103, 106, 109, 112, 115, 117, 120, 123, 126, 128, 131,
    134, 136, 139, 142, 144, 147, 149, 152, 154, 157, 159, 162, 164, 167, 169, 171, 174, 176, 178,
    180, 183, 185, 187, 189, 191, 193, 195, 197, 199, 201, 203, 205, 207, 208, 210, 212, 214, 215,
    217, 219, 220, 222, 223, 225, 226, 228, 229, 231, 232, 233, 234, 236, 237, 238, 239, 240, 241,
    242, 243, 244, 245, 246, 247, 247, 248, 249, 249, 250, 251, 251, 252, 252, 253, 253, 253, 254,
    254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 254, 254, 253, 253, 253,
    252, 252, 251, 251, 250, 249, 249, 248, 247, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238,
    237, 236, 234, 233, 232, 231, 229, 228, 226, 225, 223, 222, 220, 219, 217, 215, 214, 212, 210,
    208, 207, 205, 203, 201, 199, 197, 195, 193, 191, 189, 187, 185, 183, 180, 178, 176, 174, 171,
    169, 167, 164, 162, 159, 157, 154, 152, 149, 147, 144, 142, 139, 136, 134, 131, 128, 126, 123,
    120, 117, 115, 112, 109, 106, 103, 100, 98, 95, 92, 89, 86, 83, 80, 77, 74, 71, 68, 65, 62, 59,
    56, 53, 50, 47, 44, 41, 37, 34, 31, 28, 25, 22, 19, 16, 13, 9, 6, 3,
];

/// This is a biased look up table, so that we favor "more saturation", because
/// less saturated colors are all just washed-out bright white colors
const SAT_LUT: [u8; 256] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254,
    254, 254, 254, 254, 254, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253,
    253, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 251, 251, 251, 251, 251, 251,
    251, 251, 251, 250, 250, 250, 250, 250, 250, 250, 250, 249, 249, 249, 249, 249, 249, 249, 248,
    248, 248, 248, 248, 247, 247, 247, 247, 247, 246, 246, 246, 246, 246, 245, 245, 245, 245, 244,
    244, 244, 244, 243, 243, 243, 243, 242, 242, 242, 241, 241, 241, 240, 240, 240, 239, 239, 239,
    238, 238, 237, 237, 237, 236, 236, 235, 235, 234, 234, 233, 233, 232, 232, 231, 231, 230, 230,
    229, 229, 228, 227, 227, 226, 225, 225, 224, 223, 223, 222, 221, 220, 220, 219, 218, 217, 216,
    215, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 204, 203, 202, 201, 200, 199, 197, 196,
    195, 193, 192, 191, 189, 188, 186, 185, 183, 182, 180, 178, 177, 175, 173, 171, 169, 167, 165,
    164, 161, 159, 157, 155, 153, 151, 148, 146, 144, 141, 139, 136, 133, 131, 128, 125, 122, 119,
    116, 113, 110, 107, 104, 100, 97, 94, 90, 86, 83, 79, 75, 71, 67, 63, 59, 54, 50, 45, 41, 36,
    31, 26, 21, 16, 11, 5,
];
