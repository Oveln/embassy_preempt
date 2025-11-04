//! Core platform functionality trait definition

use core::ptr::NonNull;
use crate::{OS_STK, INT64U};

/// Core platform functionality required by the RTOS
pub trait Platform {
    /// Initialize platform-specific peripherals (RCC, clocks, etc.)
    fn init_platform(&self);

    /// Initialize core peripherals (NVIC, SCB, etc.)
    fn init_core_peripherals(&self);

    /// Perform context switch to start the first task
    fn restore_thread_task(&self);

    /// Set the program stack pointer
    fn set_program_sp(&self, sp: *mut u32);

    /// Set the interrupt stack and switch to PSP
    fn set_int_change_2_psp(&self, int_ptr: *mut u32);

    /// Initialize the task stack
    fn init_task_stack(&self, stk_ref: NonNull<OS_STK>) -> NonNull<OS_STK>;

    /// Run idle task (typically WFE or similar low-power instruction)
    fn run_idle(&self);

    /// Enter critical section
    fn enter_critical_section(&self);

    /// Exit critical section
    fn exit_critical_section(&self);

    /// Get current timestamp in ticks
    fn now(&self) -> INT64U;

    /// Schedule a wake-up for the given timestamp
    fn schedule_wake(&self, at: INT64U, waker: &core::task::Waker);

    /// Initialize timer driver
    fn init_timer_driver(&self);

    /// Initialize GPIO/button driver
    fn init_gpio_driver(&self);

    /// Wait for a bottom (button press or similar event)
    fn wait_bottom(&self);
}