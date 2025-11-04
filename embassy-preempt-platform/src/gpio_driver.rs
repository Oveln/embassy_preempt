//! GPIO driver trait definition

/// GPIO/External Interrupt driver functionality
pub trait GpioDriver {
    /// Initialize the GPIO driver
    fn init(&self);

    /// Set task for bottom driver
    fn set_task(&self, task: *mut ());

    /// Wait for a bottom event (button press)
    fn wait_bottom(&self);
}