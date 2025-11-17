#![no_std]
#![no_main]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub use embassy_preempt_platform::hal as hal;

/// the mod which set the LED
pub mod led;
pub mod led_hal;