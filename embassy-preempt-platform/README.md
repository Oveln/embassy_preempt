# Embassy Preempt Platform Abstraction Layer

这个 crate 提供了一个基于 trait 的抽象层，允许 embassy_preempt RTOS 在不同的微控制器架构上运行。

## 文件结构

```
embassy-preempt-platform/
├── src/
│   ├── lib.rs              # 主入口文件，重新导出所有公共接口
│   ├── types.rs            # 通用类型定义和数据结构
│   ├── platform.rs         # 核心平台功能 trait 定义
│   ├── timer_driver.rs     # 定时器驱动 trait 定义
│   ├── gpio_driver.rs      # GPIO 驱动 trait 定义
│   ├── time_driver.rs      # 增强的时间驱动 trait 定义
│   └── stm32f401re/        # STM32F401RE 平台实现
│       ├── mod.rs          # 模块导出
│       ├── platform.rs     # STM32F401RE 平台实现
│       ├── timer_driver.rs # STM32F401RE 定时器驱动实现
│       ├── gpio_driver.rs  # STM32F401RE GPIO 驱动实现
│       └── cfg.rs          # STM32F401RE 平台配置常量
└── README.md               # 本文档
```

## Trait 定义

### Core Platform (`platform.rs`)

- **Platform**: 核心平台功能，包括外设初始化、任务栈管理、上下文切换等

### Driver Traits

- **TimerDriver** (`timer_driver.rs`): 基础定时器驱动功能
- **GpioDriver** (`gpio_driver.rs`): GPIO/外部中断驱动功能
- **TimeDriver** (`time_driver.rs`): 增强的时间驱动，支持报警器管理

### Types (`types.rs`)

- 通用类型别名 (INT8U, INT16U, INT32U, INT64U, OS_STK 等)
- AlarmHandle: 报警器句柄
- AlarmState: 报警器状态结构
- RtcDriver: RTC 驱动实现

## 平台实现

### STM32F401RE

位于 `stm32f401re/` 目录下，提供：

1. **静态平台实例**: `PLATFORM` - 预配置的平台对象
2. **驱动实现**:
   - `Stm32f401reTimerDriver`: 定时器驱动
   - `Stm32f401reGpioDriver`: GPIO 驱动
3. **平台配置**: `cfg.rs` - 平台特定的配置常量

## 使用方式

### 用户代码

```rust
use embassy_preempt_platform::Platform;

#[cortex_m_rt::entry]
fn main() -> ! {
    // 直接使用静态平台对象
    embassy_preempt::PLATFORM.init_platform();

    // 业务逻辑...
}
```

### 平台开发者

要支持新的微控制器，需要：

1. 创建新的平台目录 (例如 `stm32h743/`)
2. 实现 `Platform`, `TimerDriver`, `GpioDriver` 等 trait
3. 提供平台配置 (`cfg.rs`)
4. 在 `lib.rs` 中添加条件编译支持

## 设计优势

1. **模块化**: 每个功能模块独立，便于维护
2. **可扩展**: 支持添加新平台和驱动
3. **类型安全**: 强类型检查
4. **零成本抽象**: trait 调用在编译时解析
5. **向后兼容**: 保持现有 API 兼容性

## 示例：添加新平台

```rust
// src/new_platform/mod.rs
pub mod platform;
pub mod timer_driver;
pub mod gpio_driver;
pub mod cfg;

pub use platform::{NewPlatformPlatform};
pub use timer_driver::{NewPlatformTimerDriver};
pub use gpio_driver::{NewPlatformGpioDriver};
pub use cfg::*;

// 静态实例
static TIME_DRIVER: NewPlatformTimerDriver = NewPlatformTimerDriver::new(...);
static GPIO_DRIVER: NewPlatformGpioDriver = NewPlatformGpioDriver::new();

pub static PLATFORM: NewPlatformPlatform = NewPlatformPlatform::new(
    &TIME_DRIVER,
    &GPIO_DRIVER,
);
```

## 版本历史

- v0.1.0: 初始版本，支持 STM32F401RE
- v0.1.1: 重构 trait 定义到独立文件
- v0.1.2: 添加静态平台实例支持