//! RISC-V Trap 处理逻辑
//!
//! 这个模块包含 trap 的 Rust 级别处理逻辑，包括：
//! - trap 主处理函数
//! - 中断/异常分发
//! - 上下文切换协调

use crate::TrapFrame;
use portable_atomic::Ordering;

// Re-export from lib for now
pub use crate::{IN_TRAP, NEED_CONTEXT_SWITCH};

// 外部声明：上下文切换处理函数
//
// 这个函数由调度器（executor）提供，在 `embassy-preempt-executor` 中实现。
// 通过链接器符号解析，允许平台提供自定义实现。
extern "C" {
    fn __ContextSwitchHandler(trap_frame: &TrapFrame) -> &TrapFrame;
}

// ============================================================================
// Trap 主处理函数
// ============================================================================

/// Trap 处理主函数
///
/// 这个函数由汇编入口点调用，处理所有的 trap（中断和异常）。
///
/// ## 处理流程
///
/// ```text
/// [trap_handler 入口]
///       ↓
/// [设置 IN_TRAP 标志]
///       ↓
/// [根据 mcause 分发]
///       ├─→ Interrupt → handlers::interrupt::dispatch
///       └─→ Exception → handlers::exception::dispatch
///       ↓
/// [清除 IN_TRAP 标志]
///       ↓
/// [检查上下文切换]
///       ↓
/// [返回 TrapFrame 指针]
/// ```
///
/// ## 返回值
///
/// 返回的 TrapFrame 指针可能是：
/// - 原始 trap_frame（无上下文切换）
/// - 新任务的 TrapFrame（发生了上下文切换）
#[no_mangle]
pub extern "C" fn trap_handler(trap_frame: &mut TrapFrame) -> &TrapFrame {
    IN_TRAP.store(true, Ordering::Release);

    match riscv::register::mcause::read().cause() {
        riscv::interrupt::Trap::Interrupt(code) => {
            unsafe {
                crate::handlers::interrupt::dispatch_interrupt(trap_frame, code);
            }
        }
        riscv::interrupt::Trap::Exception(code) => {
            unsafe {
                crate::handlers::exception::dispatch_exception(trap_frame, code);
            }
        }
    }

    IN_TRAP.store(false, Ordering::Release);

    context_switch_handler(trap_frame)
}

/// 上下文切换处理
///
/// 检查是否需要上下文切换，如果需要则调用调度器的上下文切换函数。
///
/// ## 逻辑流程
///
/// ```text
/// [检查 NEED_CONTEXT_SWITCH]
///       ↓
///   [是?] → [调用 __ContextSwitchHandler]
///   [否] → [直接返回 trap_frame]
///       ↓
/// [清除 NEED_CONTEXT_SWITCH]
///       ↓
/// [返回 TrapFrame 指针]
/// ```
fn context_switch_handler(trap_frame: &TrapFrame) -> &TrapFrame {
    let ret = if NEED_CONTEXT_SWITCH.load(Ordering::Acquire) {
        unsafe { __ContextSwitchHandler(trap_frame) }
    } else {
        trap_frame
    };
    NEED_CONTEXT_SWITCH.store(false, Ordering::Release);
    ret
}

// ============================================================================
// Machine 模式 Ecall 处理
// ============================================================================

/// Machine 模式的 ecall 处理函数
///
/// 这个函数由异常处理调用，用于触发上下文切换。
/// 通过 `ecall` 指令从 Machine 模式请求上下文切换。
///
/// ## 使用方式
///
/// ```rust,ignore
/// // 在任务中触发上下文切换
/// unsafe {
///     riscv::asm::ecall();
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn MachineEnvCall(trap_frame: &mut TrapFrame) {
    // 触发上下文切换标志
    NEED_CONTEXT_SWITCH.store(true, Ordering::Release);

    // 跳过 ecall 指令（4 字节）
    trap_frame.mepc += 4;
}

// ============================================================================
// 测试支持
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trap_frame_size() {
        // TrapFrame 必须是 256 字节
        assert_eq!(core::mem::size_of::<TrapFrame>(), 256);
    }

    #[test]
    fn test_context_stack_size_constant() {
        // CONTEXT_STACK_SIZE 应该等于 TrapFrame 的大小
        assert_eq!(CONTEXT_STACK_SIZE, core::mem::size_of::<TrapFrame>());
    }
}
