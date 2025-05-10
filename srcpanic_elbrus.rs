#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışır

use core::panic::PanicInfo;
use core::fmt::Write; // `write!` ve `writeln!` makrolarını kullanmak için

// Karnal64 API'sından gerekli öğeleri içeri aktaralım.
// Gerçek bir çekirdek projesinde, bu muhtemelen ana Karnal64 crate'inden veya modüllerinden yapılacaktır.
// Burada, bağımlılıkları göstermek için gerekli trait ve tipleri varsayımsal olarak çağırıyoruz.
// Gerçek kresource implementasyonunda panik anında güvenli bir konsol erişimi olmalıdır.
use karnal64::{KError, kresource::ResourceProvider};
use karnal64::arch; // Mimariye özgü düşük seviye işlemler için varsayımsal modül

// Panik anında konsola yazabilmek için ResourceProvider traitini kullanan
// basit bir yazıcı yapısı tanımlayalım.
struct ConsoleWriter;

// core::fmt::Write traitini implemente ederek `write!` ve `writeln!` makrolarını
// bu yazıcı ile kullanabiliriz.
impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // Panik anında bellek tahsisi veya kilitlenme gibi yan etkileri olan
        // karmaşık Karnal64 fonksiyonlarını çağırmaktan kaçınmalıyız.
        // Konsol çıktısı için, kresource modülünün panik anında güvenli
        // bir şekilde konsol ResourceProvider'ına erişim sağlayan özel bir
        // fonksiyonu olduğunu varsayıyoruz.

        // Varsayımsal olarak: kresource modülünde panik konsol sağlayıcısını döndüren fonksiyon
        // Bu fonksiyon gerçek kresource modülünde implemente edilmelidir.
        // Panik anında güvenli olması (reentrant olmaması vb.) KRİTİKTİR.
        if let Some(console_provider) = karnal64::kresource::get_panic_console_provider() {
            // ResourceProvider'ın `write` metodunu kullanarak konsola yaz.
            // Konsol için ofset genellikle 0'dır.
            let write_result = console_provider.write(s.as_bytes(), 0);

            match write_result {
                Ok(_) => Ok(()), // Yazma başarılıysa fmt::Result::Ok döndür
                Err(_) => Err(core::fmt::Error), // Karnal64 hatasını fmt::Error'a map et
            }
        } else {
            // Eğer panik konsol sağlayıcısına erişilemiyorsa, yazma başarısız olur.
            // Bu durumda panik mesajı görülemeyebilir, ancak sistem yine de duracaktır.
            Err(core::fmt::Error)
        }
    }
}


// Rust'ın panik işleyici kancasını (`#[panic_handler]`) implemente ediyoruz.
// Bir panik oluştuğunda bu fonksiyon çağrılacaktır.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Tüm kesmeleri (interrupts) devre dışı bırak.
    // Bu, panik sırasında başka kesmelerin işleyiciyi kesmesini engeller.
    // Bu işlem mimariye özgüdür. Varsayımsal arch modülünü kullanalım.
    arch::interrupts::disable();

    // 2. Konsola panik mesajını yazmaya çalış.
    let mut writer = ConsoleWriter;

    // writeln! makrosu ConsoleWriter'ımızı kullanarak çıktı gönderecektir.
    // Konsol sağlayıcısı kullanılamıyorsa yazma hataları sessizce göz ardı edilir.
    let _ = writeln!(writer, "\n--- KERNEL PANIC ---");

    // Panik konum bilgisini yazdır
    if let Some(location) = info.location() {
        let _ = writeln!(writer, "Location: {}:{}", location.file(), location.line());
    } else {
        let _ = writeln!(writer, "Location: Unknown");
    }

    // Panik mesajını (varsa) yazdır
    if let Some(message) = info.message() {
        // message bir fmt::Arguments olabilir, formatlı çıktıyı yakalamak için {} kullanırız.
        let _ = writeln!(writer, "Message: {}", message);
    } else {
        let _ = writeln!(writer, "Message: <No message>");
    }

    // TODO: Daha gelişmiş hata ayıklama bilgileri ekle (isteğe bağlı ve ileri düzey):
    // - Görev/iş parçacığı kimliği
    // - CPU registerlarının dökümü
    // - Kısmi yığın izi (stack trace) - sembolikleştirme olmadan bile adresler faydalı olabilir.

    let _ = writeln!(writer, "--- HALTING SYSTEM ---");

    // 3. Sistemi durdur.
    // Panik işleyicisi asla geri dönmemelidir. Sistem bu noktada ya donar ya da yeniden başlatılır.
    // Çoğu çekirdek hata ayıklama için sonsuz döngüye girer. Mimariye özgü durdurma talimatı da kullanılabilir.
    arch::cpu::halt(); // Varsayımsal mimariye özgü durdurma fonksiyonu

    // halt() fonksiyonu teorik olarak geri dönmeyeceği için buradaki loop
    // teknik olarak unreachable (erişilemez) kod olmalıdır.
    loop {}
}