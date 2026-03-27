//! RISC-V 栈回溯模块
//!
//! 提供基于帧指针的栈回溯功能，用于调试和异常处理。

/// JH7110 内存布局常量
pub(crate) const CODE_START: usize = 0x4100_0000;
pub(crate) const CODE_END: usize = 0x4200_0000;
pub(crate) const STACK_START: usize = 0x4140_0000;
pub(crate) const STACK_END: usize = 0x4160_0000;

/// 最大栈回溯深度
const MAX_BACKTRACE_FRAMES: usize = 16;

/// 检查地址是否可能是有效的代码地址
#[inline]
pub(crate) fn is_valid_code_address(addr: usize) -> bool {
    addr >= CODE_START && addr < CODE_END
}

/// 检查地址是否可能是有效的栈地址
#[inline]
pub(crate) fn is_valid_stack_address(addr: usize) -> bool {
    addr >= STACK_START && addr < STACK_END
}

/// 打印栈回溯信息 - 使用帧指针进行准确回溯
///
/// # 参数
/// - `ra`: 当前返回地址 (x1 寄存器)
/// - `fp`: 当前帧指针 (x8/s0 寄存器)
/// - `mepc`: 异常程序计数器
///
/// # 工作原理
/// 使用 RISC-V 标准栈帧布局进行回溯：
/// - 栈帧中保存了返回地址和上一个帧指针
/// - 通过读取每个栈帧的 ra 和 fp 来构建调用链
///
/// # RISC-V 栈帧布局
/// ```text
/// 高地址
///   +--------+
///   | 上一个帧指针 (fp/s0/x8)  <- sp + 8
///   +--------+
///   | 返回地址 (ra/x1)         <- sp + 0
///   +--------+
///   低地址 <- sp
/// ```
pub unsafe fn print_backtrace(ra: usize, fp: usize, mepc: usize) {
    os_log!(error, "=== Stack Backtrace ===");
    os_log!(error, "  frame 0: mepc={:#016x}", mepc);

    // 如果 mepc 是有效的代码地址，显示更多信息
    if is_valid_code_address(mepc) {
        let offset = mepc - CODE_START;
        os_log!(error, "         (code offset: +{:#x})", offset);
    }

    // 尝试使用帧指针回溯（s0/fp 寄存器）
    let mut current_fp = fp;
    let mut frame_count = 0;

    // 首先打印当前 ra
    if ra != 0 && is_valid_code_address(ra) {
        os_log!(error, "  frame 1: ra={:#016x}", ra);
        let offset = ra - CODE_START;
        os_log!(error, "         fp={:#016x} (code offset: +{:#x})", current_fp, offset);
    }

    // 使用帧指针回溯
    // RISC-V 标准栈帧布局:
    //   sp + 0:  返回地址 (ra/x1)
    //   sp + 8:  上一个帧指针 (fp/s0/x8)
    for i in 2..=MAX_BACKTRACE_FRAMES {
        // 检查帧指针是否有效
        if current_fp == 0 || !is_valid_stack_address(current_fp) {
            if current_fp == 0 {
                os_log!(error, "  frame {}: <fp is null - end of stack>", i);
            } else {
                os_log!(error, "  frame {}: <invalid fp {:#x}>", i, current_fp);
            }
            break;
        }

        // 检查是否超出栈范围
        if current_fp + 16 > STACK_END {
            os_log!(error, "  frame {}: <fp {:#x} exceeds stack bound>", i, current_fp);
            break;
        }

        // 读取返回地址 (fp + 0)
        let frame_ra = *(current_fp as *const usize);
        // 读取上一个帧指针 (fp + 8)
        let prev_fp = *((current_fp + 8) as *const usize);

        if frame_ra == 0 {
            os_log!(error, "  frame {}: <ra is null - likely end of stack>", i);
            break;
        }

        // 检查返回地址是否有效
        if !is_valid_code_address(frame_ra) {
            os_log!(error, "  frame {}: <invalid ra {:#x}>", i, frame_ra);
            break;
        }

        // 打印当前帧信息
        let offset = frame_ra - CODE_START;
        os_log!(error, "  frame {}: ra={:#016x}", i, frame_ra);
        os_log!(error, "         fp={:#016x} (code offset: +{:#x})", current_fp, offset);

        // 检测可能的循环
        if prev_fp <= current_fp {
            os_log!(error, "  frame {}: <fp not increasing - possible stack corruption>", i);
            break;
        }

        // 更新到上一帧
        current_fp = prev_fp;
        frame_count = i;
    }

    os_log!(error, "=== End Backtrace ({} frames) ===", frame_count);
}

/// 打印所有通用寄存器
///
/// 从 mscratch 指向的保存上下文中读取并打印所有寄存器值。
/// 如果不在任务上下文中（mscratch 为 0），则跳过。
///
/// # 参数
/// - `sp`: 当前栈指针，用于检查 mscratch
pub unsafe fn print_all_registers(sp: usize) {
    use crate::ucstk::UcStk;

    // 尝试获取保存的上下文（如果在任务上下文中）
    let mscratch: usize;
    core::arch::asm!("csrr {}, mscratch", out(reg) mscratch);

    if mscratch != 0 {
        // mscratch 指向保存的上下文
        let ctx = mscratch as *const UcStk;
        os_log!(error, "=== Saved Context Registers ===");
        os_log!(error, "  ra   = {:#016x}", (*ctx).ra);
        os_log!(error, "  gp   = {:#016x}", (*ctx).gp);
        os_log!(error, "  tp   = {:#016x}", (*ctx).tp);
        os_log!(error, "  t0   = {:#016x}", (*ctx).t0);
        os_log!(error, "  t1   = {:#016x}", (*ctx).t1);
        os_log!(error, "  t2   = {:#016x}", (*ctx).t2);
        os_log!(error, "  s0   = {:#016x}", (*ctx).s0);
        os_log!(error, "  s1   = {:#016x}", (*ctx).s1);
        os_log!(error, "  a0   = {:#016x}", (*ctx).a0);
        os_log!(error, "  a1   = {:#016x}", (*ctx).a1);
        os_log!(error, "  a2   = {:#016x}", (*ctx).a2);
        os_log!(error, "  a3   = {:#016x}", (*ctx).a3);
        os_log!(error, "  a4   = {:#016x}", (*ctx).a4);
        os_log!(error, "  a5   = {:#016x}", (*ctx).a5);
        os_log!(error, "  a6   = {:#016x}", (*ctx).a6);
        os_log!(error, "  a7   = {:#016x}", (*ctx).a7);
        os_log!(error, "  s2   = {:#016x}", (*ctx).s2);
        os_log!(error, "  s3   = {:#016x}", (*ctx).s3);
        os_log!(error, "  s4   = {:#016x}", (*ctx).s4);
        os_log!(error, "  s5   = {:#016x}", (*ctx).s5);
        os_log!(error, "  s6   = {:#016x}", (*ctx).s6);
        os_log!(error, "  s7   = {:#016x}", (*ctx).s7);
        os_log!(error, "  s8   = {:#016x}", (*ctx).s8);
        os_log!(error, "  s9   = {:#016x}", (*ctx).s9);
        os_log!(error, "  s10  = {:#016x}", (*ctx).s10);
        os_log!(error, "  s11  = {:#016x}", (*ctx).s11);
        os_log!(error, "  t3   = {:#016x}", (*ctx).t3);
        os_log!(error, "  t4   = {:#016x}", (*ctx).t4);
        os_log!(error, "  t5   = {:#016x}", (*ctx).t5);
        os_log!(error, "  t6   = {:#016x}", (*ctx).t6);
        os_log!(error, "  mepc = {:#016x}", (*ctx).mepc);
    }
}
