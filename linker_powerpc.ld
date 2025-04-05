OUTPUT_ARCH(powerpc)
ENTRY(_start)

MEMORY
{
  ram (wxa!r) : ORIGIN = 0x80000000, LENGTH = 128M /* RAM başlangıç adresi ve boyutu */
}

SECTIONS
{
  /* .text bölümü (kod) */
  .text : {
    KEEP(*(.text.entry)) /* Başlangıç noktası (_start) */
    *(.text .text.*)    /* Diğer kod bölümleri */
  } >ram

  /* .rodata bölümü (salt okunur veriler) */
  .rodata : {
    *(.rodata .rodata.*)
  } >ram

  /* .data bölümü (başlatılmış veriler) */
  .data : {
    *(.data .data.*)
  } >ram

  /* .bss bölümü (başlatılmamış veriler) */
  .bss : {
    *(.bss .bss.*)
    *(COMMON)
  } >ram

  /* .stack bölümü (yığın) */
  .stack : {
    . = ALIGN(16);
    *(.stack)
    _stack_start = .; /* Yığın başlangıç adresi */
    . = . + 16K;      /* Yığın boyutu */
    _stack_end = .;   /* Yığın bitiş adresi */
  } >ram

  /* .heap bölümü (dinamik bellek) */
  .heap : {
    . = ALIGN(16);
    _heap_start = .; /* Yığın başlangıç adresi */
    . = . + 16K;      /* Yığın boyutu */
    _heap_end = .;   /* Yığın bitiş adresi */
  } >ram

  /DISCARD/ : { *(.comment) } /* Yorum bölümlerini at */

  /* Sembol tanımları (isteğe bağlı) */
  _start_address = ADDR(.text.entry); /* Başlangıç adresini bir sembole ata */
}