use core::task::{Context, Poll};
use core::pin::Pin;

use crate::PLATFORM;

/// Future that completes when button is pressed
pub struct ButtonFuture {
    yielded_once: bool,
}

impl ButtonFuture {
    /// Create a new button future
    pub fn new() -> Self {
        Self {
            yielded_once: false,
        }
    }
}

impl Default for ButtonFuture {
    fn default() -> Self {
        Self::new()
    }
}

impl core::future::Future for ButtonFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        
        critical_section::with(|cs| {
            let button = PLATFORM.button.borrow(cs);
            // First poll - register waker
            if self.yielded_once {
                os_log!(info, "ButtonFuture::poll: yielded once");
                if button.get_pressed() {
                    return Poll::Ready(());
                }
                Poll::Pending
            } else {
                // Check if button was pressed
                if button.get_pressed() {
                    os_log!(info, "ButtonFuture::poll: button pressed");
                    Poll::Ready(())
                } else {
                    os_log!(info, "ButtonFuture::poll: button not pressed");
                    // Re-register waker and continue pending
                    button.register_waker(cx.waker());
                    self.yielded_once = true;
                    Poll::Pending
                }
            }
        })
    }
}

/// Wait for button press
pub async fn wait_for_button() {
    ButtonFuture::new().await
}