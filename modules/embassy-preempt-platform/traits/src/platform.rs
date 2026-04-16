//! Core platform functionality trait definition

use core::ptr::NonNull;

pub trait PlatformStatic {
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

    /// Early putstr for boot-time debugging before RTOS initialization
    ///
    /// This function should output a byte slice to UART without
    /// using any RTOS services. It's called during early boot when
    /// the full logging infrastructure is not yet available.
    ///
    /// # Parameters
    /// - `c`: The byte slice to output
    ///
    /// # Returns
    /// The number of bytes actually written (may be less than `c.len()` if
    /// the UART FIFO is full and cannot accept all data at once)
    fn early_putstr(c: &[u8]) -> usize;

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