//! StarFive JH7110 RISC-V64 SoC platform implementation

pub mod platform;
pub mod timer_driver;
pub mod ucstk;

pub use platform::{PlatformImpl};

core::arch::global_asm!(
        ".section .trap, \"ax\"",
        ".global MachineEnvCall",
        "MachineEnvCall:",
        "csrrw sp, mscratch, sp",
        "j __ContextSwitchHandler"
);

// #[riscv_rt::exception(riscv::interrupt::Exception::MachineEnvCall)]
// fn ecall() {
// }