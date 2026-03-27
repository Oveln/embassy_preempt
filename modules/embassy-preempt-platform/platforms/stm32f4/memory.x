/* STM32F401RETx with 96K RAM and 512K Flash */

MEMORY
{
  /* NOTE: You may need to adjust these values based on your specific STM32F401 variant */
  RAM : ORIGIN = 0x20000000, LENGTH = 96K
  FLASH : ORIGIN = 0x08000000, LENGTH = 512K
}

/* This is where we store the stack start address */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);