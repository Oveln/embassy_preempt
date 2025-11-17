use embassy_preempt_platform::hal::{gpio::{GpioExt, Output, Pin}, pac, rcc::RccExt};

#[allow(dead_code)]
pub struct Led {
    led: Pin<'A', 5, Output>,
}
#[allow(dead_code)]
impl Led {
    pub fn new() -> Self {
        let dp = pac::Peripherals::take().unwrap();
        let mut rcc = dp.RCC.constrain();
        // enable the RCC
        rcc.ahb1enr().modify(|r,w| {
            w.gpioaen().enabled()
        });

        let gpioa = dp.GPIOA.split(&mut rcc);

        let led = gpioa.pa5
        .into_push_pull_output()
        .speed(embassy_preempt_platform::hal::gpio::Speed::High);
        Self {
            led
        }
    }
    pub fn on(&mut self) {
        self.led.set_high();
    }
    pub fn off(&mut self) {
        self.led.set_low();
    }
    pub fn toggle(&mut self) {
        self.led.toggle();
    }
}