//! RISC-V 中断处理器
//!
//! 提供 RISC-V 中断的分发和回调注册机制。
//!
//! ## 弱符号机制
//!
//! 默认的中断处理器通过链接脚本定义为弱符号：
//! - `SupervisorSoft` - Supervisor 软件中断
//! - `MachineSoft` - Machine 软件中断 (IPI)
//! - `SupervisorTimer` - Supervisor 定时器中断
//! - `MachineTimer` - Machine 定时器中断
//! - `SupervisorExternal` - Supervisor 外部中断
//! - `MachineExternal` - Machine 外部中断
//!
//! 平台可以通过提供同名的 `#[no_mangle]` 函数来覆盖默认实现。
//!
//! ## 回调机制
//!
//! 对于 Machine Timer 和 Machine Software (IPI) 中断，
//! 此模块提供回调注册机制，允许平台动态注册处理函数。

use crate::TrapFrame;

// ============================================================================
// 中断类型定义
// ============================================================================

/// RISC-V 中断类型
///
/// 根据 RISC-V 规范，中断编码如下：
/// - 1: Supervisor Software Interrupt
/// - 3: Machine Software Interrupt
/// - 5: Supervisor Timer Interrupt
/// - 7: Machine Timer Interrupt
/// - 9: Supervisor External Interrupt
/// - 11: Machine External Interrupt
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    SupervisorSoft = 1,
    MachineSoft = 3,
    SupervisorTimer = 5,
    MachineTimer = 7,
    SupervisorExternal = 9,
    MachineExternal = 11,
}

impl Interrupt {
    /// 从中断编码创建 Interrupt 枚举
    fn from_code(code: usize) -> Option<Self> {
        match code {
            1 => Some(Self::SupervisorSoft),
            3 => Some(Self::MachineSoft),
            5 => Some(Self::SupervisorTimer),
            7 => Some(Self::MachineTimer),
            9 => Some(Self::SupervisorExternal),
            11 => Some(Self::MachineExternal),
            _ => None,
        }
    }
}

// ============================================================================
// 回调注册
// ============================================================================

/// Timer 中断回调类型
///
/// 当 Machine Timer 中断发生时调用此回调。
/// 平台应该注册一个回调来处理定时器中断。
pub type TimerInterruptCallback = unsafe extern "C" fn();

/// Timer 中断回调函数指针
static mut TIMER_CALLBACK: Option<TimerInterruptCallback> = None;

/// 注册 Timer 中断回调函数
///
/// # Safety
/// 必须在中断禁用状态下调用，防止竞态条件
pub unsafe fn register_timer_callback(callback: TimerInterruptCallback) {
    TIMER_CALLBACK = Some(callback);
}

/// IPI (Inter-Processor Interrupt) 回调类型
///
/// 当 MSIP 中断发生时调用此回调。
/// 使用普通的 Rust 函数指针以匹配 Platform trait 的定义
pub type IpiCallback = fn(*mut ());

/// IPI 回调函数指针
static mut IPI_CALLBACK: Option<(IpiCallback, *mut ())> = None;

/// 注册 IPI 回调函数
///
/// # Safety
/// 必须在中断禁用状态下调用，防止竞态条件
pub unsafe fn register_ipi_callback(callback: IpiCallback, ctx: *mut ()) {
    IPI_CALLBACK = Some((callback, ctx));
}

// ============================================================================
// 默认中断处理器实现
// ============================================================================

// 中断处理器函数声明（外部符号，由链接脚本解析为弱符号）
//
// 以下函数由链接脚本提供，平台可以提供自己的实现来覆盖默认行为。
// 如果平台未提供实现，将使用链接脚本中的默认实现（通常指向 ExceptionHandler 或 DefaultHandler）。
extern "C" {
    fn SupervisorSoft(trap_frame: &mut TrapFrame);
    fn MachineSoft(trap_frame: &mut TrapFrame);
    fn SupervisorTimer(trap_frame: &mut TrapFrame);
    fn MachineTimer(trap_frame: &mut TrapFrame);
    fn SupervisorExternal(trap_frame: &mut TrapFrame);
    fn MachineExternal(trap_frame: &mut TrapFrame);
}

/// Machine Timer 中断的默认实现
///
/// 此函数提供默认的 Machine Timer 处理逻辑，
/// 调用已注册的 Timer 回调。
/// 链接脚本会将 `MachineTimer` 弱符号指向此函数。
#[no_mangle]
unsafe extern "C" fn __MachineTimer_default(trap_frame: &mut TrapFrame) {
    os_log!(trace, "[IRQ] Machine Timer Interrupt");

    // 调用注册的 Timer 回调
    if let Some(callback) = TIMER_CALLBACK {
        callback();
    }

    let _ = trap_frame; // 未使用
}

/// Machine Software (IPI) 中断的默认实现
///
/// 此函数提供默认的 MSIP 处理逻辑：
/// 1. 清除 MSIP 位
/// 2. 调用已注册的 IPI 回调
/// 链接脚本会将 `MachineSoft` 弱符号指向此函数。
#[no_mangle]
unsafe extern "C" fn __MachineSoft_default(trap_frame: &mut TrapFrame) {
    os_log!(trace, "[IRQ] MSIP (IPI)");

    // 清除 MSIP (CLINT 基地址 + 0x0000)
    let msip: *mut usize = 0x02000000 as *mut usize;
    core::ptr::write_volatile(msip, 0);

    // 调用注册的 IPI 回调
    if let Some((callback, ctx)) = IPI_CALLBACK {
        callback(ctx);
    }

}

// ============================================================================
// 中断分发器
// ============================================================================

/// 中断分发函数
///
/// 根据 mcause 的中断编码调用对应的处理器。
///
/// # Safety
/// 从 trap_handler 调用，必须在中断上下文中执行
#[no_mangle]
pub unsafe extern "C" fn dispatch_interrupt(trap_frame: &mut TrapFrame, code: usize) {
    match Interrupt::from_code(code) {
        Some(Interrupt::SupervisorSoft) => {
            SupervisorSoft(trap_frame);
        }
        Some(Interrupt::MachineSoft) => {
            MachineSoft(trap_frame);
        }
        Some(Interrupt::SupervisorTimer) => {
            SupervisorTimer(trap_frame);
        }
        Some(Interrupt::MachineTimer) => {
            MachineTimer(trap_frame);
        }
        Some(Interrupt::SupervisorExternal) => {
            SupervisorExternal(trap_frame);
        }
        Some(Interrupt::MachineExternal) => {
            MachineExternal(trap_frame);
        }
        None => {
            panic!("Unhandled interrupt: code={:#x}", code);
        }
    }
}
