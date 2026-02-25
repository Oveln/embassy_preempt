use core::arch::asm;
use core::ptr::NonNull;

use crate::riscv64::chip::jh7110::ucstk::CONTEXT_STACK_SIZE;
use crate::traits::memory_layout::PlatformMemoryLayout;
use crate::traits::platform::PlatformStatic;
use crate::Platform;

pub struct PlatformImpl {
    pub timer: crate::riscv64::chip::jh7110::timer_driver::Jh7110Timer,
}

impl PlatformImpl {
    pub fn new() -> Self {
        os_log!(info, "Init JH7110 Platform");
        // RISC-V64 初始化代码
        let timer = crate::riscv64::chip::jh7110::timer_driver::Jh7110Timer::new();
        PlatformImpl { timer }
    }
}

impl PlatformStatic for PlatformImpl {
    fn trigger_context_switch() {
        unsafe {
            // 使用 ecall 触发上下文切换
            asm!("ecall");
        }
    }

    #[inline(always)]
    unsafe fn save_task_context() {
        core::arch::asm!(
            // 保存通用寄存器（64位）
            "csrrw sp, mscratch, sp",
            "addi sp, sp, -264",  // 33 * 8 = 264 bytes
            "sd x1, 0(sp)",    // ra
            "sd x2, 8(sp)",    // sp
            "sd x3, 16(sp)",   // gp
            "sd x4, 24(sp)",   // tp
            "sd x5, 32(sp)",   // t0
            "sd x6, 40(sp)",   // t1
            "sd x7, 48(sp)",   // t2
            "sd x8, 56(sp)",   // s0
            "sd x9, 64(sp)",   // s1
            "sd x10, 72(sp)",  // a0
            "sd x11, 80(sp)",  // a1
            "sd x12, 88(sp)",  // a2
            "sd x13, 96(sp)",  // a3
            "sd x14, 104(sp)", // a4
            "sd x15, 112(sp)", // a5
            "sd x16, 120(sp)", // a6
            "sd x17, 128(sp)", // a7
            "sd x18, 136(sp)", // s2
            "sd x19, 144(sp)", // s3
            "sd x20, 152(sp)", // s4
            "sd x21, 160(sp)", // s5
            "sd x22, 168(sp)", // s6
            "sd x23, 176(sp)", // s7
            "sd x24, 184(sp)", // s8
            "sd x25, 192(sp)", // s9
            "sd x26, 200(sp)", // s10
            "sd x27, 208(sp)", // s11
            "sd x28, 216(sp)", // t3
            "sd x29, 224(sp)", // t4
            "sd x30, 232(sp)", // t5
            "sd x31, 240(sp)", // t6
            // 保存 mepc, mstatus 到最后两个 64-bit 位置
            "csrr t0, mepc",
            "sd t0, 248(sp)",
            "csrr t0, mstatus",
            "sd t0, 256(sp)",
            "csrrw sp, mscratch, sp",
        );
    }

    #[inline(always)]
    unsafe fn restore_task_context(stack_pointer: *mut usize, interrupt_stack: *mut usize, _return_value: u32) {
        core::arch::asm!(
            "csrw mscratch, a1",
            "mv sp, a0",
            "sd x1, 0(sp)",    // ra
            "sd x3, 16(sp)",   // gp
            "sd x4, 24(sp)",   // tp
            "sd x5, 32(sp)",   // t0
            "sd x6, 40(sp)",   // t1
            "sd x7, 48(sp)",   // t2
            "sd x8, 56(sp)",   // s0
            "sd x9, 64(sp)",   // s1
            "sd x11, 80(sp)",  // a1
            "sd x12, 88(sp)",  // a2
            "sd x13, 96(sp)",  // a3
            "sd x14, 104(sp)", // a4
            "sd x15, 112(sp)", // a5
            "sd x16, 120(sp)", // a6
            "sd x17, 128(sp)", // a7
            "sd x18, 136(sp)", // s2
            "sd x19, 144(sp)", // s3
            "sd x20, 152(sp)", // s4
            "sd x21, 160(sp)", // s5
            "sd x22, 168(sp)", // s6
            "sd x23, 176(sp)", // s7
            "sd x24, 184(sp)", // s8
            "sd x25, 192(sp)", // s9
            "sd x26, 200(sp)", // s10
            "sd x27, 208(sp)", // s11
            "sd x28, 216(sp)", // t3
            "sd x29, 224(sp)", // t4
            "sd x30, 232(sp)", // t5
            "sd x31, 240(sp)", // t6

            // 恢复 mepc, mstatus
            "ld a0, 248(sp)",
            "addi a0, a0, 4",
            "csrw mepc, a0",
            "ld a0, 256(sp)",
            "csrw mstatus, a0",

            // 恢复 a0 和 sp
            "ld x10, 72(sp)",  // a0
            "addi sp, sp, 264",

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
        scheduler_log!(trace, "init_task_stack for JH7110");
        let executor_function_ptr = executor_function as *const () as usize;
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function_ptr);

        // Get stack pointer and align to 8-byte boundary
        let ptos = stk_ref.as_ptr() as *mut usize;
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;

        // Reserve space for the context frame (264 bytes for RISC-V64)
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp = ptos as *mut crate::riscv64::chip::jh7110::ucstk::UcStk;

        unsafe {
            (*psp).ra = 0x0000_0721_0721_0721;
            (*psp).sp = 0x0721_0721_0721_0721;
            (*psp).gp = 0x0000_0721_0721_0721;
            (*psp).tp = 0x0000_0721_0721_0721;
            (*psp).t0 = 0x0000_0721_0721_0721;
            (*psp).t1 = 0x0000_0721_0721_0721;
            (*psp).t2 = 0x0000_0721_0721_0721;
            (*psp).s0 = 0x0000_0721_0721_0721;
            (*psp).s1 = 0x0000_0721_0721_0721;
            (*psp).a0 = 0x0000_0721_0721_0721;
            (*psp).a1 = 0x0000_0721_0721_0721;
            (*psp).a2 = 0x0000_0721_0721_0721;
            (*psp).a3 = 0x0000_0721_0721_0721;
            (*psp).a4 = 0x0000_0721_0721_0721;
            (*psp).a5 = 0x0000_0721_0721_0721;
            (*psp).a6 = 0x0000_0721_0721_0721;
            (*psp).a7 = 0x0000_0721_0721_0721;
            (*psp).s2 = 0x0000_0721_0721_0721;
            (*psp).s3 = 0x0000_0721_0721_0721;
            (*psp).s4 = 0x0000_0721_0721_0721;
            (*psp).s5 = 0x0000_0721_0721_0721;
            (*psp).s6 = 0x0000_0721_0721_0721;
            (*psp).s7 = 0x0000_0721_0721_0721;
            (*psp).s8 = 0x0000_0721_0721_0721;
            (*psp).s9 = 0x0000_0721_0721_0721;
            (*psp).s10 = 0x0000_0721_0721_0721;
            (*psp).s11 = 0x0000_0721_0721_0721;
            (*psp).t3 = 0x0000_0721_0721_0721;
            (*psp).t4 = 0x0000_0721_0721_0721;
            (*psp).t5 = 0x0000_0721_0721_0721;
            (*psp).t6 = 0x0000_0721_0721_0721;

            (*psp).mepc = executor_function_ptr as usize;
            (*psp).mstatus = 0x0000_1800;
        }

        NonNull::new(ptos as *mut usize).unwrap()
    }

    fn enter_idle_state() {}

    fn shutdown() {
        loop {
            // 使用 RISC-V WFI 指令进入低功耗状态
            unsafe { asm!("wfi"); }
        }
    }

    unsafe fn get_current_stack_pointer() -> *mut usize {
        let mut sp: usize;
        asm!("csrr {}, mscratch", out(reg) sp);
        sp as *mut usize
    }
}

impl PlatformMemoryLayout for PlatformImpl {
    fn get_stack_start() -> usize {
        // JH7110 的内存布局 - 根据实际硬件调整
        0x41400000
    }

    fn get_max_programs() -> usize {
        10
    }

    fn get_heap_size() -> usize {
        10 * 1024 // 10 KiB
    }

    fn get_program_stack_size() -> usize {
        8192 // 8 KiB (RISC-V64 需要更大的栈空间)
    }

    fn get_interrupt_stack_size() -> usize {
        4096 // 4 KiB
    }
}

impl Platform for PlatformImpl {
    fn get_timer_driver(&'static self) -> &'static dyn crate::traits::timer::Driver {
        &self.timer
    }
}