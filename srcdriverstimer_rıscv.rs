#![no_std]
use core::sync::atomic::{AtomicU64, Ordering};
use riscv::register::{mie, mip};
use crate::drivers::clint;
use crate::platform;
use crate::interrupt; // Varsayımsal interrupt modülü eklenmiş

// AtomicU64 ile veri yarışlarını önle, SeqCst sıralaması kullanılarak daha güçlü garanti sağlanır
static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn ticks() -> u64 {
    // SeqCst sıralaması ile okunarak tutarlılık artırılır
    TICKS.load(Ordering::SeqCst)
}

pub fn delay_ms(ms: u64) {
    let target_ticks = ticks() + (ms * platform::CLOCK_FREQ / 1000);
    while ticks() < target_ticks {
        // İşlemciyi meşgul etmemek için bekleme döngüsüne bir ipucu ekle
        core::hint::spin_loop_hint();
    }
}

// Zamanlayıcı kesmesi işleyicisi
#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    // Kesme bayrağını temizleme (RISC-V donanımı STIP bayrağını otomatik olarak temizler)
    // mip::clear_stip(); // Gerek yok - RISC-V donanımı otomatik temizler

    // Bir sonraki kesmeyi ayarla (daha doğru zamanlama için mevcut zamanı kullan)
    let current_mtime = clint::mtime();
    clint::set_timer(platform::hart_id(), current_mtime + platform::CLOCK_FREQ / 1000);

    // Atomik olarak artır, SeqCst sıralaması kullanılarak tutarlılık sağlanır
    TICKS.fetch_add(1, Ordering::SeqCst);
}

pub fn init() {
    // Zamanlayıcı kesmesini etkinleştir
    unsafe {
        mie::set_mtimer();
        // Global kesmeleri etkinleştir (gerekliyse)
        riscv::interrupt::enable();
    }

    // İlk kesmeyi ayarla
    clint::set_timer(platform::hart_id(), clint::mtime() + platform::CLOCK_FREQ / 1000);

    // Kesme işleyicisini ayarla
    interrupt::set_interrupt_handler(7, timer_interrupt_handler);
}