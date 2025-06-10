.section .text.entry
.global _start

_start:
    /* CSR'ları (Control and Status Registers) ayarla */
    /* Örneğin, LoongArch'ta bazı CSR'lar farklı adlara sahip olabilir. */
    /* Ayrıca, LoongArch'a özgü bazı CSR'ları da ayarlamanız gerekebilir. */

    /* MSTATUS (Machine Status Register) benzeri bir register ayarla */
    /* LoongArch'ta bu register'ın adı farklı olabilir. */
    /* Örneğin: csrw <LoongArch_MSTATUS_adı>, 0 /* Kesmeleri devre dışı bırak */

    /* MEPC (Machine Exception Program Counter) benzeri bir register ayarla */
    la t0, main      /* Rust main fonksiyonunun adresini yükle */
    /* LoongArch'ta MEPC'ye karşılık gelen register'ın adını kullanın. */
    /* Örneğin: csrw <LoongArch_MEPC_adı>, t0 */

    /* MIE (Machine Interrupt Enable) benzeri bir register ayarla */
    /* Örneğin: csrw <LoongArch_MIE_adı>, 0 /* Kesmeleri devre dışı bırak */

    /* Yığını ayarla */
    la sp, _stack_end  /* Yığın sonunu ayarla */

    /* MMode'dan UMode'a geçiş (gerekirse) */
    /* LoongArch'ta mod geçişi farklı mekanizmalarla yapılabilir. */
    /* Gerekirse, LoongArch'a özgü mod geçişi komutlarını kullanın. */

    /* Rust main fonksiyonuna atla */
    j main

.size _start, . - _start
