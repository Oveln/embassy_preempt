use critical_section::{Impl, RawRestoreState, set_impl};

use crate::{Platform};

struct SingleCoreCriticalSection;
set_impl!(SingleCoreCriticalSection);

unsafe impl Impl for SingleCoreCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        crate::get_platform_trait().enter_critical_section()
    }

    unsafe fn release(was_active: RawRestoreState) { unsafe {
        // Only re-enable interrupts if they were enabled before the critical section.
        if was_active {
            crate::get_platform_trait().exit_critical_section();
        }
    }}
}