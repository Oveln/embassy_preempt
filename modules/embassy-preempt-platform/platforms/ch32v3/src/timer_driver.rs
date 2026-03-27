use embassy_preempt_traits::timer::{AlarmHandle, Driver};

pub struct Ch32v307Timer {
}

impl Driver for Ch32v307Timer {
    fn now(&self) -> u64 {
        0
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        Some(AlarmHandle::new(1))
    }

    fn set_alarm_callback(&self, _alarm: AlarmHandle, _callback: fn(*mut ()), _ctx: *mut ()) {
    }

    fn set_alarm(&self, _alarm: AlarmHandle, _timestamp: u64) -> bool {
        true
    }

    unsafe fn on_interrupt(&self) {
    }
}
