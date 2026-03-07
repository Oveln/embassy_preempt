//! RISC-V 异常处理模块
//!
//! 提供 JH7110 平台的异常处理功能，包括异常原因解析、
//! 异常处理入口和系统终止功能。

use core::arch::asm;

use crate::riscv64::chip::jh7110::backtrace::{print_backtrace, print_all_registers};

/// 异常原因常量
pub(crate) const M_ECALL: usize = 11;  // Environment call from M-mode
pub(crate) const EXCEPTION_CODE_MASK: usize = 0x7F;
pub(crate) const INTERRUPT_BIT: usize = 1 << 31;

/// 解析 mcause 寄存器获取异常原因
#[inline]
pub(crate) unsafe fn get_exception_reason(mcause: usize) -> &'static str {
    // mcause 的低位表示异常类型
    const EXCEPTION_MASK: usize = 0x7F;
    const INTERRUPT_BIT: usize = 1 << 31;

    let is_interrupt = mcause & INTERRUPT_BIT != 0;
    let exception_code = mcause & EXCEPTION_MASK;

    if is_interrupt {
        match exception_code {
            3 => "Machine software interrupt",
            7 => "Machine timer interrupt",
            11 => "Machine external interrupt",
            _ => "Unknown machine interrupt",
        }
    } else {
        match exception_code {
            0 => "Instruction address misaligned",
            1 => "Instruction access fault",
            2 => "Illegal instruction",
            3 => "Breakpoint",
            4 => "Load address misaligned",
            5 => "Load access fault",
            6 => "Store/AMO address misaligned",
            7 => "Store/AMO access fault",
            8 => "Environment call from U-mode",
            9 => "Environment call from S-mode",
            11 => "Environment call from M-mode",
            12 => "Instruction page fault",
            13 => "Load page fault",
            15 => "Store/AMO page fault",
            _ => "Unknown exception",
        }
    }
}

/// 系统终止函数
///
/// 打印完整的系统状态信息后进入死循环。
/// 包含：
/// - 异常摘要
/// - 核心控制寄存器
/// - 通用寄存器
/// - mstatus 详细信息
/// - 中断挂位信息
/// - mtvec 配置
/// - 内存布局信息
/// - 栈回溯
#[no_mangle]
pub fn abort() -> ! {
    // 输出寄存器的值，帮助调试
    let mut ra: usize;
    let mut fp: usize;  // 帧指针 s0
    let mut sp: usize;
    let mut mepc: usize;
    let mut mstatus: usize;
    let mut mcause: usize;
    let mut mtval: usize;
    let mut mtvec: usize;
    let mut mip: usize;

    unsafe {
        core::arch::asm!("mv {}, ra", out(reg) ra);
        core::arch::asm!("mv {}, x8", out(reg) fp);  // s0/fp
        core::arch::asm!("mv {}, sp", out(reg) sp);
        core::arch::asm!("csrr {}, mepc", out(reg) mepc);
        core::arch::asm!("csrr {}, mstatus", out(reg) mstatus);
        core::arch::asm!("csrr {}, mcause", out(reg) mcause);
        core::arch::asm!("csrr {}, mtval", out(reg) mtval);
        core::arch::asm!("csrr {}, mtvec", out(reg) mtvec);
        core::arch::asm!("csrr {}, mip", out(reg) mip);
    }

    os_log!(error, "");
    os_log!(error, "╔════════════════════════════════════════════════════════════════════╗");
    os_log!(error, "║                    ABORT: SYSTEM HALTED                            ║");
    os_log!(error, "╚════════════════════════════════════════════════════════════════════╝");
    os_log!(error, "");

    // 打印异常原因
    let reason = unsafe { get_exception_reason(mcause) };
    let is_interrupt = (mcause & INTERRUPT_BIT) != 0;
    let exception_code = mcause & EXCEPTION_CODE_MASK;

    os_log!(error, "=== Exception Summary ===");
    os_log!(error, "  Type: {}", if is_interrupt { "Interrupt" } else { "Exception" });
    os_log!(error, "  Reason: {}", reason);
    os_log!(error, "  Code: {}", exception_code);
    os_log!(error, "");

    // 打印核心控制寄存器
    os_log!(error, "=== Core Control Registers ===");
    os_log!(error, "  mepc    = {:#016x}  (Exception Program Counter)", mepc);
    os_log!(error, "  mtvec   = {:#016x}  (Trap Vector Base)", mtvec);
    os_log!(error, "  mcause  = {:#016x}  (Machine Cause)", mcause);
    os_log!(error, "  mtval   = {:#016x}  (Trap Value / Fault Address)", mtval);
    os_log!(error, "  mip     = {:#016x}  (Interrupt Pending)", mip);
    os_log!(error, "  mstatus = {:#016x}  (Machine Status)", mstatus);
    os_log!(error, "");

    // 打印通用寄存器
    os_log!(error, "=== General Registers ===");
    os_log!(error, "  ra (x1)  = {:#016x}  (Return Address)", ra);
    os_log!(error, "  fp (x8)  = {:#016x}  (Frame Pointer)", fp);
    os_log!(error, "  sp (x2)  = {:#016x}  (Stack Pointer)", sp);
    os_log!(error, "");

    // 打印 mstatus 的详细信息
    print_mstatus_details(mstatus);

    // 打印中断挂位信息
    if mip != 0 {
        print_mip_details(mip);
    }

    // 打印 mtvec 模式
    print_mtvec_details(mtvec);

    // 打印保存的上下文寄存器（如果在任务上下文中）
    unsafe { print_all_registers(sp); }

    // 打印内存布局信息
    print_memory_layout(mepc, sp, fp);

    // 打印栈回溯（使用帧指针）
    unsafe { print_backtrace(ra, fp, mepc); }

    os_log!(error, "");
    os_log!(error, "System halted. Manual reset required.");
    os_log!(error, "For debugging: check the above backtrace and register dump.");
    os_log!(error, "");

    loop {}
}

/// 打印 mstatus 寄存器的详细信息
#[inline]
fn print_mstatus_details(mstatus: usize) {
    let mie = (mstatus & 0x8) != 0;
    let mpie = (mstatus & 0x80) != 0;
    let mpp = (mstatus >> 11) & 0x3;
    let mprv = (mstatus & 0x20000) != 0;
    let mxr = (mstatus & 0x20000) != 0;
    let sum = (mstatus & 0x40000) != 0;
    let fs = (mstatus >> 13) & 0x3;
    let vs = (mstatus >> 9) & 0x3;

    os_log!(error, "=== mstatus Breakdown ===");
    os_log!(error, "  MIE  (Machine Interrupt Enable)  = {}", mie);
    os_log!(error, "  MPIE (Previous MIE)              = {}", mpie);
    os_log!(error, "  MPP  (Previous Privilege Level)  = {}", match mpp {
        0 => "User (U-mode)",
        1 => "Supervisor (S-mode)",
        3 => "Machine (M-mode)",
        _ => "Unknown/Reserved",
    });
    os_log!(error, "  MPRV (Modify Privilege)         = {}", mprv);
    os_log!(error, "  MXR  (Make eXecutable Readable) = {}", mxr);
    os_log!(error, "  SUM  (Supervisor User Memory)   = {}", sum);
    os_log!(error, "  FS   (Floating-point State)     = {}", match fs {
        0 => "Off",
        1 => "Initial",
        2 => "Clean",
        3 => "Dirty",
        _ => "Unknown",
    });
    os_log!(error, "  VS   (Vector State)             = {}", match vs {
        0 => "Off",
        1 => "Initial",
        2 => "Clean",
        3 => "Dirty",
        _ => "Unknown",
    });
    os_log!(error, "");
}

/// 打印 mip 寄存器的详细信息
#[inline]
fn print_mip_details(mip: usize) {
    os_log!(error, "=== Interrupt Pending (mip) ===");
    if mip & (1 << 3) != 0 { os_log!(error, "  MSIP  - Machine Software Interrupt Pending"); }
    if mip & (1 << 7) != 0 { os_log!(error, "  MTIP  - Machine Timer Interrupt Pending"); }
    if mip & (1 << 11) != 0 { os_log!(error, "  MEIP  - Machine External Interrupt Pending"); }
    if mip & (1 << 5) != 0 { os_log!(error, "  STIP  - Supervisor Timer Interrupt Pending"); }
    if mip & (1 << 9) != 0 { os_log!(error, "  SEIP  - Supervisor External Interrupt Pending"); }
    os_log!(error, "");
}

/// 打印 mtvec 寄存器的详细信息
#[inline]
fn print_mtvec_details(mtvec: usize) {
    let mtvec_mode = mtvec & 0x3;
    let mtvec_base = mtvec & !0x3;
    os_log!(error, "=== mtvec Configuration ===");
    os_log!(error, "  Base Address = {:#016x}", mtvec_base);
    os_log!(error, "  Mode         = {}", match mtvec_mode {
        0 => "Direct",
        1 => "Vectored",
        _ => "Reserved",
    });
    os_log!(error, "");
}

/// 打印内存布局信息
#[inline]
fn print_memory_layout(mepc: usize, sp: usize, fp: usize) {
    use crate::riscv64::chip::jh7110::backtrace::{
        is_valid_code_address, is_valid_stack_address,
        CODE_START, CODE_END, STACK_START, STACK_END
    };

    os_log!(error, "=== Memory Layout ===");
    os_log!(error, "  Code Segment  : {:#016x} - {:#016x}", CODE_START, CODE_END);
    os_log!(error, "  Stack Segment : {:#016x} - {:#016x}", STACK_START, STACK_END);
    os_log!(error, "");

    // 检查地址有效性
    if !is_valid_code_address(mepc) {
        os_log!(error, "  WARNING: mepc is outside valid code segment!");
    }
    if !is_valid_stack_address(sp) {
        os_log!(error, "  WARNING: sp is outside valid stack segment!");
    }
    if !is_valid_stack_address(fp) && fp != 0 {
        os_log!(error, "  WARNING: fp is outside valid stack segment!");
    }
    os_log!(error, "");
}
