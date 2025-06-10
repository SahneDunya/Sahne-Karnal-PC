#include <asm/csr.h>

.section .text.entry
.global _start

_start:
    /* CSR'ları (Control and Status Registers) ayarla */
    /* MSTATUS (Machine Status Register) ayarla */
    csrw mstatus, 0 /* Kesmeleri devre dışı bırak */

    /* MEPC (Machine Exception Program Counter) ayarla */
    la t0, main     /* Rust main fonksiyonunun adresini yükle */
    csrw mepc, t0   /* MEPC'ye yaz */

    /* MIE (Machine Interrupt Enable) ayarla (gerekirse) */
    csrw mie, 0

    /* MDeleg (Machine Interrupt Delegation) ayarla (gerekirse) */
    csrw medeleg, 0
    csrw mideleg, 0

    /* Yığını ayarla */
    la sp, _stack_end

    /* MMode'dan UMode'a geçiş (gerekirse) */
    /* MSTATUS'daki MPV bitini ayarla ve MRET komutunu kullan */
    /* li t0, MSTATUS_MPV */
    /* csrs mstatus, t0 */
    /* mret */

    /* Rust main fonksiyonuna atla */
    j main

.size _start, . - _start
