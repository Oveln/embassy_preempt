#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::cell::UnsafeCell;
use core::ffi::c_void;

use embassy_preempt_driver::led_hal::Led;
use embassy_preempt_executor::os_time::OSTimeDly;
use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::AsyncOSTaskCreate;
use embassy_preempt_log::task_log;
use spin::{Mutex, Once};

static LED: Once<Mutex<Led>> = Once::new();

#[embassy_preempt_macros::entry]
fn test_hardware() -> ! {
    // os初始化
    OSInit();
    // 为了测试硬件以及time driver的正确性，只创建1个任务以避免抢占
    AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);
    // 启动os
    OSStart();
}

async fn task1(_args: *mut c_void) {
    let led = LED.call_once(|| Mutex::new(Led::new()));
    loop {
        let mut led = led.lock();
        led.toggle();
        task_log!(info, "toggle in task");
        OSTimeDly(50000);
    }
}