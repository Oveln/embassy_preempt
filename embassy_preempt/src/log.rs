//! Logging macros for the Embassy Preempt RTOS
//! 
//! This module provides logging macros that wrap around the `defmt` crate.
//! When the "logs" feature is enabled, these macros will output log messages.
//! When the feature is disabled, the macros become no-ops for performance.

use defmt_rtt as _;

/// Logs a debug message using defmt.
/// 
/// When the "logs" feature is enabled, this macro forwards the arguments to `defmt::debug!`.
/// Otherwise, it becomes a no-op.
/// 
/// # Example
/// 
/// ```rust
/// debug!("This is a debug message");
/// debug!("Value: {}", value);
/// ```
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        defmt::debug!($($arg)*)
    };
}

/// Logs an error message using defmt.
/// 
/// When the "logs" feature is enabled, this macro forwards the arguments to `defmt::error!`.
/// Otherwise, it becomes a no-op.
/// 
/// # Example
/// 
/// ```rust
/// error!("This is an error message");
/// error!("Error code: {}", error_code);
/// ```
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        defmt::error!($($arg)*)
    };
}

/// Logs an info message using defmt.
/// 
/// When the "logs" feature is enabled, this macro forwards the arguments to `defmt::info!`.
/// Otherwise, it becomes a no-op.
/// 
/// # Example
/// 
/// ```rust
/// info!("This is an info message");
/// info!("System initialized at {}", timestamp);
/// ```
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        defmt::info!($($arg)*)
    };
}

/// Logs a trace message using defmt.
/// 
/// When the "logs" feature is enabled, this macro forwards the arguments to `defmt::trace!`.
/// Otherwise, it becomes a no-op.
/// 
/// # Example
/// 
/// ```rust
/// trace!("This is a trace message");
/// trace!("Function called with parameter: {}", param);
/// ```
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        defmt::trace!($($arg)*)
    };
}

/// Logs a warning message using defmt.
/// 
/// When the "logs" feature is enabled, this macro forwards the arguments to `defmt::warn!`.
/// Otherwise, it becomes a no-op.
/// 
/// # Example
/// 
/// ```rust
/// warn!("This is a warning message");
/// warn!("Resource usage is high: {}%", usage);
/// ```
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        defmt::warn!($($arg)*)
    };
}

/// No-op debug macro when defmt is disabled
/// 
/// When the "logs" feature is not enabled, this macro becomes a no-op.
#[cfg(not(log_enabled))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

/// No-op error macro when defmt is disabled
/// 
/// When the "logs" feature is not enabled, this macro becomes a no-op.
#[cfg(not(log_enabled))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {};
}

/// No-op info macro when defmt is disabled
/// 
/// When the "logs" feature is not enabled, this macro becomes a no-op.
#[cfg(not(log_enabled))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {};
}

/// No-op trace macro when defmt is disabled
/// 
/// When the "logs" feature is not enabled, this macro becomes a no-op.
#[cfg(not(log_enabled))]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

/// No-op warn macro when defmt is disabled
/// 
/// When the "logs" feature is not enabled, this macro becomes a no-op.
#[cfg(not(log_enabled))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

/// Logs messages using the OS logging system with a specific log level.
/// 
/// This macro allows you to specify the log level at runtime and is useful for 
/// component-specific logging within the RTOS operations.
/// 
/// # Example
/// 
/// ```rust
/// os_log!(info, "Task {} created", task_id);
/// os_log!(error, "Critical system error: {}", error_code);
/// ```
#[cfg(feature = "log-os")]
#[macro_export]
macro_rules! os_log {
    ($level:ident, $($args:tt)*) => {
        $crate::$level!($($args)*);
    };
}

/// No-op os_log macro when log-os feature is disabled
/// 
/// When the "log-os" feature is not enabled, this macro becomes a no-op.
#[cfg(not(feature = "log-os"))]
#[macro_export]
macro_rules! os_log {
    ($level:ident, $($args:tt)*) => {};
}

/// Logs messages for task-related operations using a specific log level.
/// 
/// This macro allows you to specify the log level at runtime and is useful for 
/// component-specific logging within task management.
/// 
/// # Example
/// 
/// ```rust
/// task_log!(info, "Task {} created", task_id);
/// task_log!(error, "Task {} failed to execute", task_id);
/// ```
#[cfg(feature = "log-task")]
#[macro_export]
macro_rules! task_log {
    ($level:ident, $($args:tt)*) => {
        $crate::$level!($($args)*);
    };
}

/// No-op task_log macro when log-task feature is disabled
/// 
/// When the "log-task" feature is not enabled, this macro becomes a no-op.
#[cfg(not(feature = "log-task"))]
#[macro_export]
macro_rules! task_log {
    ($level:ident, $($args:tt)*) => {};
}

/// Logs messages for scheduler-related operations using a specific log level.
/// 
/// This macro allows you to specify the log level at runtime and is useful for 
/// component-specific logging within scheduler operations.
/// 
/// # Example
/// 
/// ```rust
/// scheduler_log!(debug, "Scheduler tick: {}", tick_count);
/// scheduler_log!(warn, "Priority inversion detected");
/// ```
#[cfg(feature = "log-scheduler")]
#[macro_export]
macro_rules! scheduler_log {
    ($level:ident, $($args:tt)*) => {
        $crate::$level!($($args)*);
    };
}

/// No-op scheduler_log macro when log-scheduler feature is disabled
/// 
/// When the "log-scheduler" feature is not enabled, this macro becomes a no-op.
#[cfg(not(feature = "log-scheduler"))]
#[macro_export]
macro_rules! scheduler_log {
    ($level:ident, $($args:tt)*) => {};
}

/// Logs messages for timer-related operations using a specific log level.
/// 
/// This macro allows you to specify the log level at runtime and is useful for 
/// component-specific logging within timer operations.
/// 
/// # Example
/// 
/// ```rust
/// timer_log!(info, "Timer {} expired", timer_id);
/// timer_log!(error, "Timer {} failed to start", timer_id);
/// ```
#[cfg(feature = "log-timer")]
#[macro_export]
macro_rules! timer_log {
    ($level:ident, $($args:tt)*) => {
        $crate::$level!($($args)*);
    };
}

/// No-op timer_log macro when log-timer feature is disabled
/// 
/// When the "log-timer" feature is not enabled, this macro becomes a no-op.
#[cfg(not(feature = "log-timer"))]
#[macro_export]
macro_rules! timer_log {
    ($level:ident, $($args:tt)*) => {};
}

/// Logs messages for memory-related operations using a specific log level.
/// 
/// This macro allows you to specify the log level at runtime and is useful for 
/// component-specific logging within memory management operations.
/// 
/// # Example
/// 
/// ```rust
/// mem_log!(debug, "Memory allocated: {} bytes", size);
/// mem_log!(warn, "Low memory warning: {} bytes remaining", remaining);
/// ```
#[cfg(feature = "log-mem")]
#[macro_export]
macro_rules! mem_log {
    ($level:ident, $($args:tt)*) => {
        $crate::$level!($($args)*);
    };
}

/// No-op mem_log macro when log-mem feature is disabled
/// 
/// When the "log-mem" feature is not enabled, this macro becomes a no-op.
#[cfg(not(feature = "log-mem"))]
#[macro_export]
macro_rules! mem_log {
    ($level:ident, $($args:tt)*) => {};
}