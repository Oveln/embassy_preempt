//! Panic handler for RISC-V platforms

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    os_log!(error, "PANIC: {}", _info);
    loop {}
}