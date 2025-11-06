//! Logging macros for the Embassy Preempt RTOS
//!
//! This module provides logging macros that wrap around the `defmt` crate.
//! When the "logs" feature is enabled, these macros will output log messages.
//! When the feature is disabled, the macros become no-ops for performance.

#![no_std]

#[cfg(feature = "log-base")]
use defmt_rtt as _;

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

// Forward to defmt or become no-op based on features
#[cfg(feature = "log-base")]
#[macro_export]
macro_rules! __log {
    ($level:ident, $($arg:tt)*) => {
        defmt::$level!($($arg)*)
    };
}

#[cfg(not(feature = "log-base"))]
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