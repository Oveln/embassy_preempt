#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use core::ffi::c_void;
use core::sync::atomic::{Ordering};
use portable_atomic::AtomicBool;

use critical_section::Mutex;
use embassy_preempt_executor::{os_time::blockdelay::delay, os_time::timer::Timer, AsyncOSTaskCreate, OSInit, OSStart, SyncOSTaskCreate};
use embassy_preempt_log::task_log;
use embassy_preempt_platform::{chip::constants::interrupt::MSIP, get_platform, get_platform_trait};

static EXECUTION_ORDER: Mutex<[&'static str; 20]> = Mutex::new([""; 20]);
static mut ORDER_INDEX: usize = 0;

// ============================================================================
// Hart 间通信共享结构体
// ============================================================================

/// Hart 同步标志位
/// 位于共享内存，用于 hart0 和 hart1 之间的通信
#[repr(C)]
pub struct HartSyncFlags {
    pub magic_number: u16,
    /// Hart1 上的 OS 是否已经启动
    /// true = 已启动, false = 未启动
    pub hart1_os_ready: AtomicBool,
    /// Hart0 是否发送了 IPI（Inter-Processor Interrupt）
    /// true = 已发送, false = 未发送
    pub hart0_ipi_sent: AtomicBool,
}

impl HartSyncFlags {
    /// 初始化 HartSyncFlags
    #[inline]
    pub fn init(&self) {
        // 验证共享内存是否已正确初始化
        const MAGIC: u16 = 0x0721;
        unsafe {
            // 使用 volatile 写入确保魔术数字被写入
            (self as *const Self as *mut u16).write_volatile(MAGIC);
        }
        self.hart1_os_ready.store(false, Ordering::SeqCst);
        self.hart0_ipi_sent.store(false, Ordering::SeqCst);
    }

    /// 验证魔术数字是否正确
    #[inline]
    pub fn is_valid(&self) -> bool {
        unsafe {
            (self as *const Self as *const u16).read_volatile() == 0x0721
        }
    }

    /// Hart0 检查 Hart1 是否已准备好
    #[inline]
    pub fn is_hart1_ready(&self) -> bool {
        self.hart1_os_ready.load(Ordering::SeqCst)
    }

    /// Hart1 设置 OS 启动完成标志
    #[inline]
    pub fn set_hart1_ready(&self) {
        self.hart1_os_ready.store(true, Ordering::SeqCst);
    }

    /// Hart0 设置 IPI 发送标志
    #[inline]
    pub fn set_ipi_sent(&self) {
        self.hart0_ipi_sent.store(true, Ordering::SeqCst);
    }

    /// Hart1 清除 IPI 发送标志
    #[inline]
    pub fn clear_ipi_sent(&self) {
        self.hart0_ipi_sent.store(false, Ordering::SeqCst);
    }

    /// Hart1 检查是否收到 IPI
    #[inline]
    pub fn is_ipi_sent(&self) -> bool {
        self.hart0_ipi_sent.load(Ordering::SeqCst)
    }
}

// ============================================================================
// CSR 寄存器读取辅助模块
// ============================================================================

mod csr {
    use core::arch::asm;

    /// 读取当前硬件线程 ID
    #[inline]
    pub unsafe fn mhartid() -> usize {
        let mut value: usize;
        asm!("csrr {}, mhartid", out(reg) value);
        value
    }

    /// 读取机器模式陷阱向量基地址
    #[inline]
    pub unsafe fn mtvec() -> usize {
        let mut value: usize;
        asm!("csrr {}, mtvec", out(reg) value);
        value
    }

    /// 读取机器模式状态寄存器
    #[inline]
    pub unsafe fn mstatus() -> usize {
        let mut value: usize;
        asm!("csrr {}, mstatus", out(reg) value);
        value
    }

    /// 读取机器模式异常程序计数器
    #[inline]
    pub unsafe fn mepc() -> usize {
        let mut value: usize;
        asm!("csrr {}, mepc", out(reg) value);
        value
    }

    /// 读取机器模式中断使能寄存器
    #[inline]
    pub unsafe fn mie() -> usize {
        let mut value: usize;
        asm!("csrr {}, mie", out(reg) value);
        value
    }

    /// 读取机器模式中断挂起寄存器
    #[inline]
    pub unsafe fn mip() -> usize {
        let mut value: usize;
        asm!("csrr {}, mip", out(reg) value);
        value
    }

    /// 读取机器模式临时寄存器
    #[inline]
    pub unsafe fn mscratch() -> usize {
        let mut value: usize;
        asm!("csrr {}, mscratch", out(reg) value);
        value
    }

    /// 读取栈指针
    #[inline]
    pub unsafe fn sp() -> usize {
        let mut value: usize;
        asm!("mv {}, sp", out(reg) value);
        value
    }

    /// 读取返回地址
    #[inline]
    pub unsafe fn ra() -> usize {
        let mut value: usize;
        asm!("mv {}, ra", out(reg) value);
        value
    }
}

// ============================================================================
// BSS 段清理
// ============================================================================

fn clear_bss() {
    unsafe extern "C" {
        static __sbss: u8;
        static __ebss: u8;
    }
    unsafe {
        core::slice::from_raw_parts_mut(
            &__sbss as *const u8 as *mut u8,
            &__ebss as *const u8 as usize - &__sbss as *const u8 as usize,
        )
        .fill(0);
    }
}

// ============================================================================
// 系统信息输出函数
// ============================================================================

fn print_trap_vector_info() {
    let mtvec = unsafe { csr::mtvec() };
    let mode = mtvec & 0x3;
    let base = mtvec & !0x3;
    let mode_str = match mode {
        0 => "Direct (all traps to BASE)",
        1 => "Vectored (exceptions to BASE, interrupts to BASE+4*cause)",
        _ => "Reserved",
    };
    task_log!(info, "[Trap Vector]");
    task_log!(info, "  mtvec: {:#x}", mtvec);
    task_log!(info, "    Mode: {}", mode_str);
    task_log!(info, "    Base: {:#x}", base);
}

fn print_hart_info() {
    let hartid = unsafe { csr::mhartid() };
    task_log!(info, "[Hart Information]");
    task_log!(info, "  mhartid (Hart ID): {:#x} ({})", hartid, hartid);
}

fn print_machine_status() {
    let mstatus = unsafe { csr::mstatus() };
    let mpp = match (mstatus >> 11) & 0x3 {
        0 => "User",
        1 => "Supervisor",
        3 => "Machine",
        _ => "Unknown",
    };
    task_log!(info, "[Machine Status]");
    task_log!(info, "  mstatus: {:#x}", mstatus);
    task_log!(
        info,
        "    MIE: {}, MPIE: {}, MPP: {}",
        if (mstatus >> 3) & 1 == 1 { "1" } else { "0" },
        if (mstatus >> 7) & 1 == 1 { "1" } else { "0" },
        mpp
    );
}

fn print_interrupt_info() {
    // 中断使能
    let mie = unsafe { csr::mie() };
    task_log!(info, "[Interrupt Enable]");
    task_log!(info, "  mie: {:#x}", mie);
    task_log!(info, "    MIE bits:");
    task_log!(
        info,
        "      SSIP: {}, MSIP: {}, STIP: {}, MTIP: {}, SEIP: {}, MEIP: {}",
        if (mie >> 1) & 1 == 1 { "1" } else { "0" },
        if (mie >> 3) & 1 == 1 { "1" } else { "0" },
        if (mie >> 5) & 1 == 1 { "1" } else { "0" },
        if (mie >> 7) & 1 == 1 { "1" } else { "0" },
        if (mie >> 9) & 1 == 1 { "1" } else { "0" },
        if (mie >> 11) & 1 == 1 { "1" } else { "0" }
    );

    // 中断挂起
    let mip = unsafe { csr::mip() };
    task_log!(info, "[Interrupt Pending]");
    task_log!(info, "  mip: {:#x}", mip);
    task_log!(info, "    MIP bits:");
    task_log!(
        info,
        "      SSIP: {}, MSIP: {}, STIP: {}, MTIP: {}, SEIP: {}, MEIP: {}",
        if (mip >> 1) & 1 == 1 { "1" } else { "0" },
        if (mip >> 3) & 1 == 1 { "1" } else { "0" },
        if (mip >> 5) & 1 == 1 { "1" } else { "0" },
        if (mip >> 7) & 1 == 1 { "1" } else { "0" },
        if (mip >> 9) & 1 == 1 { "1" } else { "0" },
        if (mip >> 11) & 1 == 1 { "1" } else { "0" }
    );
}

fn print_stack_info() {
    let mscratch = unsafe { csr::mscratch() };
    let sp = unsafe { csr::sp() };
    task_log!(info, "[Stack Information]");
    task_log!(info, "  mscratch: {:#x}", mscratch);
    task_log!(info, "  sp (stack pointer): {:#x}", sp);
}

fn print_system_info() {
    task_log!(info, "========================================");
    task_log!(info, "  Embassy Preempt - System Info");
    task_log!(info, "  VisionFive2 JH7110 Platform");
    task_log!(info, "========================================");

    print_hart_info();
    print_trap_vector_info();
    print_machine_status();

    // 异常程序计数器
    let mepc = unsafe { csr::mepc() };
    task_log!(info, "[Exception Program Counter]");
    task_log!(info, "  mepc: {:#x}", mepc);

    print_interrupt_info();
    print_stack_info();

    // 代码地址
    let ra = unsafe { csr::ra() };
    task_log!(info, "[Code Location]");
    task_log!(info, "  ra (return address): {:#x}", ra);

    task_log!(info, "========================================");
    task_log!(info, "  UART Logger Initialized");
    task_log!(info, "========================================");
}

pub fn get_hart_sync() -> &'static HartSyncFlags {
    let addr: usize = 0xc8000000;
    unsafe {
        &*(addr as *const HartSyncFlags)
    }
}

// ============================================================================
// 主程序入口
// ============================================================================

#[embassy_preempt_macros::entry]
fn test_hardware() -> ! {
    // 清除 BSS 段
    clear_bss();

    // 输出陷阱向量信息（早期调试）
    print_trap_vector_info();

    // OS 初始化
    OSInit();


    task_log!(info, "[OS Status] OSInit completed!");
    task_log!(info, "========================================");
    task_log!(info, "  Hello, Embassy Preempt on VisionFive2!");
    task_log!(info, "========================================\r\n");

    // 输出完整系统信息
    print_system_info();

    unsafe {
        task_log!(info, "test machine soft interrupt in embassy preempt");
        let msip: *mut u32 =  0x0200_0000 as *mut u32;
        core::ptr::write_volatile(msip, 1);
    }
    let hart_sync: &HartSyncFlags = get_hart_sync();

    task_log!(info, "HART_SYNC addr is {:#x}", core::ptr::addr_of!(*hart_sync) as usize);

    // 初始化 HartSyncFlags（设置魔术数字和初始状态）
    hart_sync.init();

    task_log!(info, "HART_SYNC magic: {:#04x}, valid: {}", 0x0721, hart_sync.is_valid());

    // ========================================================================
    // 任务创建
    // ========================================================================
    // 优先级: 数值越大，优先级越高
    SyncOSTaskCreate(task1, core::ptr::null_mut(), core::ptr::null_mut(), 30);
    SyncOSTaskCreate(task2, core::ptr::null_mut(), core::ptr::null_mut(), 25);
    AsyncOSTaskCreate(task3, core::ptr::null_mut(), core::ptr::null_mut(), 20);
    SyncOSTaskCreate(task4, core::ptr::null_mut(), core::ptr::null_mut(), 15);
    SyncOSTaskCreate(task5, core::ptr::null_mut(), core::ptr::null_mut(), 10);
    SyncOSTaskCreate(task6, core::ptr::null_mut(), core::ptr::null_mut(), 35);
    AsyncOSTaskCreate(task7, core::ptr::null_mut(), core::ptr::null_mut(), 36);

    // 启动 OS（永不返回）
    OSStart();
}

// ============================================================================
// 任务定义
// ============================================================================

const LONG_TIME: usize = 10;
const MID_TIME: usize = 5;
const SHORT_TIME: usize = 3;

/// 任务1 - 长时间延迟
fn task1(_args: *mut c_void) {
    task_log!(info, "---task1 begin---");
    delay(LONG_TIME);
    task_log!(info, "---task1 end---");
    delay(SHORT_TIME);
}

/// 任务2 - 中等时间延迟
fn task2(_args: *mut c_void) {
    task_log!(info, "---task2 begin---");
    delay(MID_TIME);
    task_log!(info, "---task2 end---");
    delay(SHORT_TIME);
}

/// 任务3 - 异步任务
async fn task3(_args: *mut c_void) {
    task_log!(info, "---task3 begin---");
    // Timer::after_ticks(LONG_TIME as u64).await;
    task_log!(info, "---task3 end---");
    delay(SHORT_TIME);
}

/// 任务4 - 动态创建任务测试
fn task4(_args: *mut c_void) {
    task_log!(info, "---task4 begin---");
    SyncOSTaskCreate(task1, core::ptr::null_mut(), core::ptr::null_mut(), 34);
    delay(SHORT_TIME);
    task_log!(info, "---task4 end---");
    delay(SHORT_TIME);
}

/// 任务5 - 获取新创建任务栈指针测试
fn task5(_args: *mut c_void) {
    task_log!(info, "---task5 begin---");
    let ptos = core::ptr::null_mut::<usize>();
    task_log!(info, "ptos is {:p}", ptos);
    SyncOSTaskCreate(task1, core::ptr::null_mut(), ptos, 9);
    task_log!(info, "created task1 in task5");
    delay(SHORT_TIME);
    task_log!(info, "---task5 end---");
    delay(SHORT_TIME);
}

/// 任务6 - 相同优先级任务创建测试
fn task6(_args: *mut c_void) {
    task_log!(info, "---task6 begin---");
    SyncOSTaskCreate(task1, core::ptr::null_mut(), core::ptr::null_mut(), 35);
    delay(SHORT_TIME);
    task_log!(info, "---task6 end---");
    delay(SHORT_TIME);
}



/// 任务7 - 定时器循环任务
async fn task7(_args: *mut c_void) {
    let hart_sync = get_hart_sync();
    fn trigger_misp_hart1() {
        unsafe {
            task_log!(info, "test machine soft interrupt for hart 1");
            // Hart 1 的 msip 地址是 0x02000004
            let msip_hart1: *mut u32 = 0x0200_0004 as *mut u32;
            core::ptr::write_volatile(msip_hart1, 1);
        }
    }
    loop {
        task_log!(info, "hello");
        Timer::after_ticks(16_000_000).await;
        if hart_sync.is_hart1_ready() {
            hart_sync.set_ipi_sent();
            trigger_misp_hart1();
        }
    }
}
