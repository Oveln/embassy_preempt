//! # JH7110 GPIO 驱动
//!
//! ## 寄存器布局 (基于 StarFive JH7110 技术参考手册)
//!
//! - 基地址: 0x13040000 (sys_gpio)
//! - DOEN (Output Enable): 0=使能输出, 1=禁用输出
//! - DOUT (Data Output): 控制0/1输出电平
//! - DIN (Data Input): 读取输入电平
//! - 每4个GPIO一组，每组4字节，每GPIO占8位
//!
//! ## 寄存器计算
//!
//! - 偏移计算: `offset = (gpio >> 2) << 2`
//! - 位移计算: `shift = (gpio & 0x3) << 3`
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use embassy_preempt_platform_jh7110::gpio;
//!
//! // 初始化 GPIO 控制器
//! unsafe {
//!     gpio::init();
//! }
//!
//! // 配置 GPIO45 为输出模式
//! let gpio = unsafe { gpio::gpio_controller() };
//! gpio.set_output(45);
//!
//! // 设置高电平
//! gpio.set_high(45);
//!
//! // 设置低电平
//! gpio.set_low(45);
//!
//! // 翻转状态
//! gpio.toggle(45);
//! ```

use core::ptr::{read_volatile, write_volatile};

// GPIO 寄存器基地址 (sys_gpio: 0x13040000)
const SYS_GPIO_BASE: usize = 0x13040000;

// 寄存器偏移 (基于 StarFive U-Boot)
const GPIO_DOEN: usize = 0x00; // Output Enable 寄存器基地址
const GPIO_DOUT: usize = 0x40; // Data Output 寄存器基地址
const GPIO_DIN: usize = 0x80;  // Data Input 寄存器基地址

// 掩码定义
const GPIO_DOEN_MASK: u32 = 0x3f;
const GPIO_DOUT_MASK: u32 = 0x7f;

// 输出电平常量
const GPOUT_LOW: u8 = 0;
const GPOUT_HIGH: u8 = 1;

// 输出使能常量
const GPOEN_ENABLE: u8 = 0;  // 0 = 使能输出
const GPOEN_DISABLE: u8 = 1; // 1 = 禁用输出

/// GPIO 控制器
pub struct GpioController {
    base: usize,
}

impl GpioController {
    /// 创建新的 GPIO 控制器
    ///
    /// # Safety
    ///
    /// 调用者必须确保 GPIO 寄存器地址可访问
    pub const unsafe fn new() -> Self {
        Self {
            base: SYS_GPIO_BASE,
        }
    }

    /// 读取 GPIO 寄存器
    #[inline]
    unsafe fn read_reg(&self, offset: usize) -> u32 {
        read_volatile((self.base + offset) as *const u32)
    }

    /// 写入 GPIO 寄存器
    #[inline]
    unsafe fn write_reg(&self, offset: usize, value: u32) {
        write_volatile((self.base + offset) as *mut u32, value);
    }

    /// 修改寄存器位: 清除 clr_mask 并设置 set_mask
    #[inline]
    unsafe fn clrsetbits(&self, offset: usize, clr_mask: u32, set_mask: u32) {
        let current = self.read_reg(offset);
        self.write_reg(offset, (current & !clr_mask) | set_mask);
    }

    /// 计算 GPIO 引脚的寄存器偏移
    ///
    /// U-Boot 宏: `#define gpio_offset(gpio) (((gpio) >> 2) << 2)`
    #[inline]
    const fn gpio_offset(gpio: u32) -> usize {
        ((gpio >> 2) << 2) as usize
    }

    /// 计算 GPIO 引脚的位移
    ///
    /// U-Boot 宏: `#define gpio_shift(gpio) ((gpio) & 0x3) << 3`
    #[inline]
    const fn gpio_shift(gpio: u32) -> u32 {
        (gpio & 0x3) << 3
    }

    /// 设置 DOEN 寄存器
    ///
    /// 基于 U-Boot 的 `sys_iomux_doen`
    #[inline]
    unsafe fn set_doen(&self, gpio: u32, oen: u32) {
        let offset = Self::gpio_offset(gpio);
        let shift = Self::gpio_shift(gpio);
        self.clrsetbits(
            GPIO_DOEN + offset,
            GPIO_DOEN_MASK << shift,
            oen << shift,
        );
    }

    /// 设置 DOUT 寄存器
    ///
    /// 基于 U-Boot 的 `sys_iomux_dout`
    #[inline]
    unsafe fn set_dout(&self, gpio: u32, gpo: u32) {
        let offset = Self::gpio_offset(gpio);
        let shift = Self::gpio_shift(gpio);
        self.clrsetbits(
            GPIO_DOUT + offset,
            GPIO_DOUT_MASK << shift,
            (gpo & GPIO_DOUT_MASK) << shift,
        );
    }

    /// 读取 DIN 寄存器
    ///
    /// 基于 U-Boot 的 `sys_iomux_din_read`
    #[inline]
    unsafe fn read_din(&self, gpio: u32) -> bool {
        let offset = GPIO_DIN + (((gpio >> 5) * 4) as usize);
        let value = self.read_reg(offset);
        ((value >> (gpio & 0x1F)) & 0x1) != 0
    }

    /// 配置 GPIO 引脚为输出模式（初始低电平）
    pub fn set_output(&self, pin: u32) {
        unsafe {
            // 设置为输出模式 (oen = 0)
            self.set_doen(pin, GPOEN_ENABLE as u32);
            // 初始输出低电平
            self.set_dout(pin, GPOUT_LOW as u32);
        }
    }

    /// 配置 GPIO 引脚为输入模式
    pub fn set_input(&self, pin: u32) {
        unsafe {
            // 设置为输入模式 (oen = 1)
            self.set_doen(pin, GPOEN_DISABLE as u32);
        }
    }

    /// 设置 GPIO 引脚输出高电平
    pub fn set_high(&self, pin: u32) {
        unsafe {
            self.set_dout(pin, GPOUT_HIGH as u32);
        }
    }

    /// 设置 GPIO 引脚输出低电平
    pub fn set_low(&self, pin: u32) {
        unsafe {
            self.set_dout(pin, GPOUT_LOW as u32);
        }
    }

    /// 翻转 GPIO 引脚状态
    pub fn toggle(&self, pin: u32) {
        unsafe {
            let offset = Self::gpio_offset(pin);
            let shift = Self::gpio_shift(pin);
            let dout_offset = GPIO_DOUT + offset;

            // 读取当前值并翻转
            let reg_val = self.read_reg(dout_offset);
            let current = (reg_val >> shift) & 0x7f;

            let new_value = if current == 0 { 1 } else { 0 };
            self.clrsetbits(
                dout_offset,
                GPIO_DOUT_MASK << shift,
                new_value << shift,
            );
        }
    }

    /// 读取 GPIO 引脚的输入值
    pub fn read_input(&self, pin: u32) -> bool {
        unsafe { self.read_din(pin) }
    }
}

// 全局 GPIO 控制器存储
static mut GPIO_CONTROLLER: Option<GpioController> = None;

/// 初始化 GPIO 控制器
///
/// # Safety
///
/// 必须在平台初始化时调用一次，且不能重复调用
pub unsafe fn init() {
    GPIO_CONTROLLER = Some(GpioController::new());
}

/// 获取 GPIO 控制器引用
///
/// # Safety
///
/// 必须在 `init()` 之后调用
pub unsafe fn gpio_controller() -> &'static GpioController {
    GPIO_CONTROLLER.as_ref().expect("GPIO controller not initialized")
}
