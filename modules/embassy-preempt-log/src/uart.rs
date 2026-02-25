//! UART Logger for defmt
//!
//! This module provides a UART-based logger for defmt.
//! It uses the existing uart16550 driver to output defmt log messages.

use spin::Once;
use uart16550::{Register, Uart16550};
use core::fmt;

/// Fixed UART base address for JH7110 VisionFive 2
const UART_BASE: usize = 0x10010000;

/// Log levels, following defmt conventions
pub mod level {
    pub const TRACE: &str = "[TRACE] ";
    pub const DEBUG: &str = "[DEBUG] ";
    pub const INFO:  &str = "[INFO]  ";
    pub const WARN:  &str = "[WARN]  ";
    pub const ERROR: &str = "[ERROR] ";
}

/// Newtype wrapper to make raw pointer Send + Sync
struct UartPtr(*const Uart16550<u32>);

unsafe impl Send for UartPtr {}
unsafe impl Sync for UartPtr {}

/// Global UART pointer, initialized with Once
static UART_PTR: Once<UartPtr> = Once::new();

/// Get the UART pointer, initializing it if necessary
fn get_uart() -> *const Uart16550<u32> {
    UART_PTR.call_once(|| UartPtr(UART_BASE as *const Uart16550<u32>)).0
}

pub unsafe fn uart_write_str(s: &str) {
    let uart_ptr = get_uart();
    let mut bytes = s.as_bytes();
    while !bytes.is_empty() {
        let count = (*uart_ptr).write(bytes);
        bytes = &bytes[count..];
    }
}

pub struct Uart;

/// 实现 core::fmt::Write trait
/// 这使得 Uart 类型可以用于 write! 宏
impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // 调用你已有的底层 unsafe 函数
        // 这里的 unsafe 块是必要的，因为操作硬件寄存器是 unsafe 的
        unsafe { uart_write_str(s) };
        Ok(())
    }
}