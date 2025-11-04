//! STM32F401RE platform implementation

// Re-export the platform components
pub mod platform;
pub mod timer_driver;
pub mod gpio_driver;
pub mod cfg;

pub use platform::{Stm32f401rePlatform, PLATFORM};
pub use timer_driver::{Stm32f401reTimerDriver};
pub use gpio_driver::{Stm32f401reGpioDriver};
pub use cfg::*;