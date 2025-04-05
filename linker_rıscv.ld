OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
  ram (wxa!r) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
  .text : {
    KEEP(*(.text.entry))
    *(.text .text.*)
  } >ram

  .rodata : {
    *(.rodata .rodata.*)
  } >ram

  .data : {
    *(.data .data.*)
  } >ram

  .bss : {
    *(.bss .bss.*)
    *(COMMON)
  } >ram

  .stack : {
    . = ALIGN(16);
    *(.stack)
    _stack_start = .;
    . = . + 16K;
    _stack_end = .;
  } >ram

  .heap : {
    . = ALIGN(16);
    _heap_start = .;
    . = . + 16K;
    _heap_end = .;
  } >ram

  /DISCARD/ : { *(.comment) }

  _start_address = ADDR(.text.entry);
}