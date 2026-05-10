/* Minimal STM32F405 linker script · 给 v2a Pico-pivot 的 stm32-blink demo 用
 *
 * Memory layout(STM32F405xx):
 *   FLASH @ 0x08000000  size 1024K(实际 demo 远用不到这么多)
 *   RAM   @ 0x20000000  size  128K(主 SRAM,CCM 64K @ 0x10000000 暂不用)
 *
 * 段安排:
 *   .isr_vector  → FLASH 起始地址(Cortex-M4 复位拉取栈顶 + 跳 Reset_Handler)
 *   .text / .rodata → FLASH
 *   .data        → 在 FLASH 里有 LMA,启动时拷到 RAM
 *   .bss         → RAM,启动时清零
 *
 * 不接 SDK / 不依赖 newlib,纯裸金属。
 */

ENTRY(Reset_Handler)

MEMORY
{
    FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 1024K
    RAM   (rwx): ORIGIN = 0x20000000, LENGTH = 128K
}

/* 栈顶 = RAM 末尾 */
_estack = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS
{
    .isr_vector :
    {
        KEEP(*(.isr_vector))
    } > FLASH

    .text :
    {
        *(.text*)
        *(.rodata*)
        . = ALIGN(4);
        _etext = .;
    } > FLASH

    .data : AT(_etext)
    {
        _sdata = .;
        *(.data*)
        . = ALIGN(4);
        _edata = .;
    } > RAM

    .bss :
    {
        _sbss = .;
        *(.bss*)
        *(COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > RAM

    /* 调试信息按需保留 */
    /DISCARD/ :
    {
        *(.ARM.exidx*)
        *(.ARM.attributes)
    }
}
