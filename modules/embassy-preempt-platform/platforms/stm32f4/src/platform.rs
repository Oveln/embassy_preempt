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

use crate::ucstk::CONTEXT_STACK_SIZE;
use crate::driver::button::driver::Button;
use crate::driver::led::driver::Led;
use embassy_preempt_traits::memory_layout::PlatformMemoryLayout;
use embassy_preempt_traits::platform::PlatformStatic;
use embassy_preempt_traits::Platform;

/// STM32F401RE platform implementation
///
/// This structure implements the PlatformStatic trait for the STM32F401RE microcontroller.
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
    pub timer: crate::timer_driver::RtcDriver,
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

        // Take ownership of hardware peripherals
        let dp = stm32f4xx_hal::pac::Peripherals::take().unwrap();
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
        crate::timer_driver::store_clock_config(&rcc.clocks);

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
        let timer = crate::timer_driver::RtcDriver::new();
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

impl PlatformStatic for PlatformImpl {
    /// Trigger a context switch via PendSV interrupt
    ///
    /// ARM Cortex-M specific implementation that sets the PendSV flag
    /// in the NVIC interrupt control register. PendSV has the lowest
    /// priority and will execute after all other pending interrupts.
    fn trigger_context_switch() {
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

    /// Enter low-power idle state
    ///
    /// Puts the CPU into a low-power state until an interrupt occurs.
    /// The behavior depends on the logging configuration:
    /// - With logging disabled: Use WFE (Wait For Event) instruction for lowest power
    /// - With logging enabled: Use delay loop to avoid RTT interference
    fn enter_idle_state() {
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
    fn shutdown() {
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

    /// Get current Process Stack Pointer value
    ///
    }

impl PlatformMemoryLayout for PlatformImpl {
    const MAX_PROGRAMS: usize = 20;
    const PROGRAM_STACK_SIZE: usize = 2048; // 2 KiB
    const INTERRUPT_STACK_SIZE: usize = 2048; // 2 KiB
}

impl Platform for PlatformImpl {
    fn get_timer_driver(&'static self) -> &'static dyn embassy_preempt_traits::timer::Driver {
        &self.timer
    }

    fn set_ipi_callback(&'static self, _callback: fn(*mut ()), _ctx: *mut ()) {
        // STM32F4 does not support IPI (single core)
    }
}
