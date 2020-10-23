/* memory.x - Linker script for the STM32F103C8T6 */
/*
 MEMORY
 {
   FLASH : ORIGIN = 0x08000000, LENGTH = 64K
   RAM : ORIGIN = 0x20000000, LENGTH = 20K
 }
*/

/* sizing for a STM32F407VG */
MEMORY
{
  /* NOTE K = KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1M 
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
  CCRAM : ORIGIN = 0x10000000, LENGTH = 64K
}

_stack_size = 0x1000;
_heap_start = .;
_heap_end = ORIGIN(RAM) + LENGTH(RAM) - _stack_size;
