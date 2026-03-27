/*
*********************************************************************************************************
*                                 Platform Timer Driver - JH7110 RISC-V实现
*********************************************************************************************************
*/

//! JH7110 Timer Driver Implementation
//!
//! This module provides a hardware-specific timer driver for the StarFive JH7110
//! RISC-V SoC that implements the `Driver` trait. It uses the CLINT (Core-Local Interruptor)
//! mtime register to provide high-precision timing and alarm functionality for the RTOS.
//!
//! ## Timer Architecture
//!
//! The driver uses the RISC-V CLINT mtime register:
//! - **mtime**: 64-bit real-time counter (monotonically increasing)
//! - **mtimecmp**: 64-bit timer compare register for alarm generation
//! - **Timer frequency**: 4MHz (typical for JH7110)
//! - **Alarm support**: Single alarm using mtimecmp register
//!
//! ## Hardware Configuration
//!
//! - **Clock source**: Internal oscillator (typically 4MHz)
//! - **Timer frequency**: Fixed at 4MHz for JH7110
//! - **Interrupts**: Machine timer interrupt when mtime >= mtimecmp
//! - **Memory-mapped registers**: CLINT at 0x02000000
//!
//! ## CLINT Register Layout
//!
//! ```text
//! CLINT_BASE = 0x02000000
//! - MSIP0         0x02000000  Hart 0 software interrupt
//! - MTIMECMP0     0x02004000  Hart 0 timer compare
//! - MTIME         0x0200BFF8  Global real-time counter (64-bit)
//! ```
//!
//! ## Features
//!
//! - **64-bit timer**: Native 64-bit mtime register (no overflow handling needed)
//! - **Alarm support**: One alarm using mtimecmp register
//! - **Interrupt-driven**: Efficient event handling with minimal CPU overhead
//! - **Hardware abstraction**: Clean interface for the RTOS scheduler

use core::cell::Cell;
use core::{ptr, u64};
use core::sync::atomic::{compiler_fence, Ordering};

use aclint::SifiveClint;
use portable_atomic::AtomicU8;

use critical_section::{CriticalSection, Mutex};
use riscv::register::{mie, mip};

use embassy_preempt_traits::timer::{AlarmHandle, AlarmState, Driver};

/*
*********************************************************************************************************
*                                           CLINT Register Definitions
*********************************************************************************************************
*/

/// CLINT base address for JH7110
const CLINT_BASE: usize = 0x02000000;

pub const CLINT: *const SifiveClint = CLINT_BASE as *const SifiveClint;

/// Number of alarm channels available
/// RISC-V CLINT typically has one mtimecmp per hart
const ALARM_COUNT: usize = 1;

/// Timer frequency in Hz (4MHz for JH7110)
const TIMER_HZ: u64 = 4_000_000;

/*
*********************************************************************************************************
*                                           var declaration
*********************************************************************************************************
*/

/// JH7110 Timer Driver using CLINT mtime register
///
/// Implements the Driver trait using the RISC-V CLINT mtime register.
/// Provides high-precision timing and alarm functionality for the RTOS.
///
/// ## Architecture
///
/// The driver uses the native 64-bit mtime register:
/// - **Direct time reading**: mtime is already 64-bit, no overflow handling needed
/// - **Alarm support**: Uses mtimecmp register to generate timer interrupt
/// - **Interrupt**: Machine timer interrupt when mtime >= mtimecmp
///
/// ## Fields
///
/// - `alarm_count`: Number of allocated alarm instances
/// - `alarms`: Array of alarm states with callbacks and timestamps
pub struct Jh7110Timer {
    /// Counter for tracking allocated alarm instances
    alarm_count: AtomicU8,

    /// Array of alarm states storing callbacks, contexts, and trigger timestamps
    /// u64::MAX indicates no alarm is scheduled for that slot
    alarms: Mutex<[AlarmState; ALARM_COUNT]>,
}

/*
*********************************************************************************************************
*                                              implentations
*********************************************************************************************************
*/

impl Jh7110Timer {
    /// Create a new timer driver instance
    ///
    /// Initializes the driver with default values:
    /// - No alarms allocated initially
    /// - All alarm states in unconfigured state
    ///
    /// # Returns
    /// A new Jh7110Timer instance ready for initialization
    pub(crate) fn new() -> Self {
        const ALARM_STATE_NEW: AlarmState = AlarmState::new();
        Jh7110Timer {
            alarm_count: AtomicU8::new(0),
            alarms: Mutex::new([ALARM_STATE_NEW; ALARM_COUNT]),
        }
    }

    /// Initialize the timer driver and hardware
    ///
    /// This method performs timer-specific initialization:
    /// 1. Clear mtimecmp to disable timer interrupt
    /// 2. Enable machine timer interrupt in MIE
    ///
    /// Note: The mtime register is read-only and runs continuously.
    pub fn init(&self) {
        os_log!(trace, "Initializing JH7110 Jh7110Timer (CLINT mtime)");

        // Disable timer interrupt by setting mtimecmp to maximum value
        // This prevents spurious interrupts during initialization
        self.set_mtimecmp(u64::MAX);

        // Enable machine timer interrupt in mie (Machine Interrupt Enable)
        unsafe {
            mie::set_mtimer();
        }

        os_log!(info, "JH7110 Jh7110Timer initialized at {} Hz", TIMER_HZ);
    }

    /// Read the current value of the mtime register
    ///
    /// mtime is a 64-bit monotonically increasing counter.
    /// This function performs an atomic 64-bit read.
    ///
    /// # Returns
    /// Current 64-bit mtime value
    #[inline(always)]
    fn read_mtime(&self) -> u64 {
        unsafe {
            (*CLINT).read_mtime()
        }
    }

    /// Set the mtimecmp register for hart 0
    ///
    /// When mtime >= mtimecmp, a machine timer interrupt will be generated.
    ///
    /// # Parameters
    /// - `value`: The compare value to set
    #[inline(always)]
    fn set_mtimecmp(&self, value: u64) {
        unsafe {
            (*CLINT).write_mtimecmp(0, value);
        }
    }

    fn get_alarm<'a>(&'a self, cs: CriticalSection<'a>, alarm: AlarmHandle) -> &'a AlarmState {
        // safety: we're allowed to assume the AlarmState is created by us, and
        // we never create one that's out of bounds.
        unsafe { self.alarms.borrow(cs).get_unchecked(alarm.id() as usize) }
    }

    fn trigger_alarm(&self, n: usize, cs: CriticalSection) {
        let alarm = &self.alarms.borrow(cs)[n];
        alarm.timestamp.set(u64::MAX);

        // Call after clearing alarm, so the callback can set another alarm.

        // safety:
        // - we can ignore the possibility of `f` being unset (null) because of the safety contract of `allocate_alarm`.
        // - other than that we only store valid function pointers into alarm.callback
        let f: fn(*mut ()) = unsafe { core::mem::transmute(alarm.callback.get()) };
        f(alarm.ctx.get());
    }
}

impl Driver for Jh7110Timer {
    fn now(&self) -> u64 {
        self.read_mtime()
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        critical_section::with(|_| {
            let id = self.alarm_count.load(Ordering::Relaxed);
            if id < ALARM_COUNT as u8 {
                self.alarm_count.store(id + 1, Ordering::Relaxed);
                Some(AlarmHandle::new(id))
            } else {
                None
            }
        })
    }

    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ()) {
        critical_section::with(|cs| {
            let alarm = self.get_alarm(cs, alarm);
            alarm.callback.set(callback as *const ());
            alarm.ctx.set(ctx);
        })
    }

    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool {
        timer_log!(trace, "set_alarm at {}", timestamp);
        let n = alarm.id() as usize;

        // Check the timestamp. If timestamp is u64::MAX, there is no need to set the alarm
        if timestamp == u64::MAX {
            critical_section::with(|cs| {
                let alarm = self.get_alarm(cs, alarm);
                alarm.timestamp.set(u64::MAX);
                // Disable alarm by setting mtimecmp to maximum
                self.set_mtimecmp(u64::MAX);
            });
            return true;
        }

        critical_section::with(|cs| {
            let alarm = self.get_alarm(cs, alarm);
            alarm.timestamp.set(timestamp);

            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed the alarm will not fire.
                // Disable the alarm and return `false` to indicate that.
                self.set_mtimecmp(u64::MAX);
                alarm.timestamp.set(u64::MAX);
                timer_log!(trace, "Alarm timestamp has passed (current time {})", t);
                return false;
            }

            // Set mtimecmp to the alarm timestamp
            // The interrupt will fire when mtime >= mtimecmp
            self.set_mtimecmp(timestamp);
            timer_log!(trace, "Alarm set for {} (current time {})", timestamp, t);
            // We're confident the alarm will ring in the future.
            true
        })
    }

    unsafe fn on_interrupt(&self) {
        critical_section::with(|cs| {
            // Check if machine timer interrupt is pending
            if mip::read().mtimer() {
                // Clear the interrupt by setting mtimecmp to maximum
                // This will prevent the interrupt from immediately re-firing
                self.set_mtimecmp(u64::MAX);

                // Trigger the alarm callback
                // We only have one alarm (n=0) for CLINT
                for n in 0..ALARM_COUNT {
                    let alarm = &self.alarms.borrow(cs)[n];
                    let timestamp = alarm.timestamp.get();

                    // Check if this alarm should trigger
                    // The alarm timestamp should be <= current time
                    let current_time = self.now();
                    if timestamp <= current_time && timestamp != u64::MAX {
                        self.trigger_alarm(n, cs);
                    }
                }
            }
        })
    }
}

/*
*********************************************************************************************************
*                                           auxiliary function
*********************************************************************************************************
*/

/// Get the timer frequency in Hz
///
/// Returns the frequency of the mtime counter for timestamp conversion.
///
/// # Returns
/// Timer frequency in Hz (4,000,000 for JH7110)
pub fn get_timer_frequency() -> u64 {
    TIMER_HZ
}
