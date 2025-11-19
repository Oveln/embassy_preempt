//! about the cpu

use core::mem;
use core::ptr::NonNull;

use embassy_preempt_platform::{OsStk, PLATFORM, Platform};

use crate::GlobalSyncExecutor;
use embassy_preempt_mem::heap::{get_program_stack, get_interrupt_stack};
use embassy_preempt_cfg::ucosii::OSCtxSwCtr;

/// finish the init part of the CPU/MCU
pub fn OSInitHookBegin() {}

// the pendsv handler used to switch the task
#[unsafe(no_mangle)]
extern "C" fn PendSV() {
    const EXC_RETURN_TO_PSP: u32 = 0xFFFFFFFD;
    // first close the interrupt and save context
    unsafe {
        PLATFORM().save_task_context();
    }
    os_log!(info, "PendSV");
    // then switch the task
    let global_executor = GlobalSyncExecutor().as_ref().unwrap();
    let prio_cur = global_executor.OSPrioCur.get_unmut();
    let prio_highrdy = global_executor.OSPrioHighRdy.get_unmut();
    if prio_highrdy == prio_cur {
        // we will reset the msp to the original
        let msp_stk = get_interrupt_stack().get().STK_REF.as_ptr();
        let current_psp = unsafe { PLATFORM().get_current_stack_pointer() };
        unsafe {
            PLATFORM().restore_task_context(current_psp, msp_stk, EXC_RETURN_TO_PSP);
        }
    }
    #[cfg(feature = "OS_TASK_PROFILE_EN")]
    {
        // add the task's context switch counter
        unsafe { global_executor.OSTCBHighRdy.get().OSTCBCtxSwCtr.add(1); }
    }

    // add global context switch counter
    OSCtxSwCtr.fetch_add(1, core::sync::atomic::Ordering::SeqCst);

    
    os_log!(info, "OSCtxSwCtr is {}", OSCtxSwCtr.load(core::sync::atomic::Ordering::SeqCst));

    let stk_ptr: embassy_preempt_mem::heap::OS_STK_REF = global_executor.OSTCBHighRdy.get_mut().take_stk();
    let stk_heap_ref = stk_ptr.HEAP_REF;
    let program_stk_ptr = stk_ptr.STK_REF.as_ptr();
    // the swap will return the ownership of PROGRAM_STACK's original value and set the new value(check it when debuging!!!)
    let mut old_stk = get_program_stack().swap(stk_ptr);
    
    let tcb_cur = global_executor.OSTCBCur.get_mut();

    // see if it is a thread
    if *tcb_cur.needs_stack_save.get_unmut() {
        let old_stk_ptr = unsafe { PLATFORM().get_current_stack_pointer() };
        old_stk.STK_REF = NonNull::new(old_stk_ptr as *mut OsStk).unwrap();
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
    let msp_stk = get_interrupt_stack().get().STK_REF.as_ptr();
    unsafe {
        PLATFORM().restore_task_context(program_stk_ptr, msp_stk, EXC_RETURN_TO_PSP);
    }
}

/// the function to mock/init the stack of the task
/// set the pc to the executor's poll function
pub fn OSTaskStkInit(stk_ref: NonNull<OsStk>) -> NonNull<OsStk> {
    scheduler_log!(trace, "OSTaskStkInit");
    let executor_function: fn() = || unsafe {
        scheduler_log!(info, "entering the executor function");
        let global_executor = GlobalSyncExecutor().as_ref().unwrap();
        let task = global_executor.OSTCBHighRdy.get_mut().clone();
        global_executor.single_poll(task);
        global_executor.poll();
    };
    PLATFORM().init_task_stack(stk_ref, executor_function)
}