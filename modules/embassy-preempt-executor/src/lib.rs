#![no_std]
#![feature(never_type)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

//! Raw task storage and pool.

pub extern crate alloc;

pub mod state_atomics;
/// The executor for the uC/OS-II RTOS.
pub mod timer_queue;
pub mod waker;
pub mod task;
pub mod os_task;
pub mod os_core;
pub mod os_time;
pub mod os_cpu;

#[macro_use]
extern crate embassy_preempt_log;
use core::alloc::Layout;
// use core::future::Future;
// use core::mem;
// use core::ops::{Deref, DerefMut};
// use core::pin::Pin;
use core::ptr::NonNull;
// use core::task::{Context, Poll};

// Import logging macros when logging is enabled
use core::sync::atomic::Ordering;
use embassy_preempt_platform::{OsStk, PLATFORM, Platform, traits::timer::{AlarmHandle, Driver}};
use spin::Once;
use state_atomics::State;
use embassy_preempt_platform::timer_driver::RTC_DRIVER;
use task::{OS_TCB, OS_TCB_REF};
pub use os_task::*;
pub use os_core::*;

pub use self::waker::task_from_waker;
// use arena::ARENA;
use embassy_preempt_cfg::*;
use embassy_preempt_mem::heap::{alloc_stack, OS_STK_REF, get_program_stack, TASK_STACK_SIZE};

use crate::os_cpu::OSTaskStkInit;
#[cfg(feature = "delay_idle")]
use crate::os_time::blockdelay::delay;
// use crate::ucosii::*;
use embassy_preempt_cfg::ucosii::*;
use embassy_preempt_structs::cell::SyncUnsafeCell;

/*
****************************************************************************************************************************************
*                                                             global variables
****************************************************************************************************************************************
*/
// create a global executor
static GLOBAL_EXECUTOR: Once<Option<SyncExecutor>> = Once::new();

/// the global executor will be initialized at os init
pub fn get_global_executor() -> &'static Option<SyncExecutor> {
    GLOBAL_EXECUTOR.call_once(|| Some(SyncExecutor::new()))
}

/// Legacy compatibility function that maintains the same API as lazy_static
pub fn GlobalSyncExecutor() -> &'static Option<SyncExecutor> {
    get_global_executor()
}
/*
****************************************************************************************************************************************
*                                                             type define
****************************************************************************************************************************************
*/


/// The executor for the uC/OS-II RTOS.
pub struct SyncExecutor {
    // run_queue: RunQueue,
    // the prio tbl stores a relation between the prio and the task_ref
    os_prio_tbl: SyncUnsafeCell<[OS_TCB_REF; (OS_LOWEST_PRIO + 1) as usize]>,
    // indicate the current running task
    pub OSPrioCur: SyncUnsafeCell<OS_PRIO>,
    pub OSTCBCur: SyncUnsafeCell<OS_TCB_REF>,
    // highest priority task in the ready queue
    pub OSPrioHighRdy: SyncUnsafeCell<OS_PRIO>,
    pub OSTCBHighRdy: SyncUnsafeCell<OS_TCB_REF>,
    // by liam: add a bitmap to record the status of the task
    #[cfg(feature = "OS_PRIO_LESS_THAN_64")]
    OSRdyGrp: SyncUnsafeCell<u8>,
    #[cfg(feature = "OS_PRIO_LESS_THAN_64")]
    OSRdyTbl: SyncUnsafeCell<[u8; OS_RDY_TBL_SIZE]>,
    #[cfg(feature = "OS_PRIO_LESS_THAN_256")]
    OSRdyGrp: u16,
    #[cfg(feature = "OS_PRIO_LESS_THAN_256")]
    OSRdyTbl: [u16; OS_RDY_TBL_SIZE],
    pub(crate) timer_queue: timer_queue::TimerQueue,
    pub(crate) alarm: AlarmHandle,
}

impl SyncExecutor {
    pub(crate) fn new() -> Self {
        let alarm = unsafe { RTC_DRIVER.allocate_alarm().unwrap() };
        Self {
            os_prio_tbl: SyncUnsafeCell::new([OS_TCB_REF::default(); (OS_LOWEST_PRIO + 1) as usize]),

            OSPrioCur: SyncUnsafeCell::new(OS_TASK_IDLE_PRIO),
            OSTCBCur: SyncUnsafeCell::new(OS_TCB_REF::default()),

            OSPrioHighRdy: SyncUnsafeCell::new(OS_TASK_IDLE_PRIO),
            OSTCBHighRdy: SyncUnsafeCell::new(OS_TCB_REF::default()),

            OSRdyGrp: SyncUnsafeCell::new(0),
            OSRdyTbl: SyncUnsafeCell::new([0; OS_RDY_TBL_SIZE]),
            timer_queue: timer_queue::TimerQueue::new(),
            alarm,
        }
    }

    #[allow(dead_code)]
    /// set the current to be highrdy
    pub unsafe fn set_cur_highrdy(&self) { unsafe {
        scheduler_log!(trace, "set_cur_highrdy");
        self.OSPrioCur.set(self.OSPrioHighRdy.get());
        self.OSTCBCur.set(self.OSTCBHighRdy.get());
    }}

    /// Enqueue a task in the task queue
    #[inline(always)]
    pub unsafe fn enqueue(&self, task: OS_TCB_REF) {
        // according to the priority of the task, we place the task in the right place of os_prio_tbl
        // also we will set the corresponding bit in the OSRdyTbl and OSRdyGrp
        let prio = task.OSTCBPrio as usize;
        let tmp = self.OSRdyGrp.get_mut();
        *tmp = *tmp | task.OSTCBBitY;
        let tmp = self.OSRdyTbl.get_mut();
        tmp[task.OSTCBY as usize] |= task.OSTCBBitX;
        // set the task in the right place of os_prio_tbl
        let tmp = self.os_prio_tbl.get_mut();
        tmp[prio] = task;
    }

    pub unsafe fn set_highrdy(&self) { unsafe {
        task_log!(trace, "set_highrdy");
        let tmp = self.OSRdyGrp.get_unmut();
        // if there is no task in the ready queue, return None also set the current running task to the lowest priority
        if *tmp == 0 {
            self.OSPrioHighRdy.set(OS_TASK_IDLE_PRIO);
            self.OSTCBHighRdy
                .set(self.os_prio_tbl.get_unmut()[OS_TASK_IDLE_PRIO as usize]);
            return;
        }
        let prio = tmp.trailing_zeros() as usize;
        let tmp = self.OSRdyTbl.get_unmut();
        let prio = prio * 8 + tmp[prio].trailing_zeros() as usize;
        // set the current running task
        self.OSPrioHighRdy.set(prio as OS_PRIO);
        self.OSTCBHighRdy.set(self.os_prio_tbl.get_unmut()[prio]);
    }}
    pub(crate) unsafe fn set_highrdy_with_prio(&self, prio: OS_PRIO) { unsafe {
        // set the current running task
        self.OSPrioHighRdy.set(prio as OS_PRIO);
        self.OSTCBHighRdy.set(self.os_prio_tbl.get_unmut()[prio as usize]);
    }}
    pub(crate) fn find_highrdy_prio(&self) -> OS_PRIO {
        scheduler_log!(trace, "find_highrdy_prio");
        let tmp = self.OSRdyGrp.get_unmut();
        if *tmp == 0 {
            return OS_TASK_IDLE_PRIO;
        }
        let prio = tmp.trailing_zeros() as usize;
        let tmp = self.OSRdyTbl.get_unmut();
        let prio = prio * 8 + tmp[prio].trailing_zeros() as usize;
        prio as OS_PRIO
    }
    pub unsafe fn set_task_unready(&self, task: OS_TCB_REF) {
        scheduler_log!(trace, "set_task_unready");
        // added by liam: we have to make this process in critical section
        // because the bitmap is shared by all the tasks
        critical_section::with(|_| {
            let tmp = self.OSRdyTbl.get_mut();
            tmp[task.OSTCBY as usize] &= !task.OSTCBBitX;
            // when the group is empty, we need to set the corresponding bit in the OSRdyGrp to 0
            if tmp[task.OSTCBY as usize] == 0 {
                let tmp = self.OSRdyGrp.get_mut();
                *tmp &= !task.OSTCBBitY;
            }
        });
    }
    // check if an prio is exiting
    pub fn prio_exist(&self, prio: OS_PRIO) -> bool {
        let prio_tbl: &[OS_TCB_REF; (OS_LOWEST_PRIO + 1) as usize];
        prio_tbl = self.os_prio_tbl.get_unmut();
        prio_tbl[prio as usize].ptr.is_some()
    }
    // to take up space in the bitmap
    pub fn reserve_bit(&self, prio: OS_PRIO) {
        let prio_tbl: &mut [OS_TCB_REF; (OS_LOWEST_PRIO + 1) as usize];
        prio_tbl = self.os_prio_tbl.get_mut();
        // use the dangling pointer(Some) to reserve the bit
        prio_tbl[prio as usize].ptr = Some(NonNull::dangling());
    }
    pub fn clear_bit(&self, prio: OS_PRIO) {
        let prio_tbl: &mut [OS_TCB_REF; (OS_LOWEST_PRIO + 1) as usize];
        prio_tbl = self.os_prio_tbl.get_mut();
        // use the dangling pointer(Some) to reserve the bit
        prio_tbl[prio as usize].ptr = None;
    }

    // by noah:TEST print the ready queue
    #[allow(dead_code)]
    pub fn print_ready_queue(&self) {
        let tmp: [u8; OS_RDY_TBL_SIZE];
        unsafe {
            tmp = self.OSRdyTbl.get();
        }
        {
            scheduler_log!(trace, "the ready queue is:");
            for i in 0..OS_LOWEST_PRIO + 1 {
                if tmp[(i / 8) as usize] & (1 << (i % 8)) != 0 {
                    scheduler_log!(trace, "the {}th task is ready", i);
                }
            }
        }
    }
    
    #[cfg(feature = "OS_TASK_NAME_EN")]
    /// set task's name
    pub fn set_name(&self, prio: OS_PRIO, name: alloc::string::String) {
        let prio_tbl = self.os_prio_tbl.get_mut();
        prio_tbl[prio as usize].OSTCBTaskName = name;
    }

    // #[cfg(feature = "OS_TASK_PROFILE_EN")]
    // /// add the task's context switch counter
    // pub fn add_ctx_sw_ctr(&self) {
    //     unsafe {
    //         self.OSTCBHighRdy.get().OSTCBCtxSwCtr += 1;
    //     }
    // }

    #[allow(dead_code)]
    pub fn get_prio_tbl(&self) -> &[OS_TCB_REF; (OS_LOWEST_PRIO + 1) as usize] {
        self.os_prio_tbl.get_unmut()
    }
}

impl SyncExecutor {
    fn alarm_callback(ctx: *mut ()) {
        scheduler_log!(trace, "alarm_callback");
        let this: &Self = unsafe { &*(ctx as *const Self) };
        // first to dequeue all the expired task, note that there must
        // have a task in the tiemr_queue because the alarm is triggered
        loop {
            unsafe { this.timer_queue.dequeue_expired(RTC_DRIVER.now(), wake_task_no_pend) };
            // then we need to set a new alarm according to the next expiration time
            let next_expire = unsafe { this.timer_queue.next_expiration() };
            // by noah：we also need to updater the set_time of the timer_queue
            unsafe {
                this.timer_queue.set_time.set(next_expire);
            }
            if RTC_DRIVER.set_alarm(this.alarm, next_expire) {
                break;
            }
        }
        // call Interrupt Context Switch
        unsafe { this.IntCtxSW() };
    }
    // as an interface to join the scheduler logic
    pub unsafe fn IntCtxSW(&'static self) {
              scheduler_log!(trace, "IntCtxSW");
        // set the cur task's is_in_thread_poll to false, as it is preempted in the interrupt context
        scheduler_log!(trace, "IntCtxSW");
        if critical_section::with(|_| unsafe {
            let new_prio = self.find_highrdy_prio();
            task_log!(trace, 
                " the new_prio is {}, the highrdy task's prio is {}, the cur task's prio is {}",
                new_prio,
                self.OSPrioHighRdy.get_unmut(),
                self.OSTCBCur.get_unmut().OSTCBPrio
            );
            if new_prio >= self.OSPrioCur.get() {
                task_log!(trace, "no need to switch task");
                false
            } else {
                // If the new task has a higher priority than the current task 
                // and is not on interrupt as well as does not have a scheduling lock, we need to switch the task
                if OSIntNesting.load(Ordering::Acquire) == 0{
                    if OSLockNesting.load(Ordering::Acquire) == 0{
                        task_log!(trace, "need to switch task");
                        self.set_highrdy_with_prio(new_prio);

                        return true;
                    }
                }
                false
            }
        }) 
        {
            unsafe { self.interrupt_poll() }
        }
    }

    /// this function must be called in the interrupt context, and it will trigger pendsv to switch the task
    /// when this function return, the caller interrupt will also return and the pendsv will run.
    pub unsafe fn interrupt_poll(&'static self) { unsafe {
        // test: print the ready queue
        #[cfg(feature = "log-scheduler")]
        critical_section::with(|_| {
            scheduler_log!(trace, "in interrupt_poll");
            self.print_ready_queue();
            scheduler_log!(trace, "the highrdy task's prio is {}", self.OSPrioHighRdy.get_unmut());
        });

        task_log!(trace, "interrupt_poll");
        if *self.OSPrioCur.get_unmut() != OS_TASK_IDLE_PRIO {
            self.OSTCBCur.get().needs_stack_save.set(true);
            // If the current task will be deleted, 
            // setting 'needs_stack_save' to 'false' will destroy the stack in PenSV
            if self.os_prio_tbl.get_unmut()[*self.OSPrioCur.get_unmut() as usize].ptr.is_none() {
                self.OSTCBCur.get().needs_stack_save.set(false);
            }
        }
        let mut task = critical_section::with(|_| self.OSTCBHighRdy.get());

        // then we need to restore the highest priority task
        task_log!(trace, "interrupt poll :the highrdy task's prio is {}", task.OSTCBPrio);
        task_log!(trace, "interrupt poll :the cur task's prio is {}", self.OSPrioCur.get_unmut());
        scheduler_log!(trace, "the current task is {}", *self.OSPrioCur.get_unmut());
        // task_log!(info, "alloc stack for the task {}", *self.OSPrioHighRdy.get_unmut());
        if task.OSTCBStkPtr.is_none() {
            mem_log!(trace, "the task's stk is none");
            // if the task has no stack, it's a task, we need to mock a stack for it.
            // we need to alloc a stack for the task

            // by noah: *TEST*. Maybe when alloc_stack is called, we need the cs
            let mut stk: OS_STK_REF;
            if *self.OSPrioCur.get_unmut() == OS_TASK_IDLE_PRIO {
                        task_log!(info, "the current task is idle");             
                // if is idle, we don't need to alloc stack,just use the idle stack
                // by yck: but this branch will not be executed
                let mut program_stk = get_program_stack().exclusive_access();
                program_stk.STK_REF = NonNull::new(
                    program_stk.HEAP_REF.as_ptr().offset(program_stk.layout.size() as isize) as *mut OsStk,
                )
                .unwrap();
                stk = program_stk.clone();
            } else {
                  {
                    // scheduler_log!(trace, "the current task is {}", *self.OSPrioCur.get_unmut());
                    mem_log!(trace, "alloc stack for the prio {} task", *self.OSPrioHighRdy.get_unmut());
                }
                let layout = Layout::from_size_align(TASK_STACK_SIZE, 4).unwrap();
                stk = alloc_stack(layout);
                {
                    mem_log!(trace, "the bottom of the allocated stk is {:?}", stk.STK_REF);
                }
            }
            // then we need to mock the stack for the task(the stk will change during the mock)
            stk.STK_REF = OSTaskStkInit(stk.STK_REF);

            task.OSTCBStkPtr = Some(stk);
        } else {
              {
                mem_log!(trace, "the highrdy task {} have a stack {}", *self.OSPrioHighRdy.get_unmut(), task.OSTCBStkPtr.as_ref().unwrap().STK_REF);
            }
        }
        // restore the task from stk
        critical_section::with(|_| {
            if task.OSTCBPrio == *self.OSPrioHighRdy.get_unmut() {
                scheduler_log!(trace, "restore the task/thread");
                PLATFORM().trigger_context_switch();
            }
        });
    }}

    /// since when it was called, there is no task running, we need poll all the task that is ready in bitmap
    pub unsafe fn poll(&'static self) -> ! { unsafe {
        task_log!(trace, "poll");
        RTC_DRIVER.set_alarm_callback(self.alarm, Self::alarm_callback, self as *const _ as *mut ());
        // build this as a loop
        loop {
            // test: print the ready queue
            #[cfg(feature = "log-scheduler")]
            critical_section::with(|_| {
                scheduler_log!(info, "in poll");
                self.print_ready_queue();
                scheduler_log!(info, "the highrdy task's prio is {}", self.OSPrioHighRdy.get_unmut());
            });
            // if the highrdy task is the idle task, we need to delay some time
            #[cfg(feature = "delay_idle")]
            if critical_section::with(|_| *self.OSPrioHighRdy.get_unmut() == OS_TASK_IDLE_PRIO) {
                scheduler_log!(trace, "begin delay the idle task");
                delay(block_delay_poll);
                scheduler_log!(trace, "end delay the idle task");
            }
            // in the executor's thread poll, the highrdy task must be polled, there we don't set cur to be highrdy
            let task = critical_section::with(|_| {
                let mut task = self.OSTCBHighRdy.get();
                if task.OSTCBStkPtr.is_none() {
                    self.OSPrioCur.set(task.OSTCBPrio);
                    self.OSTCBCur.set(task);
                } else {
                    // if the task has stack, it's a thread, we need to resume it not poll it
                    task_log!(trace, "resume the task");
                    task_log!(trace, "the highrdy task's prio is {}", task.OSTCBPrio);

                    task.restore_context_from_stk();
                    return None;
                }
                Some(task)
            });
            if task.is_none() {
                continue;
            }
            let task = task.unwrap();
            // execute the task depending on if it has stack
            self.single_poll(task);
        }
    }}

    pub unsafe fn single_poll(&'static self,mut task: OS_TCB_REF) { unsafe {
        task_log!(trace, "single_poll");
        task.OS_POLL_FN.get().unwrap_unchecked()(task);
            // by noah：Remove tasks from the ready queue in advance to facilitate subsequent unified operations
            // update timer
            // by yck: but the following part will not be executed, because OS_POLL_FN will execute task's 'poll', 
            // which in turn will go to the task body, and will not return here
            critical_section::with(|_| {
                task.needs_stack_save.set(false);
                self.timer_queue.dequeue_expired(RTC_DRIVER.now(), wake_task_no_pend);
                self.set_task_unready(task);
                // set the task's stack to None
                // check: this seems no need to set it to None as it will always be None
                task.OSTCBStkPtr = None;
                let mut next_expire = self.timer_queue.update(task);
                if next_expire < *self.timer_queue.set_time.get_unmut() {
                    self.timer_queue.set_time.set(next_expire);
                    // by noah：if the set alarm return false, it means the expire arrived.
                    // So we can not set the **task which is waiting for the next_expire** as unready
                    // The **task which is waiting for the next_expire** must be current task
                    // we must do this until we set the alarm successfully or there is no alarm required
                    while !RTC_DRIVER.set_alarm(self.alarm, next_expire) {
                        task_log!(trace, "the set alarm return false");
                        // by noah: if set alarm failed, it means the expire arrived, so we should not set the task unready
                        // we should **dequeue the task** from time_queue, **clear the set_time of the time_queue** and continue the loop
                        // (just like the operation in alarm_callback)
                        self.timer_queue.dequeue_expired(RTC_DRIVER.now(), wake_task_no_pend);
                        // then we need to set a new alarm according to the next expiration time
                        next_expire = self.timer_queue.next_expiration();
                        // by noah：we also need to updater the set_time of the timer_queue
                        self.timer_queue.set_time.set(next_expire);
                    }
                }
                // by noah：maybe we can set the task unready, and call dequeue when set_alarm return false
                // find the highest priority task in the ready queue
                // adapt the method above
                self.set_highrdy()
            });
    }}
    
}

/*
****************************************************************************************************************************************
*                                                           function define
****************************************************************************************************************************************
*/

/// Wake a task by `TaskRef`.
///
/// You can obtain a `TaskRef` from a `Waker` using [`task_from_waker`].
pub fn wake_task(task: OS_TCB_REF) {
    task_log!(trace, "wake_task");
    let header = task.header();
    if header.OSTCBStat.run_enqueue() {
        // We have just marked the task as scheduled, so enqueue it.
        unsafe {
            let executor = GlobalSyncExecutor().as_ref().unwrap_unchecked();
            executor.enqueue(task);
        }
    }
}

/// Wake a task by `TaskRef`.
pub fn wake_task_no_pend(task: OS_TCB_REF) {
    task_log!(trace, "wake_task_no_pend");
    // We have just marked the task as scheduled, so enqueue it.
    unsafe {
        let executor = GlobalSyncExecutor().as_ref().unwrap();
        executor.enqueue(task);
    }
}
