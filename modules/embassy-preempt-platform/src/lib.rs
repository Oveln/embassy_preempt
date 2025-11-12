#![no_std]

//! Platform abstraction layer for embassy_preempt RTOS
//!
//! This crate provides a trait-based abstraction for platform-specific
//! functionality, allowing embassy_preempt to run on different microcontroller
//! architectures (Cortex-M, RISC-V, etc.)
//!
//! ## Organization
//!
//! - [`types`]: Common type definitions and structures
//! - [`platform`]: Core platform functionality trait
//! - [`timer_driver`]: Timer driver trait
//! - [`gpio_driver`]: GPIO driver trait
//! - [`time_driver`]: Enhanced time driver trait
//!
//! ## Platform Implementations
//!
//! - [`stm32f401re`]: STM32F401RE microcontroller support

#[macro_use]
extern crate embassy_preempt_log;

// Declare modules
pub mod platform;

// Re-export all modules and traits for convenience
pub use platform::*;

// Platform-specific implementations
#[cfg(feature = "stm32f401re")]
pub mod stm32f401re;
#[cfg(feature = "stm32f401re")]
pub use stm32f401re::PLATFORM as PLATFORM;
#[cfg(feature = "stm32f401re")]
pub use stm32_metapac as pac;