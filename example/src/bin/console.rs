#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]

use embassy_preempt_executor::OSStart;
use embassy_preempt_executor::OSInit;
use embassy_preempt_executor::uart_logger::uart_write_str;

/// CSR 寄存器读取辅助函数
mod csr {
    /// 读取 mhartid - 当前硬件线程 ID
    #[inline]
    pub unsafe fn mhartid() -> usize {
        let mut hartid: usize;
        unsafe {
            core::arch::asm!("csrr {}, mhartid", out(reg) hartid);
        }
        hartid
    }

    /// 读取 mtvec - 机器模式陷阱向量基地址
    #[inline]
    pub unsafe fn mtvec() -> usize {
        let mut mtvec: usize;
        unsafe {
            core::arch::asm!("csrr {}, mtvec", out(reg) mtvec);
        }
        mtvec
    }

    /// 读取 mstatus - 机器模式状态寄存器
    #[inline]
    pub unsafe fn mstatus() -> usize {
        let mut mstatus: usize;
        unsafe {
            core::arch::asm!("csrr {}, mstatus", out(reg) mstatus);
        }
        mstatus
    }

    /// 读取 mepc - 机器模式异常程序计数器
    #[inline]
    pub unsafe fn mepc() -> usize {
        let mut mepc: usize;
        unsafe {
            core::arch::asm!("csrr {}, mepc", out(reg) mepc);
        }
        mepc
    }

    /// 读取 mie - 机器模式中断使能寄存器
    #[inline]
    pub unsafe fn mie() -> usize {
        let mut mie: usize;
        unsafe {
            core::arch::asm!("csrr {}, mie", out(reg) mie);
        }
        mie
    }

    /// 读取 mip - 机器模式中断挂起寄存器
    #[inline]
    pub unsafe fn mip() -> usize {
        let mut mip: usize;
        unsafe {
            core::arch::asm!("csrr {}, mip", out(reg) mip);
        }
        mip
    }

    /// 读取 mscratch - 机器模式临时寄存器
    #[inline]
    pub unsafe fn mscratch() -> usize {
        let mut mscratch: usize;
        unsafe {
            core::arch::asm!("csrr {}, mscratch", out(reg) mscratch);
        }
        mscratch
    }

    /// 读取 sp - 栈指针
    #[inline]
    pub unsafe fn sp() -> usize {
        let mut sp: usize;
        unsafe {
            core::arch::asm!("mv {}, sp", out(reg) sp);
        }
        sp
    }

    /// 读取 ra - 返回地址
    #[inline]
    pub unsafe fn ra() -> usize {
        let mut ra: usize;
        unsafe {
            core::arch::asm!("mv {}, ra", out(reg) ra);
        }
        ra
    }
}

/// 输出十六进制数
fn uart_write_hex(val: usize) {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    unsafe {
        uart_write_str("0x");
        for i in (0..16).rev() {
            let nibble = (val >> (i * 4)) & 0xf;
            let c = HEX_CHARS[nibble as usize] as char;
            uart_write_str(c.encode_utf8(&mut [0u8; 4]));
        }
    }
}

/// 将 usize 转换为十进制字符串
fn usize_to_str(val: usize, buf: &mut [u8; 20]) -> &str {
    if val == 0 {
        buf[0] = b'0';
        return unsafe { core::str::from_utf8_unchecked(&buf[0..1]) };
    }

    let mut i = 0;
    let mut v = val;
    while v > 0 {
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }

    // 反转字符串
    let mut len = i;
    for j in 0..len / 2 {
        buf.swap(j, len - 1 - j);
    }

    unsafe { core::str::from_utf8_unchecked(&buf[0..len]) }
}

/// 输出 mtvec 模式
fn print_mtvec_mode(mtvec: usize) {
    let mode = mtvec & 0x3;  // 低两位表示模式
    let base = mtvec & !0x3; // 基地址（低两位清零）
    unsafe {
        uart_write_str("    Mode: ");
        match mode {
            0 => uart_write_str("Direct (all traps to BASE)"),
            1 => uart_write_str("Vectored (exceptions to BASE, interrupts to BASE+4*cause)"),
            _ => uart_write_str("Reserved"),
        }
        uart_write_str("\r\n    Base: ");
        uart_write_hex(base);
    }
}

/// 输出 mstatus 字段
fn print_mstatus_fields(mstatus: usize) {
    unsafe {
        uart_write_str("\r\n    MIE: ");
        uart_write_str(if (mstatus >> 3) & 1 == 1 { "1" } else { "0" });
        uart_write_str(", MPIE: ");
        uart_write_str(if (mstatus >> 7) & 1 == 1 { "1" } else { "0" });
        uart_write_str(", MPP: ");
        let mpp = (mstatus >> 11) & 0x3;
        match mpp {
            0 => uart_write_str("U"),
            1 => uart_write_str("S"),
            3 => uart_write_str("M"),
            _ => uart_write_str("?"),
        }
    }
}

/// 输出中断寄存器字段
fn print_interrupt_fields(reg: usize, reg_name: &str) {
    unsafe {
        uart_write_str("\r\n    ");
        uart_write_str(reg_name);
        uart_write_str(" bits:");
        uart_write_str("\r\n      SSIP: ");
        uart_write_str(if (reg >> 1) & 1 == 1 { "1" } else { "0" });
        uart_write_str(", MSIP: ");
        uart_write_str(if (reg >> 3) & 1 == 1 { "1" } else { "0" });
        uart_write_str("\r\n      STIP: ");
        uart_write_str(if (reg >> 5) & 1 == 1 { "1" } else { "0" });
        uart_write_str(", MTIP: ");
        uart_write_str(if (reg >> 7) & 1 == 1 { "1" } else { "0" });
        uart_write_str("\r\n      SEIP: ");
        uart_write_str(if (reg >> 9) & 1 == 1 { "1" } else { "0" });
        uart_write_str(", MEIP: ");
        uart_write_str(if (reg >> 11) & 1 == 1 { "1" } else { "0" });
    }
}

/// 清除 BSS 段
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

#[embassy_preempt_macros::entry]
fn test_hardware() -> ! {
    // 清除 BSS 段
    clear_bss();

    // Initialize UART logger in executor module for debugging OSInit
    unsafe {
        embassy_preempt_executor::uart_logger::init_uart_logger(0x10010000);

        // ========== 系统信息输出开始 ==========
        uart_write_str("\r\n");
        uart_write_str("========================================\r\n");
        uart_write_str("  Embassy Preempt - System Info\r\n");
        uart_write_str("  VisionFive2 JH7110 Platform\r\n");
        uart_write_str("========================================\r\n");

        // Hart ID
        uart_write_str("\r\n[Hart Information]\r\n");
        let hartid = csr::mhartid();
        uart_write_str("  mhartid (Hart ID): ");
        uart_write_hex(hartid);
        uart_write_str(" (");
        let mut buf = [0u8; 20];
        uart_write_str(usize_to_str(hartid, &mut buf));
        uart_write_str(")");

        // 陷阱向量
        uart_write_str("\r\n\r\n[Trap Vector]\r\n");
        let mtvec = csr::mtvec();
        uart_write_str("  mtvec: ");
        uart_write_hex(mtvec);
        print_mtvec_mode(mtvec);

        // 状态寄存器
        uart_write_str("\r\n\r\n[Machine Status]\r\n");
        let mstatus = csr::mstatus();
        uart_write_str("  mstatus: ");
        uart_write_hex(mstatus);
        print_mstatus_fields(mstatus);

        // 异常程序计数器
        uart_write_str("\r\n\r\n[Exception Program Counter]\r\n");
        let mepc = csr::mepc();
        uart_write_str("  mepc: ");
        uart_write_hex(mepc);

        // 中断使能
        uart_write_str("\r\n\r\n[Interrupt Enable]\r\n");
        let mie = csr::mie();
        uart_write_str("  mie: ");
        uart_write_hex(mie);
        print_interrupt_fields(mie, "MIE");

        // 中断挂起
        uart_write_str("\r\n\r\n[Interrupt Pending]\r\n");
        let mip = csr::mip();
        uart_write_str("  mip: ");
        uart_write_hex(mip);
        print_interrupt_fields(mip, "MIP");

        // 栈信息
        uart_write_str("\r\n\r\n[Stack Information]\r\n");
        let mscratch = csr::mscratch();
        uart_write_str("  mscratch: ");
        uart_write_hex(mscratch);
        uart_write_str("\r\n  sp (stack pointer): ");
        uart_write_hex(csr::sp());

        // 代码地址
        uart_write_str("\r\n\r\n[Code Location]\r\n");
        uart_write_str("  ra (return address): ");
        uart_write_hex(csr::ra());

        uart_write_str("\r\n\r\n========================================\r\n");
        uart_write_str("  UART Logger Initialized\r\n");
        uart_write_str("========================================\r\n");
    }

    // os初始化
    OSInit();

    unsafe {
        uart_write_str("\r\n[OS Status]\r\n");
        uart_write_str("  OSInit completed!\r\n");
        uart_write_str("\r\n========================================\r\n");
        uart_write_str("  Hello, Embassy Preempt on VisionFive2!\r\n");
        uart_write_str("========================================\r\n\r\n");
    }

    loop{}
      // 为了测试硬件以及time driver的正确性，只创建1个任务以避免抢占
    // AsyncOSTaskCreate(task1, 0 as *mut c_void, 0 as *mut usize, 10);
    // 启动os
    // OSStart();
}