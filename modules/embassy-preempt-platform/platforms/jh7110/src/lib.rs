//! # JH7110 平台实现
//!
//! 这个模块提供了 JH7110 RISC-V64 SoC 平台的完整实现，包括：
//! - 平台初始化和配置
//! - 异常和中断处理
//! - 栈回溯和调试支持
//! - 定时器驱动
//! - 任务上下文管理
//!
//! ## 模块组织
//!
//! ### 核心模块
//! - [`platform`] - 平台初始化和核心功能实现
//! - [`timer_driver`] - 硬件定时器驱动 (基于 CLINT mtime)
//! - [`ucstk`] - 用户栈和上下文结构定义
//!
//! ### 异常和中断处理
//! - [`trap`] - Trap 处理的汇编入口点和常量定义
//! - [`exception`] - 同步异常处理和系统终止功能
//! - [`interrupt`] - 中断分发和外部中断处理 (PLIC)
//! - [`backtrace`] - 基于帧指针的栈回溯功能
//!
//! ## 异常处理流程
//!
//! ```text
//! __trap_entry (汇编)
//!       │
//!       ├─→ mcause = 11 (M-mode ecall)
//!       │       └─→ MachineEnvCall
//!       │               └─→ __ContextSwitchHandler
//!       │
//!       └─→ 其他异常/中断
//!               └─→ handle_exception
//!                       ├─→ 中断 (bit 31 = 1)
//!                       │       ├─→ 定时器中断 (7)
//!                       │       ├─→ 外部中断 (11) → PLIC 处理
//!                       │       └─→ 软件中断 (3)
//!                       │
//!                       └─→ 异常 (bit 31 = 0)
//!                               └─→ abort() → 系统终止
//! ```
//!
//! ## 栈回溯
//!
//! 使用帧指针 (fp/s0/x8) 进行精确的栈回溯：
//! - 适用于编译时启用了帧指针 (-g -fno-omit-frame-pointer)
//! - 能够显示完整的调用链
//! - 在异常发生时自动触发
//!
//! ## 中断处理
//!
//! ### 定时器中断
//! - 由 riscv_rt 的 `#[riscv_rt::core_interrupt]` 属性处理
//! - 触发 RTOS 任务调度
//!
//! ### 外部中断
//! - 通过 PLIC (Platform-Level Interrupt Controller) 管理
//! - 支持 UART、SPI、I2C、GPIO、Timer 等外设中断
//! - 详见 [`interrupt::plic`] 模块
//!
//! ## 内存布局
//!
//! | 区域       | 起始地址      | 结束地址      | 大小    |
//! |------------|---------------|---------------|---------|
//! | 代码段     | 0x4100_0000   | 0x4200_0000   | 256 MB  |
//! | 栈段       | 0x4140_0000   | 0x4160_0000   | 32 MB   |
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use embassy_preempt_platform_jh7110::PlatformImpl;
//!
//! // 初始化平台
//! let platform = PlatformImpl::new();
//!
//! // 触发上下文切换
//! PlatformImpl::trigger_context_switch();
//!
//! // 注册中断处理函数
//! unsafe {
//!     extern "C" fn my_handler() {
//!         // 中断处理代码
//!     }
//!     register_interrupt_handler(11, my_handler).unwrap();
//! }
//! ```

#![no_std]
#![feature(naked_functions_rustic_abi)]


#[macro_use]
extern crate embassy_preempt_log;

// 平台核心实现
pub mod platform;
pub mod timer_driver;
pub mod ucstk;

// 异常和中断处理
pub mod exception;
pub mod interrupt;
pub mod trap;
pub mod panic_handler;

// 调试支持
pub mod backtrace;

// Re-export from traits crate
pub use embassy_preempt_traits::{
    memory_layout::PlatformMemoryLayout, platform::PlatformStatic, Platform, OsStk,
};

// 公共导出
pub use platform::PlatformImpl;

// 重新导出异常处理相关的公共接口
pub use exception::abort;
pub use interrupt::{plic, register_interrupt_handler, register_ipi_callback, InterruptHandler};

// 重新导出 trap 处理相关的常量
pub use trap::constants;