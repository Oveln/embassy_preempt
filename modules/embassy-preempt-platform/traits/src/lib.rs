#![no_std]

//! Platform trait definitions for embassy_preempt RTOS
//!
//! This crate provides trait-based abstractions for platform-specific
//! functionality, allowing embassy_preempt to run on different microcontroller
//! architectures (Cortex-M, RISC-V, etc.)

pub mod memory_layout;
pub mod platform;
pub mod timer;

// Re-export for convenience
pub use memory_layout::PlatformMemoryLayout;
pub use platform::{Platform, PlatformStatic};

/// OS stack type
pub type OsStk = usize;
