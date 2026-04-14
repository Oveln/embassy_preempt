//! RISC-V 定时器驱动
//!
//! 提供 RISC-V 平台的定时器驱动实现。

mod clint;

pub use clint::{ClintConfig, ClintTimer};
