/*
*********************************************************************************************************
*                                 Generic RISC-V CLINT Timer Driver
*********************************************************************************************************
*/

//! Generic RISC-V CLINT Timer Driver
//!
//! This module provides a hardware-agnostic timer driver for RISC-V systems that use
//! the CLINT (Core-Local Interruptor) mtime register. It implements the `Driver` trait
//! and can be configured for different platforms via the `ClintConfig` trait.
//!
//! ## Timer Architecture
//!
//! The driver uses the RISC-V CLINT mtime register:
//! - **mtime**: 64-bit real-time counter (monotonically increasing)
//! - **mtimecmp**: 64-bit timer compare register for alarm generation
//! - **Timer frequency**: Configurable via `ClintConfig`
//! - **Alarm support**: Configurable number of alarms via const generic
//!
//! ## CLINT Register Layout
//!
//! ```text
//! CLINT_BASE (platform-specific)
//! - MSIP0         BASE + 0x0000  Hart 0 software interrupt
//! - MTIMECMP0    BASE + 0x4000  Hart 0 timer compare
//! - MTIME        BASE + 0xBFF8  Global real-time counter (64-bit)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use embassy_preempt_riscv64_rt::{ClintTimer, ClintConfig};
//!
//! // Define platform-specific configuration
//! struct MyPlatformConfig;
//! impl ClintConfig for MyPlatformConfig {
//!     const CLINT_BASE: usize = 0x02000000;
//!     const TIMER_HZ: u64 = 4_000_000;
//!     const HART_ID: usize = 0;
//! }
//!
//! // Create timer instance with 1 alarm
//! let timer = ClintTimer::<MyPlatformConfig, 1>::new();
//! timer.init();
//! ```

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use aclint::SifiveClint;
use portable_atomic::AtomicU8;

use critical_section::{CriticalSection, Mutex};
use riscv::register::{mie, mip};

use embassy_preempt_traits::timer::{AlarmHandle, AlarmState, Driver};

/*
*********************************************************************************************************
*                                           CLINT Configuration Trait
*********************************************************************************************************
*/

/// CLINT configuration trait
///
/// Platforms must implement this trait to provide platform-specific CLINT configuration.
/// This allows the same timer driver implementation to be used across different RISC-V platforms.
///
/// ## Example
///
/// ```rust,ignore
/// struct Jh7110Config;
///
/// impl ClintConfig for Jh7110Config {
///     const CLINT_BASE: usize = 0x02000000;
///     const TIMER_HZ: u64 = 4_000_000;
///     const HART_ID: usize = 0;
/// }
/// ```
pub trait ClintConfig {
    /// CLINT register base address
    const CLINT_BASE: usize;

    /// Timer frequency in Hz
    const TIMER_HZ: u64;

    /// Hart ID (default is 0)
    const HART_ID: usize = 0;
}

/*
*********************************************************************************************************
*                                           ClintTimer Implementation
*********************************************************************************************************
*/

/// Generic CLINT Timer Driver
///
/// Implements the Driver trait using the RISC-V CLINT mtime register.
/// Provides high-precision timing and alarm functionality for the RTOS.
///
/// ## Type Parameters
///
/// - `C`: CLINT configuration type implementing `ClintConfig`
/// - `N`: Number of alarm channels (const generic)
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
/// - `phantom`: PhantomData for the configuration type
/// - `alarm_count`: Counter for tracking allocated alarm instances
/// - `alarms`: Array of alarm states with callbacks and timestamps
pub struct ClintTimer<C: ClintConfig, const N: usize> {
    /// PhantomData for the configuration type
    phantom: PhantomData<C>,

    /// Counter for tracking allocated alarm instances
    alarm_count: AtomicU8,

    /// Array of alarm states storing callbacks, contexts, and trigger timestamps
    /// u64::MAX indicates no alarm is scheduled for that slot
    alarms: Mutex<[AlarmState; N]>,
}

/*
*********************************************************************************************************
*                                              Implementation
*********************************************************************************************************
*/

impl<C: ClintConfig, const N: usize> ClintTimer<C, N> {
    /// Create a new timer driver instance
    ///
    /// Initializes the driver with default values:
    /// - No alarms allocated initially
    /// - All alarm states in unconfigured state
    ///
    /// # Returns
    /// A new ClintTimer instance ready for initialization
    pub fn new() -> Self {
        const ALARM_STATE_NEW: AlarmState = AlarmState::new();

        // Build array of alarm states
        let alarms_array: [AlarmState; N] = [ALARM_STATE_NEW; N];

        ClintTimer {
            phantom: PhantomData,
            alarm_count: AtomicU8::new(0),
            alarms: Mutex::new(alarms_array),
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
        os_log!(trace, "Initializing CLINT Timer (CLINT_BASE={:#x}, TIMER_HZ={}, ALARMS={})", C::CLINT_BASE, C::TIMER_HZ, N);

        // Disable timer interrupt by setting mtimecmp to maximum value
        // This prevents spurious interrupts during initialization
        self.set_mtimecmp(u64::MAX);

        // Enable machine timer interrupt in mie (Machine Interrupt Enable)
        unsafe {
            mie::set_mtimer();
        }

        os_log!(info, "CLINT Timer initialized at {} Hz with {} alarm(s)", C::TIMER_HZ, N);
    }

    /// Get the CLINT register base address
    ///
    /// # Returns
    /// Pointer to the CLINT registers
    #[inline(always)]
    fn clint(&self) -> *const SifiveClint {
        C::CLINT_BASE as *const SifiveClint
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
            (*self.clint()).read_mtime()
        }
    }

    /// Set the mtimecmp register for the configured hart
    ///
    /// When mtime >= mtimecmp, a machine timer interrupt will be generated.
    ///
    /// # Parameters
    /// - `value`: The compare value to set
    #[inline(always)]
    fn set_mtimecmp(&self, value: u64) {
        unsafe {
            (*self.clint()).write_mtimecmp(C::HART_ID, value);
        }
    }

    /// Get the timer frequency in Hz
    ///
    /// Returns the frequency of the mtime counter for timestamp conversion.
    ///
    /// # Returns
    /// Timer frequency in Hz
    #[inline(always)]
    pub fn timer_frequency() -> u64 {
        C::TIMER_HZ
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

/*
*********************************************************************************************************
*                                           Driver Trait Implementation
*********************************************************************************************************
*/

// Safety: ClintTimer is Send/Sync because:
// - Mutex provides interior mutability with critical_section synchronization
// - AtomicU8 provides atomic operations
// - C is a ZST (zero-sized type) marker
unsafe impl<C: ClintConfig + 'static, const N: usize> Send for ClintTimer<C, N> {}
unsafe impl<C: ClintConfig + 'static, const N: usize> Sync for ClintTimer<C, N> {}

impl<C: ClintConfig + 'static, const N: usize> Driver for ClintTimer<C, N> {
    fn now(&self) -> u64 {
        self.read_mtime()
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        critical_section::with(|_| {
            let id = self.alarm_count.load(Ordering::Relaxed);
            if id < N as u8 {
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
                for n in 0..N {
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
