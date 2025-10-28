#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::ffi::c_void;

use embassy_preempt::task_log;
// <- derive attribute
use embassy_preempt::os_core::{OSInit, OSStart};
use embassy_preempt::os_task::{AsyncOSTaskCreate, SyncOSTaskCreate};
use embassy_preempt::os_time::timer::Timer;
use embassy_preempt::os_time::OSTimeDly;
// use embassy_preempt::{self as _};

// 1s
const LONG_TIME: u64 = 100000;
// 1 ms
const MID_TIME: u64 = 10000;
// 1 us
const SHORT_TIME: u64 = 1000;

#[cortex_m_rt::entry]
fn main_test() -> ! {
    loop {
        test_basic_schedule();
    }
}
fn test_basic_schedule() {
    // os初始化
    OSInit();
    // 创建4个任务
    AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);
    SyncOSTaskCreate(task2, 0 as *mut c_void, 0 as *mut usize, 11);
    AsyncOSTaskCreate(task3, 0 as *mut c_void, 0 as *mut usize, 12);
    SyncOSTaskCreate(task4, 0 as *mut c_void, 0 as *mut usize, 13);
    // 启动os
    OSStart();
}

async fn task1(_args: *mut c_void) {
    loop{
        // 任务1
        task_log!(info, "---task1 begin---");
        Timer::after_ticks(SHORT_TIME).await;
        task_log!(info, "---task1 mid---");
        Timer::after_ticks(MID_TIME).await;
        task_log!(info, "---task1 end---");
        Timer::after_ticks(LONG_TIME).await;
    }
}
fn task2(_args: *mut c_void) {
    loop{
        // 任务2
        task_log!(info, "---task2 begin---");
        OSTimeDly(SHORT_TIME);
        task_log!(info, "---task2 mid---");
        OSTimeDly(MID_TIME);
        task_log!(info, "---task2 end---");
        OSTimeDly(LONG_TIME);
    }
}
async fn task3(_args: *mut c_void) {
    // 任务3
    loop {
        //
        task_log!(info, "---task3 begin---");
        Timer::after_ticks(LONG_TIME).await;
        task_log!(info, "---task3 mid---");
        Timer::after_ticks(MID_TIME).await;
        task_log!(info, "---task3 end---");
        Timer::after_ticks(SHORT_TIME).await;
    }
}
fn task4(_args: *mut c_void) {
    loop{
        // 任务4
        task_log!(info, "---task4 begin---");
        OSTimeDly(LONG_TIME);
        task_log!(info, "---task4 mid---");
        OSTimeDly(MID_TIME);
        task_log!(info, "---task4 end---");
        OSTimeDly(SHORT_TIME);
    }
}
