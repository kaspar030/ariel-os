use crate::{arch::OptionalPeripherals, Spawner};
use esp_hal as hal;
use esp_hal::Rng;
use esp_wifi::wifi::{WifiDeviceMode, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::{initialize, EspWifiInitFor, EspWifiInitialization};
use once_cell::sync::OnceCell;

pub use esp_wifi::wifi::{WifiController, WifiDevice};

pub type NetworkDevice = WifiDevice<'static, WifiStaDevice>;

pub static WIFI_INIT: OnceCell<EspWifiInitialization> = OnceCell::new();

pub fn init(peripherals: &mut OptionalPeripherals, spawner: Spawner) -> NetworkDevice {
    let wifi = peripherals.WIFI.take().unwrap();
    let init = WIFI_INIT.get().unwrap();
    let (device, controller) = esp_wifi::wifi::new_with_mode(init, wifi, WifiStaDevice).unwrap();

    spawner.spawn(connection(controller)).ok();

    device
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    use riot_rs_rt::debug::println;

    use embassy_time::{Duration, Timer};
    use esp_wifi::wifi::{ClientConfiguration, Configuration};
    use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};

    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: super::WIFI_NETWORK.try_into().unwrap(),
                password: super::WIFI_PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}
