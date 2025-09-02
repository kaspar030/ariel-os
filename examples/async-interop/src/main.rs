#![no_main]
#![no_std]

use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel, signal::Signal,
};

use heapless::Vec;

use ariel_os::{
    asynch,
    debug::{ExitCode, exit, log::*},
    time::{Instant, Timer},
};

// static SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();
static CHANNEL: PubSubChannel<CriticalSectionRawMutex, Vec<u8, 4>, 20, 1, 4> = PubSubChannel::new();

#[embassy_executor::task(pool_size = 4)]
async fn async_task(id: u8) {
    info!("async_task({}): starting", id);

    let publisher = CHANNEL.publisher().unwrap();

    let mut counter = 0u8;
    loop {
        info!("async_task({}): signaling, counter={}", id, counter);
        // SIGNAL.(counter);
        let mut buf = Vec::new();
        buf.push(counter).unwrap();
        buf.push(b'9').unwrap();
        buf.push(b'9').unwrap();
        buf.push(b'9').unwrap();

        // buf[0] = counter;
        // buf[1] = b's';
        // buf[2] = b'e';
        // buf[3] = b'c';

        publisher.publish(buf).await;

        Timer::after_millis(100).await;
        counter += 1;
    }
}

#[ariel_os::task(autostart)]
async fn main() {
    info!("main(): starting");

    let spawner = asynch::Spawner::for_current_executor().await;

    let mut subscriber = CHANNEL.subscriber().unwrap();

    spawner.spawn(async_task(0)).unwrap();
    spawner.spawn(async_task(1)).unwrap();
    spawner.spawn(async_task(2)).unwrap();
    spawner.spawn(async_task(3)).unwrap();

    Timer::after_millis(100).await;

    loop {
        let counter = subscriber.next_message_pure().await;
        info!(
            "main(): now={}ms async_task() counter={:?}",
            Instant::now().as_millis(),
            core::str::from_utf8(counter.trim_ascii()).unwrap()
        );

        // match subscriber.try_next_message() {
        //     Some(counter) => info!(
        //         "main(): now={}ms async_task() counter={:?}",
        //         Instant::now().as_millis(),
        //         counter
        //     ),
        //     None => Timer::after_millis(100).await,
        // }
    }

    // for _ in 0..10 {
    //     let counter = SIGNAL.wait().await;
    //
    //     let now = Instant::now().as_millis();
    //     info!("main(): now={}ms async_task() counter={}", now, counter);
    // }
    //
    // info!("main(): all good, exiting.");
    //
    // exit(ExitCode::SUCCESS);
}
