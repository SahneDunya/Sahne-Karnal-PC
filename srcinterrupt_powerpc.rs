#![no_std]
use core::arch::asm;
use core::ptr;

// Kesme vektör tablosu (IVT) için adres ve boyut tanımları
const IVT_BASE: usize = 0x1000; // Örnek adres, gerçek donanıma göre ayarlanmalı
const IVT_SIZE: usize = 256; // Kesme vektör tablosu boyutu (genellikle 256 veya 512 bayt)

// Kesme numaraları (IRQ numaraları)
const TIMER_IRQ: usize = 0; // Örnek zamanlayıcı kesme numarası
const USB_IRQ: usize = 1; // Örnek USB kesme numarası

// Kesme işleyicisi fonksiyon prototipleri
extern "C" {
    fn timer_interrupt_handler();
    fn usb_interrupt_handler();
}

pub fn init() {
    unsafe {
        // Kesme vektör tablosunu (IVT) başlat
        let ivt_ptr = IVT_BASE as *mut usize;
        for i in 0..IVT_SIZE / core::mem::size_of::<usize>() {
            ptr::write_volatile(ivt_ptr.add(i), 0); // Başlangıçta tüm kesme vektörlerini sıfırla
        }

        // Kesme işleyicilerinin adreslerini IVT'ye yaz
        let timer_handler_addr = timer_interrupt_handler as usize;
        ptr::write_volatile(ivt_ptr.add(TIMER_IRQ), timer_handler_addr);

        let usb_handler_addr = usb_interrupt_handler as usize;
        ptr::write_volatile(ivt_ptr.add(USB_IRQ), usb_handler_addr);

        // CPSR (Current Program Status Register) ayarlarını yap (kesmeleri etkinleştir)
        // Bu kısım işlemciye özgüdür ve datasheet'e bakılarak doğru ayarlar yapılmalıdır.
        // Örnek: MSR (Machine State Register) veya benzeri bir register kullanılabilir.
        asm!(
            "mtspr {0}, {1}", // Örnek: MSR'ı ayarla
            in(reg) 1, // MSR register numarası (örnek)
            in(reg) 0x8000 // Kesmeleri etkinleştirme biti (örnek)
        );
    }
}

// Zamanlayıcı kesme işleyicisi
#[no_mangle]
pub extern "C" fn timer_interrupt_handler() {
    // Zamanlayıcı ile ilgili işlemleri gerçekleştir
    // ...

    // Kesme bayrağını temizle (donanıma özgü)
    // ...
}

// USB kesme işleyicisi
#[no_mangle]
pub extern "C" fn usb_interrupt_handler() {
    // USB ile ilgili işlemleri gerçekleştir
    // ...

    // Kesme bayrağını temizle (donanıma özgü)
    // ...
}