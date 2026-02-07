OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS {
    . = 0x80200000; /* OpenSBI usually expects the kernel at 0x80200000 */

    .text : ALIGN(4096) {
        KEEP(*(.text.entry))
        *(.text*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata*)
    }

    .data : ALIGN(4096) {
        *(.data.boot)
        *(.data*)
    }

    . = ALIGN(4096);
    _bss_start = .;

    .bss : {
        *(.bss*)
    }
    .bss.stack (NOLOAD) : ALIGN(4096) {
        KEEP(*(.bss.stack))
    }
    .bss.trap_stack (NOLOAD) : ALIGN(4096) {
        KEEP(*(.bss.trap_stack))
    }

    . = ALIGN(8);
    _bss_end = .;

    _end = .;
}
