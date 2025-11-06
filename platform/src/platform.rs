//! Core platform functionality trait definition

use core::ptr::NonNull;

/// Core platform functionality required by the RTOS
pub trait Platform {
    type OS_STK;
    /// Initialize core peripherals (NVIC, SCB, etc.)
    fn init_core_peripherals(&'static self);

    /// Perform context switch to start the first task
    fn restore_thread_task(&'static self);

    /// Set the program stack pointer
    fn set_program_sp(&'static self, sp: *mut u32);

    /// Set the interrupt stack and switch to PSP
    fn set_int_change_2_psp(&'static self, int_ptr: *mut u32);

    /// Initialize the task stack
    fn init_task_stack(&'static self, stk_ref: NonNull<Self::OS_STK>) -> NonNull<Self::OS_STK>;

    /// Run idle task (typically WFE or similar low-power instruction)
    fn run_idle(&'static self);

    /// Enter critical section
    fn enter_critical_section(&'static self);

    /// Exit critical section
    fn exit_critical_section(&'static self);

    /// Get current timestamp in ticks
    fn now(&'static self) -> u64;

    /// Initialize timer driver
    fn init_timer_driver(&'static self);
}