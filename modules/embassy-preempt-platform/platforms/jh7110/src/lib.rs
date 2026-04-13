//! # JH7110 平台实现
//!
//! 这个模块提供了 JH7110 RISC-V64 SoC 平台的完整实现，包括：
//! - 平台初始化和配置
//! - 异常和中断处理
//! - 栈回溯和调试支持
//! - 定时器驱动
//! - GPIO 驱动
//! - 任务上下文管理
//!
//! ## 模块组织
//!
//! ### 核心模块
//! - [`platform`] - 平台初始化和核心功能实现
//! - [`timer_driver`] - 硬件定时器驱动 (基于 CLINT mtime)
//! - [`gpio`] - GPIO 驱动 (基于 sys_gpio)

#![no_std]
#![feature(naked_functions_rustic_abi)]


#[macro_use]
extern crate embassy_preempt_log;

// 平台核心实现
pub mod platform;
pub mod timer_driver;
pub mod gpio;

pub mod panic_handler;

// trap 模块重新导出 riscv64-rt 的内容
// 这保持了与原来代码结构的兼容性
pub mod trap {
    pub use embassy_preempt_riscv64_rt::{
        TrapFrame, CONTEXT_STACK_SIZE, IN_TRAP, NEED_CONTEXT_SWITCH,
    };
}


// Re-export from traits crate
pub use embassy_preempt_traits::{
    memory_layout::PlatformMemoryLayout, platform::PlatformStatic, Platform, OsStk,
};

// Re-export from riscv64-rt
pub use embassy_preempt_riscv64_rt::{
    CONTEXT_STACK_SIZE, IN_TRAP, NEED_CONTEXT_SWITCH, TrapFrame,
    MachineEnvCall, register_ipi_callback, register_timer_callback,
    trap_entry_addr,
};

// 公共导出
pub use platform::PlatformImpl;

// 重新导出 GPIO 相关的公共接口
pub use gpio::{init as gpio_init, gpio_controller, GpioController};