#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::ffi::c_void;

// extern crate embassy_preempt;
use embassy_preempt_log::task_log;
use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::AsyncOSTaskCreate;
use embassy_preempt_executor::os_time::timer::Timer;
// use embassy_preempt::{self as _};

#[embassy_preempt_macros::entry]
fn test_hardware() -> ! {
    // os初始化
    OSInit();
    task_log!(info, "init success");
    // 为了测试硬件以及time driver的正确性，只创建1个任务以避免抢占
    AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);
    task_log!(info, "create task success");
    // 启动os
    OSStart();
}

async fn task1(_args: *mut c_void) {
    loop {
        task_log!(info, "hardware test running");
        // delay 5s
        Timer::after_ticks(20000).await;
    }
}