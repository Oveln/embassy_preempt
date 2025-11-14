#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

// FFI接口
use core::ffi::c_void;

use critical_section::CriticalSection;
use embassy_preempt_log::task_log;
use embassy_preempt_executor::SyncOSTaskCreate;
use embassy_preempt_platform::Platform;

#[defmt_test::tests]
mod tests {
    use embassy_preempt_executor::os_core::{OSInit, OSStart};
    use embassy_preempt_executor::AsyncOSTaskCreate;
    use core::ffi::c_void;
    use crate::task1;

    #[test]
    fn task_create_test(){

        OSInit();
        
        AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);

        OSStart();
    }
}

async fn task1(_args: *mut c_void) {
    task_log!(info, "hello from task1");
    SyncOSTaskCreate(task2, 0 as *mut c_void, 0 as *mut usize, 9);
}

fn task2(_args: *mut c_void) {
    task_log!(info,"hello from task2");
    embassy_preempt_platform::PLATFORM.shutdown();
}