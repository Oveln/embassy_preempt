use core::arch::asm;
use core::ptr::NonNull;

use crate::riscv64::chip::jh7110::ucstk::CONTEXT_STACK_SIZE;
use crate::traits::memory_layout::PlatformMemoryLayout;
use crate::traits::platform::PlatformStatic;
use crate::Platform;

pub struct PlatformImpl {
    pub timer: crate::riscv64::chip::jh7110::timer_driver::Jh7110Timer,
}

impl PlatformImpl {
    pub fn new() -> Self {
        os_log!(info, "Init JH7110 Platform");
        // RISC-V64 初始化代码
        unsafe {
            riscv::register::mstatus::set_mie(); // Enable machine interrupts
            
            /// 外部声明 trap 入口
            extern "C" {
                fn __trap_entry();
            }
            use riscv::register::mtvec::{self, Mtvec, TrapMode};
            // 初始化 mtvec 指向我们的 trap 处理函数
            mtvec::write(
                Mtvec::new(__trap_entry as usize, TrapMode::Direct)
            );
        }
        let timer = crate::riscv64::chip::jh7110::timer_driver::Jh7110Timer::new();
        timer.init();
        PlatformImpl { timer }
    }
}

impl PlatformStatic for PlatformImpl {
    fn trigger_context_switch() {
        unsafe {
            // 使用 ecall 触发上下文切换
            asm!("ecall");
        }
    }

    #[inline(always)]
    unsafe fn save_task_context() {
        core::arch::asm!(
            // 保存通用寄存器（64位）
            "csrrw sp, mscratch, sp",
            "addi sp, sp, -256",  // 32 * 8 = 256 bytes (不保存 mstatus)
            "sd x1, 0(sp)",    // ra
            "sd x2, 8(sp)",    // sp
            "sd x3, 16(sp)",   // gp
            "sd x4, 24(sp)",   // tp
            "sd x5, 32(sp)",   // t0
            "sd x6, 40(sp)",   // t1
            "sd x7, 48(sp)",   // t2
            "sd x8, 56(sp)",   // s0
            "sd x9, 64(sp)",   // s1
            "sd x10, 72(sp)",  // a0
            "sd x11, 80(sp)",  // a1
            "sd x12, 88(sp)",  // a2
            "sd x13, 96(sp)",  // a3
            "sd x14, 104(sp)", // a4
            "sd x15, 112(sp)", // a5
            "sd x16, 120(sp)", // a6
            "sd x17, 128(sp)", // a7
            "sd x18, 136(sp)", // s2
            "sd x19, 144(sp)", // s3
            "sd x20, 152(sp)", // s4
            "sd x21, 160(sp)", // s5
            "sd x22, 168(sp)", // s6
            "sd x23, 176(sp)", // s7
            "sd x24, 184(sp)", // s8
            "sd x25, 192(sp)", // s9
            "sd x26, 200(sp)", // s10
            "sd x27, 208(sp)", // s11
            "sd x28, 216(sp)", // t3
            "sd x29, 224(sp)", // t4
            "sd x30, 232(sp)", // t5
            "sd x31, 240(sp)", // t6
            // 保存 mepc, mstatus 到最后两个 64-bit 位置
            // mepc 保存时+4，使其存储"返回地址"而非"异常地址"
            // 这样无论是新任务还是被ecall打断的任务都能正确恢复
            "csrr t0, mepc",
            "addi t0, t0, 4",
            "sd t0, 248(sp)",
            "csrrw sp, mscratch, sp",
        );
    }

    #[inline(always)]
    unsafe fn restore_task_context(stack_pointer: *mut usize, interrupt_stack: *mut usize, _return_value: u32) {
        core::arch::asm!(
            "csrw mscratch, a1",
            "mv sp, a0",
            "ld x1, 0(sp)",
            "ld x3, 16(sp)",
            "ld x4, 24(sp)",
            "ld x5, 32(sp)",
            "ld x6, 40(sp)",
            "ld x7, 48(sp)",
            "ld x8, 56(sp)",
            "ld x9, 64(sp)",
            // "ld x10, 64(sp)",
            "ld x11, 80(sp)",
            "ld x12, 88(sp)",
            "ld x13, 96(sp)",
            "ld x14, 104(sp)",
            "ld x15, 112(sp)",
            "ld x16, 120(sp)",
            "ld x17, 128(sp)",
            "ld x18, 136(sp)",
            "ld x19, 144(sp)",
            "ld x20, 152(sp)",
            "ld x21, 160(sp)",
            "ld x22, 168(sp)",
            "ld x23, 176(sp)",
            "ld x24, 184(sp)",
            "ld x25, 192(sp)",
            "ld x26, 200(sp)",
            "ld x27, 208(sp)",
            "ld x28, 216(sp)",
            "ld x29, 224(sp)",
            "ld x30, 232(sp)",
            "ld x31, 240(sp)",

            // 恢复 mepc, mstatus
            // mepc 已经是返回地址（在保存时已+4），直接恢复即可
            "ld a0, 248(sp)",
            "csrw mepc, a0",

            // 恢复 a0 和 sp
            "ld x10, 72(sp)",  // a0
            "addi sp, sp, 256",

            "mret",

            in("a0") stack_pointer,
            in("a1") interrupt_stack,
            options(noreturn)
        );
    }

    fn set_program_stack_pointer(sp: *mut u8) {
        unsafe {
            asm!(
                "csrw mscratch, a0",
                in("a0") sp
            );
        }
    }

    #[inline(never)]
    fn configure_interrupt_stack(interrupt_stack: *mut u8) {
        unsafe {
            asm!(
                "mv sp, a0",
                "csrrw sp, mscratch, sp",
                "ret",
                in("a0") interrupt_stack
            );
        }
    }

    fn init_task_stack(stk_ref: NonNull<usize>, executor_function: fn()) -> NonNull<usize> {
        scheduler_log!(trace, "init_task_stack for JH7110");
        let executor_function_ptr = executor_function as *const () as usize;
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function_ptr);

        // Get stack pointer and align to 8-byte boundary
        let ptos = stk_ref.as_ptr() as *mut usize;
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;

        // Reserve space for the context frame (264 bytes for RISC-V64)
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp = ptos as *mut crate::riscv64::chip::jh7110::ucstk::UcStk;

        unsafe {
            (*psp).ra = 0x0000_0721_0721_0721;
            (*psp).sp = 0x0721_0721_0721_0721;
            (*psp).gp = 0x0000_0721_0721_0721;
            (*psp).tp = 0x0000_0721_0721_0721;
            (*psp).t0 = 0x0000_0721_0721_0721;
            (*psp).t1 = 0x0000_0721_0721_0721;
            (*psp).t2 = 0x0000_0721_0721_0721;
            (*psp).s0 = 0x0000_0721_0721_0721;
            (*psp).s1 = 0x0000_0721_0721_0721;
            (*psp).a0 = 0x0000_0721_0721_0721;
            (*psp).a1 = 0x0000_0721_0721_0721;
            (*psp).a2 = 0x0000_0721_0721_0721;
            (*psp).a3 = 0x0000_0721_0721_0721;
            (*psp).a4 = 0x0000_0721_0721_0721;
            (*psp).a5 = 0x0000_0721_0721_0721;
            (*psp).a6 = 0x0000_0721_0721_0721;
            (*psp).a7 = 0x0000_0721_0721_0721;
            (*psp).s2 = 0x0000_0721_0721_0721;
            (*psp).s3 = 0x0000_0721_0721_0721;
            (*psp).s4 = 0x0000_0721_0721_0721;
            (*psp).s5 = 0x0000_0721_0721_0721;
            (*psp).s6 = 0x0000_0721_0721_0721;
            (*psp).s7 = 0x0000_0721_0721_0721;
            (*psp).s8 = 0x0000_0721_0721_0721;
            (*psp).s9 = 0x0000_0721_0721_0721;
            (*psp).s10 = 0x0000_0721_0721_0721;
            (*psp).s11 = 0x0000_0721_0721_0721;
            (*psp).t3 = 0x0000_0721_0721_0721;
            (*psp).t4 = 0x0000_0721_0721_0721;
            (*psp).t5 = 0x0000_0721_0721_0721;
            (*psp).t6 = 0x0000_0721_0721_0721;

            (*psp).mepc = executor_function_ptr as usize;
        }

        NonNull::new(ptos as *mut usize).unwrap()
    }

    fn enter_idle_state() {}

    fn shutdown() {
        loop {
            // 使用 RISC-V WFI 指令进入低功耗状态
            unsafe { asm!("wfi"); }
        }
    }

    unsafe fn get_current_stack_pointer() -> *mut usize {
        let mut sp: usize;
        asm!("csrr {}, mscratch", out(reg) sp);
        sp as *mut usize
    }
}

impl PlatformMemoryLayout for PlatformImpl {
    fn get_stack_start() -> usize {
        // JH7110 的内存布局 - 根据实际硬件调整
        0x41400000
    }

    fn get_max_programs() -> usize {
        10
    }

    fn get_heap_size() -> usize {
        10 * 1024 // 10 KiB
    }

    fn get_program_stack_size() -> usize {
        8192 // 8 KiB (RISC-V64 需要更大的栈空间)
    }

    fn get_interrupt_stack_size() -> usize {
        4096 // 4 KiB
    }
}

impl Platform for PlatformImpl {
    fn get_timer_driver(&'static self) -> &'static dyn crate::traits::timer::Driver {
        &self.timer
    }
}

// ===== 系统信息输出辅助函数 =====

/// 读取 sp - 栈指针
#[inline]
fn sp() -> usize {
    let mut sp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }
    sp
}

/// 读取 ra - 返回地址
#[inline]
fn ra() -> usize {
    let mut ra: usize;
    unsafe {
        core::arch::asm!("mv {}, ra", out(reg) ra);
    }
    ra
}

impl PlatformImpl {
    /// 打印完整的系统信息
    ///
    /// 输出当前系统状态，包括：
    /// - Hart ID
    /// - Trap Vector 配置
    /// - Machine Status
    /// - 中断使能和挂起状态
    /// - 栈信息
    /// - 代码位置
    pub fn print_system_info() {
        use riscv::register::{mhartid, mepc, mie, mip, mscratch, mtvec, mstatus};

        os_log!(info, "========================================");
        os_log!(info, "  Embassy Preempt - System Info");
        os_log!(info, "  VisionFive2 JH7110 Platform");
        os_log!(info, "========================================");

        // Hart ID
        let hartid = mhartid::read();
        os_log!(info, "[Hart Information]");
        os_log!(info, "  mhartid (Hart ID): {:#x} ({})", hartid, hartid);

        // 陷阱向量
        let mtvec = mtvec::read();
        let mode = mtvec.trap_mode();
        let base = mtvec.address();
        os_log!(info, "[Trap Vector]");
        os_log!(info, "  mtvec: {:#x}", mtvec.bits());
        os_log!(info, "    Mode: {}", match mode {
            riscv::register::mtvec::TrapMode::Direct => "Direct (all traps to BASE)",
            riscv::register::mtvec::TrapMode::Vectored => "Vectored (exceptions to BASE, interrupts to BASE+4*cause)",
        });
        os_log!(info, "    Base: {:#x}", base);

        // 状态寄存器
        let mstatus = mstatus::read();
        let mpp = match mstatus.mpp() {
            riscv::register::mstatus::MPP::User => "U",
            riscv::register::mstatus::MPP::Supervisor => "S",
            riscv::register::mstatus::MPP::Machine => "M",
            _ => "?",
        };
        os_log!(info, "[Machine Status]");
        os_log!(info, "  mstatus: {:#x}", mstatus.bits());
        os_log!(info, "    MIE: {}, MPIE: {}, MPP: {}",
            if mstatus.mie() { "1" } else { "0" },
            if mstatus.mpie() { "1" } else { "0" },
            mpp
        );

        // 异常程序计数器
        let mepc = mepc::read();
        os_log!(info, "[Exception Program Counter]");
        os_log!(info, "  mepc: {:#x}", mepc);

        // 中断使能
        let mie = mie::read();
        os_log!(info, "[Interrupt Enable]");
        os_log!(info, "  mie: {:#x}", mie.bits());
        os_log!(info, "    MIE bits:");
        os_log!(info, "      SSIP: {}, MSIP: {}, STIP: {}, MTIP: {}, SEIP: {}, MEIP: {}",
            if mie.ssoft() { "1" } else { "0" },
            if mie.msoft() { "1" } else { "0" },
            if mie.stimer() { "1" } else { "0" },
            if mie.mtimer() { "1" } else { "0" },
            if mie.sext() { "1" } else { "0" },
            if mie.mext() { "1" } else { "0" }
        );

        // 中断挂起
        let mip = mip::read();
        os_log!(info, "[Interrupt Pending]");
        os_log!(info, "  mip: {:#x}", mip.bits());
        os_log!(info, "    MIP bits:");
        os_log!(info, "      SSIP: {}, MSIP: {}, STIP: {}, MTIP: {}, SEIP: {}, MEIP: {}",
            if mip.ssoft() { "1" } else { "0" },
            if mip.msoft() { "1" } else { "0" },
            if mip.stimer() { "1" } else { "0" },
            if mip.mtimer() { "1" } else { "0" },
            if mip.sext() { "1" } else { "0" },
            if mip.mext() { "1" } else { "0" }
        );

        // 栈信息
        let mscratch = mscratch::read();
        let sp = sp();
        os_log!(info, "[Stack Information]");
        os_log!(info, "  mscratch: {:#x}", mscratch);
        os_log!(info, "  sp (stack pointer): {:#x}", sp);

        // 代码地址
        let ra = ra();
        os_log!(info, "[Code Location]");
        os_log!(info, "  ra (return address): {:#x}", ra);

        os_log!(info, "========================================");
        os_log!(info, "  System Info Complete");
        os_log!(info, "========================================");
    }
}