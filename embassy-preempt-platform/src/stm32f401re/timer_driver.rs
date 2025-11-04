//! STM32F401RE Timer Driver Implementation

use core::cell::Cell;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use critical_section::{CriticalSection, Mutex};
use stm32_metapac::timer::{TimGp16, vals};
use stm32_metapac::rcc::vals::{Pllm, Plln, Pllp, Pllq, Pllsrc, Hpre, Ppre, Sw};
use stm32_metapac::flash::vals::Latency;

use crate::{AlarmHandle, AlarmState, TimerDriver};

/// Timer driver implementation for STM32F401RE
pub struct Stm32f401reTimerDriver {
    pub timer: TimGp16,
    pub period: AtomicU32,
    pub alarm_count: AtomicU8,
    pub alarms: Mutex<[AlarmState; 3]>, // Could be configurable based on timer capabilities
    pub apb_hz: u32,
    pub tick_hz: u32,
}

impl Stm32f401reTimerDriver {
    pub const fn new(timer: TimGp16, apb_hz: u32, tick_hz: u32) -> Self {
        const ARRAY_REPEAT_VALUE: AlarmState = AlarmState::new();
        Self {
            timer,
            period: AtomicU32::new(0),
            alarm_count: AtomicU8::new(0),
            alarms: Mutex::new([ARRAY_REPEAT_VALUE; 3]),
            apb_hz,
            tick_hz,
        }
    }

    pub fn init(&self) {
        // rcc config - this will call the RCC initialization
        rcc_init();
        // enable the Timer Driver
        self.enable_timer();

        // disable the Timer
        self.timer.cr1().modify(|w| w.set_cen(false));
        // clear the cnt num
        self.timer.cnt().write(|w| w.set_cnt(0));

        // calculate the psc
        let psc = (self.apb_hz / self.tick_hz) as u32 - 1;
        let psc: u16 = match psc.try_into() {
            Err(_) => panic!("psc division overflow: {}", psc),
            Ok(n) => n,
        };
        // set the psc
        self.timer.psc().write_value(psc) ;
        // set the auto reload value(Timer 3 is u16)
        self.timer.arr().write(|w| w.set_arr(u16::MAX)) ;

        // Set URS, generate update and clear URS
        self.timer.cr1().modify(|w| w.set_urs(vals::Urs::COUNTERONLY)) ;
        self.timer.egr().write(|w| w.set_ug(true)) ;
        self.timer.cr1().modify(|w| w.set_urs(vals::Urs::ANYEVENT)) ;

        // Mid-way point, there will be a cc(capture/compare) interrupt at 0x8000
        self.timer.ccr(0).write(|w| w.set_ccr(0x8000)) ;

        self.timer.dier().write(|w| {
            // Enable overflow
            w.set_uie(true);
            // and half-overflow interrupts
            w.set_ccie(0, true);
        });

        cortex_m::peripheral::NVIC::unpend(stm32_metapac::Interrupt::TIM3);
        // Unpend and unmask TIM3 interrupt
        unsafe {
            core::sync::atomic::compiler_fence(Ordering::SeqCst);
            cortex_m::peripheral::NVIC::unmask(stm32_metapac::Interrupt::TIM3);
        }

        // enable the Timer
        self.timer.cr1().modify(|w| w.set_cen(true));
    }

    fn on_interrupt(&self) {
        critical_section::with(|cs| {
            let sr = unsafe { self.timer.sr().read() };
            let dier = unsafe { self.timer.dier().read() };

            // Clear all interrupt flags. Bits in SR are "write 0 to clear", so write the bitwise NOT.
            // Other approaches such as writing all zeros, or RMWing won't work, they can
            // miss interrupts.
            unsafe { self.timer.sr().write_value(stm32_metapac::timer::regs::SrGp16(!sr.0)) };

            // Overflow
            if sr.uif() {
                self.next_period();
            }

            // Half overflow
            if sr.ccif(0) {
                self.next_period();
            }

            for n in 0..3 {
                if sr.ccif(n + 1) && dier.ccie(n + 1) {
                    self.trigger_alarm(n, cs);
                }
            }
        })
    }

    fn next_period(&self) {
        // We only modify the period from the timer interrupt, so we know this can't race.
        let period = self.period.load(Ordering::Relaxed) + 1;
        self.period.store(period, Ordering::Relaxed);
        let t = (period as u64) << 15;

        critical_section::with(move |cs| {
            unsafe {
                self.timer.dier().modify(move |w| {
                    for n in 0..3 {
                        let alarm = &self.alarms.borrow(cs)[n];
                        let at = alarm.timestamp.get();

                        if at < t + 0xc000 {
                            // just enable it. `set_alarm` has already set the correct CCR val.
                            w.set_ccie(n + 1, true);
                        }
                    }
                })
            }
        })
    }

    fn get_alarm<'cs>(&'cs self, cx: CriticalSection<'cs>, alarm: AlarmHandle) -> &'cs AlarmState {
        // safety: we're allowed to assume the AlarmState is created by us, and
        // we never create one that's out of bounds.
        // unsafe { self.alarms.get_unchecked(alarm.id() as usize) }
        // &self.alarms.borrow(cx)[alarm.id() as usize]
        unsafe { self.alarms.borrow(cx).get_unchecked(alarm.id() as usize) }
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

    fn enable_timer(&self) {
        // by noah: in current project, we use Timer 3 as the time driver
        let rcc = unsafe { stm32_metapac::rcc::Rcc::from_ptr(stm32_metapac::RCC.as_ptr() as *mut ()) };
        rcc.apb1enr().modify(|v| v.set_tim3en(true));
    }
}

impl TimerDriver for Stm32f401reTimerDriver {
    fn now(&self) -> u64 {
        let period = self.period.load(Ordering::Relaxed);
        core::sync::atomic::compiler_fence(Ordering::Acquire);
        let counter = unsafe { self.timer.cnt().read().cnt() };
        calc_now(period, counter)
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        critical_section::with(|_| {
            let id = self.alarm_count.load(Ordering::Relaxed);
            if id < 3 { // 3 as default alarm count for this timer
                self.alarm_count.store(id + 1, Ordering::Relaxed);
                Some(AlarmHandle::new(id))
            } else {
                None
            }
        })
    }

    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ()) {
        critical_section::with(|cs| {
            let alarm_ref = self.get_alarm(cs, alarm);

            alarm_ref.callback.set(callback as *const ());
            alarm_ref.ctx.set(ctx);
        })
    }

    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool {
        let n = alarm.id() as usize;
        // by noah：check the timestamp. if timestamp is u64::MAX, there is no need to set the alarm
        if timestamp == u64::MAX {
            // return true to indicate that there is no need to set the alarm, poll can execute directly
            // before return, unset the ccie bit
            critical_section::with(|cs| {
                let alarm_ref = self.get_alarm(cs, alarm);
                alarm_ref.timestamp.set(u64::MAX);
                unsafe { self.timer.dier().modify(|w| w.set_ccie(n + 1, false)) };
            });
            return true;
        }
        let result = critical_section::with(|cs| {
            let alarm_ref = self.get_alarm(cs, alarm);
            // by noah：for timestamp is u64, so I just copy it here
            // when the timestamp is less than the alarm's timestamp, the alarm will be set.
            // if the timestamp is large than the allarm's timestamp, the alarm will not be set.
            // because the allarm's timestamp has been set in the last call of set_alarm
            // by noah：we will check the timestamp outside this func
            // if timestamp<alarm.timestamp.get(){
            alarm_ref.timestamp.set(timestamp);
            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed the alarm will not fire.
                // Disarm the alarm and return `false` to indicate that.
                unsafe { self.timer.dier().modify(|w| w.set_ccie(n + 1, false)) };

                alarm_ref.timestamp.set(u64::MAX);

                return false;
            }

            // Write the CCR value regardless of whether we're going to enable it now or not.
            // This way, when we enable it later, the right value is already set.
            unsafe { self.timer.ccr(n + 1).write(|w| w.set_ccr(timestamp as u16)) };

            // Enable it if it'll happen soon. Otherwise, `next_period` will enable it.
            let diff = timestamp - t;
            unsafe { self.timer.dier().modify(|w| w.set_ccie(n + 1, diff < 0xc000)) };

            // Reevaluate if the alarm timestamp is still in the future
            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed since we set it, we have a race condition and
                // the alarm may or may not have fired.
                // Disarm the alarm and return `false` to indicate that.
                // It is the caller's responsibility to handle this ambiguity.
                unsafe { self.timer.dier().modify(|w| w.set_ccie(n + 1, false)) };

                alarm_ref.timestamp.set(u64::MAX);

                return false;
            }
            // We're confident the alarm will ring in the future.
            // if timestamp > alarm.timestamp, this func will not set the alarm and will return directly
            true
        });
        result
    }
}

// Calculate current time based on period and counter
fn calc_now(period: u32, counter: u16) -> u64 {
    ((period as u64) << 15) + ((counter as u32 ^ ((period & 1) << 15)) as u64)
}

// RCC initialization for STM32F401RE
fn rcc_init() {
    unsafe {
        let rcc = stm32_metapac::rcc::Rcc::from_ptr(stm32_metapac::RCC.as_ptr() as *mut ());

        rcc.cr().modify(|v| {
            // Disable PLL
            v.set_pllon(false);
            // Disable PLL2S
            v.set_plli2son(false);
        });

        rcc.pllcfgr().modify(|v| {
            // Set PLLM=4
            v.set_pllm(Pllm::DIV4);
            // Set PLLN=84
            v.set_plln(Plln::MUL84);
            // Set PLLP=2
            v.set_pllp(Pllp::DIV2);
            // Set PLLQ=4
            v.set_pllq(Pllq::DIV4);
            // Set the HSE as the PLL source
            v.set_pllsrc(Pllsrc::HSE);
        });

        rcc.cfgr().modify(|v| {
            // Set the frequency division coefficient of AHB as 1
            v.set_hpre(Hpre::DIV1);
            // Set the frequency division coefficient of APB1 as 2
            v.set_ppre1(Ppre::DIV2);
            // Set the frequency division coefficient of APB2 as 1
            v.set_ppre2(Ppre::DIV1);
        });

        rcc.cr().modify(|v| {
            // Enable the HSE
            v.set_hseon(true);
            // Enable the PLL
            v.set_pllon(true);
            // Enable the PLL2S
            v.set_plli2son(true);
        });

        // Check the state of HSE, PLL, PLL2S
        while !rcc.cr().read().hserdy() || !rcc.cr().read().pllrdy() || !rcc.cr().read().plli2srdy() {}

        // Enable FLASH prefetch buffer
        let flash = stm32_metapac::flash::Flash::from_ptr(stm32_metapac::FLASH.as_ptr() as *mut ());
        flash.acr().modify(|v| v.set_prften(true));
        // Set the wait state of FLASH as 2
        flash.acr().modify(|v| v.set_latency(Latency::WS2));
        // Set the system clock as PLL
        rcc.cfgr().modify(|v| v.set_sw(Sw::PLL1_P));
        // Close the HSI
        rcc.cr().modify(|v| v.set_hsion(false));
    }
}