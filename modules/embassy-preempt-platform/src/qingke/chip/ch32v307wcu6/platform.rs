use core::arch::asm;
use core::ptr::NonNull;

use qingke::riscv::asm::{ecall, wfi};

use crate::chip::ucstk::CONTEXT_STACK_SIZE;
use crate::traits::memory_layout::PlatformMemoryLayout;
use crate::traits::platform::PlatformStatic;
use crate::Platform;

pub struct PlatformImpl {
    pub timer: crate::qingke::chip::ch32v307wcu6::timer_driver::Ch32v307Timer,
}

impl PlatformImpl {
    pub fn new() -> Self {
        os_log!(info, "Init Platform");
        unsafe {
            asm!("li t0, 0x0", "csrw 0x804, t0",);
        }
        let timer = crate::qingke::chip::ch32v307wcu6::timer_driver::Ch32v307Timer {};
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

    #[inline(always)]
    unsafe fn save_task_context() {
        core::arch::asm!(
            // 保存通用寄存器（跳过 x0）
            "csrrw sp, mscratch, sp",
            "addi sp, sp, -132",
            "sw x1, 0(sp)",
            "sw x2, 4(sp)",
            "sw x3, 8(sp)",
            "sw x4, 12(sp)",
            "sw x5, 16(sp)",
            "sw x6, 20(sp)",
            "sw x7, 24(sp)",
            "sw x8, 28(sp)",
            "sw x9, 32(sp)",
            "sw x10, 36(sp)",
            "sw x11, 40(sp)",
            "sw x12, 44(sp)",
            "sw x13, 48(sp)",
            "sw x14, 52(sp)",
            "sw x15, 56(sp)",
            "sw x16, 60(sp)",
            "sw x17, 64(sp)",
            "sw x18, 68(sp)",
            "sw x19, 72(sp)",
            "sw x20, 76(sp)",
            "sw x21, 80(sp)",
            "sw x22, 84(sp)",
            "sw x23, 88(sp)",
            "sw x24, 92(sp)",
            "sw x25, 96(sp)",
            "sw x26, 100(sp)",
            "sw x27, 104(sp)",
            "sw x28, 108(sp)",
            "sw x29, 112(sp)",
            "sw x30, 116(sp)",
            "sw x31, 120(sp)",
            // 保存 mepc, mstatus 到最后两个 word（使用 t0 临时寄存器）
            "csrr t0, mepc",
            "sw t0, 124(sp)",
            "csrr t0, mstatus",
            "sw t0, 128(sp)",
            "csrrw sp, mscratch, sp",
            // 禁用硬件压栈
            // "li t0, 0x23",
            // "csrw 0x804, t0",
        );
    }
    #[inline(always)]
    unsafe fn restore_task_context(stack_pointer: *mut usize, interrupt_stack: *mut usize, _return_value: u32) {
        core::arch::asm!(
            "csrw mscratch, a1",
            "mv sp, a0",
            "lw x1, 0(sp)",
            "lw x3, 8(sp)",
            "lw x4, 12(sp)",
            "lw x5, 16(sp)",
            "lw x6, 20(sp)",
            "lw x7, 24(sp)",
            "lw x8, 28(sp)",
            "lw x9, 32(sp)",
            // "lw x10, 36(sp)",
            "lw x11, 40(sp)",
            "lw x12, 44(sp)",
            "lw x13, 48(sp)",
            "lw x14, 52(sp)",
            "lw x15, 56(sp)",
            "lw x16, 60(sp)",
            "lw x17, 64(sp)",
            "lw x18, 68(sp)",
            "lw x19, 72(sp)",
            "lw x20, 76(sp)",
            "lw x21, 80(sp)",
            "lw x22, 84(sp)",
            "lw x23, 88(sp)",
            "lw x24, 92(sp)",
            "lw x25, 96(sp)",
            "lw x26, 100(sp)",
            "lw x27, 104(sp)",
            "lw x28, 108(sp)",
            "lw x29, 112(sp)",
            "lw x30, 116(sp)",
            "lw x31, 120(sp)",

            "lw a0, 124(sp)",
            "addi a0, a0, 4",
            "csrw mepc, a0",
            "lw a0, 128(sp)",
            "csrw mstatus, a0",

            "lw x10, 36(sp)",  //a0
            "addi sp, sp, 132",

            "mret",

            in("a0") stack_pointer,
            in("a1") interrupt_stack,
            options(noreturn)
        );
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
        scheduler_log!(trace, "init_task_stack");
        let executor_function_ptr = executor_function as *const () as usize;
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function_ptr);
        // Get stack pointer and align to 8-byte boundary
        let ptos = stk_ref.as_ptr() as *mut usize;
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;
        // Reserve space for the context frame
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp = ptos as *mut crate::chip::ucstk::UcStk;

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
    fn get_stack_start() -> usize {
        0x2000B800
    }

    fn get_max_programs() -> usize {
        10
    }

    fn get_heap_size() -> usize {
        10 * 1024 // 10 KiB
    }

    fn get_program_stack_size() -> usize {
        // 2048 // 2 KiB
        4096
    }

    fn get_interrupt_stack_size() -> usize {
        // 2048 // 2 KiB
        4096
    }
}

impl Platform for PlatformImpl {
    fn get_timer_driver(&'static self) -> &'static dyn crate::traits::timer::Driver {
        &self.timer
    }
}
