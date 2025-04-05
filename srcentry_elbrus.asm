.global _start

_start:
    ; 1. Korumalı Moda Geçiş
    ; (Elbrus'a özgü korumalı mod geçiş mekanizması burada uygulanmalı)
    ; Örneğin:
    ; mov cr0, 0x1 ; PE biti ayarla (korumalı modu etkinleştir)
    ; jmp 0x08:protected_mode_entry ; Kod segmenti seçicisi ile korumalı moda atla

protected_mode_entry:
    ; 2. Segmentlerin ve Kesme Tanımlama Tablosu (IDT) Kurulumu
    ; (Elbrus mimarisine uygun segment tanımlamaları ve IDT kurulumu)
    ; Örnek (basitleştirilmiş):
    ; lgdt [gdt_descriptor] ; Global Descriptor Table (GDT) yükle
    ; lidt [idt_descriptor] ; Interrupt Descriptor Table (IDT) yükle

    ; 3. Yığın (Stack) Kurulumu
    ; (Yığın için yeterli alan ayırın ve yığın işaretçisini ayarlayın)
    ; Örnek:
    ; mov esp, stack_top

    ; 4. C Koduna Geçiş
    ; (Yüksek seviyeli dil koduna geçiş için gerekli ayarlamaları yapın)
    ; Örnek:
    ; call kernel_main ; C kodunda kernel_main fonksiyonuna atla

    ; ... (GDT, IDT, yığın tanımları, vs.) ...

    ; Halt (Sistemi durdur)
    hlt

; ... (Diğer tanımlamalar) ...

gdt_descriptor:
    dw gdt_size - 1 ; GDT boyutu
    dd gdt_base ; GDT adresi

idt_descriptor:
    dw idt_size - 1 ; IDT boyutu
    dd idt_base ; IDT adresi

; ... (GDT, IDT ve yığın için gerekli veri yapıları) ...

gdt_base: ; GDT başlangıç adresi
    ; ... (GDT girişleri) ...

idt_base: ; IDT başlangıç adresi
    ; ... (IDT girişleri) ...

stack_top: ; Yığın üst sınırı
    .space STACK_SIZE ; Yığın için ayrılan alan

gdt_size equ $ - gdt_base ; GDT boyutu
idt_size equ $ - idt_base ; IDT boyutu

STACK_SIZE equ 4096 ; Örnek yığın boyutu (4KB)