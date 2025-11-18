#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

//! # 优先级调度测试 (Priority Scheduling Test)
//!
//! 这个测试验证 embassy-preempt 操作系统的优先级调度器是否正确工作。
//!
//! ## 测试原理
//!
//! 测试创建多个具有不同优先级的任务，验证它们按照正确的优先级顺序执行。
//! 在这个系统中，**较低的数字表示较高的优先级**（priority 10 > priority 20）。
//!
//! ## 任务优先级和执行顺序
//!
//! | 任务 | 优先级 | 描述 | 创建时机 |
//! |------|--------|------|----------|
//! | task5 | 10 | 最高优先级任务 | 初始创建 |
//! | task1_from_task5 | 11 | task5中创建的子任务 | task5运行时 |
//! | task4 | 15 | 中等优先级任务 | 初始创建 |
//! | task3 | 20 | 异步任务 | 初始创建 |
//! | task2 | 25 | 同步任务 | 初始创建 |
//! | task1 | 30 | 低优先级任务 | 初始创建 |
//! | task1_from_task4 | 34 | task4中创建的子任务 | task4运行时 |
//! | task6 | 35 | 较低优先级任务 | 初始创建 |
//! | task7 | 62 | **验证任务**（最低优先级） | 初始创建 |
//!
//! ## 预期执行顺序
//!
//! 1. `task5_begin` - task5开始（优先级10）
//! 2. `task5_created_task1` - task5创建子任务
//! 3. `task5_end` - task5结束
//! 4. `task1_from_task5_begin` - task5创建的子任务开始（优先级11）
//! 5. `task1_from_task5_end` - 子任务结束
//! 6. `task4_begin` - task4开始（优先级15）
//! 7. `task4_end` - task4结束
//! 8. `task3_begin` - 异步task3开始（优先级20）
//! 9. `task3_end` - task3结束
//! 10. `task2_begin` - task2开始（优先级25）
//! 11. `task2_end` - task2结束
//! 12. `task1_begin` - 原始task1开始（优先级30）
//! 13. `task1_end` - task1结束
//! 14. `task1_from_task4_begin` - task4创建的子任务开始（优先级34）
//! 15. `task1_from_task4_end` - 子任务结束
//! 16. `task6_begin` - task6开始（优先级35）
//! 17. `task6_end` - task6结束
//! 18. **验证阶段** - task7执行所有验证逻辑
//!
//! ## 验证内容
//!
//! 1. **精确顺序验证**：每个关键执行点都在正确的位置
//! 2. **完整步骤验证**：总共应该有17个执行步骤
//! 3. **优先级调度验证**：高优先级任务抢占低优先级任务
//! 4. **动态创建验证**：运行时创建的任务被正确调度
use core::any::type_name;
use core::ffi::c_void;

use critical_section::Mutex;
use embassy_preempt_log::task_log;
use embassy_preempt_executor as _; // memory layout + panic handler
use embassy_preempt_executor::{OSInit, OSStart};
use embassy_preempt_executor::{AsyncOSTaskCreate, SyncOSTaskCreate};
use embassy_preempt_executor::os_time::blockdelay::delay;
use embassy_preempt_executor::os_time::timer::Timer;
use embassy_preempt_platform::OsStk;
use embassy_preempt_platform::Platform;
use defmt::assert;

const LONG_TIME: usize = 10;
const MID_TIME: usize = 5;
const SHORT_TIME: usize = 3;

static EXECUTION_ORDER: Mutex<[&'static str; 20]> = Mutex::new([""; 20]);
static mut ORDER_INDEX: usize = 0;

// 记录执行顺序的宏
macro_rules! record_execution {
    ($task_name:expr) => {
        unsafe {
            critical_section::with(|cs| {
                let order = EXECUTION_ORDER.borrow(cs);
                let index = ORDER_INDEX;
                if index < 20 {
                    // 使用可变引用来修改数组
                    let order_mut = order as *const [&'static str; 20] as *mut [&'static str; 20];
                    (*order_mut)[index] = $task_name;
                    ORDER_INDEX += 1;
                    task_log!(info, "Execution order[{}]: {}", index, $task_name);
                }
            })
        }
    };
}

// See https://crates.io/crates/defmt-test/0.3.0 for more documentation (e.g. about the 'state'
// feature)
#[defmt_test::tests]
mod tests {
    use embassy_preempt_log::task_log;
    use embassy_preempt_executor::{OSInit, OSStart, AsyncOSTaskCreate, SyncOSTaskCreate};
    use embassy_preempt_executor::os_time::blockdelay::delay;
    use embassy_preempt_executor::os_time::timer::Timer;
    use core::ffi::c_void;
    use core::any::type_name;
    use embassy_preempt_platform::OsStk;
    use defmt::assert;
    use crate::*;

    const LONG_TIME: usize = 10;
    const MID_TIME: usize = 5;
    const SHORT_TIME: usize = 3;

    #[init]
    fn init() -> () {
        task_log!(info, "Initializing priority scheduling test");
        // 重置执行顺序记录
        unsafe {
            ORDER_INDEX = 0;
            critical_section::with(|cs| {
                let order = EXECUTION_ORDER.borrow(cs);
                // 使用可变引用来修改数组
                let order_mut = order as *const [&'static str; 20] as *mut [&'static str; 20];
                for i in 0..20 {
                    (*order_mut)[i] = "";
                }
            });
        }
    }

    #[test]
    fn test_priority_scheduling() {
        task_log!(info, "Starting priority scheduling test");
        task_log!(info, "Stack type: {}", type_name::<OsStk>());

        // os初始化
        OSInit();

        // 创建6个任务，测试优先级调度的顺序是否正确
        // 调度顺序应该为：task5->task1(task5中创建)->task4->task3->task2->task1->task1(在task4中创建)->task6(由于优先级相同输出相关信息)
        SyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 30);
        SyncOSTaskCreate(task2, 0 as *mut c_void, 0 as *mut usize, 25);
        AsyncOSTaskCreate(task3, 0 as *mut c_void, 0 as *mut usize, 20);
        SyncOSTaskCreate(task4, 0 as *mut c_void, 0 as *mut usize, 15);
        SyncOSTaskCreate(task5, 0 as *mut c_void, 0 as *mut usize, 10);
        SyncOSTaskCreate(task6, 0 as *mut c_void, 0 as *mut usize, 35);
        SyncOSTaskCreate(task7, 0 as *mut c_void, 0 as *mut usize, 62);

        // 启动os
        OSStart();
    }
}

fn task7(_args: *mut c_void) {
    unsafe {
        critical_section::with(|cs| {
            let order = EXECUTION_ORDER.borrow(cs);
            let index = ORDER_INDEX;

            task_log!(info, "Total execution steps: {}", index);

            // 记录实际的执行顺序
            for i in 0..index {
                task_log!(info, "Step {}: {}", i, order[i]);
            }

            // 验证关键调度点
            // 1. task5 应该最先执行（优先级 10，最低数字最高优先级）
            assert!(order[0] == "task5_begin", "Expected task5_begin first, got {}", order[0]);

            // 2. task5 创建任务的记录点
            assert!(order[1] == "task5_created_task1", "Expected task5_created_task1 second, got {}", order[1]);

            // 3. task5 结束
            assert!(order[2] == "task5_end", "Expected task5_end third, got {}", order[2]);

            // 4. task5 创建的 task1 (优先级 11) 然后执行
            assert!(order[3] == "task1_from_task5_begin", "Expected task1_from_task5_begin fourth, got {}", order[3]);

            // 5. 然后 task4 (优先级 15)
            assert!(order[5] == "task4_begin", "Expected task4_begin fifth, got {}", order[5]);

            // 6. 然后 task3 (优先级 20)
            assert!(order[7] == "task3_begin", "Expected task3_begin seventh, got {}", order[7]);

            // 7. 然后 task2 (优先级 25)
            assert!(order[9] == "task2_begin", "Expected task2_begin ninth, got {}", order[9]);

            // 8. 然后 task1 (优先级 30)
            assert!(order[11] == "task1_begin", "Expected task1_begin eleventh, got {}", order[11]);

            // 9. task4 中创建的 task1 (优先级 34)
            assert!(order[13] == "task1_from_task4_begin", "Expected task1_from_task4_begin thirteenth, got {}", order[13]);

            // 10. task6 (优先级 35，最高优先级）
            assert!(order[15] == "task6_begin", "Expected task6_begin fifteenth, got {}", order[15]);

            // 验证总执行步骤数应该是17
            assert!(index == 17, "Expected 17 total execution steps, got {}", index);

            task_log!(info, "Priority scheduling order verification PASSED");
        });
        
    }
    embassy_preempt_platform::PLATFORM.shutdown();
}

fn task1(_args: *mut c_void) {
    record_execution!("task1_begin");
    task_log!(info, "---task1 begin---");
    delay(LONG_TIME);
    record_execution!("task1_end");
    task_log!(info, "---task1 end---");
    delay(SHORT_TIME);
}

fn task2(_args: *mut c_void) {
    record_execution!("task2_begin");
    task_log!(info, "---task2 begin---");
    delay(MID_TIME);
    record_execution!("task2_end");
    task_log!(info, "---task2 end---");
    delay(SHORT_TIME);
}

async fn task3(_args: *mut c_void) {
    record_execution!("task3_begin");
    task_log!(info, "---task3 begin---");
    Timer::after_ticks(LONG_TIME as u64).await;
    record_execution!("task3_end");
    task_log!(info, "---task3 end---");
    delay(SHORT_TIME);
}

fn task4(_args: *mut c_void) {
    record_execution!("task4_begin");
    task_log!(info, "---task4 begin---");
    // 任务4中涉及任务创建
    SyncOSTaskCreate(task1_from_task4, 0 as *mut c_void, 0 as *mut usize, 34);
    delay(SHORT_TIME);
    record_execution!("task4_end");
    task_log!(info, "---task4 end---");
    delay(SHORT_TIME);
}

fn task5(_args: *mut c_void) {
    record_execution!("task5_begin");
    task_log!(info, "---task5 begin---");
    let ptos = 0 as *mut usize;
    task_log!(info, "ptos is {:x}", ptos);
    // 任务5中涉及任务创建
    SyncOSTaskCreate(task1_from_task5, 0 as *mut c_void, ptos, 11);
    record_execution!("task5_created_task1");
    task_log!(info, "created task1 in task5");
    delay(SHORT_TIME);
    record_execution!("task5_end");
    task_log!(info, "---task5 end---");
    delay(SHORT_TIME);
}

/* 任务6用于测试优先级相同的情况 */
fn task6(_args: *mut c_void) {
    record_execution!("task6_begin");
    task_log!(info, "---task6 begin---");
    // 任务6中涉及任务创建，新创建的优先级与当前任务相同
    SyncOSTaskCreate(task1_from_task6, 0 as *mut c_void, 0 as *mut usize, 35);
    delay(SHORT_TIME);
    record_execution!("task6_end");
    task_log!(info, "---task6 end---");
    delay(SHORT_TIME);
}

// 不同上下文创建的 task1 变体
fn task1_from_task4(_args: *mut c_void) {
    record_execution!("task1_from_task4_begin");
    task_log!(info, "---task1_from_task4 begin---");
    delay(LONG_TIME);
    record_execution!("task1_from_task4_end");
    task_log!(info, "---task1_from_task4 end---");
    delay(SHORT_TIME);
}

fn task1_from_task5(_args: *mut c_void) {
    record_execution!("task1_from_task5_begin");
    task_log!(info, "---task1_from_task5 begin---");
    delay(LONG_TIME);
    record_execution!("task1_from_task5_end");
    task_log!(info, "---task1_from_task5 end---");
    delay(SHORT_TIME);
}

fn task1_from_task6(_args: *mut c_void) {
    record_execution!("task1_from_task6_begin");
    task_log!(info, "---task1_from_task6 begin---");
    delay(LONG_TIME);
    record_execution!("task1_from_task6_end");
    task_log!(info, "---task1_from_task6 end---");
    delay(SHORT_TIME);
}