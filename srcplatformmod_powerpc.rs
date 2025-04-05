#[cfg(target_arch = "powerpc")]
pub mod powerpc {
    use core::arch::asm;

    // PowerPC'ye özgü fonksiyonlar buraya eklenecek

    /// Örnek: PowerPC için özel bir bellek bariyeri
    pub fn mb() {
        unsafe {
            asm!("sync"); // "sync" PowerPC bellek bariyeri komutu
        }
    }

    /// Örnek: PowerPC için kritik bölge başlangıcı
    pub fn critical_section_start() {
        // Kesmeleri devre dışı bırakma veya diğer senkronizasyon mekanizmaları
        unsafe {
            asm!("wrpmsr 0x8000, r0"); // Örnek: MSR'deki (Makine Durum Kaydı) uygun biti ayarlayarak kesmeleri devre dışı bırak
        }
    }


    /// Örnek: PowerPC için kritik bölge sonu
    pub fn critical_section_end() {
       // Kesmeleri etkinleştirme veya önceki duruma geri yükleme
       unsafe {
           asm!("wrpmsr 0x0000, r0"); // Örnek: MSR'deki biti temizleyerek kesmeleri etkinleştir. Dikkatli olun! Önceki durumu yüklemek daha güvenli olabilir.
       }
    }


    /// Başka bir örnek: PowerPC'ye özgü bir donanım özelliğini okuma
    pub fn read_special_register() -> u32 {
        let value: u32;
        unsafe {
            asm!("mfs {}, spr(268)", out(reg) value); // Örnek: SPR 268'i (Özel Amaçlı Kayıt) okuma
        }
        value
    }

     /// Örnek: PowerPC'ye özgü bir donanım özelliğini yazma
    pub fn write_special_register(value: u32) {
        unsafe {
            asm!("mts spr(268), {}", in(reg) value); // Örnek: SPR 268'e (Özel Amaçlı Kayıt) yazma
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mb() {
            // Test kodu (eğer varsa)
            mb(); // Sadece çağırarak test edilebilir. Derleyici optimizasyonları nedeniyle, etkisini doğrudan gözlemlemek zor olabilir.
            println!("mb fonksiyonu çağrıldı (test)"); // Test çıktısı
        }

        #[test]
        fn test_critical_section() {
           critical_section_start();
           // Kritik kod buraya
           critical_section_end();
           println!("Kritik bölüm test edildi");
        }

        #[test]
        fn test_special_register() {
            let original_value = read_special_register();
            println!("Özel kayıt değeri: {}", original_value);
            // Değeri değiştirip geri yüklemek gibi testler eklenebilir.
        }
    }
}

#[cfg(not(target_arch = "powerpc"))]
pub mod powerpc {
    // PowerPC olmayan mimariler için boş uygulama veya alternatifler
    pub fn mb() {}
    pub fn critical_section_start() {}
    pub fn critical_section_end() {}
    pub fn read_special_register() -> u32 { 0 }
    pub fn write_special_register(_: u32) {}

    #[cfg(test)]
    mod tests {
        #[test]
        fn test_dummy() {
            // PowerPC değilse testler geçersiz sayılır.
            println!("PowerPC mimarisi değil. Testler geçersiz.");
        }
    }
}