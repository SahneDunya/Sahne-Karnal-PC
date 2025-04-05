.section .text.entry
.global _start

_start:
    /* CSR'ları ayarla (Control and Status Registers) */
    /* Elbrus'a özgü CSR'lar ve değerleri için belgelere bakın */

    /* MSTATUS (Machine Status Register) */
    csrw mstatus, 0  /* Kesmeleri devre dışı bırak (gerekirse) */
    /* Veya Elbrus'a özgü bir değer */

    /* MEPC (Machine Exception Program Counter) */
    la t0, main      /* Rust main fonksiyonunun adresi */
    csrw mepc, t0

    /* MIE (Machine Interrupt Enable) */
    csrw mie, 0      /* Kesmeleri devre dışı bırak (gerekirse) */
    /* Veya Elbrus'a özgü bir değer */

    /* MDeleg (Machine Interrupt Delegation) */
    csrw medeleg, 0 /* Gerekirse */
    csrw mideleg, 0 /* Gerekirse */

    /* Yığını ayarla */
    la sp, _stack_end /* Yığın sonu adresi */
    /* Elbrus'ta yığın başlangıç ve bitiş adresleri farklı olabilir */

    /* MMode'dan UMode'a geçiş (gerekirse) */
    /* Elbrus'ta bu geçiş farklılık gösterebilir */
    /* Belgelere bakın */

    /* Rust main fonksiyonuna atla */
    j main

.size _start, . - _start

/* Yığın alanı */
.section .bss
.align 4
_stack_start:
.space 4096 /* Örnek yığın boyutu (değiştirilebilir) */
_stack_end: