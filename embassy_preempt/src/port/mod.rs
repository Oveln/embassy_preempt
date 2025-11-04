#![allow(non_camel_case_types)]

use embassy_preempt_platform::{PLATFORM, Platform};
pub use embassy_preempt_platform::{BOOLEAN, INT8U, INT16U, INT32U, INT64U, OS_STK, OS_CPU_SR, USIZE};

/*
**************************************************************************************************************************************
*                                                               type define
*                                           this part needs to change according to the platform
**************************************************************************************************************************************
*/
/// ENABLE
pub const ENABLE: bool = true;
/// DISABLE
pub const DISENABLE: bool = false;
// /// Each stack entry is 32-bit wide
// pub type OS_STK = usize;
// /// Define size of CPU status register (PSR = 32 bits)
// pub type OS_CPU_SR = u32;
/// the timer used as the time Driver
#[cfg(feature = "time_driver_tim1")]
const TIMER: TimGp16 = stm32_metapac::TIM1;
#[cfg(feature = "time_driver_tim2")]
const TIMER: TimGp32 = stm32_metapac::TIM2;
#[cfg(feature = "time_driver_tim3")]
// by noah: in current project, we use Timer 3 as the time driver
/// set the time driver to be Timer3
pub const TIMER: TimGp16 = stm32_metapac::TIM3;
#[cfg(feature = "time_driver_tim4")]
const TIMER: TimGp16 = stm32_metapac::TIM4;
#[cfg(feature = "time_driver_tim5")]
const TIMER: TimGp32 = stm32_metapac::TIM5;
#[cfg(feature = "time_driver_tim8")]
const TIMER: TimGp16 = stm32_metapac::TIM8;
#[cfg(feature = "time_driver_tim9")]
const TIMER: TimGp16 = stm32_metapac::TIM9;
#[cfg(feature = "time_driver_tim12")]
const TIMER: TimGp16 = stm32_metapac::TIM12;
#[cfg(feature = "time_driver_tim15")]
const TIMER: TimGp16 = stm32_metapac::TIM15;
#[cfg(feature = "time_driver_tim20")]
const TIMER: TimGp16 = stm32_metapac::TIM20;
#[cfg(feature = "time_driver_tim21")]
const TIMER: TimGp16 = stm32_metapac::TIM21;
#[cfg(feature = "time_driver_tim22")]
const TIMER: TimGp16 = stm32_metapac::TIM22;
#[cfg(feature = "time_driver_tim23")]
const TIMER: TimGp16 = stm32_metapac::TIM23;
#[cfg(feature = "time_driver_tim24")]
const TIMER: TimGp16 = stm32_metapac::TIM24;

/// the ptr size. define this to use raw ptr
pub type PTR = *mut ();
///the language items
pub mod lang_items;
pub mod os_cpu;
/// the time driver
pub mod time_driver;

/// the bottom driver
pub mod bottom_driver;
/*
********************************************************************************************************************************************
*                                                               critical section
********************************************************************************************************************************************
*/
use critical_section::{set_impl, Impl, RawRestoreState};
#[cfg(feature = "time_driver_tim3")]
use stm32_metapac::timer::TimGp16;

use crate::os_log;

struct SingleCoreCriticalSection;
set_impl!(SingleCoreCriticalSection);

unsafe impl Impl for SingleCoreCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        // Using the platform abstraction for critical section operations
        PLATFORM.enter_critical_section();
        true // Placeholder value - platform specific
    }

    unsafe fn release(was_active: RawRestoreState) {
        // Using the platform abstraction for critical section operations
        PLATFORM.exit_critical_section();
    }
}

/*
********************************************************************************************************************************************
*                                                          core peripherals init
********************************************************************************************************************************************
*/

/// by noah: init the core peripherals. For the task() just can be called **once**, we should init the core peripherals together
pub fn init_core_peripherals() {
    // Use the platform abstraction to initialize core peripherals
    PLATFORM.init_core_peripherals();
}
