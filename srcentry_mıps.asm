.text          # Kod bölümünü belirtir
.global _start # _start sembolünü global olarak tanımlar (giriş noktası)

_start:
    # 1. İstifleyiciyi (Stack Pointer) ayarla
    la $sp, KERNEL_STACK_TOP  # KERNEL_STACK_TOP, yığın için ayrılan adresin en üst noktasıdır.

    # 2. Çekirdek kodunun geri kalanını çağır (C veya başka bir dilde yazılmış olabilir)
    jal kernel_main # kernel_main, çekirdek fonksiyonunuzun adıdır.

    # 3. (Opsiyonel) Eğer kernel_main geri dönerse, sonsuz döngüye girilebilir veya sistem kapatılabilir.
    halt # Veya başka bir sonlandırma mekanizması

KERNEL_STACK_TOP: # Yığın için yeterli alan ayırın
    .space 4096 # Örneğin, 4KB yığın alanı ayır