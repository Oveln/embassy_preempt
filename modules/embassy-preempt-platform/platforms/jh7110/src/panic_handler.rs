//! Panic handler for RISC-V platforms

use core::panic::PanicInfo;

#[cfg(feature = "log-base")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    defmt::panic!()
}

#[cfg(not(feature = "log-base"))]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}