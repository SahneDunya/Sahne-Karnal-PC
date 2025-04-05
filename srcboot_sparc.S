.global _start

_start:
    /* Pencere kayıtlarını (window registers) temizle */
    clr %o0
    wr %o0, %g0, %wim

    /* Yığını ayarla */
    /* SPARC'ta yığın genellikle %sp register'ı ile gösterilir */
    /* Yığın adresini uygun bir değere ayarlayın */
    sethi %hi(_stack_end), %sp
    or %sp, %lo(_stack_end), %sp

    /* Kontrolü Rust main fonksiyonuna aktar */
    /* Rust main fonksiyonunun adresini bir register'a yükleyin */
    sethi %hi(main), %o0
    or %o0, %lo(main), %o0

    /* main fonksiyonuna dallan */
    jmp %o0
    nop /* Gecikme slotu için boş işlem */

.section .data
.align 8
_stack_end:
    .skip 8192 /* Örnek olarak 8KB yığın alanı ayırın */