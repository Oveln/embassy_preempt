mod ucstk;
pub mod platform;
pub mod timer_driver;

pub use platform::{PlatformImpl};
pub use ucstk::UcStk;

core::arch::global_asm!(
        ".section .trap, \"ax\"",
        ".global MachineEnvCall",
        "MachineEnvCall:",
        "csrrw sp, mscratch, sp",
        // 禁用硬件压栈
        // "li t0, 0x23",
        // "csrw 0x804, t0",
        "jal __ContextSwitchHandler"
);