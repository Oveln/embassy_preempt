
#[cfg(all(feature = "cortex-m", not(feature = "semihosting")))]
use panic_probe as _;

#[cfg(any(not(feature = "cortex-m"), feature = "semihosting"))]
use panic_halt as _;


// ==================================================================
// SEMIHOSTING
// ==================================================================
#[cfg(feature = "semihosting")]
use cortex_m_semihosting::debug;
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
#[cfg(feature = "semihosting")]
pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cfg(feature = "semihosting")]
#[cortex_m_rt::exception]
unsafe fn HardFault(_frame: &cortex_m_rt::ExceptionFrame) -> ! {
    loop {
        debug::exit(debug::EXIT_FAILURE);
    }
}