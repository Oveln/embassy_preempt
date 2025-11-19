
#[repr(C, align(4))]
pub(crate) struct UcStk {
    // below are the remaining part of the task's context
    pub(crate) r4: u32,
    pub(crate) r5: u32,
    pub(crate) r6: u32,
    pub(crate) r7: u32,
    pub(crate) r8: u32,
    pub(crate) r9: u32,
    pub(crate) r10: u32,
    pub(crate) r11: u32,
    pub(crate) r14: u32,
    // below are stored when the interrupt occurs
    pub(crate) r0: u32,
    pub(crate) r1: u32,
    pub(crate) r2: u32,
    pub(crate) r3: u32,
    pub(crate) r12: u32,
    pub(crate) lr: u32,
    pub(crate) pc: u32,
    pub(crate) xpsr: u32,
}
pub(crate) const CONTEXT_STACK_SIZE: usize = 17;