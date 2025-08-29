OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)

SECTIONS
{
  /* place kernel at 1 MiB to avoid low-memory issues with real-mode structures */
  . = 0x0010_0000;

  .text ALIGN(4096) : {
    __kernel_start = .;
    *(.multiboot)      /* if you use multiboot / bootloader sections */
    *(.text .text.*)
    *(.rodata .rodata.*)
    PROVIDE(__text_start = .);
  } 

  .rodata ALIGN(4096) : {
    *(.rodata .rodata.*)
  }

  .data ALIGN(4096) : {
    *(.data .data.*)
    PROVIDE(__data_start = .);
  }

  .bss (NOLOAD) ALIGN(4096) : {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  }

  /* symbol that marks end of kernel image */
  __kernel_end = .;

  /DISCARD/ : {
    *(.eh_frame)
  }
}