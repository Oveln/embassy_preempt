//! RISC-V 中断处理模块
//!
//! 提供中断分发和处理功能，支持定时器中断、外部中断等。

use core::arch::asm;

use crate::riscv64::chip::jh7110::exception::{EXCEPTION_CODE_MASK, INTERRUPT_BIT, get_exception_reason};

/// 中断处理函数类型
pub type InterruptHandler = unsafe extern "C" fn();

/// 中断处理函数表
///
/// 索引对应 RISC-V 中断编号：
/// - 1: Supervisor software interrupt
/// - 3: Machine software interrupt
/// - 5: Supervisor timer interrupt
/// - 7: Machine timer interrupt
/// - 9: Supervisor external interrupt
/// - 11: Machine external interrupt
///
/// 注意：Machine timer interrupt 由 riscv_rt 的 `#[riscv_rt::core_interrupt]`
/// 属性处理，不在此处注册。
static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 16] = [
    None,                           // 0: 保留
    None,                           // 1: Supervisor software interrupt
    None,                           // 2: 保留
    None,                           // 3: Machine software interrupt
    None,                           // 4: Supervisor timer interrupt
    None,                           // 5: 保留
    None,                           // 6: 保留
    None,                           // 7: Machine timer interrupt (由 riscv_rt 处理)
    None,                           // 8: Supervisor external interrupt
    None,                           // 9: 保留
    None,                           // 10: 保留
    None,                           // 11: Machine external interrupt
    None, None, None, None,          // 12-15: 保留
];

/// 注册中断处理函数
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
    if interrupt_num >= 16 {
        return Err("Interrupt number out of range");
    }
    INTERRUPT_HANDLERS[interrupt_num] = Some(handler);
    os_log!(info, "Registered handler for interrupt {}", interrupt_num);
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
/// - mcause bit 31 = 1: 中断
/// - mcause bit 31 = 0: 异常
/// - mcause[4:0]: 中断/异常码
///
/// # 注意
/// 寄存器的保存和恢复由 `__trap_entry` 汇编代码处理。
/// 对于中断，此函数正常返回，由汇编代码执行 mret。
/// 对于异常，此函数调用 abort() 不会返回。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn handle_exception(mcause: usize) {
    let is_interrupt = (mcause & INTERRUPT_BIT) != 0;
    let exception_code = mcause & EXCEPTION_CODE_MASK;

    if is_interrupt {
        // 中断处理路径
        handle_interrupt(exception_code);
    } else {
        // 异常处理路径
        handle_exception_sync(mcause, exception_code);
        // 异常处理调用 abort()，不会返回
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
    crate::riscv64::chip::jh7110::exception::abort();
}

/// 处理中断
#[inline]
unsafe fn handle_interrupt(exception_code: usize) {
    match exception_code {
        3 => {
            // Machine software interrupt
            // 通常用于核间通信，JH7110 是单核，暂时忽略
            os_log!(trace, "Machine software interrupt received");
        }
        7 => {
            // Machine timer interrupt
            // 由 riscv_rt 的 machine_timer 函数处理
            // 这里不需要做额外处理，直接返回
            os_log!(trace, "Machine timer interrupt received");
        }
        11 => {
            // Machine external interrupt
            // 需要查询 PLIC (Platform-Level Interrupt Controller)
            // 获取具体的中断源并分发处理
            os_log!(trace, "Machine external interrupt received");
            handle_external_interrupt();
        }
        _ => {
            // 未知中断
            os_log!(error, "Unknown interrupt: code {}", exception_code);
        }
    }
}

/// 处理外部中断（通过 PLIC）
///
/// JH7110 使用 PLIC (Platform-Level Interrupt Controller)
/// 来管理外部中断源。
///
/// # PLIC 寄存器布局
/// - 基地址: 0x0C00_0000
/// - M-mode claim register: 0x0020_0000
/// - M-mode enable register: 0x0020_0004
///
/// # 中断处理流程
/// 1. 读取 claim 寄存器获取最高优先级的挂起中断 ID
/// 2. 根据中断 ID 分发到具体的中断处理函数
/// 3. 完成后写回中断 ID 到 claim 寄存器完成中断
unsafe fn handle_external_interrupt() {
    // PLIC base address for JH7110
    const PLIC_BASE: usize = 0x0C00_0000;

    // PLIC 寄存器偏移
    // M-mode context 的 claim/complete 寄存器
    const PLIC_MCLAIM: usize = PLIC_BASE + 0x0020_0004;

    // 读取 claim 寄存器获取中断 ID
    let claim = PLIC_MCLAIM as *mut u32;
    let interrupt_id = claim.read_volatile();

    if interrupt_id == 0 {
        // spurious interrupt (虚假中断)
        return;
    }

    os_log!(trace, "External interrupt ID: {}", interrupt_id);

    // TODO: 根据中断 ID 分发到具体的中断处理函数
    // 例如：
    // match interrupt_id {
    //     1 => handle_uart_interrupt(),
    //     2 => handle_gpio_interrupt(),
    //     ...
    //     _ => os_log!(error, "Unhandled external interrupt: {}", interrupt_id),
    // }

    // 完成中断处理 - 写回中断 ID
    claim.write_volatile(interrupt_id);
}

/// PLIC 外部中断源 ID 定义
///
/// JH7110 PLIC 中断源 (参考 JH7110 数据手册)
pub mod plic {
    /// PLIC 中断源编号
    ///
    /// 这些编号对应 JH7110 的不同外设中断
    #[repr(u32)]
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
    }
}
