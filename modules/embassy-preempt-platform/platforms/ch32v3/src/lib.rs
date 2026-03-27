#![no_std]
#![feature(naked_functions_rustic_abi)]

//! CH32V3 platform implementation for embassy_preempt RTOS
//!
//! This crate provides platform support for CH32V307WCU6 and similar
//! Qingke RISC-V microcontrollers from WCH.

// Re-export from traits crate
pub use embassy_preempt_traits::{
    memory_layout::PlatformMemoryLayout, platform::PlatformStatic, Platform, OsStk,
};

// Platform modules
pub mod platform;
pub mod timer_driver;
pub mod ucstk;

// Public exports
pub use platform::PlatformImpl;
