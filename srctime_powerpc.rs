#![no_std]

use core::arch::asm;

// Global değişken zamanlayıcı tik sayısını tutmak için
static mut TIMER_TICKS: u32 = 0;

// Zamanlayıcı kesme işleyicisi (interrupt handler)
#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    unsafe {
        // Zamanlayıcı kesme bayrağını temizle
        asm!("mtspr 272, {}", in(reg) 0); // SPR 272: Decrementer Interrupt Status Register (DISR)

        // Zamanlayıcı tik sayacını artır
        TIMER_TICKS = TIMER_TICKS.wrapping_add(1);

        // Yeni bir zamanlayıcı kesmesi ayarla (örneğin, Decrementer'ı yeniden yükle)
        asm!("mtspr 271, {}", in(reg) 1000000); // SPR 271: Decrementer Register (DEC)
    }
}

// Zamanlayıcıyı başlatma fonksiyonu
pub fn init() {
    unsafe {
        // Kesme işleyicisini ayarla (platforma özgü kesme vektörü tablosu veya benzeri bir mekanizma kullanılarak).
        // Bu kısım platforma özeldir ve gerçek bir sistemde uygun şekilde ayarlanmalıdır.
        // Örnek olarak, bir vektör tablosuna fonksiyon işaretçisi yazılabilir.
        // ...

        // Decrementer'ı başlat (ilk kesme süresini ayarla)
        asm!("mtspr 271, {}", in(reg) 1000000); // SPR 271: Decrementer Register (DEC)

        // Decrementer kesmesini etkinleştir (platforma özgü kesme kontrol mekanizması kullanılarak).
        // Bu kısım da platforma özeldir ve gerçek bir sistemde uygun şekilde ayarlanmalıdır.
        // Örnek olarak, bir kontrol register'ı ayarlanabilir.
        // ...
    }
}

// Gecikme fonksiyonu (timer tabanlı busy-waiting)
pub fn delay(ms: u32) {
    unsafe {
        // Hedef tik sayısını hesapla.
        // Bu örnekte, her 1ms için yaklaşık olarak 100 tik olduğunu varsayıyoruz.
        // Bu değer, sisteminizin zamanlayıcı frekansına ve Decrementer ayarına bağlı olarak değişecektir.
        let hedef_tik = TIMER_TICKS.wrapping_add(ms * 100); // Örnek: 1ms = 100 tik varsayımı

        // Şu anki tik sayısını al
        let baslangic_tik = TIMER_TICKS;

        // Hedef tik sayısına ulaşana kadar bekle (busy-waiting).
        // **İyileştirme burada: Artık CPU döngüleri yerine zamanlayıcı tiklerini bekliyoruz.**
        while TIMER_TICKS.wrapping_sub(baslangic_tik) < ms * 100 {
            asm!("nop"); // Hala busy-waiting ama timer tiklerine göre
        }
    }
}

fn main() {
    init(); // Zamanlayıcıyı başlat

    // Örnek kullanım:
    loop {
        // ... işlerinizi yapın ...

        delay(1000); // 1 saniye (1000ms) bekle
        // ... diğer işlerinizi yapın ...

        delay(500);  // 0.5 saniye (500ms) bekle
    }
}