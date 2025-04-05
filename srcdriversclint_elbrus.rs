#![no_std]
use core::arch::asm;

// ELBURS platformu için CLINT adresleri ve yapılandırması
// Platforma özgü adresler ve saat frekansı burada tanımlanmalıdır.
mod platform_config {
    // *** DİKKAT: Bunlar örnek adreslerdir ve ELBURS platformunuza göre AYARLANMALIDIR! ***
    pub const CLINT_MTIMECMP0: usize = 0xABCD4000; // Örnek başlangıç adresi
    pub const CLINT_MTIME: usize = 0xABCDFFF8;   // Örnek MTIME adresi
    pub const CLOCK_FREQ: u64 = 25_000_000; // Örnek saat frekansı (25 MHz), platformunuza göre değiştirin
}

/// ELBURS platformunda zamanlayıcıyı ayarlamak için fonksiyon.
///
/// # Arguments
///
/// * `hartid`: Hedef çekirdeğin ID'si.
/// * `delay_ms`: Gecikme süresi milisaniye cinsinden.
///
/// # Safety
///
/// `unsafe` bloklar içerir, donanım kayıtlarına doğrudan erişim yapar.
/// Yanlış `hartid` veya `delay_ms` değerleri riskli olabilir.
/// `platform_config` modülünün ELBURS platformuna uygun yapılandırıldığından emin olun.
pub fn set_timer(hartid: usize, delay_ms: u64) {
    // Platform yapılandırmasından CLINT adreslerini ve saat frekansını al
    let mtimecmp = platform_config::CLINT_MTIMECMP0 + hartid * 8; // Hart ID'ye göre mtimecmp adresi
    let mtime = platform_config::CLINT_MTIME;
    let clock_frequency = platform_config::CLOCK_FREQ;

    // İsteğe bağlı: Sıfır gecikme kontrolü
    if delay_ms == 0 {
        return; // Sıfır gecikme, hemen çık
    }

    unsafe {
        // Şu anki mtime değerini atomik olarak oku
        let current_mtime: u64;
        asm!("lr.d {}, ({})", out(reg) current_mtime, in(reg) mtime, options(nostack));

        // Hedef zamanı hesapla
        // Çarpma taşmasını önlemek için `saturating_mul` kullanılıyor.
        let delay_cycles = delay_ms.saturating_mul(clock_frequency) / 1000;
        let target_time = current_mtime.saturating_add(delay_cycles);

        // Hedef zamanı mtimecmp kaydına atomik olarak yaz
        asm!("sc.d {}, {}, ({})", in(reg) target_time, out(reg) _, in(reg) mtimecmp, options(nostack));
    }
}

// Örnek kullanım (test veya ana fonksiyon içinde)
fn main() {
    // Hart ID ve gecikme süresini ayarla
    let hartid = 0; // Örnek olarak çekirdek 0
    let delay_ms = 50; // 50 milisaniye gecikme

    set_timer(hartid, delay_ms);

    // ... diğer işlemler ...
}