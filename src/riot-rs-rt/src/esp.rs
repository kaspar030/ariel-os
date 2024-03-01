#[esp_hal::entry]
fn main() -> ! {
    super::startup();
}

pub fn init() {
    //    esp_println::logger::init_logger(log::LevelFilter::Debug);
}

pub fn benchmark<F: Fn() -> ()>(iterations: usize, f: F) -> core::result::Result<usize, ()> {
    unimplemented!();
}
