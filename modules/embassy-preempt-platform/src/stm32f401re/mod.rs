use crate::Platform;
pub struct STM32F401RE {}

pub static PLATFORM: STM32F401RE = STM32F401RE {
};

impl Platform for STM32F401RE {
    type OS_STK = u32;
    fn init_core_peripherals(&'static self) {
        todo!()
    }

    fn restore_thread_task(&'static self) {
        todo!()
    }

    fn set_program_sp(&'static self, sp: *mut u32) {
        todo!()
    }

    fn set_int_change_2_psp(&'static self, int_ptr: *mut u32) {
        todo!()
    }

    fn init_task_stack(&'static self, stk_ref: core::ptr::NonNull<Self::OS_STK>) -> core::ptr::NonNull<Self::OS_STK> {
        todo!()
    }

    fn run_idle(&'static self) {
        todo!()
    }

    fn enter_critical_section(&'static self) {
        todo!()
    }

    fn exit_critical_section(&'static self) {
        todo!()
    }

    fn now(&'static self) -> u64 {
        todo!()
    }

    fn init_timer_driver(&'static self) {
        todo!()
    }
}

pub fn rcc_init() {
    const DISABLE: bool = false;
    const ENABLE: bool = true;
    use stm32_metapac::rcc::vals::*;
    use stm32_metapac::flash::vals::*;
    use stm32_metapac::{FLASH, RCC};
    RCC.cr().modify(|v| {
        v.set_pllon(DISABLE);
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