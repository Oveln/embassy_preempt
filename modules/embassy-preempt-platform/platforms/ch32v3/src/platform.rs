use core::arch::asm;
use core::ptr::NonNull;

use qingke::riscv::asm::{ecall, wfi};

use crate::ucstk::CONTEXT_STACK_SIZE;
use embassy_preempt_traits::memory_layout::PlatformMemoryLayout;
use embassy_preempt_traits::platform::PlatformStatic;
use embassy_preempt_traits::Platform;

pub struct PlatformImpl {
    pub timer: crate::timer_driver::Ch32v307Timer,
}

impl PlatformImpl {
    pub fn new() -> Self {
        os_log!(info, "Init CH32V307 Platform");
        unsafe {
            asm!("li t0, 0x0", "csrw 0x804, t0",);
        }
        let timer = crate::timer_driver::Ch32v307Timer {};
        PlatformImpl { timer }
    }
}

impl PlatformStatic for PlatformImpl {
    // fn trigger_context_switch() {
    //     unsafe {
    //         // qingke::riscv::register::mip::set_ssoft();
    //         ecall();
    //     }
    // }

    fn enter_idle_state() {}

    fn shutdown() {
        loop {
            wfi();
        }
    }

    /// Early putchar - stub implementation for CH32V3
    ///
    /// Currently not implemented. Add UART initialization and output here if needed.
    fn early_putchar(_c: u8) {
        // Stub: no early UART output for CH32V3
    }
}

impl PlatformMemoryLayout for PlatformImpl {
    const PROGRAM_STACK_SIZE: usize = 4096;
    const INTERRUPT_STACK_SIZE: usize = 4096;
}

impl Platform for PlatformImpl {
    fn get_timer_driver(&'static self) -> &'static dyn embassy_preempt_traits::timer::Driver {
        &self.timer
    }

    fn set_ipi_callback(&'static self, _callback: fn(*mut ()), _ctx: *mut ()) {
        // CH32V3 does not support IPI (single core)
    }
}
