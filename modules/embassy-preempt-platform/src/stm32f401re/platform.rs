use core::arch::asm;
use core::ptr::NonNull;
#[cfg(feature = "log-base")]
use cortex_m::asm::delay;
use cortex_m::interrupt;
use cortex_m::peripheral::{Peripherals, SCB};
use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::register::primask;

use crate::stm32f401re::ucstk::CONTEXT_STACK_SIZE;
use crate::traits::Platform;

pub struct STM32F401RE {}

pub static PLATFORM: STM32F401RE = STM32F401RE {};

impl Platform for STM32F401RE {
    type OsStk = usize;

    fn init_core_peripherals(&'static self) {
        let mut p = Peripherals::take().unwrap();

        unsafe {
            // Set the NVIC group as 2-2 (same as port implementation)
            let aircr = p.SCB.aircr.read();
            let mut aircr = aircr & !(0b1111 << 8);
            aircr = aircr | (0b101 << 8);
            p.SCB.aircr.write(aircr);

            // Set TIM3 priority as 3 (same as port)
            p.NVIC.set_priority(stm32_metapac::Interrupt::TIM3, 32);

            #[cfg(feature = "cortex-m")]
            let _ = cortex_m_semihosting::hprintln!(
                "the prio of TIM3 is {}",
                cortex_m::peripheral::NVIC::get_priority(stm32_metapac::Interrupt::TIM3)
            );

            // Set EXTI15_10 priority as 1 (for button interrupt)
            p.NVIC.set_priority(stm32_metapac::Interrupt::EXTI15_10, 16);
            #[cfg(feature = "cortex-m")]
            let _ = cortex_m_semihosting::hprintln!(
                "the prio of EXTI15_10 is {}",
                cortex_m::peripheral::NVIC::get_priority(stm32_metapac::Interrupt::EXTI15_10)
            );

            // Set PendSV priority (lowest priority)
            #[cfg(feature = "cortex-m")]
            let _ = cortex_m_semihosting::hprintln!("the prio of PendSV is {}", SCB::get_priority(SystemHandler::PendSV));
            p.SCB.set_priority(SystemHandler::PendSV, 0xf << 4);
            #[cfg(feature = "cortex-m")]
            let _ = cortex_m_semihosting::hprintln!("the prio of PendSV is {}", SCB::get_priority(SystemHandler::PendSV));
        }
    }

    fn restore_thread_task(&'static self) {
        os_log!(trace, "restore_thread_task");
        const NVIC_INT_CTRL: u32 = 0xE000ED04;
        const NVIC_PENDSVSET: u32 = 0x10000000;
        unsafe {
            asm!(
                "STR     R1, [R0]",
                in("r0") NVIC_INT_CTRL,
                in("r1") NVIC_PENDSVSET,
            )
        }
    }

    fn set_program_sp(&'static self, sp: *mut u8) {
        use cortex_m::register::psp;
        unsafe {
            psp::write(sp as u32);
        }
    }

    #[inline(never)]
    fn set_int_change_2_psp(&'static self, int_ptr: *mut u8) {
        unsafe {
            asm!(
                // fisrt change the MSP
               "MSR msp, r1",
                // then change the control register to use the psp
                "MRS r0, control",
                "ORR r0, r0, #2",
                "MSR control, r0",
                "BX lr",
                in("r1") int_ptr,
                options(nostack, preserves_flags),
            )
        }
    }

    fn init_task_stack(&'static self, stk_ref: NonNull<Self::OsStk>, executor_function: fn()) -> NonNull<Self::OsStk> {
        scheduler_log!(trace, "init_task_stack");
        let executor_function_ptr = executor_function as *const () as usize;
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function_ptr);
        let ptos = stk_ref.as_ptr() as *mut usize;
        // do align with 8 and move the stack pointer down an align size
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp = ptos as *mut crate::stm32f401re::ucstk::UcStk;
        // initialize the stack
        unsafe {
            (*psp).r0 = 0;
            (*psp).r1 = 0x01010101;
            (*psp).r2 = 0x02020202;
            (*psp).r3 = 0x03030303;
            (*psp).r4 = 0x04040404;
            (*psp).r5 = 0x05050505;
            (*psp).r6 = 0x06060606;
            (*psp).r7 = 0x07070707;
            (*psp).r8 = 0x08080808;
            (*psp).r9 = 0x09090909;
            (*psp).r10 = 0x10101010;
            (*psp).r11 = 0x11111111;
            (*psp).r12 = 0x12121212;
            (*psp).r14 = 0xFFFFFFFD;
            (*psp).lr = 0;
            (*psp).pc = executor_function_ptr as u32;
            (*psp).xpsr = 0x01000000;
        }
        // return the new stack pointer
        NonNull::new(ptos as *mut Self::OsStk).unwrap()
    }

    fn run_idle(&'static self) {
        os_log!(trace, "run_idle");
        // undate the counter of the system
        // OSIdleCtr.fetch_add(1, Ordering::Relaxed);

        // After WFE, probe-rs reports that the RTT read pointer has been modified.
        // Therefore, when logging is enabled, avoid WFE in idle to prevent interference.
        #[cfg(not(feature = "log-base"))]
        unsafe {
            asm!("wfe");
        }
        #[cfg(feature = "log-base")]
        delay(500);
    }

    fn enter_critical_section(&'static self) -> bool {
        let was_active = primask::read().is_active();
        interrupt::disable();
        was_active
    }

    unsafe fn exit_critical_section(&'static self) {
        unsafe {
            interrupt::enable();
        }
    }

    fn shutdown(&'static self) {
        #[cfg(feature = "semihosting")]
        {
            // Use semihosting to exit cleanly for defmt-test
            use cortex_m_semihosting::debug;
            loop {
                debug::exit(debug::EXIT_SUCCESS);
            }
        }

        #[cfg(not(feature = "semihosting"))]
        {
            // If not have feature "semihosting", when test end need Ctrl+C to stop the program
            os_log!("Shutdown, please press Ctrl+C to stop the program");
            loop {}
        }
    }
}