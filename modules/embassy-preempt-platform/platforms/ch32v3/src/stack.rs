//! CH32V3 栈操作函数
//!
//! 这个模块提供 CH32V3 (Qingke RISC-V) 架构的栈操作功能

use core::arch::asm;

/// 设置程序栈指针
///
/// 将给定的栈指针写入 `mscratch` 寄存器。
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
/// # Safety
/// 必须在栈指针有意义的上下文中调用。
pub unsafe fn get_current_stack_pointer() -> *mut usize {
    qingke::riscv::register::mscratch::read() as *mut usize
}
