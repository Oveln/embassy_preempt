//! Timer Driver Traits and Types
//!
//! This module defines the timer driver interface used by the Embassy Preempt RTOS.
//! It provides an abstraction layer for hardware timers that supports alarm management
//! and time-based task scheduling.
//!
//! ## Key Components
//!
//! - **Driver**: Main trait for timer driver implementations
//! - **AlarmHandle**: Handle for managing individual alarms
//! - **AlarmState**: State storage for alarm configurations
//!
//! ## Timer Architecture
//!
//! The timer system supports:
//! - Multiple concurrent alarms per timer driver
//! - Callback-based alarm notifications
//! - Hardware timer abstraction across different platforms
//! - Interrupt-safe alarm management
//! - 64-bit timestamp support

use core::cell::Cell;
use core::ptr;

// use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

/// Handle to an alarm instance.
///
/// This handle represents a single alarm that can be configured and managed
/// through the timer driver. Each alarm has a unique identifier within its
/// timer driver context.
#[derive(Clone, Copy)]
pub struct AlarmHandle {
    /// Unique identifier for this alarm within the timer driver
    id: u8,
}

/// Alarm state storage for timer driver
///
/// Contains all the state information needed for a single alarm, including
/// the timestamp when it should trigger, the callback function to execute,
/// and the context pointer to pass to the callback.
pub struct AlarmState {
    /// The timestamp (in timer ticks) when this alarm should trigger
    pub timestamp: Cell<u64>,

    /// Function pointer to the callback that will be executed when alarm triggers
    /// This is stored as a raw pointer to allow for type-erased callback storage
    pub callback: Cell<*const ()>,

    /// Context pointer that will be passed to the callback function
    /// Allows the callback to access relevant state or data
    pub ctx: Cell<*mut ()>,
}

// Safety: AlarmState is safe to send across threads because it only uses
// atomic Cell types and raw pointers which are not simultaneously accessed
// from multiple threads due to the timer driver's synchronization guarantees
unsafe impl Send for AlarmState {}
unsafe impl Sync for AlarmState {}

/// Timer driver trait for hardware abstraction layer
///
/// This trait defines the interface that hardware-specific timer drivers must implement
/// to provide timing services for the RTOS. It supports multiple concurrent alarms,
/// callback-based notifications, and interrupt-safe operations.
///
/// ## Implementation Requirements
///
/// - Thread-safe access to hardware timer registers
/// - Proper interrupt handling and masking
/// - Atomic operations for shared state management
/// - Hardware-specific alarm allocation and management
///
/// ## Usage
///
/// Timer drivers are typically implemented once per hardware platform and provide
/// the foundation for:
/// - Task scheduling and timeouts
/// - Sleep and delay operations
/// - Periodic timer operations
/// - Time-based event notifications
pub trait Driver: Send + Sync + 'static {
    /// Get the current timer timestamp
    ///
    /// Returns the current time in timer ticks. The tick frequency and resolution
    /// are platform-specific but must be monotonically increasing.
    ///
    /// # Returns
    /// Current timestamp in platform-specific timer ticks
    fn now(&self) -> u64;

    /// Allocate a new alarm handle
    ///
    /// Attempts to allocate an alarm instance from the timer driver's pool.
    /// The number of available alarms is hardware-dependent.
    ///
    /// # Safety
    /// - Caller must ensure the returned handle is properly managed
    /// - Handle must not be used after the timer driver is destroyed
    ///
    /// # Returns
    /// - `Some(AlarmHandle)` if an alarm was successfully allocated
    /// - `None` if no more alarms are available
    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle>;

    /// Set the callback function for an alarm
    ///
    /// Configures the callback that will be executed when the specified alarm
    /// triggers. The callback will be called with the provided context pointer.
    ///
    /// # Parameters
    /// - `alarm`: The alarm handle to configure
    /// - `callback`: Function to call when alarm triggers
    /// - `ctx`: Context pointer to pass to the callback
    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ());

    /// Schedule an alarm to trigger at a specific timestamp
    ///
    /// Configures the alarm to trigger at the specified timestamp. If the timestamp
    /// is in the past, the alarm may trigger immediately.
    ///
    /// # Parameters
    /// - `alarm`: The alarm handle to schedule
    /// - `timestamp`: When the alarm should trigger (in timer ticks)
    ///
    /// # Returns
    /// - `true` if alarm was successfully scheduled
    /// - `false` if the timestamp has already passed or scheduling failed
    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool;

    /// Handle timer interrupt
    ///
    /// Called by the hardware interrupt handler when the timer generates an interrupt.
    /// This method should check for triggered alarms and invoke their callbacks.
    ///
    /// # Safety
    /// - Must only be called from interrupt context
    /// - Timer hardware must be properly initialized
    /// - Interrupt must be properly masked and cleared
    unsafe fn on_interrupt(&self);
}

impl AlarmState {
    /// Create a new, uninitialized alarm state
    ///
    /// Creates an alarm state with default values:
    /// - timestamp set to u64::MAX (no expiration)
    /// - callback set to null (no callback configured)
    /// - context set to null (no context)
    ///
    /// # Returns
    /// A new AlarmState instance in unconfigured state
    pub const fn new() -> Self {
        Self {
            timestamp: Cell::new(u64::MAX),
            callback: Cell::new(ptr::null()),
            ctx: Cell::new(ptr::null_mut()),
        }
    }
}

impl AlarmHandle {
    /// Create a new alarm handle with the specified ID
    ///
    /// # Safety
    /// - The `id` must be valid within the timer driver's alarm pool
    /// - The handle must not outlive the timer driver
    /// - Only one handle should exist per alarm ID to avoid aliasing
    ///
    /// # Parameters
    /// - `id`: The unique identifier for this alarm within the timer driver
    ///
    /// # Returns
    /// A new AlarmHandle instance
    pub unsafe fn new(id: u8) -> Self {
        Self { id }
    }

    /// Get the unique ID of this alarm handle
    ///
    /// Returns the identifier that can be used to index into the timer
    /// driver's alarm pool or state storage.
    ///
    /// # Returns
    /// The unique alarm ID
    pub fn id(&self) -> u8 {
        self.id
    }
}
