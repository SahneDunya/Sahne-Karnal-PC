.global _start

_start:
    /* Yığını ayarla */
    ldr sp, =_stack_end /* Yığın sonunu stack pointer'a yükle */

    /* Diğer gerekli donanım ayarlarını yap (varsa) */
    /* Örneğin, saat ayarları, периферий kontroller vb. */

    /* Rust main fonksiyonunun adresini yükle */
    ldr r0, =main

    /* Rust main fonksiyonuna atla */
    bx r0

/* Yığın alanı */
.bss
.align 4
_stack_start:
.skip 4096 /* 4KB yığın alanı ayır */
_stack_end: