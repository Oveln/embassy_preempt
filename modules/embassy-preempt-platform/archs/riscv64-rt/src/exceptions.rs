use crate::TrapFrame;

extern "C" {
    fn InstructionMisaligned(trap_frame: &mut TrapFrame);
    fn InstructionFault(trap_frame: &mut TrapFrame);
    fn IllegalInstruction(trap_frame: &mut TrapFrame);
    fn Breakpoint(trap_frame: &mut TrapFrame);
    fn LoadMisaligned(trap_frame: &mut TrapFrame);
    fn LoadFault(trap_frame: &mut TrapFrame);
    fn StoreMisaligned(trap_frame: &mut TrapFrame);
    fn StoreFault(trap_frame: &mut TrapFrame);
    fn UserEnvCall(trap_frame: &mut TrapFrame);
    fn SupervisorEnvCall(trap_frame: &mut TrapFrame);
    fn MachineEnvCall(trap_frame: &mut TrapFrame);
    fn InstructionPageFault(trap_frame: &mut TrapFrame);
    fn LoadPageFault(trap_frame: &mut TrapFrame);
    fn StorePageFault(trap_frame: &mut TrapFrame);
}

/// Array with all the exception handlers sorted according to their exception source code.
#[no_mangle]
pub static __EXCEPTIONS_EMBASSY_PREEMPT: [Option<unsafe extern "C" fn(&mut TrapFrame)>; 16] = [
    Some(InstructionMisaligned),
    Some(InstructionFault),
    Some(IllegalInstruction),
    Some(Breakpoint),
    Some(LoadMisaligned),
    Some(LoadFault),
    Some(StoreMisaligned),
    Some(StoreFault),
    Some(UserEnvCall),
    Some(SupervisorEnvCall),
    None,
    Some(MachineEnvCall),
    Some(InstructionPageFault),
    Some(LoadPageFault),
    None,
    Some(StorePageFault),
];

/// 异常和中断处理
///
/// Safty: Called from trap_handler.
#[no_mangle]
pub unsafe extern "C" fn dispatch_exception(trap_frame: &mut TrapFrame, code: usize) {
    extern "C" {
        fn ExceptionHandler(trap_frame: &TrapFrame);
    }
    match __EXCEPTIONS_EMBASSY_PREEMPT.get(code) {
        Some(Some(handler)) => handler(trap_frame),
        _ => ExceptionHandler(trap_frame),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ExceptionHandler(trap_frame: &TrapFrame) {
    panic!("Unhandled exception: mcause={:#x}, mepc={:#x}, mstatus={:#x}",
        riscv::register::mcause::read().bits(), trap_frame.mepc, trap_frame.mstatus);
}
