.section .text.entry
.global _start

_start:
    /* Yığını ayarla */
    /* OpenRISC'te yığın genellikle bir register ile belirtilir (örneğin, r1) */
    /* Yığın başlangıcını bir sabite veya etikete göre ayarlayın */
    l.addi r1, r0, STACK_START  /* STACK_START, yığın başlangıç adresini temsil eder */

    /* Global Pointer'ı ayarla (gerekirse) */
    /* Bazı OpenRISC uygulamaları global pointer (gp) kullanır */
    l.addi gp, r0, GLOBAL_POINTER /* GLOBAL_POINTER, global verilerin adresini gösterir */

    /* Kesmeleri devre dışı bırak (gerekirse) */
    /* OpenRISC'te kesmeleri devre dışı bırakmak için SR (Status Register) kullanılır */
    /* SR'deki ilgili bitleri temizleyerek kesmeler devre dışı bırakılabilir */
    mfsr r2, sr        /* SR'yi r2'ye oku */
    andi r2, r2, ~(SR_INTERRUPT_ENABLE) /* Kesme bitini temizle */
    mtsr sr, r2        /* Güncellenmiş SR'yi geri yaz */

    /* Belleği başlat (gerekirse) */
    /* BSS bölümünü temizleme veya diğer bellek ayarlarını yapma gerekebilir */
    /* Bu kısım, uygulamanızın gereksinimlerine göre değişir */

    /* main fonksiyonunun adresini yükle ve atla */
    l.addi r3, r0, main /* main fonksiyonunun adresini r3'e yükle */
    j r3                /* main fonksiyonuna atla */

    /* main fonksiyonu (Rust tarafından derlenen kod) */
    .global main
main:
    /* Rust kodunuz burada başlayacak */
    /* ... */

    /* Program sonlandırma (gerekirse) */
    /* OpenRISC'te programı sonlandırmak için genellikle bir sistem çağrısı veya özel bir komut kullanılır */
    /* Örnek: */
    /* syscall exit */

.section .data
/* Başlatılmamış veriler (BSS) için yer ayır */
.bss _stack_start, STACK_SIZE /* STACK_SIZE, yığın boyutunu belirtir */

/* Diğer veriler buraya eklenebilir */

.end