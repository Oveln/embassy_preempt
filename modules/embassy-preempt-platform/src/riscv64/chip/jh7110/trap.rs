//! RISC-V Trap 处理入口模块
//!
//! 提供 trap 处理的汇编入口点，分发到不同的处理函数。

/// Trap 处理入口（汇编）
///
/// 这是所有 trap（异常和中断）的统一入口点。
/// 处理流程：
/// 1. 读取 mcause 寄存器
/// 2. 检查是否为中断（bit 31）
/// 3. 如果是 M-mode ecall (code = 11)，跳转到 MachineEnvCall 进行上下文切换
/// 4. 否则保存寄存器后跳转到 handle_exception 进行异常/中断处理
///
/// # 汇编说明
/// - 先在栈上分配空间并保存 t0/t1/t2 的原始值
/// - 然后读取 mcause 并判断 trap 类型
/// - 根据类型分发到不同的处理路径
///
/// # 寄存器保存
/// 对于非 ecall 的中断/异常，在当前栈上保存以下寄存器：
/// - ra (x1): 返回地址
/// - a0-a7 (x10-x17): 参数寄存器
/// - t0-t6 (x5-x7, x28-x31): 临时寄存器
/// - mepc: 异常程序计数器
/// 总共 18 * 8 = 144 字节
///
/// # 跳转逻辑
/// - 中断 → handle_exception → 恢复 → mret
/// - M-mode ecall → MachineEnvCall（零开销，栈已分配且 t0/t1/t2 已保存）
/// - 其他异常 → handle_exception → abort()（不返回）
core::arch::global_asm!(
    ".section .trap.entry, \"ax\"",
    ".global __trap_entry",
    "__trap_entry:",
    // 先在栈上分配空间，用于保存 t0/t1/t2 的原始值（24 字节）
    "addi sp, sp, -24",
    // 保存 t0/t1/t2 的原始值到栈上
    "sd x5, 0(sp)",     // t0
    "sd x6, 8(sp)",     // t1
    "sd x7, 16(sp)",    // t2

    // 现在可以安全地使用 t0/t1/t2
    // 读取 mcause 到 t0
    "csrr t0, mcause",
    // 检查是否为中断（bit 31 = 1）
    "srli t1, t0, 31",
    "bnez t1, 2f",  // 如果是中断，跳转到标签 2
    // 不是中断，是异常：提取异常码（低7位）
    "andi t1, t0, 0x7F",
    // 检查是否为 M-mode ecall (code 11)
    "li t2, 11",
    "beq t1, t2, 1f",  // 如果是 ecall，跳转到标签 1
    // 其他异常：继续执行标签 2 的代码

    "2:",  // 非ecall 的中断/异常处理路径：需要保存更多寄存器
    // 扩展栈空间到 144 字节（已经在栈上分配了 24 字节，再分配 120 字节）
    "addi sp, sp, -120",

    // 保存调用者保存寄存器
    "sd x1, 0(sp)",     // ra
    "sd x10, 8(sp)",    // a0
    "sd x11, 16(sp)",   // a1
    "sd x12, 24(sp)",   // a2
    "sd x13, 32(sp)",   // a3
    "sd x14, 40(sp)",   // a4
    "sd x15, 48(sp)",   // a5
    "sd x16, 56(sp)",   // a6
    "sd x17, 64(sp)",   // a7
    // t0/t1/t2 的原始值已经在栈顶（sp+120），需要重新保存
    "ld t3, 120(sp)",   // 加载原 t0
    "sd t3, 72(sp)",    // 保存到新位置
    "ld t3, 128(sp)",   // 加载原 t1
    "sd t3, 80(sp)",    // 保存到新位置
    "ld t3, 136(sp)",   // 加载原 t2
    "sd t3, 88(sp)",    // 保存到新位置
    "sd x28, 96(sp)",   // t3
    "sd x29, 104(sp)",  // t4
    "sd x30, 112(sp)",  // t5
    "sd x31, 120(sp)",  // t6

    // 保存 mepc
    "csrr t1, mepc",
    "sd t1, 128(sp)",

    // 调用 handle_exception(mcause)
    "mv a0, t0",       // mcause 已经在 t0 中
    "jal handle_exception",

    "li t1, 1",
    "beq a0, t1, 2f", // 如果返回值为1，则说明需要ecall，跳转到2

    // handle_exception 返回后（仅中断路径会返回），恢复寄存器
    "ld x1, 0(sp)",     // ra
    "ld x10, 8(sp)",    // a0
    "ld x11, 16(sp)",   // a1
    "ld x12, 24(sp)",   // a2
    "ld x13, 32(sp)",   // a3
    "ld x14, 40(sp)",   // a4
    "ld x15, 48(sp)",   // a5
    "ld x16, 56(sp)",   // a6
    "ld x17, 64(sp)",   // a7
    "ld x5, 72(sp)",    // t0
    "ld x6, 80(sp)",    // t1
    "ld x7, 88(sp)",    // t2
    "ld x28, 96(sp)",   // t3
    "ld x29, 104(sp)",  // t4
    "ld x30, 112(sp)",  // t5
    "ld x31, 120(sp)",  // t6

    // 恢复 mepc
    "ld t1, 128(sp)",
    "csrw mepc, t1",

    // 释放所有栈空间（144 字节）
    "addi sp, sp, 144",
    "mret",

// ------------------------------
    "1:",  // ecall 路径：恢复 t0/t1/t2 后跳转到上下文切换
    "ld x5, 0(sp)",     // 恢复 t0
    "ld x6, 8(sp)",     // 恢复 t1
    "ld x7, 16(sp)",    // 恢复 t2
    "addi sp, sp, 24",  // 释放临时栈空间
    "j MachineEnvCall",

// ------------------------------
    "2:", // 如果handle_exception返回值为1，则需要上下文切换，跳转到ecall处理
    "ld x1, 0(sp)",     // ra
    "ld x10, 8(sp)",    // a0
    "ld x11, 16(sp)",   // a1
    "ld x12, 24(sp)",   // a2
    "ld x13, 32(sp)",   // a3
    "ld x14, 40(sp)",   // a4
    "ld x15, 48(sp)",   // a5
    "ld x16, 56(sp)",   // a6
    "ld x17, 64(sp)",   // a7
    "ld x5, 72(sp)",    // t0
    "ld x6, 80(sp)",    // t1
    "ld x7, 88(sp)",    // t2
    "ld x28, 96(sp)",   // t3
    "ld x29, 104(sp)",  // t4
    "ld x30, 112(sp)",  // t5
    "ld x31, 120(sp)",  // t6
    // 恢复 mepc
    "ld t1, 128(sp)",
    "csrw mepc, t1",

    "addi sp, sp, 144",

    "j MachineEnvCall"
// ------------------------------
);

/// MachineEnvCall 入口（上下文切换）
///
/// 这是 ecall 指令的处理入口，用于任务间的上下文切换。
///
/// # 处理流程
/// 1. 交换 sp 和 mscratch（sp 指向当前任务的栈）
/// 2. 跳转到 __ContextSwitchHandler
///
/// # 说明
/// mscratch 寄存器用于保存当前任务栈指针。
/// 交换后，sp 指向中断栈，mscratch 指向任务栈。
core::arch::global_asm!(
    ".section .trap.MachineEnvCall, \"ax\"",
    ".global MachineEnvCall",
    "MachineEnvCall:",
    "csrrw sp, mscratch, sp",  // 交换 sp 和 mscratch
    "j __ContextSwitchHandler"  // 跳转到上下文切换处理
);

/// Trap 处理相关的常量
///
/// # 中断和异常编码
/// - 中断: mcause bit 31 = 1
/// - 异常: mcause bit 31 = 0
/// - 代码: mcause[4:0]
pub mod constants {
    /// M-mode 环境调用异常码
    pub const M_ECALL: usize = 11;

    /// 异常码掩码（低7位）
    pub const EXCEPTION_CODE_MASK: usize = 0x7F;

    /// 中断位（bit 31）
    pub const INTERRUPT_BIT: usize = 1 << 31;

    /// 中断编号
    pub mod interrupt {
        /// Supervisor software interrupt
        pub const SSIP: usize = 1;

        /// Machine software interrupt
        pub const MSIP: usize = 3;

        /// Supervisor timer interrupt
        pub const STIP: usize = 5;

        /// Machine timer interrupt
        pub const MTIP: usize = 7;

        /// Supervisor external interrupt
        pub const SEIP: usize = 9;

        /// Machine external interrupt
        pub const MEIP: usize = 11;
    }

    /// 异常编号
    pub mod exception {
        /// Instruction address misaligned
        pub const IADDR_MISALIGNED: usize = 0;

        /// Instruction access fault
        pub const IACCESS_FAULT: usize = 1;

        /// Illegal instruction
        pub const ILLEGAL_INSTRUCTION: usize = 2;

        /// Breakpoint
        pub const BREAKPOINT: usize = 3;

        /// Load address misaligned
        pub const LADDR_MISALIGNED: usize = 4;

        /// Load access fault
        pub const LACCESS_FAULT: usize = 5;

        /// Store/AMO address misaligned
        pub const SADDR_MISALIGNED: usize = 6;

        /// Store/AMO access fault
        pub const SACCESS_FAULT: usize = 7;

        /// Environment call from U-mode
        pub const U_ECALL: usize = 8;

        /// Environment call from S-mode
        pub const S_ECALL: usize = 9;

        /// Environment call from M-mode
        pub const M_ECALL: usize = 11;

        /// Instruction page fault
        pub const IPAGE_FAULT: usize = 12;

        /// Load page fault
        pub const LPAGE_FAULT: usize = 13;

        /// Store/AMO page fault
        pub const SPAGE_FAULT: usize = 15;
    }
}

/// 获取异常/中断的描述字符串
///
/// # 参数
/// - `mcause`: mcause 寄存器值
///
/// # 返回
/// 异常/中断的描述字符串
#[inline]
pub const fn get_trap_description(mcause: usize) -> &'static str {
    let is_interrupt = (mcause & constants::INTERRUPT_BIT) != 0;
    let code = mcause & constants::EXCEPTION_CODE_MASK;

    if is_interrupt {
        match code {
            constants::interrupt::MSIP => "Machine software interrupt",
            constants::interrupt::MTIP => "Machine timer interrupt",
            constants::interrupt::MEIP => "Machine external interrupt",
            _ => "Unknown interrupt",
        }
    } else {
        match code {
            constants::exception::IADDR_MISALIGNED => "Instruction address misaligned",
            constants::exception::IACCESS_FAULT => "Instruction access fault",
            constants::exception::ILLEGAL_INSTRUCTION => "Illegal instruction",
            constants::exception::BREAKPOINT => "Breakpoint",
            constants::exception::LADDR_MISALIGNED => "Load address misaligned",
            constants::exception::LACCESS_FAULT => "Load access fault",
            constants::exception::SADDR_MISALIGNED => "Store/AMO address misaligned",
            constants::exception::SACCESS_FAULT => "Store/AMO access fault",
            constants::exception::U_ECALL => "Environment call from U-mode",
            constants::exception::S_ECALL => "Environment call from S-mode",
            constants::exception::M_ECALL => "Environment call from M-mode",
            _ => "Unknown exception",
        }
    }
}
