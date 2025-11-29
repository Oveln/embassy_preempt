#![allow(dead_code)]

/// RISC-V32 任务上下文（CH32V307 适用）
/// 布局与 ARM 版保持相同顺序，方便移植。
/// 总大小 17 * 4 = 68 B
#[repr(C, align(4))]
pub struct UcStk {
    pub ra: usize,      // x1
    pub sp: usize,      // x2 (useless)
    pub gp: usize,      // x3
    pub tp: usize,      // x4
    pub t0: usize,      // x5
    pub t1: usize,      // x6
    pub t2: usize,      // x7
    pub s0: usize,      // x8
    pub s1: usize,      // x9
    pub a0: usize,      // x10
    pub a1: usize,      // x11
    pub a2: usize,      // x12
    pub a3: usize,      // x13
    pub a4: usize,      // x14
    pub a5: usize,      // x15
    pub a6: usize,      // x16
    pub a7: usize,      // x17
    pub s2: usize,      // x18
    pub s3: usize,      // x19
    pub s4: usize,      // x20
    pub s5: usize,      // x21
    pub s6: usize,      // x22
    pub s7: usize,      // x23
    pub s8: usize,      // x24
    pub s9: usize,      // x25
    pub s10: usize,     // x26
    pub s11: usize,     // x27
    pub t3: usize,      // x28
    pub t4: usize,      // x29
    pub t5: usize,      // x30
    pub t6: usize,      // x31
    // 特殊寄存器
    pub mepc: usize,
    pub mstatus: usize,
}

/// 供汇编调度器使用：上下文占用的 32-bit 字数
pub(crate) const CONTEXT_STACK_SIZE: usize = 33*4;