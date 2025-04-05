.section .text.entry
.global _start

_start:
    /* Genel Amaçlı Register'ları (GPR'ler) Sıfırla (Gerekirse) */
    /* MIPS'de bazı register'lar (örneğin, k0, k1) özel amaçlar için kullanılır. */
    /* Bu register'ları sıfırlamak isteyebilirsiniz. */
    li t0, 0
    move a0, t0  /* a0 - a3 argüman register'larıdır, ilk argümanı sıfırla */
    move a1, t0
    move a2, t0
    move a3, t0
    move t1, t0
    move t2, t0
    move t3, t0
    move t4, t0
    move t5, t0
    move t6, t0
    move t7, t0
    move t8, t0
    move t9, t0

    /* Yığını Ayarla */
    la sp, _stack_end  /* Stack pointer'ı yığın sonuna ayarla */

    /* Rust main Fonksiyonunun Adresini Yükle */
    la t0, main

    /* main Fonksiyonuna Atla */
    jr t0       /* Jump register ile main fonksiyonuna atla */
    nop         /* Gecikme slotu için boş işlem */

.size _start, . - _start