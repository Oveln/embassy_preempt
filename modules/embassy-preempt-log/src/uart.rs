//! UART Logger for defmt
//!
//! This module provides a UART-based logger for defmt.
//! It uses the existing uart16550 driver to output defmt log messages.

use core::fmt;


/// Log levels, following defmt conventions
pub mod level {
    pub const TRACE: &str = "[TRACE] ";
    pub const DEBUG: &str = "[DEBUG] ";
    pub const INFO:  &str = "[INFO]  ";
    pub const WARN:  &str = "[WARN]  ";
    pub const ERROR: &str = "[ERROR] ";
}
unsafe extern "C" {
    fn __EARLY_PUTSTR(s: &[u8]);
}

pub struct Uart;

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe { __EARLY_PUTSTR(s.as_bytes()); };
        Ok(())
    }
}