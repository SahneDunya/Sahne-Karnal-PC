.section .text
.global _start

_start:
    # Multiboot başlığını tanımla (Gerekirse)
    # Eğer bir bootloader kullanıyorsanız (örn. GRUB), multiboot başlığı gerekebilir.
    # .magic 0xE8525B1A
    # .align 4
    # .flags 0x00010003
    # .checksum -(0xE8525B1A + 0x00010003)

    # Yığını ayarla
    movl $0x200000, %esp  # Yığın için bir adres belirle (Örneğin, 2MB)

    # C/C++ kodunun (main fonksiyonu) çağrılması
    call main

    # Program sonlandırma (exit)
    movl %eax, %ebx  # Çıkış kodunu ebx'e taşı
    movl $1, %eax    # sys_exit sistem çağrısı
    int $0x80       # Sistem çağrısını yap

    # (Alternatif sonlandırma yöntemi)
    # hlt            # Halt the processor (durdur)

.section .data
    # Veri bölümü (Gerekirse)

.section .bss
    # Tanımlanmamış veri bölümü (Gerekirse)

.global main  # main fonksiyonunun global olduğunu belirt
