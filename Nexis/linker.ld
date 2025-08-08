/* Minimal linker script that defines kernel start/end symbols.
   It keeps the standard layout but exposes __kernel_start and __kernel_end
   for kernel to locate its own end in physical memory. */

OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)

SECTIONS
{
  /* keep text, rodata, data, bss as usual */
  . = ALIGN(0x1000);
  __kernel_start = .;

  .text : {
    *(.text.entry)   /* bootloader or _start goes here */
    *(.text .text.*)
  } :text

  .rodata : { *(.rodata .rodata.*) }

  .data : { *(.data .data.*) }

  .bss : {
    __bss_start = .;
    *(.bss .bss.*)
    __bss_end = .;
  } 

  /* export symbol marking kernel end */
  __kernel_end = .;

  /* keep alignment at page size */
  /DISCARD/ : { *(.eh_frame) }
}
