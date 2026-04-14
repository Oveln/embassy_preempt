
/// 上下文占用的字节数
/// TrapFrame 的大小：30 个通用寄存器 + mepc + mstatus = 256 字节
pub const CONTEXT_STACK_SIZE: usize = core::mem::size_of::<TrapFrame>();

/// 中断栈上的 TrapFrame 结构
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct TrapFrame {
    pub ra: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub mepc: usize,
    pub mstatus: usize,
}

impl TrapFrame {
    pub unsafe fn init(&mut self, entry: fn()) {
            self.ra = 0x0000_0721_0721_0721;
            self.gp = 0x0000_0721_0721_0721;
            self.tp = 0x0000_0721_0721_0721;
            self.t0 = 0x0000_0721_0721_0721;
            self.t1 = 0x0000_0721_0721_0721;
            self.t2 = 0x0000_0721_0721_0721;
            self.s0 = 0x0000_0721_0721_0721;
            self.s1 = 0x0000_0721_0721_0721;
            self.a0 = 0x0000_0721_0721_0721;
            self.a1 = 0x0000_0721_0721_0721;
            self.a2 = 0x0000_0721_0721_0721;
            self.a3 = 0x0000_0721_0721_0721;
            self.a4 = 0x0000_0721_0721_0721;
            self.a5 = 0x0000_0721_0721_0721;
            self.a6 = 0x0000_0721_0721_0721;
            self.a7 = 0x0000_0721_0721_0721;
            self.s2 = 0x0000_0721_0721_0721;
            self.s3 = 0x0000_0721_0721_0721;
            self.s4 = 0x0000_0721_0721_0721;
            self.s5 = 0x0000_0721_0721_0721;
            self.s6 = 0x0000_0721_0721_0721;
            self.s7 = 0x0000_0721_0721_0721;
            self.s8 = 0x0000_0721_0721_0721;
            self.s9 = 0x0000_0721_0721_0721;
            self.s10 = 0x0000_0721_0721_0721;
            self.s11 = 0x0000_0721_0721_0721;
            self.t3 = 0x0000_0721_0721_0721;
            self.t4 = 0x0000_0721_0721_0721;
            self.t5 = 0x0000_0721_0721_0721;
            self.t6 = 0x0000_0721_0721_0721;

            self.mepc = entry as *const () as usize;
            self.mstatus = 0x200001880; // MPP=Machine mode, MIE=1
        
    }
}