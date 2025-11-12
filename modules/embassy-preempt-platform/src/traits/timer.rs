//! Timer driver trait definition

use core::cell::Cell;
use core::sync::atomic::{compiler_fence, AtomicU32, AtomicU8, Ordering};
use core::{mem, ptr};
use critical_section::{CriticalSection, Mutex};
// use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

/// Handle to an alarm.
#[derive(Clone, Copy)]
pub struct AlarmHandle {
    id: u8,
}

/// Alarm state for timer driver
pub struct AlarmState {
    pub timestamp: Cell<u64>,
    pub callback: Cell<*const ()>,
    pub ctx: Cell<*mut ()>,
}

unsafe impl Send for AlarmState {}
unsafe impl Sync for AlarmState {}

/// Time driver trait
pub trait Driver: Send + Sync + 'static {
    /// Return the current timestamp in ticks.
    fn now(&self) -> u64;

    /// Try allocating an alarm handle. Returns None if no alarms left.
    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle>;

    /// Sets the callback function to be called when the alarm triggers.
    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ());

    /// Sets an alarm at the given timestamp.
    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool;
}

impl AlarmState {
    pub const fn new() -> Self {
        Self {
            timestamp: Cell::new(u64::MAX),
            callback: Cell::new(ptr::null()),
            ctx: Cell::new(ptr::null_mut()),
        }
    }
}

impl AlarmHandle {
    /// Create an AlarmHandle
    pub unsafe fn new(id: u8) -> Self {
        Self { id }
    }

    /// Get the ID of the AlarmHandle.
    pub fn id(&self) -> u8 {
        self.id
    }
}