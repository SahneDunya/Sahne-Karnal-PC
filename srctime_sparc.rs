#![no_std]

use crate::interrupt;
use crate::drivers::timer; // SPARC için zamanlayıcı sürücüsü (timer driver)
use crate::platform;

static mut TICKS: u64 = 0;

// Sabit zaman periyodu (örneğin 10ms)
const TIMER_PERIOD_MS: u64 = 10;

#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    unsafe {
        TICKS += 1;
        timer::clear_interrupt(); // Kesme bayrağını temizle

        // Bir sonraki kesme zamanını hesapla.
        // Timer sayacının frekansına bağlı olarak, bu değeri ayarlamanız gerekebilir.
        // Burada, basitlik adına, önceki compare değerine TIMER_PERIOD_MS * birim ekliyoruz.
        // Gerçek dünyada, birim değeri sisteminizin saat frekansına ve timer sürücüsüne bağlı olacaktır.

        // Örnek bir iyileştirme: Bir "birim" hesaplama fonksiyonu kullanarak periyodu daha açık hale getirin.
        let timer_frequency = platform::timer_frequency(); // Varsayalım ki platform katmanı timer frekansını sağlıyor.
        let ticks_per_ms = timer_frequency / 1000; // Milisaniye başına düşen tick sayısı
        let next_compare_value = timer::get_counter() + (ticks_per_ms * TIMER_PERIOD_MS);
        timer::set_compare(next_compare_value);
    }
}

pub fn init() {
    interrupt::set_interrupt_handler(platform::timer_interrupt_number(), timer_interrupt_handler);

    // Başlangıç kesmesini ayarla.
    unsafe {
        let timer_frequency = platform::timer_frequency(); // Varsayalım ki platform katmanı timer frekansını sağlıyor.
        let ticks_per_ms = timer_frequency / 1000; // Milisaniye başına düşen tick sayısı
        let initial_compare_value = timer::get_counter() + (ticks_per_ms * TIMER_PERIOD_MS);
        timer::set_compare(initial_compare_value);
    }
}

pub fn ticks() -> u64 {
    unsafe { TICKS }
}

pub fn delay(ms: u64) {
    let target_ticks = ticks() + ms;
    while ticks() < target_ticks {}
}


// ÖNEMLİ NOTLAR:
// 1. platform::timer_frequency(): Bu fonksiyonun platform katmanında tanımlandığını ve sisteminizin
//    timer frekansını (örneğin Hz cinsinden) döndürdüğünü varsayıyoruz. Bu fonksiyonu platformunuza
//    uygun şekilde uygulamanız gerekecektir.
// 2. ticks_per_ms hesabı: Bu hesaplama, timer frekansının 1 kHz veya daha yüksek olduğunu varsayar.
//    Eğer daha düşük bir frekansınız varsa, ticks_per_ms 0'a yuvarlanabilir. Bu durumda, kesme periyodunuz
//    beklenenden farklı olacaktır. Gerekirse, daha hassas bir hesaplama veya farklı bir yaklaşım kullanmanız
//    gerekebilir.
// 3. Sabit TIMER_PERIOD_MS: Zaman periyodunu bir sabit olarak tanımlamak, kodu daha okunabilir ve
//    kolayca değiştirilebilir hale getirir. İhtiyacınıza göre bu değeri değiştirebilirsiniz.
// 4. Yorumlar: Kodun anlaşılırlığını artırmak için daha fazla yorum ekledim. Özellikle kritik kısımları ve
//    varsayımları vurguladım.