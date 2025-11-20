//! Core platform functionality trait definition

use core::ptr::NonNull;

/// Core platform functionality required by the RTOS
///
/// This trait provides hardware abstraction layer that supports different CPU architectures
/// (ARM Cortex-M, RISC-V, etc.) by defining common operations with architecture-specific implementations.
pub trait Platform {
    type OsStk;

    /// Trigger a context switch to start the first task or switch to next task
    ///
    /// Architecture-specific implementation:
    /// - ARM Cortex-M: Set PendSV flag
    /// - RISC-V: Set software interrupt or use ecall
    fn trigger_context_switch(&'static self);

    /// Set the program/running stack pointer
    ///
    /// Architecture-specific stack pointer:
    /// - ARM Cortex-M: PSP (Process Stack Pointer)
    /// - RISC-V: User stack pointer register
    fn set_program_stack_pointer(&'static self, sp: *mut u8);

    /// Set the interrupt stack and switch from privileged to user mode if applicable
    ///
    /// Architecture-specific behavior:
    /// - ARM Cortex-M: Set MSP and switch to PSP mode
    /// - RISC-V: Set interrupt stack and switch privilege mode
    fn configure_interrupt_stack(&'static self, interrupt_stack: *mut u8);

    /// Initialize task stack with proper context frame
    ///
    /// Creates the initial stack frame for task startup with architecture-specific
    /// register layout and exception return values.
    fn init_task_stack(&'static self, stk_ref: NonNull<Self::OsStk>, executor_function: fn()) -> NonNull<Self::OsStk>;

    /// Execute idle/inactive state (low-power mode)
    ///
    /// Architecture-specific idle behavior:
    /// - ARM Cortex-M: WFE/WFI instructions
    /// - RISC-V: WFI instruction or custom sleep
    fn enter_idle_state(&'static self);

    /// Shutdown the system with optional visual feedback
    ///
    /// Platform-specific shutdown implementation with LED effects or debug output.
    fn shutdown(&'static self);

    // ===== Context Management Functions =====
    // These functions handle low-level context switching and should be marked as
    // #[inline(always)] in implementations for performance-critical operations

    /// Save current task context to stack
    ///
    /// Saves the current CPU context (registers, program counter, etc.) to the task's stack.
    /// This is called during task switching interrupts.
    ///
    /// Architecture-specific context:
    /// - ARM Cortex-M: R4-R11, LR
    /// - RISC-V: Saved registers per ABI specification
    ///
    /// # Safety
    /// Must be called with interrupts disabled and proper stack setup.
    unsafe fn save_task_context(&'static self);

    /// Restore task context from stack and resume execution
    ///
    /// Restores a task's saved context and switches to it.
    /// This is the final step in context switching.
    ///
    /// Parameters:
    /// - `stack_pointer`: Pointer to the task's stack containing saved context
    /// - `interrupt_stack`: System interrupt stack to restore
    /// - `return_value`: Architecture-specific return value (e.g., EXC_RETURN on ARM)
    ///
    /// # Safety
    /// Must be called with valid saved context and proper stack alignment.
    unsafe fn restore_task_context(&'static self, stack_pointer: *mut usize, interrupt_stack: *mut usize, return_value: u32);

    /// Get current task's stack pointer
    ///
    /// Returns the current stack pointer value for the running task.
    /// Used when saving task state during context switches.
    ///
    /// # Safety
    /// Must be called in a context where stack pointer is meaningful.
    unsafe fn get_current_stack_pointer(&'static self) -> *mut usize;

    fn get_timer_driver(&'static self) -> &'static dyn crate::traits::timer::Driver;
}