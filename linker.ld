OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS {
    . = 0x80000000; /* QEMU virt RAM starts at 0x80000000 */

    .text : {
        KEEP(*(.text.entry))
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(.bss*)
        *(COMMON)
    }

    .bss.stack (NOLOAD) : ALIGN(4096) {
        KEEP(*(.bss.stack))
    }

    .bss.trap_stack (NOLOAD) : ALIGN(4096) {
        KEEP(*(.bss.trap_stack))
    }

    /* Mark the absolute end of all kernel-reserved memory */
    . = ALIGN(16);
    _end = .;
}
