//! RISC-V Trap 处理入口模块
//!
//! 参考 riscv-rt 的设计，实现 trap 处理的汇编入口点。

mod exceptions;
mod interrupts;

pub use interrupts::register_ipi_callback;

use core::arch::asm;

use crate::trap::exceptions::dispatch_exception;

/// 上下文占用的字节数
/// TrapFrame 的大小：30 个通用寄存器 + mepc + mstatus = 256 字节
pub const CONTEXT_STACK_SIZE: usize = core::mem::size_of::<TrapFrame>();

/// 中断栈上的 TrapFrame 结构
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct TrapFrame {
    pub ra: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub mepc: usize,
    pub mstatus: usize,
}

/// Trap 处理入口（汇编）
core::arch::global_asm!(
    ".section .trap.entry, \"ax\"",
    ".global __trap_entry",
    "__trap_entry:",
    // 保存寄存器到当前栈（256 字节）
    "addi sp, sp, -256",
    "sd x1, 0(sp)", "sd x3, 8(sp)", "sd x4, 16(sp)",
    "sd x5, 24(sp)", "sd x6, 32(sp)", "sd x7, 40(sp)", "sd x8, 48(sp)",
    "sd x9, 56(sp)", "sd x10, 64(sp)", "sd x11, 72(sp)", "sd x12, 80(sp)",
    "sd x13, 88(sp)", "sd x14, 96(sp)", "sd x15, 104(sp)", "sd x16, 112(sp)",
    "sd x17, 120(sp)", "sd x18, 128(sp)", "sd x19, 136(sp)", "sd x20, 144(sp)",
    "sd x21, 152(sp)", "sd x22, 160(sp)", "sd x23, 168(sp)", "sd x24, 176(sp)",
    "sd x25, 184(sp)", "sd x26, 192(sp)", "sd x27, 200(sp)", "sd x28, 208(sp)",
    "sd x29, 216(sp)", "sd x30, 224(sp)", "sd x31, 232(sp)",
    "csrr t0, mepc", "sd t0, 240(sp)",
    "csrr t0, mstatus", "sd t0, 248(sp)",

    "mv a0, sp", // 将 TrapFrame 指针传递给 Rust 的 trap_handler
    "csrrw sp, mscratch, sp", // 切换到系统栈

    "call_trap_handler:",
    // 调用 Rust 的 trap_handler 函数，传递 TrapFrame 指针（当前栈顶），返回新的 TrapFrame 指针
    "call trap_handler",

    "csrrw sp, mscratch, sp", // 切换回任务栈
    
    "mv sp, a0",
    
    "restore_context:",
    // 恢复寄存器 (不恢复sp)
    "ld x1, 0(sp)", "ld x3, 8(sp)", "ld x4, 16(sp)",
    "ld x5, 24(sp)", "ld x6, 32(sp)", "ld x7, 40(sp)", "ld x8, 48(sp)",
    "ld x9, 56(sp)", "ld x10, 64(sp)", "ld x11, 72(sp)", "ld x12, 80(sp)",
    "ld x13, 88(sp)", "ld x14, 96(sp)", "ld x15, 104(sp)", "ld x16, 112(sp)",
    "ld x17, 120(sp)", "ld x18, 128(sp)", "ld x19, 136(sp)", "ld x20, 144(sp)",
    "ld x21, 152(sp)", "ld x22, 160(sp)", "ld x23, 168(sp)", "ld x24, 176(sp)",
    "ld x25, 184(sp)", "ld x26, 192(sp)", "ld x27, 200(sp)", "ld x28, 208(sp)",
    "ld x29, 216(sp)", "ld x30, 224(sp)", "ld x31, 232(sp)",
    "ld t0, 240(sp)", "csrw mepc, t0",
    "ld t0, 248(sp)", "csrw mstatus, t0",
    "addi sp, sp, 256",

    "mret",
);

pub static IN_TRAP: portable_atomic::AtomicBool = portable_atomic::AtomicBool::new(false);

#[no_mangle]
extern "C" fn trap_handler(trap_frame: &mut TrapFrame) -> &TrapFrame {
    IN_TRAP.store(true, portable_atomic::Ordering::Release);

    match riscv::register::mcause::read().cause() {
        riscv::interrupt::Trap::Interrupt(code) => {
            unsafe {
                interrupts::dispatch_interrupt(trap_frame, code);
            }
        },
        riscv::interrupt::Trap::Exception(code) => {
            unsafe {
                exceptions::dispatch_exception(trap_frame, code);
            }
        },
    }

    IN_TRAP.store(false, portable_atomic::Ordering::Release);
    
    context_switch_handler(trap_frame)
}

extern "C" {
    fn __ContextSwitchHandler(trap_frame: &TrapFrame) -> &TrapFrame;
}

pub static NEED_CONTEXT_SWITCH: portable_atomic::AtomicBool = portable_atomic::AtomicBool::new(false);
#[inline(always)]
fn context_switch_handler(trap_frame: &TrapFrame) -> &TrapFrame {
    let ret = if unsafe { NEED_CONTEXT_SWITCH.load(portable_atomic::Ordering::Acquire) } {
        unsafe { __ContextSwitchHandler(trap_frame) }
    } else {
        trap_frame
    };
    NEED_CONTEXT_SWITCH.store(false, portable_atomic::Ordering::Release);
    ret
}

#[no_mangle]
pub unsafe extern "C" fn MachineEnvCall(trap_frame: &mut TrapFrame) {
    // 触发上下文切换
    NEED_CONTEXT_SWITCH.store(true, portable_atomic::Ordering::Release);

    trap_frame.mepc += 4;
}