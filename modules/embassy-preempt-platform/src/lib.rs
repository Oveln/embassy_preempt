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

mod panic_handler;
mod critical_section;

#[macro_use]
extern crate embassy_preempt_log;

// Declare modules
pub mod traits;

// Re-export traits for convenience
pub use traits::Platform;

// Platform-specific implementations
#[cfg(feature = "stm32f401re")]
pub mod stm32f401re;
#[cfg(feature = "stm32f401re")]
pub use stm32f401re::PLATFORM as PLATFORM;
#[cfg(feature = "stm32f401re")]
pub use stm32_metapac as pac;
#[cfg(feature = "stm32f401re")]
pub type OsStk = <stm32f401re::STM32F401RE as Platform>::OsStk;

// Re-export timer driver for STM32F401RE
#[cfg(feature = "stm32f401re")]
pub use stm32f401re::timer_driver as timer_driver;