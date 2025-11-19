#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
use core::ffi::c_void;

use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::AsyncOSTaskCreate;
use embassy_preempt_log::task_log;
use embassy_preempt_platform::get_platform;
use embassy_preempt_platform::driver::button::future::wait_for_button;

// #[embassy_preempt_macros::entry]
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
    loop {
        critical_section::with(|cs| get_platform().led.borrow(cs).toggle());
        task_log!(info, "waiting for button");
        wait_for_button().await;
        task_log!(info, "button pressed");
    }
}