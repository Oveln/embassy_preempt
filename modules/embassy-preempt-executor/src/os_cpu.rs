//! about the cpu

use core::mem;
use core::ptr::NonNull;

use embassy_preempt_cfg::ucosii::OSCtxSwCtr;
use embassy_preempt_mem::heap::{get_interrupt_stack, get_program_stack};
use embassy_preempt_traits::platform::PlatformStatic;
use embassy_preempt_platform::arch::{ TrapFrame, CONTEXT_STACK_SIZE};

use crate::GlobalSyncExecutor;

/// finish the init part of the CPU/MCU
pub fn OSInitHookBegin() {}

/// 上下文切换处理器
///
/// # 参数
/// - trap_frame: 当前 TrapFrame 指针
///
/// # 返回
/// 新任务的 TrapFrame 指针
///
/// # 注意
/// - 寄存器的保存/恢复由汇编入口完成
/// - 此函数只负责调度逻辑和返回新任务的 TrapFrame
#[unsafe(no_mangle)]
extern "C" fn __ContextSwitchHandler(trap_frame: *mut TrapFrame) -> *mut TrapFrame {
    unsafe { embassy_preempt_platform::chip::gpio::gpio_controller().toggle(39); }
    os_log!(info, "ContextSwitch");

    let global_executor = GlobalSyncExecutor().as_ref().unwrap();
    let prio_cur = global_executor.OSPrioCur.get_unmut();
    let prio_highrdy = global_executor.OSPrioHighRdy.get_unmut();

    // 如果优先级相同，则不用切换栈，直接返回即可
    if prio_highrdy == prio_cur {
        return trap_frame;
    }

    #[cfg(feature = "OS_TASK_PROFILE_EN")]
    {
        // add the task's context switch counter
        unsafe {
            global_executor.OSTCBHighRdy.get().OSTCBCtxSwCtr.add(1);
        }
    }

    // add global context switch counter
    OSCtxSwCtr.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    // 获取新任务的栈
    let stk_ptr: embassy_preempt_mem::heap::OS_STK_REF = global_executor.OSTCBHighRdy.get_mut().take_stk();
    let stk_heap_ref = stk_ptr.HEAP_REF;
    let program_stk_ptr = stk_ptr.STK_REF.as_ptr();
    let mut old_stk = get_program_stack().swap(stk_ptr);

    let tcb_cur = global_executor.OSTCBCur.get_mut();

    // see if it is a thread
    if *tcb_cur.needs_stack_save.get_unmut() {
        old_stk.STK_REF = NonNull::new(trap_frame as *mut usize).unwrap();
        tcb_cur.set_stk(old_stk);
    } else if old_stk.HEAP_REF != stk_heap_ref {
        drop(old_stk);
    } else {
        mem::forget(old_stk);
    }

    unsafe {
        global_executor.set_cur_highrdy();
        tcb_cur.needs_stack_save.set(false);
    }

    unsafe { embassy_preempt_platform::chip::gpio::gpio_controller().toggle(40); }
    program_stk_ptr as *mut TrapFrame
}

/// the function to mock/init the stack of the task
/// set the pc to the executor's poll function
pub fn OSTaskStkInit(stk_ref: NonNull<usize>) -> NonNull<usize> {
    scheduler_log!(trace, "OSTaskStkInit");
    let executor_function: fn() = || unsafe {
        scheduler_log!(info, "entering the executor function");
        let global_executor = GlobalSyncExecutor().as_ref().unwrap();
        let task = global_executor.OSTCBHighRdy.get_mut().clone();
        global_executor.single_poll(task);
        global_executor.poll();
    };

    scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function as *const () as usize);

    let stk_top = stk_ref.as_ptr() as usize;
    let aligned_top = (stk_top + 1) & !0b111;
    let frame_addr = aligned_top - CONTEXT_STACK_SIZE;

    let trap_frame = frame_addr as *mut TrapFrame;

    unsafe {
        (*trap_frame).init(executor_function);
    }

    NonNull::new(frame_addr as *mut usize).unwrap()
}
