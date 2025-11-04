//! Button driver trait definition

/// Button/External Interrupt driver functionality
pub trait ButtonDriver {
    /// Initialize the button driver
    fn init(&self);

    /// Set task for button driver
    fn set_task(&self, task: *mut ());

    /// Handle button interrupt
    fn on_interrupt(&self);

    /// Configure GPIO pin for button input
    fn configure_gpio_pin(&self);

    /// Configure external interrupt
    fn configure_exti_interrupt(&self);

    /// Wait for button press (blocking call)
    fn wait_bottom(&self);
}

/// Button driver state
pub struct ButtonState {
    /// the task await on the button(for now, only one task)
    pub task: core::cell::UnsafeCell<Option<*mut ()>>,
    /// the callback func of the button
    #[allow(unused)]
    pub callback: core::cell::Cell<*const ()>,
    /// the argument to the callback
    #[allow(unused)]
    pub ctx: core::cell::Cell<*mut ()>,
}

unsafe impl Sync for ButtonState {}

impl ButtonState {
    pub const fn new() -> Self {
        Self {
            task: core::cell::UnsafeCell::new(None),
            callback: core::cell::Cell::new(core::ptr::null()),
            ctx: core::cell::Cell::new(core::ptr::null_mut()),
        }
    }
}

/// Constants for interrupt control
pub const ENABLE: bool = true;
pub const DISENABLE: bool = false;