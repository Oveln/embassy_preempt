//! RISC-V 中断处理模块
//!
//! 提供中断分发和处理功能，支持定时器中断、外部中断等。

use core::{arch::asm, sync::atomic::Ordering};

use embassy_preempt_traits::Platform;
use embassy_preempt_traits::timer::Driver;
use portable_atomic::AtomicBool;

use crate::exception::{EXCEPTION_CODE_MASK, INTERRUPT_BIT, get_exception_reason};

// ============================================================================
// 类型定义
// ============================================================================

/// 中断处理函数类型
pub type InterruptHandler = unsafe extern "C" fn();

/// PLIC 外部中断处理函数类型
pub type PlicHandler = unsafe extern "C" fn(u32);

// ============================================================================
// RISC-V 中断编号常量
// ============================================================================

/// RISC-V 中断编号 (mcause.ExceptionCode)
pub mod riscv_irq {
    /// 保留
    pub const RESERVED: usize = 0;

    /// Supervisor software interrupt
    pub const S_SOFT: usize = 1;

    /// 保留
    pub const _RESERVED_2: usize = 2;

    /// Machine software interrupt
    pub const M_SOFT: usize = 3;

    /// Supervisor timer interrupt
    pub const S_TIMER: usize = 5;

    /// Machine timer interrupt
    pub const M_TIMER: usize = 7;

    /// Supervisor external interrupt
    pub const S_EXTERNAL: usize = 9;

    /// Machine external interrupt
    pub const M_EXTERNAL: usize = 11;

    /// 最大中断编号
    pub const MAX_IRQ: usize = 16;
}

// ============================================================================
// 全局状态
// ============================================================================

/// 全局标志：是否在中断处理中
/// true = 在中断处理中, false = 不在中断处理中
pub(crate) static IN_INTERRUPT: AtomicBool = AtomicBool::new(false);

/// 全局标志：是否需要延迟的上下文切换
/// true = 需要, false = 不需要
pub(crate) static NEED_CONTEXT_SWITCH: AtomicBool = AtomicBool::new(false);

/// RISC-V 中断处理函数表
///
/// 索引对应 RISC-V 中断编号：
/// - 1: Supervisor software interrupt
/// - 3: Machine software interrupt
/// - 5: Supervisor timer interrupt
/// - 7: Machine timer interrupt
/// - 9: Supervisor external interrupt
/// - 11: Machine external interrupt
static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; riscv_irq::MAX_IRQ] = [const { None }; riscv_irq::MAX_IRQ];

/// PLIC 外部中断处理函数表
///
/// 索引对应 PLIC 中断源 ID (1-127)
/// ID 0 表示无中断
static mut PLIC_HANDLERS: [Option<PlicHandler>; 128] = [const { None }; 128];

// ============================================================================
// RISC-V 中断处理
// ============================================================================

/// 注册 RISC-V 中断处理函数
///
/// # 参数
/// - `interrupt_num`: RISC-V 中断编号 (0-15)
/// - `handler`: 中断处理函数指针
///
/// # 返回
/// - `Ok(())`: 注册成功
/// - `Err(&'static str)`: 错误信息
///
/// # Safety
/// 必须在中断禁用状态下调用，handler 必须是有效的函数指针
pub unsafe fn register_interrupt_handler(
    interrupt_num: usize,
    handler: InterruptHandler,
) -> Result<(), &'static str> {
    if interrupt_num >= riscv_irq::MAX_IRQ {
        return Err("Interrupt number out of range (must be < 16)");
    }
    INTERRUPT_HANDLERS[interrupt_num] = Some(handler);
    os_log!(info, "[IRQ] Registered handler for RISC-V IRQ {}", interrupt_num);
    Ok(())
}

/// 处理非 ecall 的异常和中断
///
/// 这是主要的异常/中断处理入口，由汇编代码 `__trap_entry` 调用。
///
/// # 处理流程
/// 1. 检查 mcause 判断是中断还是异常
/// 2. 如果是中断，根据中断码分发处理，然后返回
/// 3. 如果是异常，打印错误信息并调用 abort()（不返回）
///
/// # RISC-V 中断编码
/// - mcause bit 63 = 1: 中断
/// - mcause bit 63 = 0: 异常
/// - mcause[4:0]: 中断/异常码
///
/// # 返回值
/// - 1: 需要进行上下文切换（延迟的上下文切换请求）
/// - 0: 正常返回，不需要上下文切换
///
/// # 注意
/// 寄存器的保存和恢复由 `__trap_entry` 汇编代码处理。
/// 对于中断，此函数正常返回，由汇编代码执行 mret。
/// 对于异常，此函数调用 abort() 不会返回。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn handle_exception(mcause: usize) -> usize {
    let is_interrupt = (mcause & INTERRUPT_BIT) != 0;
    let exception_code = mcause & EXCEPTION_CODE_MASK;

    if is_interrupt {
        // 设置中断标志
        IN_INTERRUPT.store(true, Ordering::SeqCst);

        // 中断处理路径
        handle_interrupt(exception_code);

        // 清除中断标志
        IN_INTERRUPT.store(false, Ordering::SeqCst);

        // 检查是否需要延迟的上下文切换
        if NEED_CONTEXT_SWITCH.load(Ordering::SeqCst) {
            NEED_CONTEXT_SWITCH.store(false, Ordering::SeqCst);
            return 1; // 需要上下文切换
        }
        return 0; // 正常返回
    } else {
        // 异常处理路径
        handle_exception_sync(mcause, exception_code);
        // 异常处理调用 abort()，不会返回
        unreachable!();
    }
}

/// 处理同步异常
#[inline]
unsafe fn handle_exception_sync(mcause: usize, exception_code: usize) {
    let reason = get_exception_reason(mcause);

    os_log!(error, "");
    os_log!(error, "=== Trap Handler ===");
    os_log!(error, "Unhandled exception: {}", reason);
    os_log!(error, "mcause = {:#016x}", mcause);

    let mut mepc: usize;
    let mut mtval: usize;
    asm!("csrr {}, mepc", out(reg) mepc);
    asm!("csrr {}, mtval", out(reg) mtval);

    os_log!(error, "mepc  = {:#016x}", mepc);
    os_log!(error, "mtval = {:#016x}", mtval);

    // 对于异常，调用 abort
    crate::exception::abort();
}

/// 处理 RISC-V 中断
#[inline]
unsafe fn handle_interrupt(exception_code: usize) {
    // 从中断处理表中查找处理函数
    if let Some(handler) = INTERRUPT_HANDLERS.get(exception_code) {
        if let Some(handler_fn) = *handler {
            // 调用注册的处理函数
            handler_fn();
            return;
        }
    }

    // 没有注册处理函数，使用默认处理逻辑
    handle_interrupt_default(exception_code);
}

/// 默认中断处理逻辑（未注册处理函数时使用）
#[inline]
unsafe fn handle_interrupt_default(exception_code: usize) {
    match exception_code {
        riscv_irq::M_SOFT => {
            let msip: *mut usize = 0x02000000 as *mut usize;
            core::ptr::write_volatile(msip, 0);
            os_log!(trace, "[IRQ] MSIP");
        }
        riscv_irq::M_TIMER => {
            os_log!(trace, "[IRQ] Machine Timer Interrupt");
            // 调用定时器驱动的中断处理
            if let Some(timer) = unsafe { crate::platform::TIMER_DRIVER } {
                unsafe {
                    timer.on_interrupt();
                }
            } else {
                os_log!(warn, "[IRQ] Timer driver not initialized");
            }
        }
        riscv_irq::M_EXTERNAL => {
            // Machine external interrupt
            // 需要查询 PLIC (Platform-Level Interrupt Controller)
            // 获取具体的中断源并分发处理
            os_log!(trace, "[IRQ] Machine External Interrupt (no handler registered)");
            handle_plic_interrupts();
        }
        _ => {
            // 未知中断
            os_log!(error, "[IRQ] Unknown interrupt: code {} (no handler registered)", exception_code);
        }
    }
}

// ============================================================================
// PLIC 外部中断处理
// ============================================================================

/// PLIC 寄存器定义
mod plic_regs {
    /// PLIC 基地址 for JH7110
    pub const PLIC_BASE: usize = 0x0C00_0000;

    /// 每个 context 的寄存器大小
    pub const CONTEXT_SIZE: usize = 0x1000;

    /// M-mode context 编号
    pub const M_CONTEXT: usize = 0;

    /// M-mode claim/complete 寄存器偏移 (4 bytes)
    pub const CLAIM_COMPLETE_OFFSET: usize = 0x0004;

    /// 计算 M-mode claim/complete 寄存器绝对地址
    #[inline]
    pub const fn m_claim_complete() -> usize {
        PLIC_BASE + (M_CONTEXT * CONTEXT_SIZE) + CLAIM_COMPLETE_OFFSET
    }
}

/// 注册 PLIC 外部中断处理函数
///
/// # 参数
/// - `interrupt_id`: PLIC 中断源 ID (1-127)
/// - `handler`: 中断处理函数指针，接收中断 ID 作为参数
///
/// # 返回
/// - `Ok(())`: 注册成功
/// - `Err(&'static str)`: 错误信息
///
/// # Safety
/// 必须在中断禁用状态下调用，handler 必须是有效的函数指针
pub unsafe fn register_plic_handler(
    interrupt_id: u32,
    handler: PlicHandler,
) -> Result<(), &'static str> {
    if interrupt_id == 0 || interrupt_id >= 128 {
        return Err("PLIC interrupt ID must be in range 1-127");
    }
    PLIC_HANDLERS[interrupt_id as usize] = Some(handler);
    os_log!(info, "[IRQ] Registered handler for PLIC IRQ {}", interrupt_id);
    Ok(())
}

/// 处理 PLIC 外部中断
///
/// JH7110 使用 PLIC (Platform-Level Interrupt Controller)
/// 来管理外部中断源。
///
/// # 中断处理流程
/// 1. 循环读取 claim 寄存器获取最高优先级的挂起中断 ID
/// 2. 根据中断 ID 分发到具体的中断处理函数
/// 3. 完成后写回中断 ID 到 claim 寄存器完成中断
/// 4. 当 claim 返回 0 时，表示没有更多挂起中断
unsafe fn handle_plic_interrupts() {
    let claim_complete = plic_regs::m_claim_complete() as *mut u32;

    // 循环处理所有挂起的中断
    loop {
        // 读取 claim 寄存器获取中断 ID
        let interrupt_id = claim_complete.read_volatile();

        if interrupt_id == 0 {
            // 没有更多挂起中断
            break;
        }

        os_log!(trace, "[IRQ] PLIC interrupt ID: {}", interrupt_id);

        // 从 PLIC 处理表中查找处理函数
        let handler = PLIC_HANDLERS.get(interrupt_id as usize)
            .and_then(|h| *h);

        if let Some(handler_fn) = handler {
            // 调用注册的处理函数
            handler_fn(interrupt_id);
        } else {
            os_log!(warn, "[IRQ] Unhandled PLIC interrupt: {}", interrupt_id);
        }

        // 完成中断处理 - 写回中断 ID
        claim_complete.write_volatile(interrupt_id);
    }
}

// ============================================================================
// PLIC 中断源 ID 定义 (JH7110)
// ============================================================================

/// PLIC 外部中断源 ID 定义
///
/// JH7110 PLIC 中断源 (参考 JH7110 数据手册)
pub mod plic {
    use super::PlicHandler;

    /// PLIC 中断源编号
    ///
    /// 这些编号对应 JH7110 的不同外设中断
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InterruptSource {
        /// UART0 中断
        UART0 = 5,
        /// UART1 中断
        UART1 = 6,
        /// SPI0 中断
        SPI0 = 9,
        /// SPI1 中断
        SPI1 = 10,
        /// SPI2 中断
        SPI2 = 11,
        /// SPI3 中断
        SPI3 = 12,
        /// I2C0 中断
        I2C0 = 13,
        /// I2C1 中断
        I2C1 = 14,
        /// I2C2 中断
        I2C2 = 15,
        /// GPIO0 边沿中断
        GPIO0_EDGE = 23,
        /// GPIO0 级或中断
        GPIO0_LEVEL = 24,
        /// Timer0 中断
        TIMER0 = 47,
        /// Timer1 中断
        TIMER1 = 48,
        /// Timer2 中断
        TIMER2 = 49,
        /// Timer3 中断
        TIMER3 = 50,
        /// PWM 中断
        PWM = 51,
        /// SDIO 中断
        SDIO = 52,
    }

    impl InterruptSource {
        /// 获取中断源 ID
        #[inline]
        pub const fn id(self) -> u32 {
            self as u32
        }

        /// 从 ID 创建中断源
        ///
        /// 如果 ID 无效，返回 None
        #[inline]
        pub const fn from_id(id: u32) -> Option<Self> {
            match id {
                5 => Some(Self::UART0),
                6 => Some(Self::UART1),
                9 => Some(Self::SPI0),
                10 => Some(Self::SPI1),
                11 => Some(Self::SPI2),
                12 => Some(Self::SPI3),
                13 => Some(Self::I2C0),
                14 => Some(Self::I2C1),
                15 => Some(Self::I2C2),
                23 => Some(Self::GPIO0_EDGE),
                24 => Some(Self::GPIO0_LEVEL),
                47 => Some(Self::TIMER0),
                48 => Some(Self::TIMER1),
                49 => Some(Self::TIMER2),
                50 => Some(Self::TIMER3),
                51 => Some(Self::PWM),
                52 => Some(Self::SDIO),
                _ => None,
            }
        }

        /// 注册此中断源的处理函数
        ///
        /// # Safety
        /// 必须在中断禁用状态下调用
        #[inline]
        pub unsafe fn register_handler(self, handler: PlicHandler) -> Result<(), &'static str> {
            crate::interrupt::register_plic_handler(self.id(), handler)
        }
    }
}
