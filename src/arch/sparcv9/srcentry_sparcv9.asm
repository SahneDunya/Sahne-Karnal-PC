.global _start  ! _start sembolünü global olarak tanımla (linker için giriş noktası)

_start:
        ! 1. İlk olarak, bazı temel ayarları yap:
        !    - Yığın (stack) ayarla
        !    - Gerekli register'ları sıfırla/ayarla

        sethi %hi(STACK_TOP), %g1  ! Yığın adresinin yüksek kısmını g1'e yükle
        or %g1, %lo(STACK_TOP), %g1  ! Yığın adresinin düşük kısmını g1'e yükle
        wr %g1, %sp, %sp        ! Yığını stack pointer'a (sp) yaz

        ! Diğer register'ları sıfırla (isteğe bağlı, ama iyi bir uygulama)
        clr %g2
        clr %g3
        clr %g4
        clr %g5
        clr %g6
        clr %g7

        ! 2. Çekirdek kodunun (C kodu veya diğer) başlangıç fonksiyonunu çağır:

        call kernel_main       ! kernel_main fonksiyonunu çağır
        nop                 ! Gecikme slotu (branch delay slot) için nop

        ! 3. (İsteğe bağlı) Eğer kernel_main geri dönerse (ki genelde dönmez), 
        !    sonsuz bir döngüye gir veya sistemi kapat:

        halt                ! Sistemi durdur (veya sonsuz döngü: loop: b loop)

STACK_TOP = 0x400000       ! Yığın için bir adres belirle (uygun bir adres seçin)

.section .text
kernel_main:
        ! Çekirdek kodunun (C veya assembly) başlangıç fonksiyonu
        ! ... çekirdek kodunuz buraya gelecek ...

        ret                 ! Fonksiyondan geri dön (genelde dönülmez, sonsuz döngüye girilir)
        nop                 ! Gecikme slotu için nop
