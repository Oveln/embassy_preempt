use crate::traits::timer::AlarmHandle;

pub struct Jh7110Timer {
}

impl Jh7110Timer {
    pub fn new() -> Self {
        Jh7110Timer {}
    }
}

impl crate::traits::timer::Driver for Jh7110Timer {
    fn now(&self) -> u64 {
        0
    }

    unsafe fn allocate_alarm(&self) -> Option<crate::traits::timer::AlarmHandle> {
        Some(AlarmHandle::new(1))
    }

    fn set_alarm_callback(&self, alarm: crate::traits::timer::AlarmHandle, callback: fn(*mut ()), ctx: *mut ()) {
    }

    fn set_alarm(&self, alarm: crate::traits::timer::AlarmHandle, timestamp: u64) -> bool {
        true
    }

    unsafe fn on_interrupt(&self) {
    }
}