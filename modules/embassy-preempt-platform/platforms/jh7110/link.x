/*
 * JH7110 平台 Linker Script
 *
 * 所有内容放入 RAM (0x0818_0000, 512KB)
 */

/* ============================================================================
 * 符号提供与别名 (PROVIDE 和 REGION_ALIAS)
 * ============================================================================ */

/* 默认 abort 入口点 */
EXTERN(_default_abort);
PROVIDE(abort = _default_abort);

/* 初始化期间的 trap 处理 */
PROVIDE(_pre_init_trap = _default_abort);

/* 多核处理钩子 (单核平台默认为 abort) */
PROVIDE(_default_mp_hook = abort);
PROVIDE(_mp_hook = _default_mp_hook);

/* 默认 trap 入口点 */
EXTERN(_default_start_trap);
PROVIDE(_start_trap = _default_start_trap);

/* 默认中断设置 */
EXTERN(_default_setup_interrupts);
PROVIDE(_setup_interrupts = _default_setup_interrupts);

/* 主函数入口 */
PROVIDE(hal_main = main);

/* 默认异常/中断处理器 */
PROVIDE(ExceptionHandler = abort);
PROVIDE(DefaultHandler = abort);
PROVIDE(_start_DefaultHandler_trap = _start_trap);

/* 异常处理器别名 */
PROVIDE(InstructionMisaligned = ExceptionHandler);
PROVIDE(InstructionFault = ExceptionHandler);
PROVIDE(IllegalInstruction = ExceptionHandler);
PROVIDE(Breakpoint = ExceptionHandler);
PROVIDE(LoadMisaligned = ExceptionHandler);
PROVIDE(LoadFault = ExceptionHandler);
PROVIDE(StoreMisaligned = ExceptionHandler);
PROVIDE(StoreFault = ExceptionHandler);
PROVIDE(UserEnvCall = ExceptionHandler);
PROVIDE(SupervisorEnvCall = ExceptionHandler);
PROVIDE(MachineEnvCall = ExceptionHandler);
PROVIDE(InstructionPageFault = ExceptionHandler);
PROVIDE(LoadPageFault = ExceptionHandler);
PROVIDE(StorePageFault = ExceptionHandler);

/* 中断处理器别名 */
PROVIDE(SupervisorSoft = DefaultHandler);
PROVIDE(MachineSoft = DefaultHandler);
PROVIDE(SupervisorTimer = DefaultHandler);
PROVIDE(MachineTimer = DefaultHandler);
PROVIDE(SupervisorExternal = DefaultHandler);
PROVIDE(MachineExternal = DefaultHandler);


/* ============================================================================
 * 全局符号定义
 * ============================================================================ */
PROVIDE(_stext = ORIGIN(RAM));
PROVIDE(_stack_start = ORIGIN(RAM) + LENGTH(RAM));
PROVIDE(_max_hart_id = 0);
PROVIDE(_hart_stack_size = 2K);
PROVIDE(_heap_size = 128K);

/* ============================================================================
 * SECTIONS 定义
 * ============================================================================ */
SECTIONS
{
    .text _stext :
    {
        __stext = .;

        /* 初始化代码放在最前面，作为程序入口点 */
        KEEP(*(.init));

        . = ALIGN(4);
        KEEP(*(.trap.vector));   /* 向量化模式 */
        KEEP(*(.trap.start));    /* trap 处理入口 */
        KEEP(*(.trap.start.*));  /* 中断 trap 入口 */
        KEEP(*(.trap.continue)); /* trap 继续点 */
        KEEP(*(.trap.rust));     /* Rust trap 函数 */
        KEEP(*(.trap .trap.*));  /* 其他 trap 符号 */

        *(.text.abort);
        *(.text .text.*);
        *(.text.switch .text.switch.*);
        *(.text.scheduler .text.scheduler.*);
        *(.text.hot .text.hot.*);

        . = ALIGN(4);
        __etext = .;
    } > RAM

    .rodata : ALIGN(4)
    {
        . = ALIGN(4);
        __srodata = .;

        *(.srodata .srodata.*);
        *(.rodata .rodata.*);

        . = ALIGN(8);
        __erodata = .;
    } > RAM

    .data : ALIGN(8)
    {
        . = ALIGN(8);
        __sdata = .;

        /* 全局指针，用于 linker relaxation 优化 */
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.* .sdata2 .sdata2.*);
        *(.data .data.*);
        *(.data.scheduler .data.scheduler.*);
        *(.data.hot .data.hot.*);

    } > RAM

    . = ALIGN(8);
    __edata = .;
    __sidata = LOADADDR(.data);

    .bss (NOLOAD) : ALIGN(8)
    {
        . = ALIGN(8);
        __sbss = .;

        *(.sbss .sbss.* .bss .bss.*);
        *(.stack.tasks .stack.tasks.*);
    } > RAM

    . = ALIGN(8);
    __ebss = .;

    /* 未初始化数据段 - 不会被运行时清零 */
    .uninit (NOLOAD) : ALIGN(8)
    {
        . = ALIGN(8);
        __suninit = .;
        *(.uninit .uninit.*);
        . = ALIGN(8);
        __euninit = .;
    } > RAM

    /* 堆 */
    .heap (NOLOAD) : ALIGN(8)
    {
        __sheap = .;
        . += _heap_size;
        . = ALIGN(8);
        __eheap = .;
    } > RAM

    /* 栈 */
    .stack (NOLOAD) :
    {
        __estack = .;
        . = ABSOLUTE(_stack_start);
        __sstack = .;
    } > RAM

    /* 用于检测动态重定位 (不支持) */
    .got (INFO) :
    {
        KEEP(*(.got .got.*));
    }
}

/* ============================================================================
 * 对齐和边界检查断言
 * ============================================================================ */

ASSERT(_stext % 4 == 0, "
错误: _stext 必须是 4 字节对齐的");

ASSERT(__sdata % 8 == 0 && __edata % 8 == 0, "
错误: .data 段必须是 8 字节对齐的");

ASSERT(__sidata % 8 == 0, "
错误: .data 段的 LMA 必须是 8 字节对齐的");

ASSERT(__sbss % 8 == 0 && __ebss % 8 == 0, "
错误: .bss 段必须是 8 字节对齐的");

ASSERT(__sheap % 8 == 0, "
错误: .heap 段的起始地址必须是 8 字节对齐的");

/* 栈空间检查 */
ASSERT(SIZEOF(.stack) >= (_max_hart_id + 1) * _hart_stack_size, "
错误: .stack 段太小，无法为所有 hart 分配栈空间。
考虑修改 _max_hart_id 或 _hart_stack_size");

/* 动态重定位检测 */
ASSERT(SIZEOF(.got) == 0, "
错误: 检测到 .got 段。不支持动态重定位。
如果链接到 C 代码，请在编译时禁用 -fPIC 标志。");
