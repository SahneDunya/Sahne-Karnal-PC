.global _start

_start:
    /* 1. İstek kipini (SVC mode) ayarlayın ve FIQ/IRQ maskelemesini yapın (isteğe bağlı) */
    mrs r0, cpsr  // Mevcut CPSR değerini oku
    bic r0, r0, #0x1f // Mod bitlerini temizle
    orr r0, r0, #0x13 // SVC modunu ayarla (0x13 = 10011 ikili)
    orr r0, r0, #0xc0 // FIQ ve IRQ maskelemesini etkinleştir (isteğe bağlı)
    msr cpsr, r0  // Yeni CPSR değerini yaz

    /* 2. Yığıt (stack) adresini ayarlayın */
    ldr sp, =_stack_top // Yığıtın en üst adresini SP'ye yükle

    /* 3. Bellek ayarlarını yapın (MMU yapılandırması - çok daha karmaşık olabilir) */
    /* Bu kısım donanıma ve çekirdek tasarımına göre değişir.
       Örneğin, basit bir bellek haritalaması için aşağıdaki gibi bir kod kullanılabilir: */
    /* (Bu kod örnektir ve gerçek bir MMU yapılandırması için yeterli değildir) */
    ldr r0, =0x40000000 // Bellek başlangıç adresi
    ldr r1, =0x100000   // Bellek boyutu (1MB)
    /* ... MMU ayarlarını yap ... */

    /* 4. Çekirdek koduna dallan */
    ldr r0, =kernel_main // kernel_main fonksiyonunun adresini yükle
    mov pc, r0          // PC'ye (Program Counter) atlayarak çekirdek koduna başla

/* Yığıt alanı (stack area) */
.align 4
.space 4096 // 4KB yığıt alanı ayır
_stack_top:

/* Diğer veriler veya fonksiyonlar buraya eklenebilir */
