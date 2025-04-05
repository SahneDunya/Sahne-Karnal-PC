#![no_std]

// x86 mimarisinde segment tabanlı bellek koruması (basitleştirilmiş örnek)
// Modern x86-64 sistemlerde paging yaygın olarak kullanılsa da,
// segmentasyon konsepti bellek koruma mekanizmalarının temelini anlamak için hala yararlıdır.
// Bu örnek, segmentasyonun basitleştirilmiş bir versiyonunu göstermektedir.

// **Uyarı:** Bu kod, x86 bellek segmentasyonunun **basitleştirilmiş** bir örneğidir.
// Gerçek dünya x86 sistemlerinde bellek koruması çok daha karmaşık mekanizmalar (paging, yetki seviyeleri vb.) ile sağlanır.
// Bu örnek sadece **eğitim amaçlıdır** ve gerçek bir güvenlik çözümü olarak kullanılmamalıdır.

// Güvenli bellek bölgesi sınırları (örnek olarak sabit kodlanmış)
const SECURE_MEMORY_START: usize = 0x100000; // 1MB
const SECURE_MEMORY_END: usize = 0x200000;   // 2MB

// Güvenli bölgeye erişimi kontrol eden fonksiyon (basitleştirilmiş)
pub fn is_address_secure(address: usize) -> bool {
    address >= SECURE_MEMORY_START && address < SECURE_MEMORY_END
}

// Bellek erişimini simüle eden fonksiyon (güvenlik kontrolü ile)
pub fn access_memory(address: usize, is_write: bool) -> Result<(), &'static str> {
    if is_address_secure(address) {
        // Güvenli bölgeye erişim izni (burada sadece bir mesaj yazdırıyoruz)
        if is_write {
            // Yazma erişimi simülasyonu
            kprintln!("Güvenli bölgeye yazma erişimi: 0x{:x}", address);
        } else {
            // Okuma erişimi simülasyonu
            kprintln!("Güvenli bölgeden okuma erişimi: 0x{:x}", address);
        }
        Ok(()) // Erişim başarılı
    } else {
        // Güvenli bölge dışına erişim engellendi
        Err("Güvenli bölge dışına yetkisiz erişim!")
    }
}

// Basit bir çekirdek println! benzeri fonksiyon (no_std ortamı için)
macro_rules! kprintln {
    ($($arg:tt)*) => {{
        let mut s = StringWriter;
        use core::fmt::Write;
        let _ = core::write!(&mut s, $($arg)*);
    }};
}

// StringWriter yapısı (kprintln! makrosu için)
use core::fmt;
struct StringWriter;

impl fmt::Write for StringWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Bu örnekte basitçe karakterleri ekrana yazdırıyoruz (gerçek bir sistemde UART vb. kullanılabilir)
        for byte in s.bytes() {
            unsafe {
                let vga_buffer = 0xb8000 as *mut u8;
                static mut VGA_INDEX: usize = 0;
                vga_buffer.add(VGA_INDEX).write_volatile(byte);
                VGA_INDEX += 1;
            }
        }
        Ok(())
    }
}


// Örnek kullanım fonksiyonu
pub fn example_usage() {
    kprintln!("x86 Güvenlik Örneği Başlatılıyor...\n");

    let safe_address = SECURE_MEMORY_START + 0x100;
    let unsafe_address = SECURE_MEMORY_END + 0x100;

    match access_memory(safe_address, false) {
        Ok(_) => kprintln!("0x{:x} adresine okuma erişimi başarılı.\n", safe_address),
        Err(e) => kprintln!("0x{:x} adresine okuma erişimi hatası: {}\n", safe_address, e),
    }

    match access_memory(safe_address, true) {
        Ok(_) => kprintln!("0x{:x} adresine yazma erişimi başarılı.\n", safe_address),
        Err(e) => kprintln!("0x{:x} adresine yazma erişimi hatası: {}\n", safe_address, e),
    }


    match access_memory(unsafe_address, false) {
        Ok(_) => kprintln!("0x{:x} adresine okuma erişimi başarılı (HATALI!):\n", unsafe_address),
        Err(e) => kprintln!("0x{:x} adresine okuma erişimi hatası (DOĞRU!): {}\n", unsafe_address, e),
    }

    match access_memory(unsafe_address, true) {
        Ok(_) => kprintln!("0x{:x} adresine yazma erişimi başarılı (HATALI!):\n", unsafe_address),
        Err(e) => kprintln!("0x{:x} adresine yazma erişimi hatası (DOĞRU!): {}\n", unsafe_address, e),
    }

    kprintln!("\n--- x86 Güvenlik Örneği Sonu ---");
}