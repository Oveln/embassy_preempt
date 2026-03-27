#![no_std]

//! Platform abstraction layer for embassy_preempt RTOS
//!
//! This crate provides a trait-based abstraction for platform-specific
//! functionality, allowing embassy_preempt to run on different microcontroller
//! architectures (Cortex-M, RISC-V, etc.)
//!
//! ## Organization
//!
//! This crate re-exports traits from `embassy-preempt-traits` and provides
//! platform-specific implementations.
//!
//! ## Platform Selection
//!
//! Platforms are selected via Cargo features:
//! - `stm32f401re`: STM32F401RE microcontroller support
//! - `jh7110`: JH7110 RISC-V SoC support
//! - `ch32v307wcu6`: CH32V307WCU6 microcontroller support

#[macro_use]
extern crate embassy_preempt_log;

use spin::Once;

// Re-export traits from embassy-preempt-traits
pub use embassy_preempt_traits::{OsStk, Platform, PlatformStatic};

// ===== PLATFORM SELECTION =====
// Each platform is implemented in a separate sub-crate and re-exported here

// STM32F4 platform (ARM Cortex-M)
#[cfg(feature = "stm32f401re")]
pub use stm32f4 as chip;

// JH7110 platform (RISC-V 64)
#[cfg(feature = "jh7110")]
pub use jh7110 as chip;

// CH32V3 platform (Qingke RISC-V)
#[cfg(feature = "ch32v307wcu6")]
pub use ch32v3 as chip;

// ===== RE-EXPORTS =====

// Re-export timer driver for the selected platform
pub use chip::timer_driver;

// Re-export platform implementation
pub use chip::PlatformImpl;

// ===== PLATFORM INSTANCE =====

static __PLATFORM: Once<PlatformImpl> = Once::new();

pub fn init_platform() -> Result<(), ()> {
    if __PLATFORM.is_completed() {
        Err(())
    } else {
        __PLATFORM.call_once(|| PlatformImpl::new());
        Ok(())
    }
}

#[inline(always)]
pub fn get_platform() -> &'static PlatformImpl {
    unsafe {
        __PLATFORM.get_unchecked()
    }
}

#[inline(always)]
pub fn get_platform_trait() -> &'static dyn Platform {
    unsafe {
        __PLATFORM.get_unchecked()
    }
}
