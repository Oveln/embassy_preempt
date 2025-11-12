/*
*********************************************************************************************************
*                                 Platform Timer Driver - STM32F401RE实现
*********************************************************************************************************
*/
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

// Timer constants for STM32F401RE
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

// Type aliases for consistency
pub type BOOLEAN = bool;
pub type INT16U = u16;
pub type INT32U = u32;
pub type INT64U = u64;
pub type INT8U = u8;
pub type USIZE = usize;

// Debug GPIO placeholders
#[cfg(feature = "early_debug_gpio")]
fn interrupt_pin_high() {
    // TODO: 实现调试GPIO功能
}

#[cfg(feature = "early_debug_gpio")]
fn interrupt_pin_low() {
    // TODO: 实现调试GPIO功能
}

#[cfg(not(feature = "early_debug_gpio"))]
/// 占位符函数 - 当没有调试GPIO时使用
#[inline(always)]
fn interrupt_pin_high() {}

#[cfg(not(feature = "early_debug_gpio"))]
/// 占位符函数 - 当没有调试GPIO时使用
#[inline(always)]
fn interrupt_pin_low() {}

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
// define the Alarm Interrupt
#[cfg(feature = "time_driver_tim3")]
#[unsafe(no_mangle)]
/// TIM3 interrupt handler
pub extern "C" fn TIM3() {
    interrupt_pin_high();

    os_log!(trace, "TIM3");
    RTC_DRIVER.on_interrupt();

    os_log!(trace, "exit TIM3");
    interrupt_pin_low();
}

/*
*********************************************************************************************************
*                                           type definitions
*********************************************************************************************************
*/
const DISABLE: bool = false;
const ENABLE: bool = true;

pub struct RtcDriver {
    /// Number of 2^15 periods elapsed since boot.
    period: AtomicU32,
    alarm_count: AtomicU8,
    /// Timestamp at which to fire alarm. u64::MAX if no alarm is scheduled.
    alarms: Mutex<[AlarmState; ALARM_COUNT]>,
    #[cfg(feature = "low-power")]
    rtc: Mutex<CriticalSectionRawMutex, Cell<Option<&'static Rtc>>>,
}

/*
*********************************************************************************************************
*                                              implentations
*********************************************************************************************************
*/

impl RtcDriver {
    pub fn init(&'static self) {
        os_log!(trace, "init of RtcDriver");
        // rcc config（need to configure RCC according to your own chip）
        rcc_init();
        // enable the Timer Driver
        enable_Timer();

        get_APBfreq();

        // disable the Timer
        TIMER.cr1().modify(|w| w.set_cen(false));
        // clear the cnt num
        TIMER.cnt().write(|w| w.set_cnt(0));

        // calculate the psc
        let psc = (*APB_HZ.get() / TICK_HZ) as INT32U - 1;
        let psc: INT16U = match psc.try_into() {
            Err(_) => panic!("psc division overflow: {}", psc),
            Ok(n) => n,
        };
        // set the psc
        TIMER.psc().write_value(psc);
        // set the auto reload value(Timer 3 is INT16U)
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

    fn on_interrupt(&self) {
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
            if sr.ccif(0) {
                self.next_period();
            }

            for n in 0..ALARM_COUNT {
                if sr.ccif(n + 1) && dier.ccie(n + 1) {
                    timer_log!(trace, "the alarm is triggered!!!");
                    self.trigger_alarm(n, cs);
                }
            }
        })
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
}

/*
*********************************************************************************************************
*                                           var declaration
*********************************************************************************************************
*/

#[allow(clippy::declare_interior_mutable_const)]
const ALARM_STATE_NEW: AlarmState = AlarmState::new();

/// the global RTC driver
pub static RTC_DRIVER: RtcDriver = RtcDriver {
    period: AtomicU32::new(0),
    alarm_count: AtomicU8::new(0),
    alarms: Mutex::new([ALARM_STATE_NEW; ALARM_COUNT]),
    #[cfg(feature = "low-power")]
    rtc: Mutex::const_new(CriticalSectionRawMutex::new(), Cell::new(None)),
};

/*
*********************************************************************************************************
*                                           auxiliary function
*********************************************************************************************************
*/

/// set the rcc of the Timer
fn rcc_init() {
    RCC.cr().modify(|v| {
        // disable PLL
        v.set_pllon(DISABLE);
        // disable PLL2S
        v.set_plli2son(DISABLE);
    });
    RCC.pllcfgr().modify(|v| {
        // set PLLM=4
        v.set_pllm(Pllm::DIV4);
        // set PLLN=84
        v.set_plln(Plln::MUL84);
        // set PLLP=2
        v.set_pllp(Pllp::DIV2);
        // set PLLQ=4
        v.set_pllq(Pllq::DIV4);
        // set the HSE as the PLL source
        v.set_pllsrc(Pllsrc::HSE);
    });
    RCC.cfgr().modify(|v| {
        // set the frequency division coefficient of AHB as 1
        v.set_hpre(Hpre::DIV1);
        // set the frequency division coefficient of APB1 as 2
        v.set_ppre1(Ppre::DIV2);
        // set the frequency division coefficient of APB2 as 1
        v.set_ppre2(Ppre::DIV1);
    });
    RCC.cr().modify(|v| {
        // enable the HSE
        v.set_hseon(ENABLE);
        // enable the PLL
        v.set_pllon(ENABLE);
        // enable the PLL2S
        v.set_plli2son(ENABLE);
    });
    // check the state of HSE, PLL, PLL2S
    while !RCC.cr().read().hserdy() || !RCC.cr().read().pllrdy() || !RCC.cr().read().plli2srdy() {}
    // enable FLASH prefetch buffer
    FLASH.acr().modify(|v| v.set_prften(ENABLE));
    // set the wait state of FLASH as 2
    FLASH.acr().modify(|v| v.set_latency(Latency::WS2));
    // set the system clock as PLL
    RCC.cfgr().modify(|v| v.set_sw(Sw::PLL1_P));
    // close the HSI
    RCC.cr().modify(|v| v.set_hsion(DISABLE));
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

/// Get the frequency of the APB bus
pub fn get_APBfreq() {
    let SYSCLK = match RCC.cfgr().read().sw() {
        Sw::HSI => 16_000_000,
        Sw::HSE => 8_000_000,
        Sw::PLL1_P => {
            let pllm = RCC.pllcfgr().read().pllm().to_bits() as u32;
            let plln = RCC.pllcfgr().read().plln().0 as u32;
            let pllp = 2 * (RCC.pllcfgr().read().pllp() as u32 + 1);
            let vco_input;
            if RCC.cr().read().hseon() && RCC.cr().read().hserdy() {
                vco_input = 8_000_000 / pllm;
            } else {
                vco_input = 16_000_000 / pllm;
            }
            let vco_output = vco_input * plln;
            vco_output / pllp
        }
        _ => 16_000_000,
    };
    SYSCLK_HZ.set(SYSCLK as u64);

    let AHB_PSC = match RCC.cfgr().read().hpre().to_bits() {
        0b0000 => 1,
        0b1000 => 2,
        0b1001 => 4,
        0b1010 => 8,
        0b1011 => 16,
        0b1100 => 64,
        0b1101 => 128,
        0b1110 => 256,
        0b1111 => 512,
        _ => 1,
    };
    APB_HZ.set((SYSCLK / AHB_PSC) as u64);
}

/*
*********************************************************************************************************
*                                           Public API
*********************************************************************************************************
*/

/// 初始化Timer驱动
/// 这个函数应该在系统启动早期调用，在RTOS内核启动前
pub fn init_timer() {
    RTC_DRIVER.init();
}

/// 获取当前时间戳
pub fn get_current_time() -> u64 {
    RTC_DRIVER.now()
}

/// 获取APB总线频率
pub fn get_apb_frequency() -> u64 {
    get_APBfreq();
    *APB_HZ.get()
}
