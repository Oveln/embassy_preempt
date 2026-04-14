//! Core platform functionality trait definition

use core::ptr::NonNull;

pub trait PlatformStatic {

    /// Trigger a context switch to start the first task or switch to next task
    ///
    /// Architecture-specific implementation:
    /// - ARM Cortex-M: Set PendSV flag
    /// - RISC-V: Set software interrupt or use ecall
    fn trigger_context_switch();

    /// Initialize task stack with proper context frame
    ///
    /// Creates the initial stack frame for task startup with architecture-specific
    /// register layout and exception return values.
    fn init_task_stack(stk_ref: NonNull<usize>, executor_function: fn()) -> NonNull<usize>;

    /// Execute idle/inactive state (low-power mode)
    ///
    /// Architecture-specific idle behavior:
    /// - ARM Cortex-M: WFE/WFI instructions
    /// - RISC-V: WFI instruction or custom sleep
    fn enter_idle_state();

    /// Shutdown the system with optional visual feedback
    ///
    /// Platform-specific shutdown implementation with LED effects or debug output.
    fn shutdown();

}

/// Core platform functionality required by the RTOS
///
/// This trait provides hardware abstraction layer that supports different CPU architectures
/// (ARM Cortex-M, RISC-V, etc.) by defining common operations with architecture-specific implementations.
pub trait Platform {
    /// Get the timer driver for time-based operations
    ///
    /// Returns a reference to the timer driver implementation that provides
    /// timing services for the RTOS.
    fn get_timer_driver(&'static self) -> &'static dyn crate::timer::Driver;

    /// Register a callback to be invoked when an IPI (Inter-Processor Interrupt) occurs
    ///
    /// The platform implementation should call the provided callback from the
    /// machine software interrupt (MSIP) handler.
    ///
    /// # Parameters
    /// - `callback`: Function to call when IPI is received
    /// - `ctx`: Context pointer passed to the callback
    fn set_ipi_callback(&'static self, callback: fn(*mut ()), ctx: *mut ());
}