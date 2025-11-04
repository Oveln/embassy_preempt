//! Timer driver trait definition

use crate::AlarmHandle;

/// Timer driver functionality
pub trait TimerDriver {
    /// Return the current timestamp in ticks
    fn now(&self) -> u64;

    /// Allocate an alarm handle
    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle>;

    /// Set the alarm callback function
    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ());

    /// Set an alarm at the given timestamp
    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool;
}