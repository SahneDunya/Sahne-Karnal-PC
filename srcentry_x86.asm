.global _start  ; _start sembolü, bağlayıcı (linker) tarafından giriş noktası olarak kullanılır.

_start:
    ; 1. Çevre Hazırlığı:
    ;   - Koruma modunu etkinleştirme (Protected Mode)
    ;   - Sayfalama (Paging) ayarlarını yapma
    ;   - Kesme (Interrupt) ayarlarını yapma
    ;   - GDT (Global Descriptor Table) ve IDT (Interrupt Descriptor Table) yükleme

    ; Örnek: Koruma moduna geçiş (basitleştirilmiş)
    mov eax, cr0
    or eax, 0x1  ; PE bitini (Protection Enable) set et
    mov cr0, eax

    ; 2. Yığın (Stack) Kurulumu:
    ;   - Çekirdek için bir yığın alanı ayırın ve yığın işaretçisini (ESP) ayarlayın.

    ; Örnek: Yığın ayarı (basitleştirilmiş)
    mov esp, kernel_stack_top

    ; 3. C/C++ Koduna Geçiş (Ana Çekirdek Mantığı):
    ;   - Çekirdek fonksiyonunuzu (örneğin, main veya kernel_main) çağırın.

    ; Örnek: C fonksiyonuna geçiş (basitleştirilmiş)
    call kernel_main

    ; 4. (İsteğe Bağlı) Sonsuz Döngü veya Halt:
    ;   - Çekirdek sonlandığında veya bir hata oluştuğunda sonsuz bir döngüye girebilir veya işlemciyi durdurabilirsiniz.

    ; Örnek: Sonsuz döngü
    hlt
    jmp $

section .bss
    ; Yığın için alan ayırma (örnek)
    kernel_stack_top: resb 4096  ; 4KB yığın alanı