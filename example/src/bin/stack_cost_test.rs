#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
// this test is used to compare with embassy

use core::arch::asm;
use core::ffi::c_void;

use embassy_preempt_log::task_log;
use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::AsyncOSTaskCreate;
use embassy_preempt_executor::os_time::timer::Timer;
use embassy_preempt_platform::pac::{gpio, GPIOA, RCC};
// use embassy_preempt_port::bottom_driver::Bottom::bottom;

const BLOCK_TIME: usize = 1;

// use embassy_preempt::{self as _};

#[embassy_preempt_macros::entry]
fn test_time_performance() -> ! {
    // hardware init
    // os初始化
    OSInit();
    AsyncOSTaskCreate(test_task, 0 as *mut c_void, 0 as *mut usize, 10);
    AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 15);
    AsyncOSTaskCreate(task2, 0 as *mut c_void, 0 as *mut usize, 14);
    AsyncOSTaskCreate(task3, 0 as *mut c_void, 0 as *mut usize, 13);
    AsyncOSTaskCreate(task4, 0 as *mut c_void, 0 as *mut usize, 12);
    AsyncOSTaskCreate(task5, 0 as *mut c_void, 0 as *mut usize, 11);
    // 启动os
    OSStart();
}

// 主要测试任务
async fn test_task(_args: *mut c_void) {
    loop {
        Timer::after_millis(50).await;
    }
}

// 用于模拟多任务执行环境
async fn task1(_args: *mut c_void) {
    loop {
        task_log!(trace, "the task1");
        Timer::after_millis(5 * 1000).await;
    }
}

// 用于模拟多任务执行环境
async fn task2(_args: *mut c_void) {
    loop {
        delay(BLOCK_TIME);
        // Timer::after_millis(10).await;
        task_log!(trace, "the task2");
        delay(BLOCK_TIME);
        Timer::after_millis(5).await;
    }
}

// 用于模拟多任务执行环境
async fn task3(_args: *mut c_void) {
    loop {
        delay(BLOCK_TIME);
        // Timer::after_millis(20).await;
        // #[cfg(feature = "alarm_test")]
        // trace!("the task3");
        delay(BLOCK_TIME);
        task_log!(trace, "the task3");
        Timer::after_millis(20).await;
    }
}

// 用于模拟多任务执行环境
async fn task4(_args: *mut c_void) {
    loop {
        delay(BLOCK_TIME);
        // Timer::after_millis(80).await;
        task_log!(trace, "the task4");
        delay(BLOCK_TIME);
        Timer::after_millis(8).await;
    }
}

// 用于模拟多任务执行环境
async fn task5(_args: *mut c_void) {
    loop {
        delay(BLOCK_TIME);
        // Timer::after_millis(300).await;
        task_log!(trace, "the task5");
        delay(BLOCK_TIME);
        Timer::after_millis(10).await;
    }
}


#[inline(never)]
pub fn delay(time: usize) {
    // 延时函数,time的单位约为0.5s，使用汇编编写从而不会被优化
    unsafe {
        asm!(
            // 先来个循环（总共是两层循环，内层循环次数8000000）
            "mov r0, #0",
            "1:",
            // 内层循环
            "mov r1, #0",
            "2:",
            "add r1, r1, #1",
            "cmp r1, r3",
            "blt 2b",
            // 外层循环
            "add r0, r0, #1",
            "cmp r0, r2",
            "blt 1b",
            in("r2") time,
            in("r3") 1000000/8,
        )
    }
}
