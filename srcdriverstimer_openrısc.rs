#![no_std]

use core::sync::atomic::{AtomicU64, Ordering};
use crate::drivers::or1200_timer; // Varsayımsal OpenRISC timer modülü
use crate::platform;
use crate::interrupt; // Varsayımsal interrupt modülü

// AtomicU64 ile veri yarışlarını önle, SeqCst sıralaması ile güçlü garanti
static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn ticks() -> u64 {
    // SeqCst sıralaması ile okunarak tutarlılık artırılır
    TICKS.load(Ordering::SeqCst)
}

pub fn delay_ms(ms: u64) {
    let target_ticks = ticks() + (ms * platform::CLOCK_FREQ / 1000);
    while ticks() < target_ticks {
        // İşlemciyi meşgul etmemek için bekleme döngüsüne ipucu
        core::hint::spin_loop_hint();
    }
}

// Zamanlayıcı kesmesi işleyici fonksiyonu
#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    // Kesme bayrağını temizle (OpenRISC için bayrak temizleme adımları platforma özel olabilir)
    or1200_timer::clear_interrupt(); // Varsayımsal fonksiyon, gerçek platforma göre değişir

    // Bir sonraki kesmeyi ayarla (mevcut zamanı kullanarak daha doğru zamanlama)
    let current_timer_value = or1200_timer::current_value(); // Varsayımsal fonksiyon
    or1200_timer::set_compare(current_timer_value + platform::CLOCK_FREQ / 1000); // Varsayımsal fonksiyon

    // Atomik olarak artır, SeqCst sıralaması ile tutarlılık
    TICKS.fetch_add(1, Ordering::SeqCst);
}

pub fn init() {
    // Zamanlayıcı kesmesini etkinleştir
    or1200_timer::enable_interrupt(); // Varsayımsal fonksiyon

    // Global kesmeleri etkinleştir (gerekliyse, platforma bağlı)
    interrupt::enable_global_interrupts(); // Varsayımsal fonksiyon

    // İlk kesmeyi ayarla
    or1200_timer::set_compare(or1200_timer::current_value() + platform::CLOCK_FREQ / 1000); // Varsayımsal fonksiyon

    // Kesme işleyicisini ayarla, OpenRISC için kesme numarası farklı olabilir
    interrupt::set_interrupt_handler(platform::TIMER_INTERRUPT_NUMBER, timer_interrupt_handler); // Varsayımsal numara
}