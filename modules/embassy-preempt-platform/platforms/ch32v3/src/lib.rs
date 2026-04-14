#![no_std]
#![feature(naked_functions_rustic_abi)]

//! CH32V3 platform implementation for embassy_preempt RTOS
//!
//! This crate provides platform support for CH32V307WCU6 and similar
//! Qingke RISC-V microcontrollers from WCH.

// Re-export from traits crate
pub use embassy_preempt_traits::{
    memory_layout::PlatformMemoryLayout, platform::PlatformStatic, Platform,
};

// Platform modules
pub mod platform;
pub mod timer_driver;
pub mod ucstk;
pub mod stack;

// Public exports
pub use platform::PlatformImpl;
pub use stack::{set_program_stack_pointer, configure_interrupt_stack, get_current_stack_pointer};
