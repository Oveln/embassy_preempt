//! about the cpu

use core::arch::asm;
use core::mem;
use core::ptr::NonNull;

use cortex_m_rt::exception;
use embassy_preempt_platform;

use super::OS_STK;
use crate::app::led::{stack_pin_high, stack_pin_low};
use crate::executor::GlobalSyncExecutor;
use crate::heap::stack_allocator::{INTERRUPT_STACK, PROGRAM_STACK};

// 导入日志宏
use crate::{os_log, scheduler_log};
// use crate::ucosii::OS_TASK_IDLE_PRIO;

// use crate::ucosii::OSIdleCtr;
// use core::sync::atomic::Ordering::Relaxed;

// use crate::heap::init_heap;

/// finish the init part of the CPU/MCU
pub fn OSInitHookBegin() {}

// the pendsv handler used to switch the task
#[exception]
fn PendSV() {
    stack_pin_high();
    // first close the interrupt
    unsafe {
        asm!(
            "CPSID I",
            "MRS     R0, PSP",
            // save the context
            "STMFD   R0!, {{R4-R11, R14}}",
            // fix: we need to write back to the PSP
            "MSR     PSP, R0",
            // "CPSIE   I",
            options(nostack, preserves_flags)
        );
    }
    os_log!(info, "PendSV");
    // then switch the task
    let global_executor = GlobalSyncExecutor.as_ref().unwrap();
    let prio_cur = global_executor.OSPrioCur.get_unmut();
    let prio_highrdy = global_executor.OSPrioHighRdy.get_unmut();
    if prio_highrdy == prio_cur {
        // we will reset the msp to the original
        let msp_stk = INTERRUPT_STACK.get().STK_REF.as_ptr();
        unsafe {
            asm!(
                // "CPSID I",
                "MRS    R0, PSP",
                "LDMFD   R0!, {{R4-R11, R14}}",
                "MSR     PSP, R0",
                // reset the msp
                "MSR     MSP, R1",
                "CPSIE   I",
                "BX      LR",
                in("r1") msp_stk,
                options(nostack, preserves_flags),
            )
        }
    }
    let stk_ptr: crate::heap::stack_allocator::OS_STK_REF = global_executor.OSTCBHighRdy.get_mut().take_stk();
    let stk_heap_ref = stk_ptr.HEAP_REF;
    let program_stk_ptr = stk_ptr.STK_REF.as_ptr();
    // the swap will return the ownership of PROGRAM_STACK's original value and set the new value(check it when debuging!!!)
    let mut old_stk = PROGRAM_STACK.swap(stk_ptr);
    let tcb_cur = global_executor.OSTCBCur.get_mut();
    // by noah: *TEST*
    // let TCB: &OS_TCB;
    if !*tcb_cur.is_in_thread_poll.get_unmut() {
        // this situation is in interrupt poll
        scheduler_log!(trace, "need to save the context");
        // we need to give the current task the old_stk to store the context
        // first we will store the remaining context to the old_stk
        let old_stk_ptr: *mut usize;
        unsafe {
            asm!(
                "MRS     R0, PSP",
                out("r0") old_stk_ptr,
                options(nostack, preserves_flags),
            )
        }
        // then as we have stored the context, we need to update the old_stk's top
        old_stk.STK_REF = NonNull::new(old_stk_ptr as *mut OS_STK).unwrap();
        scheduler_log!(info, "in pendsv, the old stk ptr is {:?}", old_stk_ptr);
        tcb_cur.set_stk(old_stk);
    } else if old_stk.HEAP_REF != stk_heap_ref {
        // the situation is in poll
        drop(old_stk);
    } else {
        mem::forget(old_stk);
    }
    // set the current task to be the highrdy
    unsafe {
        global_executor.set_cur_highrdy();
        // set the current task's is_in_thread_poll to true
        tcb_cur.is_in_thread_poll.set(true);
    }
    scheduler_log!(info, "trying to restore, the new stack pointer is {:?}", program_stk_ptr);
    // we will reset the msp to the original
    let msp_stk = INTERRUPT_STACK.get().STK_REF.as_ptr();
    stack_pin_low();
    unsafe {
        asm!(
            // "CPSID I",
            "LDMFD   R0!, {{R4-R11, R14}}",
            "MSR     PSP, R0",
            // reset the msp
            "MSR     MSP, R1",
            "CPSIE   I",
            "BX      LR",
            in("r0") program_stk_ptr,
            in("r1") msp_stk,
            options(nostack, preserves_flags),
        )
    }
}

/// the context structure store in stack
#[repr(C, align(4))]
struct UcStk {
    // below are the remaining part of the task's context
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    r14: u32,
    // below are stored when the interrupt occurs
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
}