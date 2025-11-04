#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
use core::ffi::c_void;

// extern crate embassy_preempt;
use embassy_preempt::task_log;
use embassy_preempt::app::led::{LED_Init, LED_OFF, LED_ON};
use embassy_preempt::os_core::{OSInit, OSStart};
use embassy_preempt::os_task::AsyncOSTaskCreate;
// use embassy_preempt::os_time::timer::Timer;
use embassy_preempt::port::bottom_driver::Bottom::bottom;
use embassy_preempt_platform::Platform;
// use embassy_preempt::{self as _};
#[cortex_m_rt::entry]
fn test_hardware() -> ! {
    // os初始化
    OSInit();
    // 启动os
    OSStart();
}