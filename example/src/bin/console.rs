#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::arch::asm;
use core::ffi::c_void;

use embassy_preempt_executor::os_time::blockdelay::delay;
use embassy_preempt_executor::{AsyncOSTaskCreate, SyncOSTaskCreate, OSInit, OSStart};
use embassy_preempt_log::task_log;
use embassy_preempt_executor::os_time::timer::Timer;
use critical_section::Mutex;

static EXECUTION_ORDER: Mutex<[&'static str; 20]> = Mutex::new([""; 20]);
static mut ORDER_INDEX: usize = 0;

/// CSR 寄存器读取辅助函数
mod csr {
    /// 读取 mhartid - 当前硬件线程 ID
    #[inline]
    pub unsafe fn mhartid() -> usize {
        let mut hartid: usize;
        unsafe {
            core::arch::asm!("csrr {}, mhartid", out(reg) hartid);
        }
        hartid
    }

    /// 读取 mtvec - 机器模式陷阱向量基地址
    #[inline]
    pub unsafe fn mtvec() -> usize {
        let mut mtvec: usize;
        unsafe {
            core::arch::asm!("csrr {}, mtvec", out(reg) mtvec);
        }
        mtvec
    }

    /// 读取 mstatus - 机器模式状态寄存器
    #[inline]
    pub unsafe fn mstatus() -> usize {
        let mut mstatus: usize;
        unsafe {
            core::arch::asm!("csrr {}, mstatus", out(reg) mstatus);
        }
        mstatus
    }

    /// 读取 mepc - 机器模式异常程序计数器
    #[inline]
    pub unsafe fn mepc() -> usize {
        let mut mepc: usize;
        unsafe {
            core::arch::asm!("csrr {}, mepc", out(reg) mepc);
        }
        mepc
    }

    /// 读取 mie - 机器模式中断使能寄存器
    #[inline]
    pub unsafe fn mie() -> usize {
        let mut mie: usize;
        unsafe {
            core::arch::asm!("csrr {}, mie", out(reg) mie);
        }
        mie
    }

    /// 读取 mip - 机器模式中断挂起寄存器
    #[inline]
    pub unsafe fn mip() -> usize {
        let mut mip: usize;
        unsafe {
            core::arch::asm!("csrr {}, mip", out(reg) mip);
        }
        mip
    }

    /// 读取 mscratch - 机器模式临时寄存器
    #[inline]
    pub unsafe fn mscratch() -> usize {
        let mut mscratch: usize;
        unsafe {
            core::arch::asm!("csrr {}, mscratch", out(reg) mscratch);
        }
        mscratch
    }

    /// 读取 sp - 栈指针
    #[inline]
    pub unsafe fn sp() -> usize {
        let mut sp: usize;
        unsafe {
            core::arch::asm!("mv {}, sp", out(reg) sp);
        }
        sp
    }

    /// 读取 ra - 返回地址
    #[inline]
    pub unsafe fn ra() -> usize {
        let mut ra: usize;
        unsafe {
            core::arch::asm!("mv {}, ra", out(reg) ra);
        }
        ra
    }
}

/// 清除 BSS 段
fn clear_bss() {
    unsafe extern "C" {
        static __sbss: u8;
        static __ebss: u8;
    }
    unsafe {
        core::slice::from_raw_parts_mut(
            &__sbss as *const u8 as *mut u8,
            &__ebss as *const u8 as usize - &__sbss as *const u8 as usize,
        )
        .fill(0);
    }
}

#[embassy_preempt_macros::entry]
fn test_hardware() -> ! {
    // 清除 BSS 段
    clear_bss();

    // ========== 系统信息输出开始 ==========
    task_log!(info, "========================================");
    task_log!(info, "  Embassy Preempt - System Info");
    task_log!(info, "  VisionFive2 JH7110 Platform");
    task_log!(info, "========================================");

    // Hart ID
    let hartid = unsafe { csr::mhartid() };
    task_log!(info, "[Hart Information]");
    task_log!(info, "  mhartid (Hart ID): {:#x} ({})", hartid, hartid);

    // 陷阱向量
    let mtvec = unsafe { csr::mtvec() };
    let mode = mtvec & 0x3;
    let base = mtvec & !0x3;
    let mode_str = match mode {
        0 => "Direct (all traps to BASE)",
        1 => "Vectored (exceptions to BASE, interrupts to BASE+4*cause)",
        _ => "Reserved",
    };
    task_log!(info, "[Trap Vector]");
    task_log!(info, "  mtvec: {:#x}", mtvec);
    task_log!(info, "    Mode: {}", mode_str);
    task_log!(info, "    Base: {:#x}", base);

    // 状态寄存器
    let mstatus = unsafe { csr::mstatus() };
    let mpp = match (mstatus >> 11) & 0x3 {
        0 => "U",
        1 => "S",
        3 => "M",
        _ => "?",
    };
    task_log!(info, "[Machine Status]");
    task_log!(info, "  mstatus: {:#x}", mstatus);
    task_log!(
        info,
        "    MIE: {}, MPIE: {}, MPP: {}",
        if (mstatus >> 3) & 1 == 1 { "1" } else { "0" },
        if (mstatus >> 7) & 1 == 1 { "1" } else { "0" },
        mpp
    );

    // 异常程序计数器
    let mepc = unsafe { csr::mepc() };
    task_log!(info, "[Exception Program Counter]");
    task_log!(info, "  mepc: {:#x}", mepc);

    // 中断使能
    let mie = unsafe { csr::mie() };
    task_log!(info, "[Interrupt Enable]");
    task_log!(info, "  mie: {:#x}", mie);
    task_log!(info, "    MIE bits:");
    task_log!(
        info,
        "      SSIP: {}, MSIP: {}, STIP: {}, MTIP: {}, SEIP: {}, MEIP: {}",
        if (mie >> 1) & 1 == 1 { "1" } else { "0" },
        if (mie >> 3) & 1 == 1 { "1" } else { "0" },
        if (mie >> 5) & 1 == 1 { "1" } else { "0" },
        if (mie >> 7) & 1 == 1 { "1" } else { "0" },
        if (mie >> 9) & 1 == 1 { "1" } else { "0" },
        if (mie >> 11) & 1 == 1 { "1" } else { "0" }
    );

    // 中断挂起
    let mip = unsafe { csr::mip() };
    task_log!(info, "[Interrupt Pending]");
    task_log!(info, "  mip: {:#x}", mip);
    task_log!(info, "    MIP bits:");
    task_log!(
        info,
        "      SSIP: {}, MSIP: {}, STIP: {}, MTIP: {}, SEIP: {}, MEIP: {}",
        if (mip >> 1) & 1 == 1 { "1" } else { "0" },
        if (mip >> 3) & 1 == 1 { "1" } else { "0" },
        if (mip >> 5) & 1 == 1 { "1" } else { "0" },
        if (mip >> 7) & 1 == 1 { "1" } else { "0" },
        if (mip >> 9) & 1 == 1 { "1" } else { "0" },
        if (mip >> 11) & 1 == 1 { "1" } else { "0" }
    );

    // 栈信息
    let mscratch = unsafe { csr::mscratch() };
    let sp = unsafe { csr::sp() };
    task_log!(info, "[Stack Information]");
    task_log!(info, "  mscratch: {:#x}", mscratch);
    task_log!(info, "  sp (stack pointer): {:#x}", sp);

    // 代码地址
    let ra = unsafe { csr::ra() };
    task_log!(info, "[Code Location]");
    task_log!(info, "  ra (return address): {:#x}", ra);

    task_log!(info, "========================================");
    task_log!(info, "  UART Logger Initialized");
    task_log!(info, "========================================");

    // os初始化
    OSInit();

    task_log!(info, "[OS Status]");
    task_log!(info, "  OSInit completed!");
    task_log!(info, "========================================");
    task_log!(info, "  Hello, Embassy Preempt on VisionFive2!");
    task_log!(info, "========================================\r\n");

    task_log!(info, "defmt test");
    let mip = unsafe { csr::mip() };
    task_log!(info, "mip: {:#x}", mip);
    
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

const LONG_TIME: usize = 10;
const MID_TIME: usize = 5;
const SHORT_TIME: usize = 3;
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
    task_log!(info, "ptos is {:p}", ptos);
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