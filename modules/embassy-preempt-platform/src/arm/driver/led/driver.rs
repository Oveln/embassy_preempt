use core::cell::UnsafeCell;
use stm32f4xx_hal::{gpio::{Pin, Output, PushPull}, rcc::Rcc};

/// LED driver using PA5
pub struct Led {
    pin: UnsafeCell<Pin<'A', 5, Output<PushPull>>>,
}

impl Led {
    /// Create a new LED driver using PA5
    pub fn new(rcc: &mut Rcc, pa5: Pin<'A', 5>) -> Self {
        // Enable GPIOA clock
        rcc.ahb1enr().modify(|_r, w| w.gpioaen().enabled());

        // Configure PA5 as output push-pull
        let pin = pa5.into_push_pull_output();

        Self {
            pin: UnsafeCell::new(pin),
        }
    }

    /// Turn LED on
    pub fn on(&self) {
        unsafe {
            (*self.pin.get()).set_high();
        }
    }

    /// Turn LED off
    pub fn off(&self) {
        unsafe {
            (*self.pin.get()).set_low();
        }
    }

    /// Toggle LED state
    pub fn toggle(&self) {
        unsafe {
            (*self.pin.get()).toggle();
        }
    }
}