#![no_main]
#![no_std]

use ariel_os::{
    asynch,
    debug::log::*,
    time::{Instant, Timer},
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    zerocopy_channel::{Channel, Receiver, Sender},
};
use static_cell::StaticCell;

type MessageBuffer = [u8; 512];
const BLOCK_SIZE: usize = 512;
const NUM_BLOCKS: usize = 2;

#[ariel_os::task(autostart)]
async fn main() {
    info!("main(): starting");
    let spawner = asynch::Spawner::for_current_executor().await;

    static BUF: StaticCell<[MessageBuffer; NUM_BLOCKS]> = StaticCell::new();
    let buf = BUF.init([[0; BLOCK_SIZE]; NUM_BLOCKS]);
    static CHANNEL: StaticCell<Channel<'_, CriticalSectionRawMutex, MessageBuffer>> =
        StaticCell::new();
    let channel = CHANNEL.init(Channel::new(buf));
    let (mut sender, mut receiver) = channel.split();

    spawner.must_spawn(async_task(0, sender));

    Timer::after_millis(100).await;

    handle(
        |buf| {
            info!(
                "main(): now={}ms async_task() msg={:?}",
                Instant::now().as_millis(),
                core::str::from_utf8(buf).unwrap().trim_end_matches("\0")
            );
        },
        receiver,
    )
    .await;
}

#[embassy_executor::task()]
async fn async_task(id: u8, mut sender: Sender<'static, CriticalSectionRawMutex, MessageBuffer>) {
    let mut counter: u8 = 0;
    loop {
        let text = b"hello world ";
        info!("async_task({}): sending", id);
        let buf = sender.send().await;

        buf[..text.len()].copy_from_slice(text);
        buf[text.len()..text.len() + 1].copy_from_slice(&counter.to_be_bytes());

        counter += 1;

        sender.send_done();
    }
}

async fn handle(
    handler: fn(&[u8]),
    mut receiver: Receiver<'static, CriticalSectionRawMutex, MessageBuffer>,
) {
    loop {
        Timer::after_millis(1000).await;

        let buf = receiver.receive().await;
        handler(buf);
        receiver.receive_done();
    }
}
