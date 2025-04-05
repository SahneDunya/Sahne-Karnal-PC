#![no_std]
use core::arch::asm;

// CLINT adresleri (platforma özgü olmalı ve yapılandırılabilir olmalı)
// Sabit adresler yerine yapılandırılabilir yapılar kullanmak daha iyi bir yaklaşımdır.
mod platform_config {
    pub const CLINT_MTIMECMP0: usize = 0x02004000;
    pub const CLINT_MTIME: usize = 0x0200BFF8;
    pub const CLOCK_FREQ: u64 = 10_000_000; // Örnek değer, platformunuza göre değiştirin
}

/// Zamanlayıcıyı ayarlamak için fonksiyon.
///
/// # Arguments
///
/// * `hartid`: Hedef çekirdeğin ID'si (genellikle 0'dan başlar).
/// * `delay_ms`: Gecikme süresi milisaniye cinsinden.
///
/// # Safety
///
/// Bu fonksiyon `unsafe` bloklar içerir çünkü doğrudan donanım kayıtlarına erişir.
/// Yanlış `hartid` veya `delay_ms` değerleri sistemde beklenmedik davranışlara yol açabilir.
/// Bu fonksiyonun doğru platform yapılandırması (`platform_config` modülü) ile kullanıldığından emin olun.
pub fn set_timer(hartid: usize, delay_ms: u64) {
    // CLINT adreslerini platform yapılandırmasından al
    let mtimecmp = platform_config::CLINT_MTIMECMP0 + hartid * 8; // Her çekirdek için farklı mtimecmp adresi
    let mtime = platform_config::CLINT_MTIME;
    let clock_frequency = platform_config::CLOCK_FREQ;

    // Gecikme süresinin sıfır olup olmadığını kontrol et (isteğe bağlı, ama mantıklı olabilir)
    if delay_ms == 0 {
        return; // Sıfır gecikme, hemen geri dön
    }

    unsafe {
        // Şu anki mtime değerini atomik olarak oku
        let current_mtime: u64;
        asm!("lr.d {}, ({})", out(reg) current_mtime, in(reg) mtime, options(nostack));

        // Hedef zamanı hesapla
        // `u64::saturating_mul` kullanılarak çarpma taşmasını önlenir.
        let delay_cycles = delay_ms.saturating_mul(clock_frequency) / 1000;
        let target_time = current_mtime.saturating_add(delay_cycles);


        // Hedef zamanı mtimecmp kaydına atomik olarak yaz
        asm!("sc.d {}, {}, ({})", in(reg) target_time, out(reg) _, in(reg) mtimecmp, options(nostack));
    }
}


// Örnek kullanım (test veya ana fonksiyon içinde)
fn main() {
    // Hart ID'yi ve gecikme süresini ayarlayın
    let hartid = 0; // Örnek olarak çekirdek 0
    let delay_ms = 100; // 100 milisaniye gecikme

    set_timer(hartid, delay_ms);

    // ... diğer işlemler ...
}