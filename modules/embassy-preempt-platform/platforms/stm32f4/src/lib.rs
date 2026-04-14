#![no_std]
#![feature(naked_functions_rustic_abi)]

//! STM32F4 platform implementation for embassy_preempt RTOS
//!
//! This crate provides platform support for STM32F4xx series microcontrollers,
//! including STM32F401RE and similar devices.
//!
//! ## Features
//!
//! - Platform initialization and HAL integration
//! - PendSV-based context switching
//! - Hardware timer driver (TIM2-TIM5, TIM8-TIM12)
//! - GPIO LED and button drivers
//! - Memory layout configuration
//!
//! ## Module Organization
//!
//! - [`platform`] - Core platform implementation
//! - [`timer_driver`] - Hardware timer driver
//! - [`driver`] - Peripheral drivers (LED, button)
//! - [`ucstk`] - User context and stack definitions

// Re-export from traits crate
pub use embassy_preempt_traits::{
    memory_layout::PlatformMemoryLayout, platform::PlatformStatic, Platform,
};

// Platform modules
pub mod platform;
pub mod timer_driver;
pub mod ucstk;
pub mod stack;

// Driver modules
pub mod driver;

// Public exports
pub use platform::PlatformImpl;
pub use stack::{set_program_stack_pointer, configure_interrupt_stack, get_current_stack_pointer};
