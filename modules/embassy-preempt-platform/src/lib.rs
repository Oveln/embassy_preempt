#![no_std]

//! Platform abstraction layer for embassy_preempt RTOS
//!
//! This crate provides a trait-based abstraction for platform-specific
//! functionality, allowing embassy_preempt to run on different microcontroller
//! architectures (Cortex-M, RISC-V, etc.)
//!
//! ## Organization
//!
//! - [`traits`]: Platform trait definitions
//!   - [`platform`]: Core platform functionality trait
//!   - [`timer`]: Timer driver trait
//! - [`stm32f401re`]: STM32F401RE platform implementation
//!
//! ## Platform Implementations
//!
//! - [`stm32f401re`]: STM32F401RE microcontroller support with timer driver

// mod critical_section;

#[macro_use]
extern crate embassy_preempt_log;

// Declare modules
pub mod traits;

use spin::Once;

// Re-export traits for convenience
pub use traits::Platform;

// ===== PLATFORM SELECTION =====

// ARM Cortex-M platforms
#[cfg(all(feature = "arm", feature = "cortex-m"))]
pub mod arm;

#[cfg(all(feature = "arm", feature = "cortex-m"))]
pub use arm as arch;

#[cfg(feature = "stm32f4xx")]
pub use stm32f4xx_hal as hal;

// STM32F401RE platform
#[cfg(feature = "stm32f401re")]
pub use arch::chip::stm32f401re as chip;

#[cfg(feature = "stm32f401re")]
pub use stm32_metapac as pac;

// RISC-V platforms (placeholder for future implementation)
#[cfg(all(feature = "riscv", feature = "riscv32"))]
pub mod riscv;

#[cfg(all(feature = "riscv", feature = "riscv32"))]
pub use riscv as arch;

// ===== RE-EXPORTS =====

// Re-export panic handler for the selected architecture
#[cfg(any(feature = "stm32f401re", all(feature = "riscv", feature = "riscv32")))]
pub use arch::panic_handler;

pub use arch::driver as driver;

pub type OsStk = <chip::PlatformImpl as Platform>::OsStk;

// Re-export timer driver for supported platforms
pub use chip::timer_driver as timer_driver;


// ===== PLATFORM INSTANCE =====

static __PLATFORM: Once<chip::PlatformImpl> = Once::new();

pub fn get_platform() -> &'static chip::PlatformImpl {
    __PLATFORM.call_once(|| chip::PlatformImpl::new())
}
pub use get_platform as PLATFORM;