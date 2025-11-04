//! STM32F401RE platform implementation

use core::arch::asm;
use core::cell::Cell;
use core::ptr::NonNull;

use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::register::{
    control::{self, Spsel},
    msp, psp,
};
use cortex_m::Peripherals;

use embassy_preempt_logs::{scheduler_log, task_log};

use crate::{ButtonDriver, Platform, OS_STK};

// Import the timer and GPIO drivers
use super::{Stm32f401reButtonDriver, Stm32f401reTimerDriver};
use super::{APB_HZ, TICK_HZ};

/// STM32F401RE platform implementation
pub struct Stm32f401rePlatform {
    pub timer_driver: &'static Stm32f401reTimerDriver,
    pub button_driver: &'static Stm32f401reButtonDriver,
}

impl Stm32f401rePlatform {
    pub const fn new(
        timer_driver: &'static Stm32f401reTimerDriver,
        button_driver: &'static Stm32f401reButtonDriver,
    ) -> Self {
        Self {
            timer_driver,
            button_driver,
        }
    }
}

impl Platform for Stm32f401rePlatform {
    fn init_platform(&self) {
        // Initialize RCC and clock configuration for STM32F401RE
        rcc_init();
    }

    fn init_core_peripherals(&self) {
        let mut p = unsafe { Peripherals::steal() };

        // Configure AIRCR for 2-bit preempt, 2-bit subpriority (group 2)
        unsafe {
            let aircr = p.SCB.aircr.read();
            let new_aircr = (0x05FA << 16) | (aircr & !(0b1111 << 8)) | (0b101 << 8);
            p.SCB.aircr.write(new_aircr);

            let prio = |preempt: u8| -> u8 { (preempt & 0b11) << 6 }; // sub=0 assumed

            // Set the TIM3 priority as 3
            p.NVIC.set_priority(stm32_metapac::Interrupt::TIM3, prio(3));

            // Set the EXTI13 priority as 1
            p.NVIC.set_priority(stm32_metapac::Interrupt::EXTI15_10, prio(1));

            p.SCB.set_priority(SystemHandler::PendSV, 0xf << 4);
        }
    }

    #[inline]
    fn restore_thread_task(&self) {
        const NVIC_INT_CTRL: u32 = 0xE000ED04;
        const NVIC_PENDSVSET: u32 = 0x10000000;

        unsafe {
            asm!(
                "STR     R1, [R0]",
                in("r0") NVIC_INT_CTRL,
                in("r1") NVIC_PENDSVSET,
            )
        }
    }

    #[inline]
    fn set_program_sp(&self, sp: *mut u32) {
        scheduler_log!(trace, "set program sp to {}", sp);
        unsafe {
            psp::write(sp as u32);
        }
    }

    #[inline(never)]
    fn set_int_change_2_psp(&self, int_ptr: *mut u32) {
        unsafe {
            asm!(
                // fisrt change the MSP
            "MSR msp, r1",
                // then change the control register to use the psp
                "MRS r0, control",
                "ORR r0, r0, #2",
                "MSR control, r0",
                // make sure the function will be inlined as we don't use lr to return
                // // then we need to return to the caller, this time we explicitly use the lr
                "BX lr",
                in("r1") int_ptr,
                options(nostack, preserves_flags),
            )
        }
    }

    #[inline]
    fn init_task_stack(&self, stk_ref: NonNull<OS_STK>) -> NonNull<OS_STK> {
        // Define the context structure as it was in the original code
        #[repr(C, align(4))]
        struct UcStk {
            // below are the remaining part of the task's context
            r4: u32,
            r5: u32,
            r6: u32,
            r7: u32,
            r8: u32,
            r9: u32,
            r10: u32,
            r11: u32,
            r14: u32,
            // below are stored when the interrupt occurs
            r0: u32,
            r1: u32,
            r2: u32,
            r3: u32,
            r12: u32,
            lr: u32,
            pc: u32,
            xpsr: u32,
        }

        const CONTEXT_STACK_SIZE: usize = 17;

        let executor_function_ptr: fn() = || {
            // This would be implemented in the main crate
        };
        let executor_function_ptr = executor_function_ptr as *const () as usize;

        let ptos = stk_ref.as_ptr() as *mut usize;
        // do align with 8 and move the stack pointer down an align size
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize)) };
        let psp = ptos as *mut UcStk;
        // initialize the stack
        unsafe {
            (*psp).r0 = 0;
            (*psp).r1 = 0x01010101;
            (*psp).r2 = 0x02020202;
            (*psp).r3 = 0x03030303;
            (*psp).r4 = 0x04040404;
            (*psp).r5 = 0x05050505;
            (*psp).r6 = 0x06060606;
            (*psp).r7 = 0x07070707;
            (*psp).r8 = 0x08080808;
            (*psp).r9 = 0x09090909;
            (*psp).r10 = 0x10101010;
            (*psp).r11 = 0x11111111;
            (*psp).r12 = 0x12121212;
            (*psp).r14 = 0xFFFFFFFD;
            (*psp).lr = 0;
            (*psp).pc = executor_function_ptr as u32;
            (*psp).xpsr = 0x01000000;
        }
        // return the new stack pointer
        NonNull::new(ptos as *mut OS_STK).unwrap()
    }

    #[inline]
    fn run_idle(&self) {
        task_log!(trace, "platform run idle");
        // After WFE, probe-rs reports that the RTT read pointer has been modified.
        // Therefore, when logging is enabled, avoid WFE in idle to prevent interference.
        // #[cfg(not(log_enabled))]
        // unsafe {
        //     asm!("wfe");
        // }
    }

    fn enter_critical_section(&self) {
        cortex_m::interrupt::disable();
    }

    fn exit_critical_section(&self) {
        unsafe {
            cortex_m::interrupt::enable();
        }
    }

    fn now(&self) -> u64 {
        // self.timer_driver.now()
        0
    }

    fn schedule_wake(&self, at: u64, waker: &core::task::Waker) {
        // Implementation would hook into the timer driver
        _embassy_time_schedule_wake(at, waker);
    }

    fn init_timer_driver(&self) {
        self.timer_driver.init();
    }

    fn init_button_driver(&self) {
        self.button_driver.init();
    }

    fn wait_button(&self) {
        self.button_driver.wait_bottom();
    }
}

// Time schedule wake function (placeholder)
fn _embassy_time_schedule_wake(at: u64, waker: &core::task::Waker) {
    // Implementation would handle scheduling wake events
    // This would be implemented in the main crate
}

// Static platform instance for STM32F401RE
static TIME_DRIVER: Stm32f401reTimerDriver =
    Stm32f401reTimerDriver::new(stm32_metapac::TIM3, APB_HZ as u32, TICK_HZ as u32);

static BUTTON_DRIVER: Stm32f401reButtonDriver = Stm32f401reButtonDriver::new();

/// 预配置的 STM32F401RE 平台实例
pub static PLATFORM: Stm32f401rePlatform = Stm32f401rePlatform::new(&TIME_DRIVER, &BUTTON_DRIVER);

// RCC initialization for STM32F401RE (moved from timer driver)
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
            v.set_pllm(stm32_metapac::rcc::vals::Pllm::DIV4);
            // Set PLLN=84
            v.set_plln(stm32_metapac::rcc::vals::Plln::MUL84);
            // Set PLLP=2
            v.set_pllp(stm32_metapac::rcc::vals::Pllp::DIV2);
            // Set PLLQ=4
            v.set_pllq(stm32_metapac::rcc::vals::Pllq::DIV4);
            // Set the HSE as the PLL source
            v.set_pllsrc(stm32_metapac::rcc::vals::Pllsrc::HSE);
        });

        rcc.cfgr().modify(|v| {
            // Set the frequency division coefficient of AHB as 1
            v.set_hpre(stm32_metapac::rcc::vals::Hpre::DIV1);
            // Set the frequency division coefficient of APB1 as 2
            v.set_ppre1(stm32_metapac::rcc::vals::Ppre::DIV2);
            // Set the frequency division coefficient of APB2 as 1
            v.set_ppre2(stm32_metapac::rcc::vals::Ppre::DIV1);
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
        flash
            .acr()
            .modify(|v| v.set_latency(stm32_metapac::flash::vals::Latency::WS2));
        // Set the system clock as PLL
        rcc.cfgr().modify(|v| v.set_sw(stm32_metapac::rcc::vals::Sw::PLL1_P));
        // Close the HSI
        rcc.cr().modify(|v| v.set_hsion(false));
    }
}
