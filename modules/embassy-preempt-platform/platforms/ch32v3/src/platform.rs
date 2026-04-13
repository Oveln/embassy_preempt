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
    fn trigger_context_switch() {
        unsafe {
            // qingke::riscv::register::mip::set_ssoft();
            ecall();
        }
    }

    fn set_program_stack_pointer(sp: *mut u8) {
        unsafe {
            asm!(
                "csrw mscratch, a0",
                in("a0") sp
            );
        }
    }

    #[inline(never)]
    fn configure_interrupt_stack(interrupt_stack: *mut u8) {
        unsafe {
            asm!(
                "mv sp, a0",
                "csrrw sp, mscratch, sp",
                "ret",
                in("a0") interrupt_stack
            );
        }
    }

    fn init_task_stack(stk_ref: NonNull<usize>, executor_function: fn()) -> NonNull<usize> {
        scheduler_log!(trace, "init_task_stack for CH32V307");
        let executor_function_ptr = executor_function as *const () as usize;
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function_ptr);
        // Get stack pointer and align to 8-byte boundary
        let ptos = stk_ref.as_ptr() as *mut usize;
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;
        // Reserve space for the context frame
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp = ptos as *mut crate::ucstk::UcStk;

        unsafe {
            (*psp).ra = 0x0000_0721;
            (*psp).sp = 0x0721_0721;
            (*psp).gp = 0x0000_0721;
            (*psp).tp = 0x0000_0721;
            (*psp).t0 = 0x0000_0721;
            (*psp).t1 = 0x0000_0721;
            (*psp).s0 = 0x0000_0721;
            (*psp).s1 = 0x0000_0721;
            (*psp).a0 = 0x0000_0721;
            (*psp).a1 = 0x0000_0721;
            (*psp).a2 = 0x0000_0721;
            (*psp).a3 = 0x0000_0721;
            (*psp).a4 = 0x0000_0721;
            (*psp).a5 = 0x0000_0721;
            (*psp).a6 = 0x0000_0721;
            (*psp).a7 = 0x0000_0721;
            (*psp).s2 = 0x0000_0721;
            (*psp).s3 = 0x0000_0721;
            (*psp).s4 = 0x0000_0721;
            (*psp).s5 = 0x0000_0721;
            (*psp).s6 = 0x0000_0721;
            (*psp).s7 = 0x0000_0721;
            (*psp).s8 = 0x0000_0721;
            (*psp).s9 = 0x0000_0721;
            (*psp).s10 = 0x0000_0721;
            (*psp).s11 = 0x0000_0721;
            (*psp).t3 = 0x0000_0721;
            (*psp).t4 = 0x0000_0721;
            (*psp).t5 = 0x0000_0721;
            (*psp).t6 = 0x0000_0721;

            (*psp).mepc = executor_function_ptr as usize;
            (*psp).mstatus = 0x0000_1800;
        }

        NonNull::new(ptos as *mut usize).unwrap()
    }

    fn enter_idle_state() {}

    fn shutdown() {
        loop {
            wfi();
        }
    }

    unsafe fn get_current_stack_pointer() -> *mut usize {
        qingke::riscv::register::mscratch::read() as *mut usize
    }
}

impl PlatformMemoryLayout for PlatformImpl {
    const STACK_START: usize = 0x2000B800;
    const MAX_PROGRAMS: usize = 10;
    const HEAP_SIZE: usize = 10 * 1024; // 10 KiB
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
