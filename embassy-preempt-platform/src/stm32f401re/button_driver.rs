//! STM32F401RE Button Driver Implementation

use core::cell::Cell;
use core::sync::atomic::{compiler_fence, Ordering};
use cortex_m::peripheral::NVIC;
use critical_section::Mutex;
use stm32_metapac::{gpio::vals, EXTI, RCC, SYSCFG};
#[cfg(feature = "GPIOC")]
use stm32_metapac::GPIOC;
#[cfg(feature = "GPIOA")]
use stm32_metapac::GPIOA;

use crate::{ButtonDriver, ButtonState, ENABLE, DISENABLE};

/// Button driver implementation for STM32F401RE
pub struct Stm32f401reButtonDriver {
    /// Button state management
    pub button_state: Mutex<ButtonState>,
}

impl Stm32f401reButtonDriver {
    pub const fn new() -> Self {
        Self {
            button_state: Mutex::new(ButtonState::new()),
        }
    }
}

unsafe impl Sync for Stm32f401reButtonDriver {}

impl ButtonDriver for Stm32f401reButtonDriver {
    fn init(&self) {
        // Configure GPIO pin
        self.configure_gpio_pin();

        // Configure external interrupt
        self.configure_exti_interrupt();
    }

    fn set_task(&self, task: *mut ()) {
        critical_section::with(|cs| {
            let button = self.button_state.borrow(cs);
            unsafe {
                *button.task.get() = Some(task);
            }

            // Enable the interrupt and clear pending bits
            #[cfg(feature = "GPIOC")]
            EXTI.pr(0).modify(|w| {
                w.set_line(13, ENABLE)
            });

            #[cfg(feature = "GPIOC")]
            EXTI.imr(0).modify(|w| {
                w.set_line(13, ENABLE)
            });

            #[cfg(feature = "GPIOA")]
            EXTI.imr(0).modify(|w| {
                w.set_line(0, ENABLE)
            });

            #[cfg(feature = "GPIOA")]
            EXTI.pr(0).modify(|w| {
                w.set_line(0, ENABLE)
            });
        });
    }

    fn on_interrupt(&self) {
        // Disable the interrupt
        #[cfg(feature = "GPIOC")]
        EXTI.imr(0).modify(|w| {
            w.set_line(13, DISENABLE)
        });

        // Clear the pending bit in EXTI
        #[cfg(feature = "GPIOC")]
        EXTI.pr(0).modify(|w| {
            w.set_line(13, ENABLE)
        });

        #[cfg(feature = "GPIOA")]
        EXTI.pr(0).modify(|w| {
            w.set_line(0, ENABLE)
        });

        // Clear the pending bit in NVIC
        #[cfg(feature = "GPIOC")]
        NVIC::unpend(stm32_metapac::Interrupt::EXTI15_10);

        #[cfg(feature = "GPIOA")]
        NVIC::unpend(stm32_metapac::Interrupt::EXTI0);

        critical_section::with(|cs| {
            let button = self.button_state.borrow(cs);
            let task: Option<*mut ()>;
            unsafe {
                task = *button.task.get();
            }

            // Wake up the task (implementation specific)
            if let Some(task) = task {
                // Here you would call the wake_task_no_pend function
                // This needs to be adapted to work with the platform abstraction
                unsafe {
                    // wake_task_no_pend(task);
                }
            }

            // Clear the task
            unsafe {
                *button.task.get() = None;
            }
        })
    }

    fn configure_gpio_pin(&self) {
        #[cfg(feature = "GPIOC")]
        {
            // Enable the RCC
            RCC.ahb1enr().modify(|v| {
                v.set_gpiocen(ENABLE);
            });
            // Set GPIO
            GPIOC.moder().modify(|v| {
                v.set_moder(13, vals::Moder::INPUT);
            });
            GPIOC.pupdr().modify(|v| {
                v.set_pupdr(13, vals::Pupdr::PULLDOWN);
            });
        }

        #[cfg(feature = "GPIOA")]
        {
            // Enable the RCC
            RCC.ahb1enr().modify(|v| {
                v.set_gpioaen(true);
            });
            // Set GPIO
            GPIOA.moder().modify(|v| {
                v.set_moder(0, vals::Moder::INPUT);
            });
            GPIOA.pupdr().modify(|v| {
                v.set_pupdr(0, vals::Pupdr::PULLDOWN);
            });
        }
    }

    fn configure_exti_interrupt(&self) {
        // Enable the SYSCFG
        RCC.apb2enr().modify(|v| {
            v.set_syscfgen(ENABLE);
        });

        #[cfg(feature = "GPIOC")]
        {
            // Set PC13 as the source of EXTI13
            SYSCFG.exticr(3).modify(|w| {
                w.set_exti(1, 2);
            });

            EXTI.rtsr(0).modify(|w| {
                w.set_line(13, ENABLE)
            });

            NVIC::unpend(stm32_metapac::Interrupt::EXTI15_10);

            unsafe {
                compiler_fence(Ordering::SeqCst);
                NVIC::unmask(stm32_metapac::Interrupt::EXTI15_10);
            }
        }

        #[cfg(feature = "GPIOA")]
        {
            // Set PA0 as the source of EXTI0
            SYSCFG.exticr(0).modify(|w| {
                w.set_exti(0, 0);
            });

            EXTI.rtsr(0).modify(|w| {
                w.set_line(0, ENABLE)
            });

            NVIC::unpend(stm32_metapac::Interrupt::EXTI0);

            unsafe {
                compiler_fence(Ordering::SeqCst);
                NVIC::unmask(stm32_metapac::Interrupt::EXTI0);
            }
        }
    }

    fn wait_bottom(&self) {
        // This is a blocking wait that would be used in synchronous contexts
        // In practice, this should be integrated with the RTOS task system
        // For now, this is a placeholder implementation
    }
}

// Interrupt handlers
#[no_mangle]
#[cfg(feature = "GPIOC")]
pub extern "C" fn EXTI15_10() {
    // This would need to be integrated with the actual button driver instance
    // For now, this is a placeholder that would need to be connected to the driver
}

#[cfg(feature = "GPIOA")]
#[no_mangle]
pub extern "C" fn EXTI0() {
    // This would need to be integrated with the actual button driver instance
    // For now, this is a placeholder that would need to be connected to the driver
}