.global _start

_start:
    # 1. İstif işaretçisini (stack pointer) ayarlayın
    # OpenRISC'de, istif genellikle SRAM'in en üstünde başlar ve aşağıya doğru büyür.
    # Bu adresi kendi sisteminize göre ayarlamanız gerekir.
    lis r1, 0x4000  # SRAM başlangıç adresi (Örnek)
    addi r1, r1, 0x1000  # İstif için biraz alan ayırın (Örnek)
    mtsr sp, r1

    # 2. Belleği temizleyin (BSS bölümü)
    # BSS bölümü, başlatılmamış global değişkenler için ayrılan alandır.
    # Bu bölümü sıfırlamak önemlidir.
    la r1, _bss_start  # BSS bölümünün başlangıç adresi
    la r2, _bss_end    # BSS bölümünün bitiş adresi

clear_bss_loop:
    sw r0, (r1)  # r0 genellikle sıfır değerini içerir
    addi r1, r1, 4  # Sonraki kelimeye geç
    blt r1, r2, clear_bss_loop  # BSS sonuna kadar devam et

    # 3. Çekirdek koduna atla
    # Burada C kodunuzun başlangıç noktası olan main() fonksiyonuna atlayacağız.
    j kernel_main  # kernel_main, C kodunuzdaki main() fonksiyonunun sembolik adresi

    # 4. (İsteğe bağlı) Sonsuz döngü (eğer kernel_main() geri dönerse)
hang:
    j hang
