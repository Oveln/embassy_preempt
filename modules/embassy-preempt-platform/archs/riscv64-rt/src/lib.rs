//! # RISC-V 64-bit Runtime Support
//!
//! 这个库提供了 RISC-V 64-bit 架构的运行时支持，包括：
//! - Trap 处理的汇编入口点
//! - 异常和中断分发
//! - 上下文切换支持
//!
//! ## 设计
//!
//! 这个库是与平台无关的 RISC-V RT 层，提供通用的 trap 处理框架。
//! 特定的平台（如 JH7110）需要提供以下回调：
//! - Timer 驱动的中断处理回调
//! - IPI (Inter-Processor Interrupt) 回调
//! - 上下文切换回调

#![no_std]
#![feature(naked_functions_rustic_abi)]

#[macro_use]
extern crate embassy_preempt_log;

mod exceptions;
mod interrupts;
mod trap_frame;

pub use trap_frame::{CONTEXT_STACK_SIZE, TrapFrame};
pub use exceptions::{dispatch_exception, ExceptionHandler};
pub use interrupts::{register_ipi_callback, register_timer_callback, TimerInterruptCallback};

use portable_atomic::{AtomicBool, Ordering};

/// 全局标志：当前是否在 trap 处理中
pub static IN_TRAP: AtomicBool = AtomicBool::new(false);

/// 全局标志：是否需要上下文切换
pub static NEED_CONTEXT_SWITCH: AtomicBool = AtomicBool::new(false);

// 外部声明：上下文切换处理函数
//
// 这个函数由调度器（executor）提供，在 `embassy-preempt-executor` 中实现。
extern "C" {
    fn __ContextSwitchHandler(trap_frame: &TrapFrame) -> &TrapFrame;
}

// Trap 处理入口（汇编）
core::arch::global_asm!(
    ".section .trap.entry, \"ax\"",
    ".global __trap_entry",
    ".align 4",
    "__trap_entry:",
    // 保存寄存器到当前栈（256 字节）
    "addi sp, sp, -256",
    "sd x1, 0(sp)",   // ra
    "sd x3, 8(sp)",   // gp
    "sd x4, 16(sp)",  // tp
    "sd x5, 24(sp)",  // t0
    "sd x6, 32(sp)",  // t1
    "sd x7, 40(sp)",  // t2
    "sd x8, 48(sp)",  // s0/fp
    "sd x9, 56(sp)",  // s1
    "sd x10, 64(sp)", // a0
    "sd x11, 72(sp)", // a1
    "sd x12, 80(sp)", // a2
    "sd x13, 88(sp)", // a3
    "sd x14, 96(sp)", // a4
    "sd x15, 104(sp)", // a5
    "sd x16, 112(sp)", // a6
    "sd x17, 120(sp)", // a7
    "sd x18, 128(sp)", // s2
    "sd x19, 136(sp)", // s3
    "sd x20, 144(sp)", // s4
    "sd x21, 152(sp)", // s5
    "sd x22, 160(sp)", // s6
    "sd x23, 168(sp)", // s7
    "sd x24, 176(sp)", // s8
    "sd x25, 184(sp)", // s9
    "sd x26, 192(sp)", // s10
    "sd x27, 200(sp)", // s11
    "sd x28, 208(sp)", // t3
    "sd x29, 216(sp)", // t4
    "sd x30, 224(sp)", // t5
    "sd x31, 232(sp)", // t6
    "csrr t0, mepc",
    "sd t0, 240(sp)",
    "csrr t0, mstatus",
    "sd t0, 248(sp)",

    "mv a0, sp", // 将 TrapFrame 指针传递给 Rust 的 trap_handler
    "csrrw sp, mscratch, sp", // 切换到系统栈

    "call_trap_handler:",
    "call trap_handler", // 调用 Rust 的 trap_handler 函数

    "csrrw sp, mscratch, sp", // 切换回任务栈

    "mv sp, a0", // a0 包含返回的 TrapFrame 指针

    "restore_context:",
    // 恢复寄存器 (不恢复sp)
    "ld x1, 0(sp)",   // ra
    "ld x3, 8(sp)",   // gp
    "ld x4, 16(sp)",  // tp
    "ld x5, 24(sp)",  // t0
    "ld x6, 32(sp)",  // t1
    "ld x7, 40(sp)",  // t2
    "ld x8, 48(sp)",  // s0/fp
    "ld x9, 56(sp)",  // s1
    "ld x10, 64(sp)", // a0
    "ld x11, 72(sp)", // a1
    "ld x12, 80(sp)", // a2
    "ld x13, 88(sp)", // a3
    "ld x14, 96(sp)", // a4
    "ld x15, 104(sp)", // a5
    "ld x16, 112(sp)", // a6
    "ld x17, 120(sp)", // a7
    "ld x18, 128(sp)", // s2
    "ld x19, 136(sp)", // s3
    "ld x20, 144(sp)", // s4
    "ld x21, 152(sp)", // s5
    "ld x22, 160(sp)", // s6
    "ld x23, 168(sp)", // s7
    "ld x24, 176(sp)", // s8
    "ld x25, 184(sp)", // s9
    "ld x26, 192(sp)", // s10
    "ld x27, 200(sp)", // s11
    "ld x28, 208(sp)", // t3
    "ld x29, 216(sp)", // t4
    "ld x30, 224(sp)", // t5
    "ld x31, 232(sp)", // t6
    "ld t0, 240(sp)",
    "csrw mepc, t0",
    "ld t0, 248(sp)",
    "csrw mstatus, t0",
    "addi sp, sp, 256",

    "mret",
);

/// Trap 处理函数（Rust）
///
/// 这个函数由汇编入口点调用，处理所有的 trap（中断和异常）
#[no_mangle]
extern "C" fn trap_handler(trap_frame: &mut TrapFrame) -> &TrapFrame {
    IN_TRAP.store(true, Ordering::Release);

    match riscv::register::mcause::read().cause() {
        riscv::interrupt::Trap::Interrupt(code) => {
            unsafe {
                interrupts::dispatch_interrupt(trap_frame, code);
            }
        }
        riscv::interrupt::Trap::Exception(code) => {
            unsafe {
                dispatch_exception(trap_frame, code);
            }
        }
    }

    IN_TRAP.store(false, Ordering::Release);

    context_switch_handler(trap_frame)
}

/// 上下文切换处理
///
/// 检查是否需要上下文切换，如果需要则调用调度器的上下文切换函数
fn context_switch_handler(trap_frame: &TrapFrame) -> &TrapFrame {
    let ret = if NEED_CONTEXT_SWITCH.load(Ordering::Acquire) {
        unsafe { __ContextSwitchHandler(trap_frame) }
    } else {
        trap_frame
    };
    NEED_CONTEXT_SWITCH.store(false, Ordering::Release);
    ret
}

/// Machine 模式的 ecall 处理函数
///
/// 这个函数由异常处理调用，用于触发上下文切换
#[no_mangle]
pub unsafe extern "C" fn MachineEnvCall(trap_frame: &mut TrapFrame) {
    // 触发上下文切换
    NEED_CONTEXT_SWITCH.store(true, Ordering::Release);

    // 跳过 ecall 指令
    trap_frame.mepc += 4;
}

/// 导出 trap 入口地址，供平台初始化使用
///
/// 平台初始化时应该将 mtvec 设置为这个地址
pub fn trap_entry_addr() -> usize {
    unsafe { __trap_entry as usize }
}

extern "C" {
    /// 汇编入口点
    fn __trap_entry();
}
