//! # RISC-V 64-bit Runtime Support
//!
//! 这个库提供了 RISC-V 64-bit 架构的运行时支持，包括：
//! - Trap 处理的汇编入口点
//! - 异常和中断分发
//! - 上下文切换支持
//! - 定时器驱动
//! - 栈操作工具
//!
//! ## 架构设计
//!
//! ```
//! riscv64-rt/
//! ├── trap/          # Trap 处理框架
//! │   ├── entry.rs   # 汇编入口点（支持 Direct/Vector 模式）
//! │   ├── handler.rs # Rust trap 处理逻辑
//! │   └── frame.rs   # TrapFrame 上下文结构
//! ├── handlers/      # 中断和异常处理器
//! │   ├── interrupt.rs   # 中断分发和默认 handler
//! │   └── exception.rs   # 异常分发和默认 handler
//! ├── timer/         # 定时器驱动
//! │   └── clint.rs   # CLINT 定时器实现
//! ├── start.rs       # 启动流程和 BSS 清零
//! ├── stack.rs       # 栈操作工具
//! └── panic.rs       # Panic 处理和寄存器转储
//! ```
//!
//! ## Trap 处理流程
//!
//! ```text
//! [硬件中断/异常]
//!       ↓
//! [汇编入口: __trap_entry]
//!   保存上下文 → 切换栈
//!       ↓
//! [trap_handler] (Rust)
//!   根据 mcause 分发
//!       ↓
//! [handlers 模块]
//!   中断 → interrupt::dispatch
//!   异常 → exception::dispatch
//!       ↓
//! [可能的上下文切换]
//!       ↓
//! [恢复上下文 → mret]
//! ```
//!
//! ## 弱符号机制
//!
//! 平台可以通过提供同名函数来覆盖默认的 handler：
//!
//! ```text
//! riscv64-rt 提供弱符号默认实现
//!              ↓
//!      [链接时符号解析]
//!             ↓
//!  平台提供强符号 → 使用平台版本
//!   平台未提供   → 使用默认版本
//! ```
//!
//! ## Vector 模式支持（预留）
//!
//! 当前使用 Direct 模式（所有 trap 共享入口点）。
//! trap/entry.rs 中预留了 Vector 模式的实现框架，
//! 可以为每个中断源设置独立的入口点。

#![no_std]
#![feature(naked_functions_rustic_abi)]

#[macro_use]
extern crate embassy_preempt_log;

// ============================================================================
// 模块声明
// ============================================================================

mod trap;
mod handlers;
mod timer;
mod start;
mod stack;
mod panic;

// ============================================================================
// 公共导出
// ============================================================================

// Trap 处理
pub use trap::{TrapFrame, CONTEXT_STACK_SIZE, __trap_entry, MachineEnvCall};

// Handler 回调和分发
pub use handlers::{register_ipi_callback, register_timer_callback, TimerInterruptCallback};

// 定时器驱动
pub use timer::{ClintConfig, ClintTimer};

// 栈操作
pub use stack::{set_program_stack_pointer, configure_interrupt_stack, get_current_stack_pointer};

// ============================================================================
// 全局状态
// ============================================================================

use portable_atomic::AtomicBool;

/// 全局标志：当前是否在 trap 处理中
///
/// 用于防止重入和调试
pub static IN_TRAP: AtomicBool = AtomicBool::new(false);

/// 全局标志：是否需要上下文切换
///
/// 由 `MachineEnvCall` 设置，由 `trap_handler` 检查
pub static NEED_CONTEXT_SWITCH: AtomicBool = AtomicBool::new(false);

// ============================================================================
// 链接脚本需要的默认函数
// ============================================================================

/// 默认的 abort 函数
///
/// 当系统遇到不可恢复的错误时调用。
/// 永远不返回，进入死循环。
#[no_mangle]
pub extern "C" fn _default_abort() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}

/// 默认的 trap 入口点
///
/// 在初始化期间如果发生 trap，将调用此函数。
/// 默认实现是 abort。
#[no_mangle]
pub extern "C" fn _default_start_trap() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}

/// 默认的中断设置函数
///
/// 在启动时调用来设置中断。
/// 默认实现为空，平台可以提供自己的实现。
#[no_mangle]
pub extern "C" fn _default_setup_interrupts() {
    // 默认实现为空
}
