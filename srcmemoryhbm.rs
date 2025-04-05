#![no_std]

use crate::memory; // memory modülünü içeri aktar

/// `HBMMemoryManager` yapısı, HBM bellek yönetimini temsil eder.
pub struct HBMMemoryManager {
    hbm_bellek_boyutu: usize, // HBM belleğinin toplam boyutu (bayt cinsinden)
    kullanilan_bellek: usize,   // Şu anda kullanılan bellek miktarı
    // ... (İleri düzey senaryolar için: serbest blok listesi, vb.)
}

impl HBMMemoryManager {
    /// Yeni bir `HBMMemoryManager` örneği oluşturur.
    ///
    /// # Parametreler
    ///
    /// * `boyut`: Yönetilecek HBM belleğinin toplam boyutu (bayt cinsinden).
    ///
    /// # Örnek
    ///
    /// ```
    /// #![no_std]
    /// # use crate::memory;
    ///
    /// let mut hbm_yonetici = HBMMemoryManager::new(1 * 1024 * 1024 * 1024); // 1GB HBM belleği
    /// ```
    pub fn new(boyut: usize) -> Self {
        HBMMemoryManager {
            hbm_bellek_boyutu: boyut,
            kullanilan_bellek: 0,
        }
    }

    /// HBM belleğinden belirli boyutta bir bellek bloğu ayırır.
    ///
    /// # Parametreler
    ///
    /// * `boyut`: Ayrılacak bellek bloğunun boyutu (bayt cinsinden).
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa, ayrılan bellek bloğunun başlangıç adresini içeren ham bir işaretçi (`*mut u8`) döndürür.
    /// Başarısız olursa (örneğin, yeterli bellek yoksa), `Err` ile bir `&'static str` hata mesajı döndürür.
    ///
    /// # Güvenlik
    ///
    /// Bu fonksiyon `unsafe` blok kullanır çünkü ham işaretçilerle çalışır ve düşük seviyeli bellek yönetimi yapar.
    /// Fonksiyonu çağıran kişi, döndürülen işaretçiyi güvenli bir şekilde kullanmaktan sorumludur (örneğin, geçerli adrese yazmaktan,
    /// bellek sızıntılarını önlemekten, vs.).
    ///
    /// # Örnek
    ///
    /// ```
    /// #![no_std]
    /// # use crate::{memory, HBMMemoryManager};
    ///
    /// let mut hbm_yonetici = HBMMemoryManager::new(1024);
    /// match hbm_yonetici.bellek_ayir(100) {
    ///     Ok(ptr) => {
    ///         // ... belleği kullan ...
    ///         hbm_yonetici.bellek_serbest_birak(ptr, 100);
    ///     }
    ///     Err(hata) => {
    ///         // ... hata işle ...
    ///     }
    /// }
    /// ```
    pub fn bellek_ayir(&mut self, boyut: usize) -> Result<*mut u8, &'static str> {
        if self.kullanilan_bellek + boyut > self.hbm_bellek_boyutu {
            return Err("Yetersiz HBM belleği");
        }

        match memory::allocate(boyut) {
            Ok(ptr) => {
                self.kullanilan_bellek += boyut;
                Ok(ptr)
            }
            Err(e) => match e {
                memory::SahneError::OutOfMemory => Err("Yetersiz HBM belleği"),
                _ => Err("HBM bellek yönetimi hatası"),
            },
        }
    }


    /// Önceden ayrılmış bir bellek bloğunu serbest bırakır.
    ///
    /// # Parametreler
    ///
    /// * `ptr`: Serbest bırakılacak bellek bloğunun başlangıç adresi (ham işaretçi).
    /// * `boyut`: Serbest bırakılacak bellek bloğunun boyutu (bayt cinsinden). *ÖNEMLİ*: Bu bilgi gereklidir
    ///           çünkü bu örnek, ayrılan blokların boyutunu takip etmez (basitlik için). Gerçek bir uygulamada,
    ///           boyut bilgisi genellikle bellek yöneticisi tarafından yönetilmelidir.
    ///
    /// # Güvenlik
    ///
    /// Bu fonksiyon da `unsafe` blok kullanır çünkü ham işaretçilerle çalışır. Fonksiyonu çağıran kişi,
    /// `ptr` ve `boyut` parametrelerinin doğru olduğundan emin olmalıdır. Yanlış kullanım (örneğin, geçersiz bir
    /// işaretçi veya yanlış boyut ile serbest bırakma), bellek güvenliği sorunlarına yol açabilir.
    ///
    /// # Örnek
    ///
    /// ```
    /// #![no_std]
    /// # use crate::{memory, HBMMemoryManager};
    ///
    /// // ... bellek_ayir fonksiyonundan elde edilen 'ptr' ve 'boyut' değerleri
    /// let mut hbm_yonetici = HBMMemoryManager::new(1024);
    /// match hbm_yonetici.bellek_ayir(100) {
    ///     Ok(ptr) => {
    ///         hbm_yonetici.bellek_serbest_birak(ptr, 100);
    ///     }
    ///     Err(_) => {}
    /// }
    /// ```
    pub fn bellek_serbest_birak(&mut self, ptr: *mut u8, boyut: usize) {
        if ptr.is_null() {
            return; // Null işaretçiyi serbest bırakmaya çalışma
        }
        match memory::free(ptr, boyut) {
            Ok(_) => {
                self.kullanilan_bellek -= boyut;
            }
            Err(e) => {
                // Burada bellek serbest bırakma hatasını loglayabilirsiniz (kernel log mekanizması varsa)
                match e {
                    memory::SahneError::InvalidAddress => {
                        // Geçersiz adres hatası
                    }
                    _ => {
                        // Diğer hatalar
                    }
                }
            }
        }
    }

    /// Şu anda kullanılan HBM bellek miktarını döndürür.
    pub fn kullanilan_bellek_miktari(&self) -> usize {
        self.kullanilan_bellek
    }

    /// Toplam HBM bellek boyutunu döndürür.
    pub fn toplam_bellek_boyutu(&self) -> usize {
        self.hbm_bellek_boyutu
    }
}

// Örnek kullanım (kernel içinde bir test fonksiyonu olabilir)
#[cfg(feature = "std")] // Bu bölüm sadece standart kütüphane ile derlenirken aktif olur
fn main() {
    let mut hbm_yonetici = HBMMemoryManager::new(2 * 1024 * 1024); // 2MB HBM simülasyonu

    println!("Toplam HBM Bellek Boyutu: {} bayt", hbm_yonetici.toplam_bellek_boyutu());
    println!("Kullanılan HBM Bellek Miktarı (başlangıç): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());

    match hbm_yonetici.bellek_ayir(512) {
        Ok(ptr) => {
            println!("512 bayt bellek ayrıldı, adres: {:?}", ptr);
            println!("Kullanılan HBM Bellek Miktarı (sonra ayırma): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());

            // Belleği kullanma örneği (isteğe bağlı)
            unsafe {
                ptr::write_bytes(ptr, 0xAA, 512); // Ayrılan belleği 0xAA ile doldur
            }

            hbm_yonetici.bellek_serbest_birak(ptr, 512);
            println!("512 bayt bellek serbest bırakıldı.");
            println!("Kullanılan HBM Bellek Miktarı (sonra serbest bırakma): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());
        }
        Err(hata) => {
            eprintln!("Bellek ayırma hatası: {}", hata);
        }
    }

    match hbm_yonetici.bellek_ayir(2 * 1024 * 1024 + 1) { // Çok büyük bir ayırma isteği (HATA örneği)
        Ok(_) => {
            // Bu kısma asla gelinmemeli çünkü bellek yetersiz olacak
        }
        Err(hata) => {
            eprintln!("Büyük bellek ayırma hatası (beklenen hata): {}", hata);
        }
    }
}