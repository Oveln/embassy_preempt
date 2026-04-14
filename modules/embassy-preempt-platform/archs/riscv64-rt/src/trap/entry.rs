//! RISC-V Trap 汇编入口点
//!
//! 提供 trap 处理的汇编入口。当前使用 Direct 模式，
//! 所有 trap 共享一个入口点 `__trap_entry`。
//!
//! ## Direct 模式
//!
//! 在 Direct 模式下，所有 trap 都跳转到同一个入口点：
//! - mtvec 指向 `__trap_entry`
//! - 由 Rust 的 `trap_handler` 函数进行分发
//!
//! ## Vector 模式（未来支持）
//!
//! Vector 模式允许为不同的中断源设置不同的入口点：
//! - 每个中断向量间隔 4 字节
//! - 可以实现中断优先级和快速处理
//! - 需要修改此文件以支持

// 汇编入口点声明
extern "C" {
    /// Trap 处理汇编入口点
    ///
    /// 这个符号由下面的 `global_asm!` 定义。
    pub fn __trap_entry();
}

// ============================================================================
// Direct 模式汇编入口
// ============================================================================

/// Direct 模式的汇编 trap 入口点
///
/// ## 调用流程
///
/// ```text
/// [硬件 trap] → [保存寄存器] → [切换栈] → [调用 trap_handler]
///                                         ↓
///                          [返回 TrapFrame 指针到 a0]
///                                         ↓
///                            [切换回任务栈] → [恢复寄存器] → [mret]
/// ```
///
/// ## 内存布局
///
/// ```text
/// 任务栈 (进入前)          中断栈 (进入后)
///     ↓                        ↓
/// [任务数据]              [TrapFrame (256字节)]
///                          [      ...        ]
///                              ↓
/// ```
core::arch::global_asm!(
    ".section .trap.entry, \"ax\"",
    ".global __trap_entry",
    ".align 4",

    // Direct 模式入口点
    "__trap_entry:",

    // === 保存上下文 (256 字节) ===
    "addi sp, sp, -256",          // 为 TrapFrame 分配空间
    "sd x1, 0(sp)",    // ra
    "sd x3, 8(sp)",    // gp
    "sd x4, 16(sp)",   // tp
    "sd x5, 24(sp)",   // t0
    "sd x6, 32(sp)",   // t1
    "sd x7, 40(sp)",   // t2
    "sd x8, 48(sp)",   // s0/fp
    "sd x9, 56(sp)",   // s1
    "sd x10, 64(sp)",  // a0
    "sd x11, 72(sp)",  // a1
    "sd x12, 80(sp)",  // a2
    "sd x13, 88(sp)",  // a3
    "sd x14, 96(sp)",  // a4
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

    // 保存系统寄存器
    "csrr t0, mepc",
    "sd t0, 240(sp)",  // mepc
    "csrr t0, mstatus",
    "sd t0, 248(sp)",  // mstatus

    // === 调用 Rust trap handler ===
    "mv a0, sp",                   // 将 TrapFrame 指针传递给 a0
    "csrrw sp, mscratch, sp",      // 切换到中断栈 (sp ↔ mscratch)

    "call_trap_handler:",
    "call trap_handler",           // 调用 Rust 处理函数

    "csrrw sp, mscratch, sp",      // 切换回任务栈
    "mv sp, a0",                   // a0 包含可能更新的 TrapFrame 指针

    // === 恢复上下文 ===
    "restore_context:",
    "ld x1, 0(sp)",    // ra
    "ld x3, 8(sp)",    // gp
    "ld x4, 16(sp)",   // tp
    "ld x5, 24(sp)",   // t0
    "ld x6, 32(sp)",   // t1
    "ld x7, 40(sp)",   // t2
    "ld x8, 48(sp)",   // s0/fp
    "ld x9, 56(sp)",   // s1
    "ld x10, 64(sp)",  // a0
    "ld x11, 72(sp)",  // a1
    "ld x12, 80(sp)",  // a2
    "ld x13, 88(sp)",  // a3
    "ld x14, 96(sp)",  // a4
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

    // 恢复系统寄存器
    "ld t0, 240(sp)",
    "csrw mepc, t0",
    "ld t0, 248(sp)",
    "csrw mstatus, t0",

    "addi sp, sp, 256",            // 释放 TrapFrame 空间

    "mret",                        // 返回任务
);

// ============================================================================
// Vector 模式支持（预留）
// ============================================================================
