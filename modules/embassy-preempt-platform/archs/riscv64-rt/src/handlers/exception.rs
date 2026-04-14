//! RISC-V 异常处理器
//!
//! 提供 RISC-V 异常的分发机制。
//!
//! ## 弱符号机制
//!
//! 所有的异常处理器都通过链接脚本定义为弱符号，
//! 默认指向 `ExceptionHandler`。
//!
//! 平台可以通过提供同名的 `#[no_mangle]` 函数来覆盖任何异常处理器。
//!
//! ## 可覆盖的异常处理器
//!
//! - `InstructionMisaligned` - 指令地址未对齐 (异常代码 0)
//! - `InstructionFault` - 指令访问错误 (异常代码 1)
//! - `IllegalInstruction` - 非法指令 (异常代码 2)
//! - `Breakpoint` - 断点 (异常代码 3)
//! - `LoadMisaligned` - 加载地址未对齐 (异常代码 4)
//! - `LoadFault` - 加载访问错误 (异常代码 5)
//! - `StoreMisaligned` - 存储地址未对齐 (异常代码 6)
//! - `StoreFault` - 存储访问错误 (异常代码 7)
//! - `UserEnvCall` - User 模式 ecall (异常代码 8)
//! - `SupervisorEnvCall` - Supervisor 模式 ecall (异常代码 9)
//! - `MachineEnvCall` - Machine 模式 ecall (异常代码 11，由 trap/handler.rs 实现)
//! - `InstructionPageFault` - 指令页错误 (异常代码 12)
//! - `LoadPageFault` - 加载页错误 (异常代码 13)
//! - `StorePageFault` - 存储页错误 (异常代码 15)

use crate::TrapFrame;

// ============================================================================
// 异常 Handler 声明 (弱符号)
// ============================================================================

/// 以下函数声明为外部符号，由链接脚本解析为弱符号。
/// 默认情况下，它们都指向 `ExceptionHandler`。
/// 平台可以通过提供同名函数来覆盖任何处理器。
extern "C" {
    fn InstructionMisaligned(trap_frame: &mut TrapFrame);
    fn InstructionFault(trap_frame: &mut TrapFrame);
    fn IllegalInstruction(trap_frame: &mut TrapFrame);
    fn Breakpoint(trap_frame: &mut TrapFrame);
    fn LoadMisaligned(trap_frame: &mut TrapFrame);
    fn LoadFault(trap_frame: &mut TrapFrame);
    fn StoreMisaligned(trap_frame: &mut TrapFrame);
    fn StoreFault(trap_frame: &mut TrapFrame);
    fn UserEnvCall(trap_frame: &mut TrapFrame);
    fn SupervisorEnvCall(trap_frame: &mut TrapFrame);
    fn MachineEnvCall(trap_frame: &mut TrapFrame);
    fn InstructionPageFault(trap_frame: &mut TrapFrame);
    fn LoadPageFault(trap_frame: &mut TrapFrame);
    fn StorePageFault(trap_frame: &mut TrapFrame);
}

// ============================================================================
// 默认异常处理器实现
// ============================================================================

/// 默认的未处理异常处理器
///
/// 当平台未提供特定异常的 handler 时，使用此函数作为后备。
/// 它会打印详细的异常信息并 panic。
///
/// 平台可以覆盖此函数来改变默认行为。
#[no_mangle]
pub unsafe extern "C" fn ExceptionHandler(trap_frame: &TrapFrame) {
    let mcause = riscv::register::mcause::read().bits();
    let is_interrupt = mcause & (1 << 63) != 0;
    let exception_code = mcause & !(1 << 63);

    if is_interrupt {
        panic!(
            "Unhandled interrupt: mcause={:#x}, mepc={:#x}, mstatus={:#x}",
            mcause, trap_frame.mepc, trap_frame.mstatus
        );
    } else {
        let exception_name = match exception_code {
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
            _ => "Unknown",
        };
        panic!(
            "Unhandled exception: {} (code {}), mcause={:#x}, mepc={:#x}, mstatus={:#x}",
            exception_name, exception_code, mcause, trap_frame.mepc, trap_frame.mstatus
        );
    }
}

// ============================================================================
// 异常分发器
// ============================================================================

/// 异常 handler 表
///
/// ## 异常编码映射
///
/// 根据 RISC-V 异常编码索引到对应的 handler 函数。
/// None 表示该编码未使用或保留。
///
/// | 索引 | 异常类型 | Handler |
/// |------|---------|---------|
/// | 0 | Instruction address misaligned | `InstructionMisaligned` |
/// | 1 | Instruction access fault | `InstructionFault` |
/// | 2 | Illegal instruction | `IllegalInstruction` |
/// | 3 | Breakpoint | `Breakpoint` |
/// | 4 | Load address misaligned | `LoadMisaligned` |
/// | 5 | Load access fault | `LoadFault` |
/// | 6 | Store/AMO address misaligned | `StoreMisaligned` |
/// | 7 | Store/AMO access fault | `StoreFault` |
/// | 8 | Environment call from U-mode | `UserEnvCall` |
/// | 9 | Environment call from S-mode | `SupervisorEnvCall` |
/// | 10 | Reserved | - |
/// | 11 | Environment call from M-mode | `MachineEnvCall` |
/// | 12 | Instruction page fault | `InstructionPageFault` |
/// | 13 | Load page fault | `LoadPageFault` |
/// | 14 | Reserved | - |
/// | 15 | Store/AMO page fault | `StorePageFault` |
#[no_mangle]
pub static __EXCEPTIONS_EMBASSY_PREEMPT: [Option<unsafe extern "C" fn(&mut TrapFrame)>; 16] = [
    Some(InstructionMisaligned),   // 0
    Some(InstructionFault),        // 1
    Some(IllegalInstruction),      // 2
    Some(Breakpoint),              // 3
    Some(LoadMisaligned),          // 4
    Some(LoadFault),               // 5
    Some(StoreMisaligned),         // 6
    Some(StoreFault),              // 7
    Some(UserEnvCall),             // 8
    Some(SupervisorEnvCall),       // 9
    None,                          // 10 (保留)
    Some(MachineEnvCall),          // 11
    Some(InstructionPageFault),    // 12
    Some(LoadPageFault),           // 13
    None,                          // 14 (保留)
    Some(StorePageFault),          // 15
];

/// 异常分发函数
///
/// 根据 mcause 的异常编码调用对应的处理器。
///
/// # Safety
/// 从 trap_handler 调用，必须在中断上下文中执行
///
/// ## 处理流程
///
/// ```text
/// [dispatch_exception 入口]
///       ↓
/// [从 __EXCEPTIONS_EMBASSY_PREEMPT 获取 handler]
///       ↓
///   [找到?] → [调用 handler]
///   [未找到] → [调用 ExceptionHandler]
///       ↓
/// [返回]
/// ```
#[no_mangle]
pub unsafe extern "C" fn dispatch_exception(trap_frame: &mut TrapFrame, code: usize) {
    match __EXCEPTIONS_EMBASSY_PREEMPT.get(code) {
        Some(Some(handler)) => handler(trap_frame),
        _ => ExceptionHandler(trap_frame),
    }
}
