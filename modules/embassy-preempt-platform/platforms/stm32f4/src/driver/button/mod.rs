//! Button driver for STM32F4 platform
//!
//! Provides button input handling using PC13 with EXTI interrupt support.

pub mod driver;

pub use driver::Button;
