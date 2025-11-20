/*
*********************************************************************************************************
*                                 Platform Timer Driver - STM32F401RE实现
*********************************************************************************************************
*/

//! STM32F401RE Timer Driver Implementation
//!
//! This module provides a hardware-specific timer driver for the STM32F401RE microcontroller
//! that implements the `Driver` trait. It uses general-purpose timers (TIM2, TIM3, TIM4, TIM5)
//! to provide high-precision timing and alarm functionality for the RTOS.
//!
//! ## Timer Architecture
//!
//! The driver uses a 16-bit timer with overflow and half-overflow interrupts to create
//! a 64-bit timestamp counter:
//! - **Timer period**: 16-bit counter (0-65535)
//! - **Period tracking**: 32-bit period counter for overflow handling
//! - **Combined timestamp**: 64-bit (32-bit period << 15 | 15-bit counter)
//! - **Alarm support**: Up to 3 concurrent alarms (1 for limited timers)
//!
//! ## Hardware Configuration
//!
//! - **Clock source**: APB1 bus clock (42MHz) with programmable prescaler
//! - **Timer frequency**: Configurable via TICK_HZ (typically 1kHz)
//! - **Interrupts**: Overflow, half-overflow, and capture/compare interrupts
//! - **Timer selection**: Configurable via feature flags (time_driver_tim*)
//!
//! ## Clock Configuration Integration
//!
//! The timer driver integrates with the HAL library for clock configuration:
//! - System clock configuration is handled by the platform initialization using HAL
//! - Clock frequencies are provided via `store_clock_config()` during platform setup
//! - Timer prescaler calculation uses the stored APB1 frequency from HAL
//!
//! ## Features
//!
//! - **High-precision timing**: 1kHz tick rate with sub-millisecond resolution
//! - **Multiple alarms**: Support for concurrent scheduled callbacks
//! - **Interrupt-driven**: Efficient event handling with minimal CPU overhead
//! - **Hardware abstraction**: Clean interface for the RTOS scheduler
//! - **Debug support**: Optional GPIO toggling for timing analysis
//! - **HAL integration**: Uses HAL library for clock configuration
use core::sync::atomic::{compiler_fence, AtomicU32, AtomicU8, Ordering};
use core::mem;

use cortex_m::peripheral::NVIC;
use critical_section::{CriticalSection, Mutex};
use embassy_preempt_cfg::{APB_HZ, SYSCLK_HZ, TICK_HZ};
use embassy_preempt_log::{os_log, timer_log};
// use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use stm32_metapac::flash::vals::Latency;
use stm32_metapac::rcc::vals::*;
use stm32_metapac::timer::{regs, vals};
use stm32_metapac::{Interrupt, FLASH, RCC};

use crate::traits::timer::{AlarmHandle, AlarmState, Driver};

/// Timer peripheral selection based on feature flags
///
/// The TIMER constant provides access to the selected timer peripheral
/// based on which time_driver_tim* feature is enabled. This allows
/// compile-time selection of the hardware timer to use.
#[cfg(feature = "time_driver_tim1")]
pub const TIMER: stm32_metapac::timer::TimGp16 = stm32_metapac::TIM1;
#[cfg(feature = "time_driver_tim2")]
pub const TIMER: stm32_metapac::timer::TimGp32 = stm32_metapac::TIM2;
#[cfg(feature = "time_driver_tim3")]
pub const TIMER: stm32_metapac::timer::TimGp16 = stm32_metapac::TIM3;
#[cfg(feature = "time_driver_tim4")]
pub const TIMER: stm32_metapac::timer::TimGp16 = stm32_metapac::TIM4;
#[cfg(feature = "time_driver_tim5")]
pub const TIMER: stm32_metapac::timer::TimGp32 = stm32_metapac::TIM5;
#[cfg(feature = "time_driver_tim8")]
pub const TIMER: stm32_metapac::timer::TimGp16 = stm32_metapac::TIM8;
#[cfg(feature = "time_driver_tim9")]
pub const TIMER: stm32_metapac::timer::TimGp16 = stm32_metapac::TIM9;
#[cfg(feature = "time_driver_tim12")]
pub const TIMER: stm32_metapac::timer::TimGp16 = stm32_metapac::TIM12;

// Type aliases for consistency with uC/OS-II naming conventions
pub type BOOLEAN = bool;
pub type INT16U = u16;
pub type INT32U = u32;
pub type INT64U = u64;
pub type INT8U = u8;
pub type USIZE = usize;

/*
*********************************************************************************************************
*                                           var declaration
*********************************************************************************************************
*/


/// Set debug GPIO pin high for interrupt timing analysis
///
/// When the early_debug_gpio feature is enabled, this function toggles a GPIO
/// pin high when entering an interrupt handler. This is useful for measuring
/// interrupt latency and timing with an oscilloscope or logic analyzer.
#[cfg(feature = "early_debug_gpio")]
fn interrupt_pin_high() {
    // TODO: 实现调试GPIO功能
}

/// Set debug GPIO pin low for interrupt timing analysis
///
/// When the early_debug_gpio feature is enabled, this function toggles a GPIO
/// pin low when exiting an interrupt handler. This allows measurement of
/// interrupt duration with external debugging tools.
#[cfg(feature = "early_debug_gpio")]
fn interrupt_pin_low() {
    // TODO: 实现调试GPIO功能
}

/// Debug placeholder function when GPIO debugging is disabled
///
/// This function does nothing but provides compile-time compatibility
/// when the early_debug_gpio feature is not enabled.
#[cfg(not(feature = "early_debug_gpio"))]
#[inline(always)]
fn interrupt_pin_high() {}

/// Debug placeholder function when GPIO debugging is disabled
///
/// This function does nothing but provides compile-time compatibility
/// when the early_debug_gpio feature is not enabled.
#[cfg(not(feature = "early_debug_gpio"))]
#[inline(always)]
fn interrupt_pin_low() {}

/// Number of alarm channels available for the selected timer
///
/// Some STM32 timers have limited capture/compare channels, which affects
/// how many concurrent alarms can be supported:
/// - Limited timers (TIM9, TIM12, etc.): 1 alarm channel
/// - General-purpose timers (TIM2, TIM3, TIM4, TIM5): 3 alarm channels
#[cfg(any(
    feature = "time_driver_tim9",
    feature = "time_driver_tim12",
    feature = "time_driver_tim15",
    feature = "time_driver_tim21",
    feature = "time_driver_tim22"
))]
const ALARM_COUNT: USIZE = 1;

#[cfg(not(any(
    feature = "time_driver_tim9",
    feature = "time_driver_tim12",
    feature = "time_driver_tim15",
    feature = "time_driver_tim21",
    feature = "time_driver_tim22"
)))]
const ALARM_COUNT: USIZE = 3;
/// TIM3 interrupt handler
///
/// This function is called when the TIM3 timer generates an interrupt.
/// It handles timer overflow, half-overflow, and alarm callback events.
/// The interrupt handler delegates to the timer driver's on_interrupt method.
#[cfg(feature = "time_driver_tim3")]
#[unsafe(no_mangle)]
pub extern "C" fn TIM3() {
    use crate::get_platform_trait;

    // Toggle debug GPIO pin high for timing analysis
    interrupt_pin_high();

    os_log!(trace, "TIM3 interrupt handler invoked");

    // Delegate to the timer driver for interrupt processing
    unsafe {
        get_platform_trait().get_timer_driver().on_interrupt();
    }

    os_log!(trace, "exiting TIM3 interrupt handler");

    // Toggle debug GPIO pin low
    interrupt_pin_low();
}

/*
*********************************************************************************************************
*                                           type definitions
*********************************************************************************************************
*/
/// Timer enable/disable constants for readability
const DISABLE: bool = false;
const ENABLE: bool = true;

/// STM32F401RE Timer Driver
///
/// Implements the Driver trait using a hardware general-purpose timer.
/// Provides high-precision timing and alarm functionality for the RTOS.
///
/// ## Architecture
///
/// The driver creates a 64-bit timestamp counter from a 16-bit hardware timer:
/// - **Timer overflow**: Increment period counter every 2^15 ticks
/// - **Half-overflow**: Handle wraparound and maintain continuity
/// - **64-bit timestamp**: (period << 15) | (counter ^ ((period & 1) << 15))
///
/// ## Fields
///
/// - `period`: Atomic counter tracking timer overflow periods
/// - `alarm_count`: Number of allocated alarm instances
/// - `alarms`: Array of alarm states with callbacks and timestamps
/// - `rtc`: Optional RTC reference for low-power mode support
pub struct RtcDriver {
    /// Number of 2^15 (32768) tick periods elapsed since system boot
    /// Each period represents one full overflow cycle of the 16-bit timer
    period: AtomicU32,

    /// Counter for tracking allocated alarm instances
    alarm_count: AtomicU8,

    /// Array of alarm states storing callbacks, contexts, and trigger timestamps
    /// u64::MAX indicates no alarm is scheduled for that slot
    alarms: Mutex<[AlarmState; ALARM_COUNT]>,

    /// Optional RTC reference for low-power mode (feature-gated)
    #[cfg(feature = "low-power")]
    rtc: Mutex<CriticalSectionRawMutex, Cell<Option<&'static Rtc>>>,
}

/*
*********************************************************************************************************
*                                              implentations
*********************************************************************************************************
*/

impl RtcDriver {
    /// Create a new timer driver instance
    ///
    /// Initializes the driver with default values:
    /// - Period counter starts at 0
    /// - No alarms allocated initially
    /// - All alarm states in unconfigured state
    ///
    /// # Returns
    /// A new RtcDriver instance ready for initialization
    pub(crate) fn new() -> Self {
        const ALARM_STATE_NEW: AlarmState = AlarmState::new();
        RtcDriver {
            period: AtomicU32::new(0),
            alarm_count: AtomicU8::new(0),
            alarms: Mutex::new([ALARM_STATE_NEW; ALARM_COUNT]),
            #[cfg(feature = "low-power")]
            rtc: Mutex::const_new(CriticalSectionRawMutex::new(), Cell::new(None)),
        }
    }
    /// Initialize the timer driver and hardware
    ///
    /// This method performs timer-specific initialization:
    /// 1. Enable timer peripheral clock
    /// 2. Calculate and set prescaler for desired tick rate
    /// 3. Configure timer interrupts (overflow, half-overflow, alarms)
    /// 4. Enable timer and unmask interrupts in NVIC
    ///
    /// Note: System RCC and clock configuration should be handled by the platform initialization.
    ///
    /// # Panics
    /// - If the prescaler calculation overflows
    /// - If clock configuration is invalid
    pub fn init(&self) {
        os_log!(trace, "Initializing RtcDriver");

        // Enable timer peripheral clock
        enable_Timer();

        // Get APB frequency for prescaler calculation
        get_APBfreq();

        // Temporarily disable timer for configuration
        TIMER.cr1().modify(|w| w.set_cen(false));

        // Clear counter to start from zero
        TIMER.cnt().write(|w| w.set_cnt(0));

        // Calculate prescaler to achieve desired tick frequency
        let psc = (*APB_HZ().get() / TICK_HZ) as INT32U - 1;
        let psc: INT16U = match psc.try_into() {
            Err(_) => panic!("psc division overflow: {}", psc),
            Ok(n) => n,
        };

        // Set the prescaler divider
        TIMER.psc().write_value(psc);

        // Set auto-reload register to maximum for 16-bit timer
        TIMER.arr().write(|w| w.set_arr(INT16U::MAX));

        // Set URS, generate update and clear URS
        // by noah： when set ug bit, a update event will be generated immediately.
        // In the update event, the counter(including prescaler counter) will be clear.
        // So in this way, we can init the counter by this way
        TIMER.cr1().modify(|w| w.set_urs(vals::Urs::COUNTERONLY));
        TIMER.egr().write(|w| w.set_ug(true));
        TIMER.cr1().modify(|w| w.set_urs(vals::Urs::ANYEVENT));

        // Mid-way point, there will be a cc(capture/compare) interrupt at 0x8000
        TIMER.ccr(0).write(|w| w.set_ccr(0x8000));

        TIMER.dier().write(|w| {
            // Enable overflow
            w.set_uie(true);
            // and half-overflow interrupts
            w.set_ccie(0, true);
        });

        // by noah：the InterruptNumber trait is implemented by the stm32_metapac crate
        // so in embassy, the InterruptExt trait will be implemented for the interrupt in pac
        #[cfg(feature = "time_driver_tim3")]
        NVIC::unpend(Interrupt::TIM3);
        #[cfg(feature = "time_driver_tim3")]
        unsafe {
            compiler_fence(Ordering::SeqCst);
            NVIC::unmask(Interrupt::TIM3);
        }
        #[cfg(feature = "time_driver_tim1")]
        {
            NVIC::unpend(Interrupt::TIM1_CC);
            NVIC::unpend(Interrupt::TIM1_UP_TIM10);
            unsafe {
                compiler_fence(Ordering::SeqCst);
                NVIC::unmask(Interrupt::TIM1_CC);
                NVIC::unmask(Interrupt::TIM1_UP_TIM10);
            }
        }
        #[cfg(feature = "time_driver_tim2")]
        {
            NVIC::unpend(Interrupt::TIM2);
            unsafe {
                compiler_fence(Ordering::SeqCst);
                NVIC::unmask(Interrupt::TIM2);
            }
        }
        #[cfg(feature = "time_driver_tim4")]
        {
            NVIC::unpend(Interrupt::TIM4);
            unsafe {
                compiler_fence(Ordering::SeqCst);
                NVIC::unmask(Interrupt::TIM4);
            }
        }
        #[cfg(feature = "time_driver_tim5")]
        {
            NVIC::unpend(Interrupt::TIM5);
            unsafe {
                compiler_fence(Ordering::SeqCst);
                NVIC::unmask(Interrupt::TIM5);
            }
        }

        // enable the Timer
        TIMER.cr1().modify(|w| w.set_cen(ENABLE));
    }

    fn next_period(&self) {
        // let r = regs_gp16();
        // We only modify the period from the timer interrupt, so we know this can't race.
        let period = self.period.load(Ordering::Relaxed) + 1;
        // #[cfg(feature = "alarm_test")]
        // os_log!(info, "RTC's period is {}", self.period.load(Ordering::Relaxed));
        self.period.store(period, Ordering::Relaxed);
        let t = (period as u64) << 15;

        critical_section::with(move |cs| {
            TIMER.dier().modify(move |w| {
                for n in 0..ALARM_COUNT {
                    let alarm = &self.alarms.borrow(cs)[n];
                    let at = alarm.timestamp.get();

                    if at < t + 0xc000 {
                        // just enable it. `set_alarm` has already set the correct CCR val.
                        w.set_ccie(n + 1, true);
                    }
                }
            })
        })
    }

    fn get_alarm<'a>(&'a self, cs: CriticalSection<'a>, alarm: AlarmHandle) -> &'a AlarmState {
        // safety: we're allowed to assume the AlarmState is created by us, and
        // we never create one that's out of bounds.
        unsafe { self.alarms.borrow(cs).get_unchecked(alarm.id() as usize) }
    }

    fn trigger_alarm(&self, n: usize, cs: CriticalSection) {
        timer_log!(trace, "trigger_alarm");
        let alarm = &self.alarms.borrow(cs)[n];
        alarm.timestamp.set(u64::MAX);

        // Call after clearing alarm, so the callback can set another alarm.

        // safety:
        // - we can ignore the possibility of `f` being unset (null) because of the safety contract of `allocate_alarm`.
        // - other than that we only store valid function pointers into alarm.callback
        let f: fn(*mut ()) = unsafe { mem::transmute(alarm.callback.get()) };
        f(alarm.ctx.get());
    }
}

impl Driver for RtcDriver {
    fn now(&self) -> u64 {
        // let r = regs_gp16();

        let period = self.period.load(Ordering::Relaxed);
        compiler_fence(Ordering::Acquire);
        let counter = TIMER.cnt().read().cnt();
        calc_now(period, counter)
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        unsafe {
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
    }

    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ()) {
        critical_section::with(|cs| {
            let alarm = self.get_alarm(cs, alarm);

            alarm.callback.set(callback as *const ());
            alarm.ctx.set(ctx);
        })
    }

    fn set_alarm(&self, alarm: AlarmHandle, timestamp: INT64U) -> BOOLEAN {
        timer_log!(trace, "set_alarm");
        timer_log!(trace, "set the alarm at {}", timestamp);
        let n = alarm.id() as usize;
        // by noah：check the timestamp. if timestamp is INT64U::MAX, there is no need to set the alarm
        if timestamp == INT64U::MAX {
            // return true to indicate that there is no need to set the alarm, poll can execute directly
            // before return, unset the ccie bit
            critical_section::with(|cs| {
                let alarm = self.get_alarm(cs, alarm);
                alarm.timestamp.set(u64::MAX);
                TIMER.dier().modify(|w| w.set_ccie(n + 1, false));
            });
            return true;
        }
        critical_section::with(|cs| {
            let alarm = self.get_alarm(cs, alarm);
            // by noah：for timestamp is INT64U, so I just copy it here
            // when the timestamp is less than the alarm's timestamp, the alarm will be set.
            // if the timestamp is large than the allarm's timestamp, the alarm will not be set.
            // because the allarm's timestamp has been set in the last call of set_alarm
            // by noah：we will check the timestamp outside this func
            // if timestamp<alarm.timestamp.get(){
            alarm.timestamp.set(timestamp);
            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed the alarm will not fire.
                // Disarm the alarm and return `false` to indicate that.
                TIMER.dier().modify(|w| w.set_ccie(n + 1, false));

                alarm.timestamp.set(u64::MAX);

                return false;
            }

            // Write the CCR value regardless of whether we're going to enable it now or not.
            // This way, when we enable it later, the right value is already set.
            TIMER.ccr(n + 1).write(|w| w.set_ccr(timestamp as u16));

            // Enable it if it'll happen soon. Otherwise, `next_period` will enable it.
            let diff = timestamp - t;
            TIMER.dier().modify(|w| w.set_ccie(n + 1, diff < 0xc000));

            // Reevaluate if the alarm timestamp is still in the future
            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed since we set it, we have a race condition and
                // the alarm may or may not have fired.
                // Disarm the alarm and return `false` to indicate that.
                // It is the caller's responsibility to handle this ambiguity.
                TIMER.dier().modify(|w| w.set_ccie(n + 1, false));

                alarm.timestamp.set(u64::MAX);

                return false;
            }
            // }
            // We're confident the alarm will ring in the future.
            // if timestamp > alarm.timestamp, this func will not set the alarm and will return directly
            true
        })
    }
    unsafe fn on_interrupt(&self) {
        // let r = regs_gp16();

        // XXX: reduce the size of this critical section ?
        critical_section::with(|cs| {
            let sr = TIMER.sr().read();
            let dier = TIMER.dier().read();

            // Clear all interrupt flags. Bits in SR are "write 0 to clear", so write the bitwise NOT.
            // Other approaches such as writing all zeros, or RMWing won't work, they can
            // miss interrupts.
            TIMER.sr().write_value(regs::SrGp16(!sr.0));

            // Overflow
            if sr.uif() {
                self.next_period();
            }

            // Half overflow
            // if sr.ccif(0) {
            //     self.next_period();
            // }

            for n in 0..ALARM_COUNT {
                if sr.ccif(n + 1) && dier.ccie(n + 1) {
                    timer_log!(trace, "the alarm is triggered!!!");
                    self.trigger_alarm(n, cs);
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

// RCC configuration has been moved to platform.rs using HAL library

/// Store clock configuration from HAL library for timer driver use
///
/// This function should be called during platform initialization to provide
/// the timer driver with the current clock configuration.
///
/// # Parameters
/// - `clocks`: Clock configuration from HAL library
pub fn store_clock_config(clocks: &stm32f4xx_hal::rcc::Clocks) {
    // Store system clock frequency
    SYSCLK_HZ().set(clocks.sysclk().raw() as u64);

    // Store APB frequency (used for timer prescaler calculation)
    // Note: TIM3 is on APB1, so we use pclk1
    APB_HZ().set(clocks.pclk1().raw() as u64);
}

/// Get the frequency of the APB bus using stored HAL configuration
pub fn get_APBfreq() {
    // The clock configuration should now be set by store_clock_config()
    // This function is kept for compatibility but now just validates the configuration
    let apb_freq = *APB_HZ().get();
    if apb_freq == 0 {
        panic!("APB frequency not set. Call store_clock_config() first.");
    }

    os_log!(info, "APB frequency: {} Hz", apb_freq);
}

fn enable_Timer() {
    #[cfg(feature = "time_driver_tim1")]
    RCC.apb2enr().modify(|v| v.set_tim1en(ENABLE));
    #[cfg(feature = "time_driver_tim2")]
    RCC.apb1enr().modify(|v| v.set_tim2en(ENABLE));

    // by noah: in current project, we use Timer 3 as the time driver
    #[cfg(feature = "time_driver_tim3")]
    RCC.apb1enr().modify(|v| v.set_tim3en(ENABLE));

    #[cfg(feature = "time_driver_tim4")]
    RCC.apb1enr().modify(|v| v.set_tim4en(ENABLE));
    #[cfg(feature = "time_driver_tim5")]
    RCC.apb1enr().modify(|v| v.set_tim5en(ENABLE));
    #[cfg(feature = "time_driver_tim8")]
    RCC.apb2enr().modify(|v| v.set_tim8en(ENABLE));
    #[cfg(feature = "time_driver_tim9")]
    RCC.apb2enr().modify(|v| v.set_tim9en(ENABLE));
    #[cfg(feature = "time_driver_tim12")]
    RCC.apb1enr().modify(|v| v.set_tim12en(ENABLE));

    // #[cfg(feature = "time_driver_tim15")]

    // #[cfg(feature = "time_driver_tim20")]

    // #[cfg(feature = "time_driver_tim21")]

    // #[cfg(feature = "time_driver_tim22")]

    // #[cfg(feature = "time_driver_tim23")]

    // #[cfg(feature = "time_driver_tim24")]

    #[cfg(not(any(
        feature = "time_driver_tim1",
        feature = "time_driver_tim2",
        feature = "time_driver_tim3",
        feature = "time_driver_tim4",
        feature = "time_driver_tim5",
        feature = "time_driver_tim8",
        feature = "time_driver_tim9",
        feature = "time_driver_tim12",
    )))]
    panic!("the Timer is not surpport. You can add it in Func enable_Timer()");
}

// by noah: here the period is shifted 15 bit because period will be increased when the counter is overflowed or half-overflowed
fn calc_now(period: INT32U, counter: INT16U) -> INT64U {
    ((period as INT64U) << 15) + ((counter as u32 ^ ((period & 1) << 15)) as u64)
}

// Old get_APBfreq() implementation removed - now using HAL library configuration