/* Minimal Cortex-M4 startup for STM32F405 (v2a stm32-blink demo).
 *
 * 职责:
 *   1. 在 FLASH 起始放向量表(初始 SP + 复位向量 + 必要异常向量)
 *   2. Reset_Handler:把 .data 从 FLASH 拷到 RAM,清 .bss,跳 main
 *
 * 不接 CMSIS,所有外设向量只占位,不做任何分发(QEMU netduinoplus2 demo 不需要)。
 */

    .syntax unified
    .cpu cortex-m4
    .thumb

    .extern _estack
    .extern _sdata
    .extern _edata
    .extern _sbss
    .extern _ebss
    .extern _etext
    .extern main

    .section .isr_vector, "a", %progbits
    .global g_pfnVectors
    .type g_pfnVectors, %object

g_pfnVectors:
    .word _estack             /* Top of stack */
    .word Reset_Handler       /* Reset */
    .word Default_Handler     /* NMI */
    .word Default_Handler     /* HardFault */
    .word Default_Handler     /* MemManage */
    .word Default_Handler     /* BusFault */
    .word Default_Handler     /* UsageFault */
    .word 0
    .word 0
    .word 0
    .word 0
    .word Default_Handler     /* SVCall */
    .word Default_Handler     /* DebugMon */
    .word 0
    .word Default_Handler     /* PendSV */
    .word Default_Handler     /* SysTick */

    /* Peripheral vectors — 全部默认处理,QEMU 的 netduinoplus2 demo 不会触发 */
    .rept 82
    .word Default_Handler
    .endr

    .size g_pfnVectors, . - g_pfnVectors

    .section .text.Reset_Handler
    .global Reset_Handler
    .type Reset_Handler, %function
Reset_Handler:
    /* Copy .data from FLASH to RAM */
    ldr r0, =_sdata
    ldr r1, =_edata
    ldr r2, =_etext
    movs r3, #0
copy_data:
    cmp r0, r1
    beq zero_bss
    ldr r4, [r2, r3]
    str r4, [r0, r3]
    adds r3, r3, #4
    adds r0, r0, #4
    b copy_data

zero_bss:
    ldr r0, =_sbss
    ldr r1, =_ebss
    movs r2, #0
zero_bss_loop:
    cmp r0, r1
    beq call_main
    str r2, [r0]
    adds r0, r0, #4
    b zero_bss_loop

call_main:
    bl main
    /* main 返回应不应该发生(blink 是 for(;;));保险起见死循环 */
hang:
    b hang
    .size Reset_Handler, . - Reset_Handler

    .section .text.Default_Handler
    .global Default_Handler
    .type Default_Handler, %function
Default_Handler:
    b Default_Handler
    .size Default_Handler, . - Default_Handler
