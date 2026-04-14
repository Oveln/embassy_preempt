//! BSS section utilities
//!
//! Provides functions for clearing the BSS (Block Started by Symbol) section,
//! which contains zero-initialized data.

/// Clear the BSS section (zero-initialized data)
#[cfg(feature = "clear_bss")]
pub fn clear_bss() {
    extern "C" {
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

// Assembly entry point
core::arch::global_asm!(
    ".section .init",
    ".global __start",
    ".align 4",
    "__start:",
    "la sp, __sstack",
    #[cfg(feature = "clear_bss")]
    "call __clear_bss",
    "j __rust_main"
);

// Export the clear_bss function for assembly to call when feature is enabled
#[cfg(feature = "clear_bss")]
#[no_mangle]
extern "C" fn __clear_bss() {
    clear_bss();
}
