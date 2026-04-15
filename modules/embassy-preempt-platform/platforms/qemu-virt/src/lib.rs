//! # QEMU Virt 平台实现
//!
//! 这个模块提供了 QEMU RISC-V 64-bit virt 平台的完整实现，包括：
//! - 平台初始化和配置
//! - 异常和中断处理
//! - 栈回溯和调试支持
//! - 定时器驱动 (基于通用 CLINT 驱动)
//! - GPIO 驱动 (stub 实现)
//! - 任务上下文管理
//!
//! ## 模块组织
//!
//! ### 核心模块
//! - [`platform`] - 平台初始化和核心功能实现
//! - [`clint_config`] - QEMU Virt CLINT 配置
//! - [`gpio`] - GPIO 驱动 (stub)

#![no_std]
#![feature(naked_functions_rustic_abi)]


#[macro_use]
extern crate embassy_preempt_log;

// 平台核心实现
pub mod platform;
pub mod clint_config;
pub mod gpio;

// 公共导出
pub use platform::PlatformImpl;

// 重新导出 GPIO 相关的公共接口
pub use gpio::{init as gpio_init, gpio_controller, GpioController};
