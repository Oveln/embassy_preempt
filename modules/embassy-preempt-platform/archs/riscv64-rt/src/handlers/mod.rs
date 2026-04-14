//! RISC-V 中断和异常处理器
//!
//! 这个模块提供中断和异常的分发机制，支持弱符号扩展。
//!
//! ## 弱符号机制
//!
//! 平台可以通过提供同名函数来覆盖默认的 handler：
//!
//! ```text
//! riscv64-rt 提供默认 handler (弱符号)
//!              ↓
//!      [链接器解析符号]
//!              ↓
//!   平台提供强符号 → 使用平台版本
//!   平台未提供   → 使用默认版本
//! ```
//!
//! ## 中断 Handler
//!
//! 可以覆盖的中断 handler：
//! - `SupervisorSoft` - Supervisor 软件中断
//! - `MachineSoft` - Machine 软件中断 (IPI)
//! - `SupervisorTimer` - Supervisor 定时器中断
//! - `MachineTimer` - Machine 定时器中断
//! - `SupervisorExternal` - Supervisor 外部中断
//! - `MachineExternal` - Machine 外部中断
//!
//! ## 异常 Handler
//!
//! 可以覆盖的异常 handler：
//! - `InstructionMisaligned` - 指令地址未对齐
//! - `InstructionFault` - 指令访问错误
//! - `IllegalInstruction` - 非法指令
//! - `Breakpoint` - 断点
//! - `LoadMisaligned` - 加载地址未对齐
//! - `LoadFault` - 加载访问错误
//! - `StoreMisaligned` - 存储地址未对齐
//! - `StoreFault` - 存储访问错误
//! - `UserEnvCall` - User 模式 ecall
//! - `SupervisorEnvCall` - Supervisor 模式 ecall
//! - `MachineEnvCall` - Machine 模式 ecall (已由 trap/handler.rs 实现)
//! - `InstructionPageFault` - 指令页错误
//! - `LoadPageFault` - 加载页错误
//! - `StorePageFault` - 存储页错误

pub mod interrupt;
pub mod exception;

pub use interrupt::{register_ipi_callback, register_timer_callback, TimerInterruptCallback};
pub use exception::{dispatch_exception, ExceptionHandler};
