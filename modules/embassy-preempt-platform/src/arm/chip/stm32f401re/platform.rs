use core::arch::asm;
use core::ptr::NonNull;

#[cfg(feature = "log-base")]
use cortex_m::asm::delay;
use cortex_m::interrupt;
use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::register::primask;
use critical_section::Mutex;
use spin::Once;
use stm32f4xx_hal::gpio::GpioExt;
#[cfg(feature = "cortex-m")]
use stm32f4xx_hal::pac::{NVIC, SCB};
use stm32f4xx_hal::prelude::_fugit_RateExtU32;
use stm32f4xx_hal::rcc::{Config, RccExt};
use stm32f4xx_hal::syscfg::SysCfgExt;

use crate::chip::ucstk::CONTEXT_STACK_SIZE;
use crate::driver::button::driver::Button;
use crate::driver::led::driver::Led;
use crate::traits::memory_layout::PlatformMemoryLayout;
use crate::traits::Platform;
/// STM32F401RE platform implementation
///
/// This structure implements the Platform trait for the STM32F401RE microcontroller.
/// It provides hardware abstraction for:
/// - GPIO-based LED and button drivers
/// - RTC-based timer driver for scheduling
/// - ARM Cortex-M specific context switching and stack management
///
/// ## Hardware Configuration
///
/// - LED: Connected to PA5 (GPIOA pin 5)
/// - Button: Connected to PC13 (GPIOC pin 13) with EXTI interrupt support
/// - Timer: RTC peripheral for timekeeping and alarm functionality
/// - Context Switching: PendSV interrupt for task switching
pub struct PlatformImpl {
    /// Button driver with mutex protection for thread-safe access
    pub button: Mutex<Button>,

    /// LED driver with mutex protection for thread-safe access
    pub led: Mutex<Led>,

    /// RTC timer driver providing timing and alarm services
    pub timer: crate::arm::chip::stm32f401re::timer_driver::RtcDriver,
}

impl PlatformImpl {
    /// Create and initialize a new STM32F401RE platform instance
    ///
    /// This method initializes all hardware peripherals required by the RTOS:
    /// - System clocks and RCC configuration using HAL library
    /// - GPIO pins for LED and button
    /// - EXTI for button interrupts
    /// - RTC timer for scheduling
    /// - Interrupt priorities for proper RTOS operation
    ///
    /// # Clock Configuration
    ///
    /// The system is configured to run at 84MHz using HSE (8MHz) as source:
    /// - **HSE**: 8MHz external crystal
    /// - **PLL**: M=4, N=84, P=2, Q=4 → 84MHz system clock
    /// - **APB1**: 42MHz (PCLK1)
    /// - **APB2**: 84MHz (PCLK2)
    /// - **Flash**: 2 wait states for 84MHz operation
    ///
    /// # Returns
    /// A fully initialized PlatformImpl instance
    ///
    /// # Panics
    /// Will panic if hardware peripherals are already taken or initialization fails
    pub(crate) fn new() -> Self {
        use stm32f4xx_hal::rcc::RccExt;

        use crate::arm::chip::stm32f401re::timer_driver::RtcDriver;

        // Take ownership of hardware peripherals
        let dp = crate::hal::pac::Peripherals::take().unwrap();
        let cp = cortex_m::Peripherals::take().unwrap();

        // Configure system clocks using HAL library
        // This replaces the previous rcc_init() function
        let rcc = dp.RCC.constrain();
        let mut rcc = rcc.freeze(
            Config::hse(8u32.MHz()) // Use 8MHz external crystal
                .hclk(84u32.MHz()) // AHB clock: 84MHz
                .pclk1(42u32.MHz()) // APB1 clock: 42MHz
                .pclk2(84u32.MHz()) // APB2 clock: 84MHz
                .sysclk(84u32.MHz()), // System clock: 84MHz)
        );

        // Initialize system components
        let mut scb = cp.SCB;
        let mut nvic = cp.NVIC;

        // Store clock configuration for timer driver use before consuming rcc
        crate::arm::chip::stm32f401re::timer_driver::store_clock_config(&rcc.clocks);

        // Configure interrupt priorities for RTOS operation
        PlatformImpl::set_interupt_prio(&mut scb, &mut nvic);

        // Initialize GPIO and external interrupts
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);
        let gpioa = dp.GPIOA.split(&mut rcc);
        let mut exti = dp.EXTI;

        // Initialize peripheral drivers
        let button = Button::new(&mut rcc, &mut exti, &mut nvic, &mut syscfg, gpioc.pc13);
        let led = Led::new(&mut rcc, gpioa.pa5);

        // Initialize RTC timer driver
        let timer = RtcDriver::new();
        timer.init();

        PlatformImpl {
            button: Mutex::new(button),
            led: Mutex::new(led),
            timer: timer,
        }
    }

    /// Configure interrupt priorities for RTOS operation
    ///
    /// Sets up the NVIC and SCB interrupt priorities to ensure proper
    /// preemption behavior:
    /// - PendSV: Lowest priority (context switching)
    /// - EXTI15_10: High priority (button interrupts)
    /// - TIM3: Medium priority (timer interrupts)
    ///
    /// # Parameters
    /// - `scb`: System Control Block for system-wide interrupts
    /// - `nvic`: Nested Vectored Interrupt Controller for peripheral interrupts
    fn set_interupt_prio(scb: &mut SCB, nvic: &mut NVIC) {
        unsafe {
            // Set the NVIC group as 2-2 (same as port implementation)
            let aircr = scb.aircr.read();
            let mut aircr = aircr & !(0b1111 << 8);
            aircr = aircr | (0b101 << 8);
            scb.aircr.write(aircr);

            // Set TIM3 priority as 3 (same as port)
            nvic.set_priority(stm32_metapac::Interrupt::TIM3, 32);

            #[cfg(feature = "semihosting")]
            let _ = cortex_m_semihosting::hprintln!(
                "the prio of TIM3 is {}",
                cortex_m::peripheral::NVIC::get_priority(stm32_metapac::Interrupt::TIM3)
            );

            // Set EXTI15_10 priority as 1 (for button interrupt)
            nvic.set_priority(stm32_metapac::Interrupt::EXTI15_10, 16);
            #[cfg(feature = "semihosting")]
            let _ = cortex_m_semihosting::hprintln!(
                "the prio of EXTI15_10 is {}",
                cortex_m::peripheral::NVIC::get_priority(stm32_metapac::Interrupt::EXTI15_10)
            );

            // Set PendSV priority (lowest priority)
            #[cfg(feature = "semihosting")]
            let _ =
                cortex_m_semihosting::hprintln!("the prio of PendSV is {}", SCB::get_priority(SystemHandler::PendSV));
            scb.set_priority(SystemHandler::PendSV, 0xf << 4);
            #[cfg(feature = "semihosting")]
            let _ =
                cortex_m_semihosting::hprintln!("the prio of PendSV is {}", SCB::get_priority(SystemHandler::PendSV));
        }
    }
}

impl Platform for PlatformImpl {
    type OsStk = usize;

    /// Trigger a context switch via PendSV interrupt
    ///
    /// ARM Cortex-M specific implementation that sets the PendSV flag
    /// in the NVIC interrupt control register. PendSV has the lowest
    /// priority and will execute after all other pending interrupts.
    fn trigger_context_switch(&'static self) {
        os_log!(trace, "trigger_context_switch");
        const NVIC_INT_CTRL: u32 = 0xE000ED04; // NVIC Interrupt Control Register
        const NVIC_PENDSVSET: u32 = 0x10000000; // PendSV Set bit
        unsafe {
            asm!(
                "STR     R1, [R0]",  // Store PendSVSET flag to NVIC register
                in("r0") NVIC_INT_CTRL,
                in("r1") NVIC_PENDSVSET,
            )
        }
    }

    /// Set the Process Stack Pointer (PSP) for task execution
    ///
    /// ARM Cortex-M specific implementation that programs the PSP register.
    /// The PSP is used for task stack management while the MSP is used
    /// for interrupt handling.
    ///
    /// # Parameters
    /// - `sp`: Stack pointer value to set as the current PSP
    fn set_program_stack_pointer(&'static self, sp: *mut u8) {
        use cortex_m::register::psp;
        unsafe {
            psp::write(sp as u32);
        }
    }

    /// Configure interrupt stack and switch to thread mode
    ///
    /// ARM Cortex-M specific implementation that:
    /// 1. Sets the Main Stack Pointer (MSP) for interrupt handling
    /// 2. Configures the CONTROL register to use PSP in thread mode
    /// 3. Enables privileged-to-unprivileged transition
    ///
    /// # Parameters
    /// - `interrupt_stack`: Pointer to the interrupt stack (MSP)
    #[inline(never)]
    fn configure_interrupt_stack(&'static self, interrupt_stack: *mut u8) {
        unsafe {
            asm!(
                // First change the MSP to interrupt stack
               "MSR msp, r1",        // Set MSP to interrupt stack pointer
                // Then change the control register to use the PSP
                "MRS r0, control",   // Read current CONTROL register
                "ORR r0, r0, #2",    // Set bit 1 to use PSP in thread mode
                "MSR control, r0",   // Write back modified CONTROL
                "BX lr",             // Return to caller
                in("r1") interrupt_stack,
                options(nostack, preserves_flags),
            )
        }
    }

    /// Initialize task stack with ARM Cortex-M context frame
    ///
    /// Creates the initial stack frame for a new task following the ARM Cortex-M
    /// procedure call standard and exception return conventions.
    ///
    /// Stack layout (high addresses to low addresses):
    /// - Exception frame: xPSR, PC, LR, R12, R3, R2, R1, R0
    /// - Callee-saved registers: R11-R4
    ///
    /// # Parameters
    /// - `stk_ref`: Reference to stack memory allocation
    /// - `executor_function`: Function to execute when task starts
    ///
    /// # Returns
    /// Pointer to the initialized task stack top
    fn init_task_stack(&'static self, stk_ref: NonNull<Self::OsStk>, executor_function: fn()) -> NonNull<Self::OsStk> {
        scheduler_log!(trace, "init_task_stack");
        let executor_function_ptr = executor_function as *const () as usize;
        scheduler_log!(info, "the executor function ptr is 0x{:x}", executor_function_ptr);

        // Get stack pointer and align to 8-byte boundary
        let ptos = stk_ref.as_ptr() as *mut usize;
        let mut ptos = ((unsafe { ptos.offset(1) } as usize) & 0xFFFFFFF8) as *mut usize;

        // Reserve space for the context frame
        ptos = unsafe { ptos.offset(-(CONTEXT_STACK_SIZE as isize) as isize) };
        let psp = ptos as *mut crate::chip::ucstk::UcStk;

        // Initialize ARM Cortex-M context frame
        unsafe {
            // General purpose registers (with debug patterns)
            (*psp).r0 = 0; // First argument register
            (*psp).r1 = 0x01010101; // Debug pattern
            (*psp).r2 = 0x02020202; // Debug pattern
            (*psp).r3 = 0x03030303; // Debug pattern
            (*psp).r4 = 0x04040404; // Callee-saved (initial value)
            (*psp).r5 = 0x05050505; // Callee-saved (initial value)
            (*psp).r6 = 0x06060606; // Callee-saved (initial value)
            (*psp).r7 = 0x07070707; // Callee-saved (initial value)
            (*psp).r8 = 0x08080808; // Callee-saved (initial value)
            (*psp).r9 = 0x09090909; // Callee-saved (initial value)
            (*psp).r10 = 0x10101010; // Callee-saved (initial value)
            (*psp).r11 = 0x11111111; // Callee-saved (initial value)
            (*psp).r12 = 0x12121212; // Intra-procedure call (initial value)
            (*psp).r14 = 0xFFFFFFFD; // LR: Return to Thread mode, PSP
            (*psp).lr = 0; // Unused in task context

            // Exception frame (automatically loaded by hardware on exception return)
            (*psp).pc = executor_function_ptr as u32; // Program counter: task entry point
            (*psp).xpsr = 0x01000000; // xPSR: T-bit set for Thumb mode
        }

        // Return the new stack pointer pointing to the context frame
        NonNull::new(ptos as *mut Self::OsStk).unwrap()
    }

    /// Enter low-power idle state
    ///
    /// Puts the CPU into a low-power state until an interrupt occurs.
    /// The behavior depends on the logging configuration:
    /// - With logging disabled: Use WFE (Wait For Event) instruction for lowest power
    /// - With logging enabled: Use delay loop to avoid RTT interference
    fn enter_idle_state(&'static self) {
        // After WFE, probe-rs reports that the RTT read pointer has been modified.
        // Therefore, when logging is enabled, avoid WFE in idle to prevent interference.

        #[cfg(not(feature = "log-base"))]
        unsafe {
            asm!("wfe"); // Wait For Event - lowest power consumption
        }

        #[cfg(feature = "log-base")]
        delay(500); // Use delay when logging to maintain RTT connectivity
    }

    /// System shutdown handler
    ///
    /// Called when the RTOS shuts down. Behavior depends on features:
    /// - With semihosting: Exit cleanly using semihosting debug interface
    /// - Without semihosting: Enter infinite loop requiring manual reset
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
            // Without semihosting, manual intervention is required
            os_log!(info, "Shutdown, please press Ctrl+C to stop the program");
            loop {} // Infinite loop waiting for reset
        }
    }

    /// Save current task context to stack
    ///
    /// ARM Cortex-M specific context saving that stores callee-saved registers
    /// (R4-R11 and LR) to the task's Process Stack Pointer (PSP).
    /// Must be called with interrupts disabled for atomicity.
    ///
    /// # Safety
    /// - Must be called with interrupts disabled
    /// - Must have valid PSP pointing to sufficient stack space
    /// - Must only be called from interrupt context
    #[inline(always)]
    unsafe fn save_task_context(&'static self) {
        asm!(
            "CPSID I",                      // Disable interrupts for atomic context save
            "MRS     R0, PSP",              // Get current Process Stack Pointer
            "STMFD   R0!, {{R4-R11, R14}}", // Save callee-saved registers (R4-R11, LR) with full descending stack
            "MSR     PSP, R0",              // Write back updated PSP
            options(nostack, preserves_flags)
        );
    }

    /// Restore task context and resume execution
    ///
    /// ARM Cortex-M specific context restoration that:
    /// 1. Restores callee-saved registers from task stack
    /// 2. Sets the task's PSP
    /// 3. Restores interrupt stack pointer
    /// 4. Re-enables interrupts
    /// 5. Branches to exception return value to resume task
    ///
    /// # Parameters
    /// - `stack_pointer`: Task's saved stack pointer (PSP)
    /// - `interrupt_stack`: System interrupt stack pointer (MSP)
    /// - `return_value`: ARM exception return value (EXC_RETURN)
    ///
    /// # Safety
    /// - Must have valid saved context on stack
    /// - Must be called from PendSV handler
    /// - Stack pointers must be properly aligned
    #[inline(always)]
    unsafe fn restore_task_context(
        &'static self,
        stack_pointer: *mut usize,
        interrupt_stack: *mut usize,
        return_value: u32,
    ) {
        asm!(
            "LDMFD   R0!, {{R4-R11, R14}}", // Restore callee-saved registers from task stack
            "MSR     PSP, R0",              // Set task's Process Stack Pointer
            "MSR     MSP, R1",              // Restore system Main Stack Pointer
            "CPSIE   I",                    // Re-enable interrupts
            "BX      R2",                   // Branch to EXC_RETURN value to resume task
            in("r0") stack_pointer,         // R0: Task stack pointer
            in("r1") interrupt_stack,       // R1: Interrupt stack pointer
            in("r2") return_value,          // R2: EXC_RETURN value
            options(nostack, preserves_flags),
        );
    }

    /// Get current Process Stack Pointer value
    ///
    /// ARM Cortex-M specific function that reads the current PSP value.
    /// Used during context switching to save the current task's stack pointer.
    ///
    /// # Returns
    /// Current Process Stack Pointer value
    ///
    /// # Safety
    /// - Must be called in a context where PSP is meaningful (thread mode)
    #[inline(always)]
    unsafe fn get_current_stack_pointer(&'static self) -> *mut usize {
        let psp_value: *mut usize;
        asm!(
            "MRS     R0, PSP", // Read Process Stack Pointer into R0
            out("r0") psp_value,
            options(nostack, preserves_flags),
        );
        psp_value
    }

    /// Get the platform's timer driver instance
    ///
    /// Returns a reference to the RTC timer driver that provides timing
    /// and alarm services for the RTOS scheduler and time functions.
    ///
    /// # Returns
    /// Reference to the timer driver implementing the Driver trait
    fn get_timer_driver(&'static self) -> &'static dyn crate::traits::timer::Driver {
        &self.timer
    }
}

impl PlatformMemoryLayout for PlatformImpl {
    const fn get_stack_start() -> usize {
        0x2000B800
    }

    const fn get_max_programs() -> usize {
        10
    }

    const fn get_heap_size() -> usize {
        10 * 1024 // 10 KiB
    }

    const fn get_program_stack_size() -> usize {
        2048 // 2 KiB
    }

    const fn get_interrupt_stack_size() -> usize {
        2048 // 2 KiB
    }
}
