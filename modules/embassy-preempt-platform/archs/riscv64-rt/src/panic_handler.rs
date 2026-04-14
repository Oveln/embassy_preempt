//! Panic handler for RISC-V platforms

use core::panic::PanicInfo;

/// RISC-V CSR (Control and Status Registers)
mod csr {
    pub const MSTATUS: u16 = 0x300;
    pub const MTVEC: u16 = 0x305;
    pub const MEPC: u16 = 0x341;
    pub const MCAUSE: u16 = 0x342;
    pub const MTVAL: u16 = 0x343;
    pub const MIP: u16 = 0x344;
}

/// Read a CSR register (CSR number must be compile-time constant)
macro_rules! read_csr {
    ($csr_num:expr) => {{
        let value: usize;
        unsafe {
            core::arch::asm!(concat!("csrr {}, ", stringify!($csr_num)),
                out(reg) value);
        }
        value
    }};
}

/// Read CSR by number (runtime)
macro_rules! read_csr_num {
    ($csr_num:expr) => {{
        let value: usize;
        unsafe {
            core::arch::asm!(
                "csrr {0}, {1}",
                out(reg) value,
                const $csr_num
            );
        }
        value
    }};
}

/// Print all registers and CSR information
fn dump_registers(sp: usize) {
    // Read CSRs using const numbers
    let mstatus = read_csr_num!(csr::MSTATUS);
    let mepc = read_csr_num!(csr::MEPC);
    let mcause = read_csr_num!(csr::MCAUSE);
    let mtval = read_csr_num!(csr::MTVAL);
    let mtvec = read_csr_num!(csr::MTVEC);
    let mip = read_csr_num!(csr::MIP);

    os_log!(error, "=== REGISTER DUMP ===");

    // Attempt to get return address from stack if not provided
    // Note: At panic, we can't easily capture all GPRs without compiler support
    // We'll output what we can from CSRs

    os_log!(error, "CSR Registers:");
    os_log!(error, "  mstatus = {:#018x}", mstatus);
    os_log!(error, "  mepc    = {:#018x} (Exception PC)", mepc);
    os_log!(error, "  mcause  = {:#018x} (Exception Cause)", mcause);
    os_log!(error, "  mtval   = {:#018x} (Trap Value)", mtval);
    os_log!(error, "  mtvec   = {:#018x} (Trap Vector)", mtvec);
    os_log!(error, "  mip     = {:#018x} (Interrupt Pending)", mip);

    os_log!(error, "Stack Pointer:");
    os_log!(error, "  sp      = {:#018x}", sp);

    // Decode mcause
    let is_interrupt = mcause & (1 << 63) != 0;
    let exception_code = mcause & !(1 << 63);

    os_log!(error, "Exception Details:");
    if is_interrupt {
        os_log!(error, "  Type: Interrupt");
        os_log!(error, "  Code: {} (Interrupt #)", exception_code);
    } else {
        os_log!(error, "  Type: Exception");
        os_log!(error, "  Code: {}", exception_code);
        // Exception codes for RISC-V
        let exception_name = match exception_code {
            0 => "Instruction address misaligned",
            1 => "Instruction access fault",
            2 => "Illegal instruction",
            3 => "Breakpoint",
            4 => "Load address misaligned",
            5 => "Load access fault",
            6 => "Store/AMO address misaligned",
            7 => "Store/AMO access fault",
            8 => "Environment call from U-mode",
            9 => "Environment call from S-mode",
            11 => "Environment call from M-mode",
            12 => "Instruction page fault",
            13 => "Load page fault",
            15 => "Store/AMO page fault",
            _ => "Unknown",
        };
        os_log!(error, "  Name: {}", exception_name);
    }

    // Try to dump some stack words around SP
    os_log!(error, "Stack dump (sp +/- 16 words):");
    unsafe {
        let sp_ptr = sp as *const usize;
        for i in -16..16 {
            if i % 4 == 0 {
                os_log!(error, "");
            }
            let offset = i * 8; // usize is 8 bytes on 64-bit
            let addr = sp as isize + offset;
            if addr >= 0 {
                let val = *sp_ptr.offset(i);
                os_log!(error, "  [{:#04x}] {:#018x}", addr, val);
            }
        }
    }

    os_log!(error, "=====================");
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    os_log!(error, "");
    os_log!(error, "!!! PANIC !!!");
    os_log!(error, "Message: {}", info);

    // Get stack pointer
    let sp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }

    dump_registers(sp);

    os_log!(error, "HALTING!");
    loop {}
}