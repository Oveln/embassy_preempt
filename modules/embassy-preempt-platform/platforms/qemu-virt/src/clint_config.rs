//! QEMU Virt CLINT Configuration
//!
//! This module provides the QEMU Virt platform-specific configuration for the
//! generic CLINT timer driver.

use embassy_preempt_riscv64_rt::ClintConfig;

/// QEMU Virt platform CLINT configuration
///
/// Provides the platform-specific parameters for the CLINT timer driver on
/// the QEMU RISC-V 64-bit virt machine.
pub struct QemuVirtClintConfig;

impl ClintConfig for QemuVirtClintConfig {
    /// CLINT base address for QEMU Virt
    const CLINT_BASE: usize = 0x02000000;

    /// Timer frequency in Hz (10MHz for QEMU Virt)
    const TIMER_HZ: u64 = 10_000_000;

    /// Hart ID (QEMU Virt has a single hart for bare-metal)
    const HART_ID: usize = 0;
}
