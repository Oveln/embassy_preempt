//! JH7110 CLINT Configuration
//!
//! This module provides the JH7110 platform-specific configuration for the
//! generic CLINT timer driver.

use embassy_preempt_riscv64_rt::ClintConfig;

/// JH7110 platform CLINT configuration
///
/// Provides the platform-specific parameters for the CLINT timer driver on
/// the StarFive JH7110 RISC-V SoC.
pub struct Jh7110ClintConfig;

impl ClintConfig for Jh7110ClintConfig {
    /// CLINT base address for JH7110
    const CLINT_BASE: usize = 0x02000000;

    /// Timer frequency in Hz (4MHz for JH7110)
    const TIMER_HZ: u64 = 4_000_000;

    /// Hart ID (JH7110 has a single hart for bare-metal)
    const HART_ID: usize = 0;
}
