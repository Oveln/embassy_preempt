#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::ffi::c_void;

use embassy_preempt_log::task_log;
use embassy_preempt_app::led::LED_Init;
// <- derive attribute
use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::{AsyncOSTaskCreate, SyncOSTaskCreate};

// the unit is tick
// test long TIME delay
const LONG_LONG_TIME: u64 = 100000;
const LONG_TIME: u64 = 10000;
// test high frequency interrupt
const MID_TIME: u64 = 100;
const SHORT_TIME: u64 = 10;
// the set alarm will return false
const SHORT_SHORT_TIME: u64 = 1;

#[cortex_m_rt::entry]
fn test_basic_schedule() -> ! {
    LED_Init();
    // os初始化
    OSInit();
    // TASK create. The prio should be interlude, and the prio should be low to high
    // create ASYNC TASK
    AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 38);
    AsyncOSTaskCreate(task2, 0 as *mut c_void, 0 as *mut usize, 36);
    AsyncOSTaskCreate(task3, 0 as *mut c_void, 0 as *mut usize, 34);
    AsyncOSTaskCreate(task4, 0 as *mut c_void, 0 as *mut usize, 32);
    AsyncOSTaskCreate(task5, 0 as *mut c_void, 0 as *mut usize, 30);
    AsyncOSTaskCreate(task6, 0 as *mut c_void, 0 as *mut usize, 28);
    AsyncOSTaskCreate(task7, 0 as *mut c_void, 0 as *mut usize, 26);
    AsyncOSTaskCreate(task8, 0 as *mut c_void, 0 as *mut usize, 24);
    AsyncOSTaskCreate(task9, 0 as *mut c_void, 0 as *mut usize, 22);
    AsyncOSTaskCreate(task10, 0 as *mut c_void, 0 as *mut usize, 20);
    AsyncOSTaskCreate(task11, 0 as *mut c_void, 0 as *mut usize, 18);
    AsyncOSTaskCreate(task12, 0 as *mut c_void, 0 as *mut usize, 16);
    AsyncOSTaskCreate(task13, 0 as *mut c_void, 0 as *mut usize, 14);
    AsyncOSTaskCreate(task14, 0 as *mut c_void, 0 as *mut usize, 12);
    AsyncOSTaskCreate(task15, 0 as *mut c_void, 0 as *mut usize, 10);
    // create SYNC TASK
    SyncOSTaskCreate(task16, 0 as *mut c_void, 0 as *mut usize, 39);
    SyncOSTaskCreate(task17, 0 as *mut c_void, 0 as *mut usize, 37);
    SyncOSTaskCreate(task18, 0 as *mut c_void, 0 as *mut usize, 35);
    SyncOSTaskCreate(task19, 0 as *mut c_void, 0 as *mut usize, 33);
    SyncOSTaskCreate(task20, 0 as *mut c_void, 0 as *mut usize, 31);
    SyncOSTaskCreate(task21, 0 as *mut c_void, 0 as *mut usize, 29);
    SyncOSTaskCreate(task22, 0 as *mut c_void, 0 as *mut usize, 27);
    SyncOSTaskCreate(task23, 0 as *mut c_void, 0 as *mut usize, 25);
    SyncOSTaskCreate(task24, 0 as *mut c_void, 0 as *mut usize, 23);
    SyncOSTaskCreate(task25, 0 as *mut c_void, 0 as *mut usize, 21);
    SyncOSTaskCreate(task26, 0 as *mut c_void, 0 as *mut usize, 19);
    SyncOSTaskCreate(task27, 0 as *mut c_void, 0 as *mut usize, 17);
    SyncOSTaskCreate(task28, 0 as *mut c_void, 0 as *mut usize, 15);
    SyncOSTaskCreate(task29, 0 as *mut c_void, 0 as *mut usize, 13);
    SyncOSTaskCreate(task30, 0 as *mut c_void, 0 as *mut usize, 11);
    // 启动os
    OSStart();
}
macro_rules! generate_tasks_async {
    (
        $(
            $fn_name:ident: ($first_delay:ident, $second_delay:ident)
        ),+ $(,)?
    ) => {
        $(
            async fn $fn_name(_args: *mut core::ffi::c_void) {
                loop {
                    task_log!(info, "---{} begin---", stringify!($fn_name));

                    embassy_preempt_executor::os_time::timer::Timer::after_ticks($first_delay).await;

                    task_log!(info, "---{} end---", stringify!($fn_name));

                    embassy_preempt_executor::os_time::timer::Timer::after_ticks($second_delay).await;
                }
            }
        )+
    };
}
macro_rules! generate_tasks_sync {
    (
        $(
            $fn_name:ident: ($first_delay:ident, $second_delay:ident)
        ),+ $(,)?
    ) => {
        $(
            fn $fn_name(_args: *mut core::ffi::c_void) {
                loop {
                    task_log!(info, "---{} begin---", stringify!($fn_name));

                    embassy_preempt_executor::os_time::OSTimeDly($first_delay);

                    task_log!(info, "---{} end---", stringify!($fn_name));

                    embassy_preempt_executor::os_time::OSTimeDly($second_delay);
                }
            }
        )+
    };
}
generate_tasks_async! {
    task1: (SHORT_SHORT_TIME, LONG_TIME),
    task2: (SHORT_SHORT_TIME, LONG_TIME),
    task3: (SHORT_SHORT_TIME, LONG_TIME),
    task4: (SHORT_SHORT_TIME, LONG_TIME),
    task5: (SHORT_SHORT_TIME, LONG_TIME),
    task6: (SHORT_SHORT_TIME, LONG_TIME),
    task7: (SHORT_SHORT_TIME, LONG_TIME),
    task8: (SHORT_SHORT_TIME, LONG_TIME),
    task9: (SHORT_SHORT_TIME, LONG_TIME),
    task10: (SHORT_SHORT_TIME, LONG_TIME),

    task11: (SHORT_TIME, LONG_TIME),
    task12: (SHORT_TIME, LONG_TIME),
    task13: (SHORT_TIME, LONG_TIME),
    task14: (SHORT_TIME, LONG_TIME),
    task15: (SHORT_TIME, LONG_TIME),
}
generate_tasks_sync! {
    task16: (SHORT_TIME, LONG_TIME),
    task17: (SHORT_TIME, LONG_TIME),
    task18: (SHORT_TIME, LONG_TIME),
    task19: (SHORT_TIME, LONG_TIME),
    task20: (SHORT_TIME, LONG_TIME),

    task21: (MID_TIME, LONG_TIME),
    task22: (MID_TIME, LONG_TIME),
    task23: (MID_TIME, LONG_TIME),
    task24: (MID_TIME, LONG_TIME),
    task25: (MID_TIME, LONG_TIME),

    task26: (LONG_LONG_TIME, LONG_TIME),
    task27: (LONG_LONG_TIME, LONG_TIME),
    task28: (LONG_LONG_TIME, LONG_TIME),
    task29: (LONG_LONG_TIME, LONG_TIME),
    task30: (LONG_LONG_TIME, LONG_TIME),
}