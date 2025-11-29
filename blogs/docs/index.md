# 技术文档

本目录包含了 Embassy Preempt RTOS 的详细技术文档，涵盖架构设计、实现细节和开发指南。

## 📚 技术文档

### [PendSV跨平台实现机制详解](./context-switch-arch.md)
- 详细阐述ARM Cortex-M和RISC-V平台上PendSV（或等效机制）的实现方法
- 统一跳转到`__ContextSwitchHandler`函数的设计原理
- 平台抽象层的设计和架构优势