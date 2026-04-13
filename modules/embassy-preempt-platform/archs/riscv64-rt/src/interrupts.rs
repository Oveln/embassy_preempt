//! RISC-V 中断处理
//!
//! 提供 RISC-V 中断的分发和处理功能。

use crate::TrapFrame;

/// RISC-V 中断类型
enum Interrupt {
    SupervisorSoft = 1,
    MachineSoft = 3,
    SupervisorTimer = 5,
    MachineTimer = 7,
    SupervisorExternal = 9,
    MachineExternal = 11,
}

impl Interrupt {
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
/// 必须在中断禁用状态下调用
pub unsafe fn register_timer_callback(callback: TimerInterruptCallback) {
    TIMER_CALLBACK = Some(callback);
}

/// IPI (Inter-Processor Interrupt) 回调类型
///
/// 当 MSIP 中断发生时调用此回调。
/// 使用普通的Rust函数指针以匹配Platform trait的定义
pub type IpiCallback = fn(*mut ());

/// IPI 回调函数指针
static mut IPI_CALLBACK: Option<(IpiCallback, *mut ())> = None;

/// 注册 IPI 回调函数
///
/// # Safety
/// 必须在中断禁用状态下调用
pub unsafe fn register_ipi_callback(callback: IpiCallback, ctx: *mut ()) {
    IPI_CALLBACK = Some((callback, ctx));
}

/// 中断分发函数
///
/// # Safety
/// 从 trap_handler 调用
#[no_mangle]
pub unsafe extern "C" fn dispatch_interrupt(trap_frame: &mut TrapFrame, code: usize) {
    match Interrupt::from_code(code) {
        Some(Interrupt::SupervisorSoft) => {
            // 处理 Supervisor 软件中断
            os_log!(trace, "[IRQ] Supervisor Software Interrupt");
        }
        Some(Interrupt::MachineSoft) => {
            // 清除 MSIP
            let msip: *mut usize = 0x02000000 as *mut usize;
            core::ptr::write_volatile(msip, 0);
            os_log!(trace, "[IRQ] MSIP (IPI)");

            // 调用注册的 IPI 回调
            if let Some((callback, ctx)) = IPI_CALLBACK {
                callback(ctx);
            }
        }
        Some(Interrupt::SupervisorTimer) => {
            // 处理 Supervisor 定时器中断
            os_log!(trace, "[IRQ] Supervisor Timer Interrupt");
        }
        Some(Interrupt::MachineTimer) => {
            // 处理 Machine 定时器中断
            os_log!(trace, "[IRQ] Machine Timer Interrupt");

            // 调用注册的 Timer 回调
            if let Some(callback) = TIMER_CALLBACK {
                callback();
            }
        }
        Some(Interrupt::SupervisorExternal) => {
            // 处理 Supervisor 外部中断
            os_log!(trace, "[IRQ] Supervisor External Interrupt");
        }
        Some(Interrupt::MachineExternal) => {
            // 处理 Machine 外部中断
            os_log!(trace, "[IRQ] Machine External Interrupt");
        }
        None => {
            panic!("Unhandled interrupt: code={:#x}", code);
        }
    }
}
