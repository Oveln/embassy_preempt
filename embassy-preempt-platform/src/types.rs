//! Common type definitions and structures

use core::cell::Cell;
use core::sync::atomic::{AtomicU32, AtomicU8};

// Platform-specific types that should remain as aliases
pub type OS_STK = usize;
pub type OS_CPU_SR = u32;

/// Handle to an alarm
#[derive(Clone, Copy)]
pub struct AlarmHandle {
    id: u8,
}

impl AlarmHandle {
    /// Create a new AlarmHandle
    pub unsafe fn new(id: u8) -> Self {
        Self { id }
    }

    /// Get the ID of the AlarmHandle
    pub fn id(&self) -> u8 {
        self.id
    }
}

/// Structure to hold alarm state
pub struct AlarmState {
    /// Timestamp at which to fire alarm
    pub timestamp: Cell<u64>,
    /// Callback function for the alarm
    pub callback: Cell<*const ()>,
    /// Context for the callback
    pub ctx: Cell<*mut ()>,
}

unsafe impl Send for AlarmState {}

impl AlarmState {
    pub const fn new() -> Self {
        Self {
            timestamp: Cell::new(u64::MAX),
            callback: Cell::new(core::ptr::null()),
            ctx: Cell::new(core::ptr::null_mut()),
        }
    }
}

/// Structure for the RTC driver implementation
pub struct RtcDriver {
    /// Number of 2^15 periods elapsed since boot
    pub period: AtomicU32,
    /// Count of allocated alarms
    pub alarm_count: AtomicU8,
    /// Alarms array
    pub alarms: [AlarmState; 3], // Using 3 as default alarm count
}

impl RtcDriver {
    pub const fn new() -> Self {
        const ARRAY_REPEAT_VALUE: AlarmState = AlarmState::new();
        Self {
            period: AtomicU32::new(0),
            alarm_count: AtomicU8::new(0),
            alarms: [ARRAY_REPEAT_VALUE, ARRAY_REPEAT_VALUE, ARRAY_REPEAT_VALUE],
        }
    }
}