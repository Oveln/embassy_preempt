//! # QEMU Virt GPIO 驱动 (Stub)
//!
//! 这是一个空实现，QEMU Virt 平台没有实际的 GPIO 硬件。
//! 所有方法都是安全的空操作或返回默认值。

/// GPIO 控制器 (Stub 实现)
pub struct GpioController;

impl GpioController {
    /// 创建新的 GPIO 控制器
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// 配置 GPIO 引脚为输出模式（空操作）
    #[inline]
    pub fn set_output(&self, _pin: u32) {
        // QEMU Virt 没有实际 GPIO，空操作
    }

    /// 配置 GPIO 引脚为输入模式（空操作）
    #[inline]
    pub fn set_input(&self, _pin: u32) {
        // QEMU Virt 没有实际 GPIO，空操作
    }

    /// 设置 GPIO 引脚输出高电平（空操作）
    #[inline]
    pub fn set_high(&self, _pin: u32) {
        // QEMU Virt 没有实际 GPIO，空操作
    }

    /// 设置 GPIO 引脚输出低电平（空操作）
    #[inline]
    pub fn set_low(&self, _pin: u32) {
        // QEMU Virt 没有实际 GPIO，空操作
    }

    /// 翻转 GPIO 引脚状态（空操作）
    #[inline]
    pub fn toggle(&self, _pin: u32) {
        // QEMU Virt 没有实际 GPIO，空操作
    }

    /// 读取 GPIO 引脚的输入值（返回 false）
    #[inline]
    pub fn read_input(&self, _pin: u32) -> bool {
        // QEMU Virt 没有实际 GPIO，返回默认值
        false
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
    GPIO_CONTROLLER = Some(GpioController);
}

/// 获取 GPIO 控制器引用
///
/// # Safety
///
/// 必须在 `init()` 之后调用
pub unsafe fn gpio_controller() -> &'static GpioController {
    GPIO_CONTROLLER.as_ref().expect("GPIO controller not initialized")
}
