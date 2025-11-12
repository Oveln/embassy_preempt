//! about the cpu

use core::arch::asm;
use core::mem;
use core::ptr::NonNull;

use cortex_m::register::psp;
use cortex_m_rt::exception;
use embassy_preempt_platform::{OsStk, PLATFORM, Platform};

use embassy_preempt_driver::led::{stack_pin_high, stack_pin_low};
use crate::GlobalSyncExecutor;
use embassy_preempt_mem::heap::{INTERRUPT_STACK, PROGRAM_STACK};
use embassy_preempt_cfg::ucosii::OSCtxSwCtr;

/// finish the init part of the CPU/MCU
pub fn OSInitHookBegin() {}

#[unsafe(no_mangle)]
/// the function to start the first task
pub extern "Rust" fn restore_thread_task() {
    PLATFORM.restore_thread_task();
}

// the pendsv handler used to switch the task
#[exception]
fn PendSV() {
    const EXC_RETURN_TO_PSP: u32 = 0xFFFFFFFD;
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
                "BX      R2",
                in("r1") msp_stk,
                in("r2") EXC_RETURN_TO_PSP,
                options(nostack, preserves_flags),
            )
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
    let mut old_stk = PROGRAM_STACK.swap(stk_ptr);
    
    let tcb_cur = global_executor.OSTCBCur.get_mut();

    // see if it is a thread
    if *tcb_cur.needs_stack_save.get_unmut() {
        let old_stk_ptr: *mut usize;
        unsafe {
            asm!(
                "MRS     R0, PSP",
                out("r0") old_stk_ptr,
                options(nostack, preserves_flags),
            )
        }
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
            "BX      R2",
            in("r0") program_stk_ptr,
            in("r1") msp_stk,
            in("r2") EXC_RETURN_TO_PSP,
            options(nostack, preserves_flags),
        )
    }
}

#[unsafe(no_mangle)]
/// the function when there is no task to run
pub extern "Rust" fn run_idle() {
    PLATFORM.run_idle();
}

#[unsafe(no_mangle)]
/// the function to mock/init the stack of the task
/// set the pc to the executor's poll function
pub extern "Rust" fn OSTaskStkInit(stk_ref: NonNull<OsStk>) -> NonNull<OsStk> {
    scheduler_log!(trace, "OSTaskStkInit");
    let executor_function: fn() = || unsafe {
        scheduler_log!(info, "entering the executor function");
        stack_pin_high();
        let global_executor = GlobalSyncExecutor.as_ref().unwrap();
        let task = global_executor.OSTCBHighRdy.get_mut().clone();
        stack_pin_low();
        global_executor.single_poll(task);
        global_executor.poll();
    };
    PLATFORM.init_task_stack(stk_ref, executor_function)
}

#[unsafe(no_mangle)]
/// the function to set the program stack
pub extern "Rust" fn set_program_sp(sp: *mut u8) {
    PLATFORM.set_program_sp(sp);
}