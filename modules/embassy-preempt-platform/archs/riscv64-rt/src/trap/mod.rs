//! RISC-V Trap 处理模块
//!
//! 这个模块提供 RISC-V 架构的 trap（中断和异常）处理框架。
//! 设计上支持 Direct 和 Vector 两种 trap 处理模式。
//!
//! ## 模块结构
//!
//! - `frame`: Trap 上下文结构体
//! - `entry`: 汇编 trap 入口点
//! - `handler`: Rust trap 处理逻辑
//!
//! ## Trap 处理流程
//!
//! ```text
//! [硬件中断/异常]
//!       ↓
//! [汇编入口: __trap_entry]
//!       ↓
//! [保存上下文到 TrapFrame]
//!       ↓
//! [切换到中断栈]
//!       ↓
//! [调用 trap_handler]
//!       ↓
//! [分发到 handlers 模块]
//!       ↓
//! [可能的上下文切换]
//!       ↓
//! [恢复上下文]
//!       ↓
//! [mret 返回]
//! ```

mod frame;
mod entry;
mod handler;

pub use frame::{TrapFrame, CONTEXT_STACK_SIZE};
pub use entry::__trap_entry;
pub use handler::{trap_handler, MachineEnvCall};
