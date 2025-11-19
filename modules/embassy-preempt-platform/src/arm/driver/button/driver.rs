use core::{cell::UnsafeCell, sync::atomic::{AtomicBool, Ordering}};

use critical_section::Mutex;
use stm32f4xx_hal::{gpio::{ExtiPin, Pin}, pac::{self, EXTI, NVIC}, rcc::Rcc, syscfg::SysCfg};

use crate::{PLATFORM, get_platform};


/// Button driver using HAL GPIO but manual EXTI configuration
pub struct Button {
    pin: UnsafeCell<Pin<'C', 13>>,
    pressed: AtomicBool,
    waker: Mutex<UnsafeCell<Option<core::task::Waker>>>,
}

impl Button {
    /// Create a new button driver using PC13
    pub fn new(rcc: &mut Rcc, exti: &mut EXTI, nvic: &mut NVIC, syscfg: &mut SysCfg, pc13: Pin<'C', 13>) -> Self {

        // Enable GPIOC clock
        rcc.ahb1enr().modify(|r, w| w.gpiocen().enabled());

        // Enable SYSCFG clock for EXTI
        rcc.apb2enr().modify(|r, w| w.syscfgen().enabled());

        // Configure PC13 as input with pull-down
        let mut pin = pc13.into_pull_down_input();

        pin.make_interrupt_source(syscfg);
        pin.trigger_on_edge(exti, stm32f4xx_hal::gpio::Edge::Falling);
        pin.enable_interrupt(exti);

        unsafe {
            nvic.set_priority(pac::interrupt::EXTI15_10, 1);
            NVIC::unmask(pac::Interrupt::EXTI15_10);
        }

        Self {
            pin: UnsafeCell::new(pin),
            pressed: AtomicBool::new(false),
            waker: Mutex::new(UnsafeCell::new(None)),
        }
    }

    /// Check if button is currently pressed (high level)
    pub fn is_pressed(&self) -> bool {
        unsafe{(*self.pin.get()).is_high()}
    }

    /// Get the button state and reset if pressed
    pub fn get_pressed(&self) -> bool {
        self.pressed.swap(false, Ordering::Relaxed)
    }

    /// Set button as pressed (called from interrupt)
    pub(crate) fn set_pressed(&self) {
        self.pressed.store(true, Ordering::Relaxed);
    }

    /// Register a waker to be notified when button is pressed
    pub(crate) fn register_waker(&self, waker: &core::task::Waker) {
        os_log!(info, "register_waker");
        critical_section::with(|cs| {
            let waker_ref = unsafe { &mut *self.waker.borrow(cs).get() };
            *waker_ref = Some(waker.clone());
        });
    }
}

/// EXTI15_10 interrupt handler
#[no_mangle]
pub unsafe extern "C" fn EXTI15_10() {
    os_log!(info, "click");
    // let button = BUTTON.as_mut().unwrap();
    critical_section::with(|cs| {
        let button = get_platform().button.borrow(cs);
        let waker = button.waker.borrow(cs).get();
        let waker = (*waker).take();
        if let Some(waker) = waker {
            waker.wake();
        } else {
            os_log!(info, "none");
        }
        button.set_pressed();
        let pin = button.pin.get();
        unsafe {
            (*pin).clear_interrupt_pending_bit();
        }
    });
}