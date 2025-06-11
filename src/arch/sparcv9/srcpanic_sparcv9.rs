#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

use core::panic::PanicInfo;
// Karnal64'ün çekirdek içi, panic-safe hizmetlerine erişim için
// bazı modülleri konsept olarak import edelim. Gerçek implementasyonları
// henüz olmasa da, bu yapıyı göstermeye yardımcı olur.
 use crate::karnal64::{kkernel, KError}; // Örnek: Çekirdek içi hizmetler

/// SPARC mimarisi için özelleştirilmiş çekirdek panic işleyicisi.
///
/// Rust çalışma zamanı bir panic durumunda bu fonksiyonu çağırır.
/// Sistem kurtarılamaz bir durumdadır, bu nedenle yapılması gerekenler:
/// 1. Kesmeleri kapatarak sistemi dondurmak.
/// 2. Mümkünse panic bilgisini bir hata ayıklama konsoluna yazdırmak.
/// 3. Sistemi durdurmak veya sonsuz döngüye girmek.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // --- 1. Kesmeleri Kapat ---
    // Sistemin daha fazla bozulmasını veya panic handler'ın kesilmesini önlemek için
    // tüm kesmeler kapatılır. Bu, mimariye özel bir işlemdir.
    // SPARC'ta bu, İşlemci Durum Yazmacı'nı (PSR) manipüle etmeyi içerebilir.
    // Burası, SPARC'a özel assembly kodu veya düşük seviye yazmaç erişimi gerektirir.
    unsafe {
        // ÖRNEK YER TUTUCU: SPARC kesme kapatma işlemi
        // Gerçek SPARC mimarisine (v8, v9 vb.) ve donanıma göre değişir.
        // Örneğin, bazı SPARC varyantlarında PSR'deki ET (Enable Traps) bitini temizlemek gerekebilir.
         asm!("rd %psr, %l0"); // PSR yazmacını oku (örnek syntax)
         asm!("andn %l0, ..., %l0"); // Kesme bitlerini temizle (örnek)
         asm!("wr %l0, %psr"); // Güncellenmiş PSR'yi yaz (örnek)
        // Bellek senkronizasyonu (membar) gerekebilir.
    }

    // --- 2. Panic Bilgisini Yazdır ---
    // Mümkün olan en temel ve güvenilir yöntemle panic mesajını bir hata ayıklama
    // konsoluna veya seri porta yazdır. Bu, Karnal64'ün normal kaynak sistemini
    // (ResourceProvider, Handle vb.) atlayarak yapılmalıdır, çünkü panik anında
    // bu sistemler bozulmuş olabilir.
    // Karnal64 konsepti içinde, bu, çekirdeğin sağladığı çok temel bir çıktı
    // fonksiyonu olabilir.

    // Konsept: Çekirdek içi, panic-safe konsol çıktı fonksiyonu kullanımı
    // Örneğin, kkernel modülü böyle bir fonksiyon sağlayabilir:
     kkernel::panic_output_str("KERNEL PANIC (SPARC):\n");
     kkernel::panic_output_str(&alloc::fmt::format(format_args!("{}", info))); // 'alloc' veya başka formatlama gerekir

    // Daha gerçekçi ve düşük seviyeli bir yaklaşım: Doğrudan seri porta yazma simülasyonu.
    // Bir SPARC sisteminde, genellikle belirli bir bellek adresindeki UART/seri
    // port veri yazmacına byte yazarak çıktı alınır.
    let panic_message = core::fmt::format(format_args!("KERNEL PANIC (SPARC) --- {}\n", info));

    unsafe {
        // ÖRNEK YER TUTUCU: SPARC seri portuna düşük seviye yazma
        // Gerçek seri port adresi ve yazma mekanizması donanıma bağlıdır!
        let serial_port_data_register: *mut u8 = 0xFFFF_F000 as *mut u8; // BU ADRES SPEKÜLATİFTİR!
        // Ayrıca status yazmaçlarını kontrol etmek (TX buffer boş mu?) gerekebilir.
        let serial_port_status_register: *const u8 = 0xFFFF_F004 as *const u8; // BU ADRES SPEKÜLATİFTİR!
        const TX_EMPTY_BIT: u8 = 0x20; // Örnek: Status yazmacında TX boş bit maskesi

        for byte in panic_message.as_bytes() {
            // TX buffer boşalana kadar bekle (çok basit bekleme döngüsü, gerçekte timeout olmalı)
             while (core::ptr::read_volatile(serial_port_status_register) & TX_EMPTY_BIT) == 0 {
            //     // Bekle...
             }
            // Veri byte'ını yaz
            core::ptr::write_volatile(serial_port_data_register, *byte);
        }
    }

    // --- 3. Sistemi Durdur ---
    // Kernel kurtarılamaz durumda olduğundan, CPU'yu durdurun veya kontrolü geri vermeyecek
    // bir sonsuz döngüye girin. Sonsuz döngü en basit ve güvenli yöntemdir.
    loop {
        // İsteğe bağlı: Düşük güç moduna girme veya 'illegal instruction' gibi
        // mimariye özel bir durdurma komutu kullanma (dikkatli olunmalıdır).
        // SPARC için 'ta 0' (Trap Always 0) bazen kullanılır, ancak bu da trap işleyicisinin
        // ayakta olmasını gerektirir, ki panik anında bu garanti değildir.
        // Güvenli olan sonsuz döngüdür.
    }
}
