//! Peripheral drivers for STM32F4 platform
//!
//! This module provides basic peripheral drivers for the STM32F4 platform,
//! including LED and button support.

pub mod button;
pub mod led;

pub use button::driver::Button;
pub use led::driver::Led;
