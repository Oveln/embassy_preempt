#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::ffi::c_void;

use embassy_preempt_log::task_log;
// <- derive attribute
use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::{AsyncOSTaskCreate, SyncOSTaskCreate};
use embassy_preempt_executor::os_time::blockdelay::delay;
use embassy_preempt_executor::os_time::timer::Timer;
use embassy_preempt_executor::os_time::OSTimeDly;
// use embassy_preempt::{self as _};

const LONG_TIME: usize = 10;
const MID_TIME: usize = 5;
const SHORT_TIME: usize = 3;

#[cortex_m_rt::entry]
fn test_basic_schedule() -> ! {
    // os初始化
    OSInit();
    // 创建两个任务
    SyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);
    SyncOSTaskCreate(task2, 0 as *mut c_void, 0 as *mut usize, 11);
    AsyncOSTaskCreate(task3, 0 as *mut c_void, 0 as *mut usize, 12);
    SyncOSTaskCreate(task4, 0 as *mut c_void, 0 as *mut usize, 13);
    // 启动os
    OSStart();
}

fn task1(_args: *mut c_void) {
    loop {
        // 任务1
        task_log!(info, "Task 1 starting execution");
        OSTimeDly(100_000);
        task_log!(info, "Task 1 completed execution cycle");
        delay(SHORT_TIME);
    }
}

fn task2(_args: *mut c_void) {
    loop {
        // 任务2
        task_log!(info, "Task 2 starting execution");
        OSTimeDly(200_000);
        task_log!(info, "Task 2 completed execution cycle");
        delay(SHORT_TIME);
    }
}

async fn task3(_args: *mut c_void) {
    // 任务3
    loop {
        task_log!(info, "Task 3 (async) starting execution");
        Timer::after_secs(LONG_TIME as u64).await;
        task_log!(info, "Task 3 (async) completed execution cycle");
        delay(SHORT_TIME);
    }
}

fn task4(_args: *mut c_void) {
    // 任务4
    task_log!(info, "Task 4 starting execution - creating Task 1");
    // Task 4 creates a new task instance
    SyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 14);
    delay(SHORT_TIME);
    task_log!(info, "Task 4 completed execution - Task 1 created");
    delay(MID_TIME);
}