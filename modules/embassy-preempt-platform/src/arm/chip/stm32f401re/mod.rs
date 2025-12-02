mod ucstk;
mod platform;
pub mod timer_driver;

pub use platform::{PlatformImpl};
pub use ucstk::UcStk;

#[no_mangle]
unsafe extern "C" fn PendSV() {
    core::arch::asm!("b __ContextSwitchHandler");
    // os_log!(info, "PendSV");
}

// #[cortex_m_rt::exception]
// fn PendSV() {
//     os_log!(info, "PendSV");
// }

// core::arch::global_asm!(
//     ".global PendSV",
//     "PendSV:",
//     "b __ContextSwitchHandler",
// );