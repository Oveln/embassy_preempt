//! Async button waiter implementation

use core::{future::Future, pin::Pin, task::{Context, Poll}};

use crate::ButtonDriver;

/// Async button waiter
pub struct Button<T: ButtonDriver + 'static> {
    driver: &'static T,
    yielded_once: bool,
}

impl<T: ButtonDriver + 'static> Future for Button<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded_once {
            Poll::Ready(())
        } else {
            // Add the button task to the button driver
            // Note: This needs to be adapted to work with the waker system
            // For now, we'll use a placeholder approach
            self.driver.set_task(cx.waker().as_ref() as *const _ as *mut ());
            self.yielded_once = true;
            Poll::Pending
        }
    }
}

impl<T: ButtonDriver + 'static> Button<T> {
    /// Create a new button waiter
    pub fn wait_for_press(driver: &'static T) -> Self {
        Self {
            driver,
            yielded_once: false,
        }
    }
}