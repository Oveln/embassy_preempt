//! Procedural macros for Embassy Preempt RTOS
//!
//! This crate provides platform-agnostic procedural macros that work across
//! different target architectures, including ARM Cortex-M, RISC-V, and QingKe.

#![deny(warnings)]

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

/// Platform-specific entry point macro
///
/// This macro provides a unified entry point attribute that works across different
/// target architectures:
/// - ARM Cortex-M targets: uses `cortex_m_rt::entry`
/// - QingKe targets: uses `qingke_rt::entry`
/// - Other targets: provides fallback implementation
///
/// # Examples
///
/// ```rust
/// use embassy_preempt_macros::entry;
///
/// #[entry]
/// fn main() -> ! {
///     // Your application code here
///     loop {}
/// }
/// ```
#[proc_macro_attribute]
pub fn entry(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    // Generate platform-specific entry point
    let expanded = quote::quote! {
        #[cfg(target_arch = "arm")]
        #[cortex_m_rt::entry]
        #input

        #[cfg(all(target_arch = "riscv32",feature = "qingke"))]
        #[qingke_rt::entry]
        #input

        #[cfg(not(any(target_arch = "arm", target_arch = "riscv32")))]
        compile_error!("Unsupported target architecture for embassy_preempt_macros::entry. Supported architectures: arm, riscv32");
    };

    expanded.into()
}

/// Platform-specific delay function macro
///
/// This macro provides optimized delay implementations for different architectures:
/// - ARM: uses ARM assembly with registers r0-r3
/// - QingKe/RISC-V: uses RISC-V assembly with registers t0-t2
/// - Other: uses Rust spin loops
#[proc_macro]
pub fn delay_asm(_input: TokenStream) -> TokenStream {
    let delay_impl = if cfg!(target_arch = "arm") {
        quote::quote! {
            /// ARM-optimized delay implementation
            #[inline(always)]
            pub fn delay(time: usize) {
                unsafe {
                    core::arch::asm!(
                        // ARM assembly delay loop
                        "mov r0, #0",
                        "1:",
                        "mov r1, #0",
                        "2:",
                        "add r1, r1, #1",
                        "cmp r1, r3",
                        "blt 2b",
                        "add r0, r0, #1",
                        "cmp r0, r2",
                        "blt 1b",
                        in("r2") time,
                        in("r3") 8000000/8,
                    )
                }
            }
        }
    } else if cfg!(target_arch = "riscv32") {
        quote::quote! {
            /// QingKe/RISC-V optimized delay implementation
            #[inline(always)]
            pub fn delay(time: usize) {
                unsafe {
                    core::arch::asm!(
                        // RISC-V assembly delay loop (works for QingKe too)
                        "li t0, 0",           // t0 = 0 (外层循环计数器)
                        "1:",
                        "li t1, 0",           // t1 = 0 (内层循环计数器)
                        "2:",
                        "addi t1, t1, 1",     // t1++
                        "li t2, 1000000",     // t2 = 1000000 (内层循环次数)
                        "blt t1, t2, 2b",     // if (t1 < t2) goto 2b
                        "addi t0, t0, 1",     // t0++
                        "blt t0, {time}, 1b", // if (t0 < time) goto 1b
                        time = in(reg) time,
                    )
                }
            }
        }
    } else {
        quote::quote! {
            /// Generic delay implementation for other architectures
            #[inline(always)]
            pub fn delay(time: usize) {
                let _inner_loop = 1000000;
                for _ in 0..time {
                    for _ in 0.._inner_loop {
                        core::hint::spin_loop();
                    }
                }
            }
        }
    };

    delay_impl.into()
}

/// Architecture detection macro
///
/// This macro expands to different constants based on the target architecture,
/// allowing for compile-time architecture detection in user code.
#[proc_macro]
pub fn arch_detection(_input: TokenStream) -> TokenStream {
    let arch_str = if cfg!(target_arch = "arm") {
        quote::quote! { "arm" }
    } else if cfg!(target_arch = "riscv32") {
        // For RISC-V, we assume it could be QingKe (CH32/WCH) or generic RISC-V
        quote::quote! { "riscv32" }
    } else if cfg!(target_arch = "riscv64") {
        quote::quote! { "riscv64" }
    } else if cfg!(target_arch = "x86_64") {
        quote::quote! { "x86_64" }
    } else {
        quote::quote! { "unknown" }
    };

    let expanded = quote::quote! {
        pub const TARGET_ARCH: &str = #arch_str;
    };

    expanded.into()
}