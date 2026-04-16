use core::arch::asm;

use embassy_preempt_log::{os_log, scheduler_log};
use embassy_preempt_traits::memory_layout::PlatformMemoryLayout;
use embassy_preempt_traits::platform::PlatformStatic;
use embassy_preempt_traits::Platform;
use embassy_preempt_traits::timer::Driver;

use crate::clint_config::QemuVirtClintConfig;
use crate::gpio;
use embassy_preempt_riscv64_rt::{ClintTimer, CONTEXT_STACK_SIZE};

/// QEMU Virt UART base address (16550-compatible UART at 0x10000000)
const UART_BASE: usize = 0x1000_0000;

// 静态存储，供中断处理访问定时器驱动
static mut TIMER_DRIVER_STORAGE: Option<ClintTimer<QemuVirtClintConfig, 1>> = None;

// 静态引用，供中断处理访问定时器驱动
pub static mut TIMER_DRIVER: Option<&'static ClintTimer<QemuVirtClintConfig, 1>> = None;

/// Timer 中断回调函数
///
/// 这个函数由 rt 库在中断处理时调用
unsafe extern "C" fn timer_interrupt_callback() {
    if let Some(timer) = TIMER_DRIVER {
        timer.on_interrupt();
    }
}

pub struct PlatformImpl {
    pub timer: &'static ClintTimer<QemuVirtClintConfig, 1>,
}

impl PlatformImpl {
    pub fn new() -> Self {
        os_log!(info, "Init QEMU Virt Platform");

        // 创建并初始化定时器驱动，存储在静态变量中
        let timer = ClintTimer::<QemuVirtClintConfig, 1>::new();
        timer.init();

        unsafe {
            TIMER_DRIVER_STORAGE = Some(timer);
            TIMER_DRIVER = TIMER_DRIVER_STORAGE.as_ref();

            // 注册 timer 中断回调
            embassy_preempt_riscv64_rt::register_timer_callback(timer_interrupt_callback);

            // RISC-V64 初始化代码
            riscv::register::mstatus::set_mie(); // Enable machine interrupts
            riscv::register::mie::set_msoft();

            gpio::init();
        }

        PlatformImpl {
            timer: unsafe { TIMER_DRIVER.unwrap() },
        }
    }
}

impl PlatformStatic for PlatformImpl {
    fn enter_idle_state() {}

    fn shutdown() {
        loop {
            // 使用 RISC-V WFI 指令进入低功耗状态
            unsafe { asm!("wfi"); }
        }
    }

    #[inline(never)]
    fn early_putstr(c: &[u8]) -> usize {
        unsafe {
            let uart = UART_BASE as *mut u8;
            for &byte in c {
                uart.write_volatile(byte);
            }
        }
        c.len()
    }
}

impl PlatformMemoryLayout for PlatformImpl {
    const PROGRAM_STACK_SIZE: usize = 0x4000;
    const INTERRUPT_STACK_SIZE: usize = 0x2000;
}

impl Platform for PlatformImpl {
    fn get_timer_driver(&'static self) -> &'static dyn embassy_preempt_traits::timer::Driver {
        self.timer
    }

    fn set_ipi_callback(&'static self, callback: fn(*mut ()), ctx: *mut ()) {
        unsafe {
            embassy_preempt_riscv64_rt::register_ipi_callback(callback, ctx);
        }
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
        os_log!(info, "  QEMU RISC-V 64-bit Virt Platform");
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
