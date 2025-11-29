use core::arch::asm;

/// block delay
#[inline]
pub fn delay(time: usize) {
    // 延时函数,time的单位约为0.5s，使用汇编编写从而不会被优化
    unsafe {
        #[cfg(target_arch = "arm")]
        {
            asm!(
                // 先来个循环（总共是两层循环，内层循环次数8000000）
                "mov r0, #0",
                "1:",
                // 内层循环
                "mov r1, #0",
                "2:",
                "add r1, r1, #1",
                "cmp r1, r3",
                "blt 2b",
                // 外层循环
                "add r0, r0, #1",
                "cmp r0, r2",
                "blt 1b",
                in("r2") time,
                in("r3") 8000000/8,
            )
        }

        #[cfg(target_arch = "riscv32")]
        {
            asm!(
                // 外层循环初始化
                "li t0, 0",           // t0 = 0 (外层循环计数器)
                "1:",
                // 内层循环初始化
                "li t1, 0",           // t1 = 0 (内层循环计数器)
                "2:",
                // 内层循环体
                "addi t1, t1, 1",     // t1++
                "li t2, 100000",     // t2 = 1000000 (内层循环次数)
                "blt t1, t2, 2b",     // if (t1 < t2) goto 2b
                // 外层循环体
                "addi t0, t0, 1",     // t0++
                "blt t0, {time}, 1b", // if (t0 < time) goto 1b
                time = in(reg) time,
            )
        }

        #[cfg(not(any(target_arch = "arm", target_arch = "riscv32")))]
        {
            // 通用Rust实现作为fallback
            let _inner_loop = 1000000;
            for _ in 0..time {
                for _ in 0.._inner_loop {
                    // 空操作循环
                    core::hint::spin_loop();
                }
            }
        }
    }
}
