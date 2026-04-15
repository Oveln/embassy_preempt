//! Logging macros for the Embassy Preempt RTOS
//!
//! This module provides logging macros that wrap around the `defmt` crate.
//! When the "logs" feature is enabled, these macros will output log messages.
//! When the feature is disabled, the macros become no-ops for performance.

#![no_std]

#[cfg(feature = "log-rtt")]
use defmt_rtt as _;

#[cfg(feature = "log-rtt")]
pub use defmt;

#[cfg(feature = "log-uart")]
pub mod uart;

// Define all core logging macros
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::__log!(debug, $($arg)*) };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::__log!(error, $($arg)*) };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::__log!(info, $($arg)*) };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { $crate::__log!(trace, $($arg)*) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::__log!(warn, $($arg)*) };
}

// Forward to defmt (RTT)
#[cfg(all(feature = "log-rtt", not(feature = "log-uart")))]
#[macro_export]
macro_rules! __log {
    ($level:ident, $($arg:tt)*) => {
        {
            use embassy_preempt_log::defmt;
            defmt::$level!($($arg)*)
        }
    };
}

// Forward to UART (using Display2Format + fmt::Arguments)
#[cfg(feature = "log-uart")]
#[macro_export]
macro_rules! __log {
    (trace, $($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = write!(embassy_preempt_log::uart::Uart, "{}", embassy_preempt_log::uart::level::TRACE);
            let _ = writeln!(embassy_preempt_log::uart::Uart, $($arg)*);
        }
    };
    (debug, $($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = write!(embassy_preempt_log::uart::Uart, "{}", embassy_preempt_log::uart::level::DEBUG);
            let _ = writeln!(embassy_preempt_log::uart::Uart, $($arg)*);
        }
    };
    (info, $($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = write!(embassy_preempt_log::uart::Uart, "{}", embassy_preempt_log::uart::level::INFO);
            let _ = writeln!(embassy_preempt_log::uart::Uart, $($arg)*);
        }
    };
    (warn, $($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = write!(embassy_preempt_log::uart::Uart, "{}", embassy_preempt_log::uart::level::WARN);
            let _ = writeln!(embassy_preempt_log::uart::Uart, $($arg)*);
        }
    };
    (error, $($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = write!(embassy_preempt_log::uart::Uart, "{}", embassy_preempt_log::uart::level::ERROR);
            let _ = writeln!(embassy_preempt_log::uart::Uart, $($arg)*);
        }
    };
}

// No logging enabled
#[cfg(not(all(feature = "log-base", any(feature = "log-rtt", feature = "log-uart"))))]
#[macro_export]
macro_rules! __log {
    ($level:ident, $($arg:tt)*) => {};
}

// OS logging macros
#[cfg(feature = "log-os")]
#[macro_export]
macro_rules! os_log {
    ($level:ident, $($args:tt)*) => { $crate::$level!($($args)*); };
}

#[cfg(not(feature = "log-os"))]
#[macro_export]
macro_rules! os_log {
    ($level:ident, $($args:tt)*) => {};
}

// Task logging macros
#[cfg(feature = "log-task")]
#[macro_export]
macro_rules! task_log {
    ($level:ident, $($args:tt)*) => { $crate::$level!($($args)*); };
}

#[cfg(not(feature = "log-task"))]
#[macro_export]
macro_rules! task_log {
    ($level:ident, $($args:tt)*) => {};
}

// Scheduler logging macros
#[cfg(feature = "log-scheduler")]
#[macro_export]
macro_rules! scheduler_log {
    ($level:ident, $($args:tt)*) => { $crate::$level!($($args)*); };
}

#[cfg(not(feature = "log-scheduler"))]
#[macro_export]
macro_rules! scheduler_log {
    ($level:ident, $($args:tt)*) => {};
}

// Timer logging macros
#[cfg(feature = "log-timer")]
#[macro_export]
macro_rules! timer_log {
    ($level:ident, $($args:tt)*) => { $crate::$level!($($args)*); };
}

#[cfg(not(feature = "log-timer"))]
#[macro_export]
macro_rules! timer_log {
    ($level:ident, $($args:tt)*) => {};
}

// Memory logging macros
#[cfg(feature = "log-mem")]
#[macro_export]
macro_rules! mem_log {
    ($level:ident, $($args:tt)*) => { $crate::$level!($($args)*); };
}

#[cfg(not(feature = "log-mem"))]
#[macro_export]
macro_rules! mem_log {
    ($level:ident, $($args:tt)*) => {};
}
