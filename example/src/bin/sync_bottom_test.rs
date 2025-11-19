#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::ffi::c_void;

use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::SyncOSTaskCreate;
// use embassy_preempt_port::bottom_driver::OSWaitBot;
use embassy_preempt_log::task_log;
// use embassy_preempt::os_time::timer::Timer;
// use embassy_preempt::port::bottom_driver::Bottom::bottom;
// use embassy_preempt::{self as _};

#[embassy_preempt_macros::entry]
fn test_hardware() -> ! {
    // os初始化
    OSInit();
    // 为了测试硬件以及time driver的正确性，只创建1个任务以避免抢占
    SyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);
    // 启动os
    OSStart();
}

fn task1(_args: *mut c_void) {
    loop {
        task_log!(info, "sync bottom test running");
        // bottom::wait_for_rising_edge().await;
        // OSWaitBot();
    }
}