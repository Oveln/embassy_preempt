use crate::Platform;

pub struct PlatformImpl {

}

impl PlatformImpl {
    pub fn new() -> Self {
        PlatformImpl {  }
    }
}

impl Platform for PlatformImpl {
    type OsStk = usize;

    fn trigger_context_switch(&'static self) {
        todo!()
    }

    fn set_program_stack_pointer(&'static self, sp: *mut u8) {
        todo!()
    }

    fn configure_interrupt_stack(&'static self, interrupt_stack: *mut u8) {
        todo!()
    }

    fn init_task_stack(&'static self, stk_ref: core::ptr::NonNull<Self::OsStk>, executor_function: fn()) -> core::ptr::NonNull<Self::OsStk> {
        todo!()
    }

    fn enter_idle_state(&'static self) {
        todo!()
    }

    fn shutdown(&'static self) {
        todo!()
    }

    unsafe fn save_task_context(&'static self) {
        todo!()
    }

    unsafe fn restore_task_context(&'static self, stack_pointer: *mut usize, interrupt_stack: *mut usize, return_value: u32) {
        todo!()
    }

    unsafe fn get_current_stack_pointer(&'static self) -> *mut usize {
        todo!()
    }

    fn get_timer_driver(&'static self) -> &'static dyn crate::traits::timer::Driver {
        todo!()
    }
}
