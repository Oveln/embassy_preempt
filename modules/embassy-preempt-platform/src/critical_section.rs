use critical_section::{Impl, RawRestoreState, set_impl};

use crate::{PLATFORM, Platform};

struct SingleCoreCriticalSection;
set_impl!(SingleCoreCriticalSection);

unsafe impl Impl for SingleCoreCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        PLATFORM.enter_critical_section()
    }

    unsafe fn release(was_active: RawRestoreState) { unsafe {
        // Only re-enable interrupts if they were enabled before the critical section.
        if was_active {
            PLATFORM.exit_critical_section();
        }
    }}
}