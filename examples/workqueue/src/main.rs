#![no_main]
#![no_std]

use ariel_os::debug::log::*;
use ariel_os::thread::sync::Channel;

static WORK_QUEUE: Channel<Job> = Channel::new();

extern "C" fn some_job(arg: usize) {
    info!("some job, usize is {}", arg);
}

#[ariel_os::thread(autostart)]
fn test_thread() {
    let job = Job {
        func: some_job,
        arg: 42,
    };
    defer_job(job.func, job.arg);
}

#[ariel_os::thread(autostart)]
fn thread0() {
    worker();
}

#[ariel_os::thread(autostart)]
fn thread1() {
    worker();
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Job {
    func: extern "C" fn(usize),
    arg: usize,
}

pub extern "C" fn defer_job(func: extern "C" fn(usize), arg: usize) {
    info!("deferring job");
    WORK_QUEUE.send(&Job { func, arg });
}

fn worker() {
    let my_id = ariel_os::thread::current_tid().unwrap();
    loop {
        info!("[{:?} Waiting for job...", my_id);
        let job = WORK_QUEUE.recv();
        info!("[{:?} Waiting got job, executing", my_id);
        (job.func)(job.arg);
        info!("[{:?} Job done.", my_id);
    }
}
