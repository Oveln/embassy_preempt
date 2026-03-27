//! LED driver for STM32F4 platform
//!
//! Provides simple GPIO-based LED control using PA5 (common on STM32F4 discovery boards).

pub mod driver;

pub use driver::Led;
