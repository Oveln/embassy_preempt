use core::arch::asm;
use core::ptr::NonNull;

use embassy_preempt_log::{os_log, scheduler_log};
use embassy_preempt_traits::memory_layout::PlatformMemoryLayout;
use embassy_preempt_traits::platform::PlatformStatic;
use embassy_preempt_traits::Platform;
use embassy_preempt_traits::timer::Driver;

use crate::clint_config::Jh7110ClintConfig;
use crate::gpio;
use embassy_preempt_riscv64_rt::{ClintTimer, CONTEXT_STACK_SIZE};

// 静态存储，供中断处理访问定时器驱动
static mut TIMER_DRIVER_STORAGE: Option<ClintTimer<Jh7110ClintConfig, 1>> = None;

// 静态引用，供中断处理访问定时器驱动
pub static mut TIMER_DRIVER: Option<&'static ClintTimer<Jh7110ClintConfig, 1>> = None;

/// Timer 中断回调函数
///
/// 这个函数由 rt 库在中断处理时调用
unsafe extern "C" fn timer_interrupt_callback() {
    if let Some(timer) = TIMER_DRIVER {
        timer.on_interrupt();
    }
}

pub struct PlatformImpl {
    pub timer: &'static ClintTimer<Jh7110ClintConfig, 1>,
}

impl PlatformImpl {
    pub fn new() -> Self {
        os_log!(info, "Init JH7110 Platform");

        // 创建并初始化定时器驱动，存储在静态变量中
        let timer = ClintTimer::<Jh7110ClintConfig, 1>::new();
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
    fn trigger_context_switch() {
        use embassy_preempt_riscv64_rt::{IN_TRAP, NEED_CONTEXT_SWITCH};
        use core::sync::atomic::Ordering;

        unsafe {
            gpio::gpio_controller().toggle(37);
            os_log!(info, "Before ecall: MIE={}， in_Interrupt={}", riscv::register::mstatus::read().mie(), IN_TRAP.load(Ordering::Acquire));
        }

        // 检查是否在中断处理中
        // 使用 Acquire 语义：确保读取到最新的值
        if IN_TRAP.load(Ordering::Acquire) {
            // 在中断中，设置延迟上下文切换标志
            // 使用 Release 语义：确保设置操作之前的写操作完成
            NEED_CONTEXT_SWITCH.store(true, Ordering::Release);
        } else {
            // 不在中断中，直接执行 ecall 触发上下文切换
            unsafe {
                asm!("ecall");
            }
        }
    }

    fn init_task_stack(stk_ref: NonNull<usize>, executor_function: fn()) -> NonNull<usize> {
        scheduler_log!(trace, "init_task_stack for JH7110");
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function as *const () as usize);

        // Get stack pointer and align to 8-byte boundary
        let ptos = stk_ref.as_ptr() as *mut usize;
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;

        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp: *mut embassy_preempt_riscv64_rt::TrapFrame = ptos as *mut embassy_preempt_riscv64_rt::TrapFrame;

        unsafe {
            (*psp).init(executor_function);
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
}

impl PlatformMemoryLayout for PlatformImpl {
    const STACK_START: usize = 0x804_0000;
    const MAX_PROGRAMS: usize = 10;
    const HEAP_SIZE: usize = 0x8000;
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
