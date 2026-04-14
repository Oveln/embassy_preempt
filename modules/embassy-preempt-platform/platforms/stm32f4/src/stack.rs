//! ARM Cortex-M 栈操作函数
//!
//! 这个模块提供 ARM Cortex-M 架构的栈操作功能

use core::arch::asm;
use cortex_m::register::psp;

/// 设置程序栈指针 (PSP)
///
/// 将给定的栈指针写入 PSP (Process Stack Pointer) 寄存器。
pub fn set_program_stack_pointer(sp: *mut u8) {
    unsafe {
        psp::write(sp as u32);
    }
}

/// 配置中断栈并切换到线程模式
///
/// ARM Cortex-M 特定实现：
/// 1. 设置主栈指针 (MSP) 用于中断处理
/// 2. 配置 CONTROL 寄存器在线程模式下使用 PSP
/// 3. 允许特权到非特权的转换
///
/// # 参数
/// - `interrupt_stack`: 中断栈指针 (MSP)
#[inline(never)]
pub fn configure_interrupt_stack(interrupt_stack: *mut u8) {
    unsafe {
        asm!(
            // First change the MSP to interrupt stack
            "MSR msp, r1",        // Set MSP to interrupt stack pointer
            // Then change the control register to use the PSP
            "MRS r0, control",   // Read current CONTROL register
            "ORR r0, r0, #2",    // Set bit 1 to use PSP in thread mode
            "MSR control, r0",   // Write back modified CONTROL
            "BX lr",             // Return to caller
            in("r1") interrupt_stack,
            options(nostack, preserves_flags),
        )
    }
}

/// 获取当前任务栈指针 (PSP)
///
/// # Safety
/// 必须在栈指针有意义的上下文中调用。
pub unsafe fn get_current_stack_pointer() -> *mut usize {
    let psp_value: *mut usize;
    asm!(
        "MRS     R0, PSP", // Read Process Stack Pointer into R0
        out("r0") psp_value,
        options(nostack, preserves_flags),
    );
    psp_value
}
