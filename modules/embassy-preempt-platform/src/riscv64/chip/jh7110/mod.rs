//! StarFive JH7110 RISC-V64 SoC platform implementation

pub mod platform;
pub mod timer_driver;
pub mod ucstk;

pub use platform::{PlatformImpl};
use riscv::register::mtvec::Mtvec;
use core::arch::asm;

/// 异常原因常量
pub(crate) const M_ECALL: usize = 11;  // Environment call from M-mode
pub(crate) const EXCEPTION_CODE_MASK: usize = 0x7F;
pub(crate) const INTERRUPT_BIT: usize = 1 << 31;

/// Trap 处理入口（汇编）
/// 检查 mcause，如果是 ecall 则直接跳转到 MachineEnvCall
core::arch::global_asm!(
    ".section .trap.entry, \"ax\"",
    ".global __trap_entry",
    "__trap_entry:",
    // 读取 mcause
    "csrr t0, mcause",
    // 检查是否为中断（bit 31）：使用 srli 提取 bit 31
    "srli t1, t0, 31",
    "bnez t1, 2f",  // 如果是中断，跳转到异常处理
    // 提取异常码（低7位）
    "andi t1, t0, 0x7F",
    // 检查是否为 M-mode ecall (11)
    "li t2, 11",
    "beq t1, t2, 1f",  // 如果是 ecall，跳转到 MachineEnvCall
    "2:",  // 异常处理路径
    "j handle_exception",
    "1:",  // ecall 路径：零开销直接跳转
    "j MachineEnvCall"
);

/// MachineEnvCall 入口（上下文切换）
core::arch::global_asm!(
    ".section .trap.MachineEnvCall, \"ax\"",
    ".global MachineEnvCall",
    "MachineEnvCall:",
    "csrrw sp, mscratch, sp",
    "j __ContextSwitchHandler"
);


/// JH7110 内存布局常量
const CODE_START: usize = 0x4100_0000;
const CODE_END: usize = 0x4200_0000;
const STACK_START: usize = 0x4140_0000;
const STACK_END: usize = 0x4160_0000;

/// 最大栈回溯深度
const MAX_BACKTRACE_FRAMES: usize = 16;

/// 检查地址是否可能是有效的代码地址
fn is_valid_code_address(addr: usize) -> bool {
    addr >= CODE_START && addr < CODE_END
}

/// 检查地址是否可能是有效的栈地址
fn is_valid_stack_address(addr: usize) -> bool {
    addr >= STACK_START && addr < STACK_END
}

/// 打印栈回溯信息
unsafe fn print_backtrace(ra: usize, sp: usize, mepc: usize) {
    os_log!(error, "=== Stack Backtrace ===");
    os_log!(error, "  frame 0: mepc={:#016x}", mepc);

    // 如果 mepc 是有效的代码地址，显示更多信息
    if is_valid_code_address(mepc) {
        let offset = mepc - CODE_START;
        os_log!(error, "         (code offset: +{:#x})", offset);
    }

    let mut current_ra = ra;
    let mut current_sp = sp;

    for i in 1..=MAX_BACKTRACE_FRAMES {
        if current_ra == 0 {
            os_log!(error, "  frame {}: <end of stack>", i);
            break;
        }

        // 检查返回地址是否在代码段范围内
        if !is_valid_code_address(current_ra) {
            os_log!(error, "  frame {}: <invalid return address {:#x}>", i, current_ra);
            break;
        }

        os_log!(error, "  frame {}: ra={:#016x}", i, current_ra);
        if is_valid_code_address(current_ra) {
            let offset = current_ra - CODE_START;
            os_log!(error, "         sp={:#016x} (code offset: +{:#x})", current_sp, offset);
        } else {
            os_log!(error, "         sp={:#016x}", current_sp);
        }

        // 检查栈指针是否有效
        if !is_valid_stack_address(current_sp) {
            os_log!(error, "  frame {}: <invalid stack pointer {:#x}>", i, current_sp);
            break;
        }

        // 从栈帧中读取上一个返回地址
        // RISC-V 标准栈帧布局：返回地址通常在栈帧的固定位置
        // 尝试读取 sp 位置的值（简化版本，实际可能需要更复杂的帧指针解析）
        let next_ra = *(current_sp as *const usize);
        if next_ra == 0 || next_ra == current_ra {
            os_log!(error, "  frame {}: <no more frames>", i);
            break;
        }

        // 如果下一个返回地址无效，停止回溯
        if !is_valid_code_address(next_ra) {
            os_log!(error, "  frame {}: <invalid next ra {:#x}>", i, next_ra);
            break;
        }

        // 更新 ra 和 sp
        current_ra = next_ra;
        // 简单的栈帧步进（假设栈帧大小为 16 字节）
        // 实际项目中可以使用帧指针 (fp/s0) 来获取更准确的结果
        current_sp = current_sp + 16;
    }
    os_log!(error, "=== End Backtrace ===");
}

/// 处理非 ecall 的异常和中断
#[unsafe(no_mangle)]
unsafe extern "C" fn handle_exception(mcause: usize) {
    let reason = get_exception_reason(mcause);

    os_log!(error, "");
    os_log!(error, "=== Trap Handler ===");
    os_log!(error, "Unhandled exception/interrupt: {}", reason);
    os_log!(error, "mcause = {:#016x}", mcause);

    let mut mepc: usize;
    let mut mtval: usize;
    asm!("csrr {}, mepc", out(reg) mepc);
    asm!("csrr {}, mtval", out(reg) mtval);

    os_log!(error, "mepc  = {:#016x}", mepc);
    os_log!(error, "mtval = {:#016x}", mtval);

    // 对于中断，处理后返回（使用 mret）
    let is_interrupt = (mcause & INTERRUPT_BIT) != 0;
    if is_interrupt {
        os_log!(error, "Interrupt handled, returning...");
        asm!("mret", options(noreturn));
    }

    // 对于异常，调用 abort
    abort();
}

/// 解析 mcause 寄存器获取异常原因
unsafe fn get_exception_reason(mcause: usize) -> &'static str {
    // mcause 的低位表示异常类型
    const EXCEPTION_MASK: usize = 0x7F;
    const INTERRUPT_BIT: usize = 1 << 31;

    let is_interrupt = mcause & INTERRUPT_BIT != 0;
    let exception_code = mcause & EXCEPTION_MASK;

    if is_interrupt {
        match exception_code {
            3 => "Machine software interrupt",
            7 => "Machine timer interrupt",
            11 => "Machine external interrupt",
            _ => "Unknown machine interrupt",
        }
    } else {
        match exception_code {
            0 => "Instruction address misaligned",
            1 => "Instruction access fault",
            2 => "Illegal instruction",
            3 => "Breakpoint",
            4 => "Load address misaligned",
            5 => "Load access fault",
            6 => "Store/AMO address misaligned",
            7 => "Store/AMO access fault",
            8 => "Environment call from U-mode",
            9 => "Environment call from S-mode",
            11 => "Environment call from M-mode",
            12 => "Instruction page fault",
            13 => "Load page fault",
            15 => "Store/AMO page fault",
            _ => "Unknown exception",
        }
    }
}

/// 打印所有通用寄存器
unsafe fn print_all_registers(sp: usize) {
    // 尝试获取保存的上下文（如果在任务上下文中）
    let mscratch: usize;
    core::arch::asm!("csrr {}, mscratch", out(reg) mscratch);

    if mscratch != 0 {
        // mscratch 指向保存的上下文
        let ctx = mscratch as *const ucstk::UcStk;
        os_log!(error, "=== Saved Context Registers ===");
        os_log!(error, "  ra   = {:#016x}", (*ctx).ra);
        os_log!(error, "  gp   = {:#016x}", (*ctx).gp);
        os_log!(error, "  tp   = {:#016x}", (*ctx).tp);
        os_log!(error, "  t0   = {:#016x}", (*ctx).t0);
        os_log!(error, "  t1   = {:#016x}", (*ctx).t1);
        os_log!(error, "  t2   = {:#016x}", (*ctx).t2);
        os_log!(error, "  s0   = {:#016x}", (*ctx).s0);
        os_log!(error, "  s1   = {:#016x}", (*ctx).s1);
        os_log!(error, "  a0   = {:#016x}", (*ctx).a0);
        os_log!(error, "  a1   = {:#016x}", (*ctx).a1);
        os_log!(error, "  a2   = {:#016x}", (*ctx).a2);
        os_log!(error, "  a3   = {:#016x}", (*ctx).a3);
        os_log!(error, "  a4   = {:#016x}", (*ctx).a4);
        os_log!(error, "  a5   = {:#016x}", (*ctx).a5);
        os_log!(error, "  a6   = {:#016x}", (*ctx).a6);
        os_log!(error, "  a7   = {:#016x}", (*ctx).a7);
        os_log!(error, "  s2   = {:#016x}", (*ctx).s2);
        os_log!(error, "  s3   = {:#016x}", (*ctx).s3);
        os_log!(error, "  s4   = {:#016x}", (*ctx).s4);
        os_log!(error, "  s5   = {:#016x}", (*ctx).s5);
        os_log!(error, "  s6   = {:#016x}", (*ctx).s6);
        os_log!(error, "  s7   = {:#016x}", (*ctx).s7);
        os_log!(error, "  s8   = {:#016x}", (*ctx).s8);
        os_log!(error, "  s9   = {:#016x}", (*ctx).s9);
        os_log!(error, "  s10  = {:#016x}", (*ctx).s10);
        os_log!(error, "  s11  = {:#016x}", (*ctx).s11);
        os_log!(error, "  t3   = {:#016x}", (*ctx).t3);
        os_log!(error, "  t4   = {:#016x}", (*ctx).t4);
        os_log!(error, "  t5   = {:#016x}", (*ctx).t5);
        os_log!(error, "  t6   = {:#016x}", (*ctx).t6);
        os_log!(error, "  mepc = {:#016x}", (*ctx).mepc);
        os_log!(error, "  mstatus = {:#016x}", (*ctx).mstatus);
    }
}

#[no_mangle]
fn abort() -> ! {
        // 输出寄存器的值，帮助调试
        let mut ra: usize;
        let mut sp: usize;
        let mut mepc: usize;
        let mut mstatus: usize;
        let mut mcause: usize;
        let mut mtval: usize;

        unsafe {
                core::arch::asm!("mv {}, ra", out(reg) ra);
                core::arch::asm!("mv {}, sp", out(reg) sp);
                core::arch::asm!("csrr {}, mepc", out(reg) mepc);
                core::arch::asm!("csrr {}, mstatus", out(reg) mstatus);
                core::arch::asm!("csrr {}, mcause", out(reg) mcause);
                core::arch::asm!("csrr {}, mtval", out(reg) mtval);
        }

        os_log!(error, "");
        os_log!(error, "╔════════════════════════════════════════════════════════════════════╗");
        os_log!(error, "║                    ABORT: SYSTEM HALTED                            ║");
        os_log!(error, "╚════════════════════════════════════════════════════════════════════╝");
        os_log!(error, "");

        // 打印异常原因
        let reason = unsafe { get_exception_reason(mcause) };
        os_log!(error, "Exception Reason: {}", reason);
        os_log!(error, "");

        // 打印关键寄存器
        os_log!(error, "=== Core Registers ===");
        os_log!(error, "  mepc    = {:#016x}  (Program Counter)", mepc);
        os_log!(error, "  mstatus = {:#016x}  (Machine Status)", mstatus);
        os_log!(error, "  mcause  = {:#016x}  (Cause: {})", mcause, reason);
        os_log!(error, "  mtval   = {:#016x}  (Trap Value)", mtval);
        os_log!(error, "  ra      = {:#016x}  (Return Address)", ra);
        os_log!(error, "  sp      = {:#016x}  (Stack Pointer)", sp);
        os_log!(error, "");

        // 打印 mstatus 的详细信息
        let mie = (mstatus & 0x8) != 0;
        let mpie = (mstatus & 0x80) != 0;
        let mpp = (mstatus >> 11) & 0x3;
        let mprv = (mstatus & 0x20000) != 0;
        os_log!(error, "=== mstatus Breakdown ===");
        os_log!(error, "  MIE  (Machine Interrupt Enable)  = {}", mie);
        os_log!(error, "  MPIE (Previous MIE)              = {}", mpie);
        os_log!(error, "  MPP  (Previous Privilege Level)  = {}", match mpp {
            0 => "User",
            1 => "Supervisor",
            3 => "Machine",
            _ => "Unknown",
        });
        os_log!(error, "  MPRV (Modify Privilege)         = {}", mprv);
        os_log!(error, "");

        // 打印保存的上下文寄存器（如果在任务上下文中）
        unsafe { print_all_registers(sp); }

        // 打印栈回溯
        unsafe { print_backtrace(ra, sp, mepc); }

        os_log!(error, "");
        os_log!(error, "System halted. Manual reset required.");

        loop {}
}