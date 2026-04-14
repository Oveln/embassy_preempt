//! RISC-V 栈操作函数
//!
//! 这个模块提供 RISC-V 架构通用的栈操作功能，包括：
//! - 设置程序栈指针
//! - 配置中断栈
//! - 获取当前栈指针
//! - 系统关闭

use core::arch::asm;

/// 设置程序栈指针
///
/// 将给定的栈指针写入 `mscratch` 寄存器。
/// `mscratch` 用于在 trap 处理时保存任务栈指针。
///
/// # 参数
/// - `sp`: 程序栈指针
pub fn set_program_stack_pointer(sp: *mut u8) {
    unsafe {
        asm!(
            "csrw mscratch, a0",
            in("a0") sp
        );
    }
}

/// 配置中断栈并切换栈指针
///
/// 这个函数执行以下操作：
/// 1. 将当前栈指针保存到 `mscratch`
/// 2. 设置新的栈指针为中断栈
///
/// 这样在中断处理时，可以使用 `mscratch` 来切换回任务栈。
///
/// # 参数
/// - `interrupt_stack`: 中断栈指针
#[inline(never)]
pub fn configure_interrupt_stack(interrupt_stack: *mut u8) {
    unsafe {
        asm!(
            "mv sp, a0",
            "csrrw sp, mscratch, sp",
            "ret",
            in("a0") interrupt_stack
        );
    }
}

/// 获取当前任务栈指针
///
/// 从 `mscratch` 寄存器读取当前任务栈指针。
/// 这个值是在 `set_program_stack_pointer` 中设置的。
///
/// # 返回
/// 当前任务栈指针
///
/// # Safety
/// 必须在栈指针有意义的上下文中调用。
pub unsafe fn get_current_stack_pointer() -> *mut usize {
    let mut sp: usize;
    asm!("csrr {}, mscratch", out(reg) sp);
    sp as *mut usize
}
