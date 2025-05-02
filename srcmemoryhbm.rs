#![no_std]

// Use the Sahne64 crate's modules/types
// Assuming this file is part of a separate crate that depends on 'sahne64'
use sahne64::{memory, SahneError}; // <-- Changed import
use core::ptr::{self, read_volatile, write_volatile}; // Added ptr module for null_mut and write_bytes example
// No need for core::mem::ManuallyDrop here either.

// Need access to the custom print/eprint macros from Sahne64's stdio_impl in no_std
// Assuming these are made available.

/// HBM Bellek Yöneticisi işlemleri sırasında oluşabilecek hataları tanımlar.
#[derive(Debug)]
pub enum HBMError { // <-- New error enum
    /// Yeterli HBM belleği yok (yerel takibe göre).
    OutOfMemory,
    /// Bellek ayırma veya serbest bırakma sırasında Sahne64 API hatası oluştu.
    MemoryOperationError(SahneError),
    /// Geçersiz parametre (örn. null pointer ile serbest bırakma).
    InvalidParameter(String),
    // Add other HBM-specific errors if needed
}

impl From<SahneError> for HBMError { // <-- Implement From for SahneError
    fn from(error: SahneError) -> Self {
        HBMError::MemoryOperationError(error)
    }
}


/// `HBMMemoryManager` yapısı, HBM bellek yönetimini temsil eder.
/// Bu yapı, alt seviye `sahne64::memory` fonksiyonlarını kullanarak
/// belirli bir toplam boyutu aşmayacak şekilde bellek tahsisini takip eder.
/// NOT: Bu basit bir simülasyondur, gerçek HBM yönetimi donanıma özel olabilir.
pub struct HBMMemoryManager {
    hbm_bellek_boyutu: usize, // HBM belleğinin toplam boyutu (bayt cinsinden) - Bir limit gibi davranır
    kullanilan_bellek: usize,   // Şu anda bu yönetici aracılığıyla ayrılmış bellek miktarı
    // ... (İleri düzey senaryolar için: ayrılan blokları takip etmek için bir yapı)
}

impl HBMMemoryManager {
    /// Yeni bir `HBMMemoryManager` örneği oluşturur.
    /// Bu yönetici, belirtilen `boyut` kadar bir limit dahilinde bellek tahsisini takip eder.
    /// Altta yatan bellek, Sahne64'ün ana bellek havuzundan `memory::allocate` ile alınır.
    ///
    /// # Parametreler
    ///
    /// * `size`: Yönetilecek HBM belleğinin toplam boyutu (bayt cinsinden limit).
    ///
    /// # Örnek
    ///
    /// ```ignore // Örneklerin derlenebilirliği için Sahne64 crate'ine bağımlılık ve uygun bir ortam gerekir.
     use your_crate_name::HBMMemoryManager;
     use sahne64::memory; // Gerekirse alt seviye kullanımlar için
    ///
     fn example_hbm_manager() {
     let mut hbm_yonetici = HBMMemoryManager::new(1 * 1024 * 1024 * 1024); // 1GB HBM belleği limiti
    /// // // Kullanım...
     }
    /// ```
    pub fn new(size: usize) -> Self {
        HBMMemoryManager {
            hbm_bellek_boyutu: size,
            kullanilan_bellek: 0,
        }
    }

    /// HBM belleğinden (yerel takibe göre) belirli boyutta bir bellek bloğu ayırır.
    /// Gerçek bellek tahsisi altta yatan `sahne64::memory::allocate` çağrısı ile yapılır.
    ///
    /// # Parametreler
    ///
    /// * `size`: Ayrılacak bellek bloğunun boyutu (bayt cinsinden).
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa, ayrılan bellek bloğunun başlangıç adresini içeren ham bir işaretçi (`*mut u8`) döndüren `Ok`.
    /// Başarısız olursa (yerel limit aşılırsa veya alt seviye tahsis hatası), `Err` ile bir `HBMError` döndürür.
    ///
    /// # Güvenlik
    ///
    /// Bu fonksiyon `unsafe` blok kullanır çünkü `memory::allocate` ham işaretçi döndürür.
    /// Döndürülen işaretçiyi güvenli bir şekilde kullanmak çağıranın sorumluluğundadır.
    ///
    /// # Örnek
    ///
    /// ```ignore // Örneklerin derlenebilirliği için Sahne64 crate'ine bağımlılık ve uygun bir ortam gerekir.
     use your_crate_name::HBMMemoryManager;
     use sahne64::memory;
    ///
     fn example_hbm_alloc(manager: &mut HBMMemoryManager) {
     match manager.bellek_ayir(100) {
         Ok(ptr) => {
    /// //         // ... belleği kullan ... (örneğin unsafe blok içinde yazma)
              unsafe { core::ptr::write_bytes(ptr, 0xAA, 100); }
             manager.bellek_serbest_birak(ptr, 100).unwrap(); // Serbest bırakma da Result döner şimdi
         }
         Err(hata) => {
    /// //         // ... hata işle ...
         }
     }
     }
    /// ```
    pub fn bellek_ayir(&mut self, size: usize) -> Result<*mut u8, HBMError> { // <-- Return HBMError
        // Yerel takip limiti kontrolü
        if self.kullanilan_bellek.checked_add(size).unwrap_or(usize::MAX) > self.hbm_bellek_boyutu { // Overflow check
            return Err(HBMError::OutOfMemory); // Yerel limit hatası
        }

        // Altta yatan Sahne64 bellek tahsisini çağır
        // memory::allocate yeni Sahne64 API'sine göre Result<*mut u8, SahneError> dönüyor
        match memory::allocate(size) {
            Ok(ptr) => {
                self.kullanilan_bellek = self.kullanilan_bellek.checked_add(size).unwrap_or(usize::MAX); // Güvenli artırma
                Ok(ptr)
            }
            Err(e) => {
                // Sahne64 API hatasını HBMError'a çevir (From implementasyonu kullanılabilir)
                Err(HBMError::from(e)) // HBMError::MemoryOperationError(e) ile aynı
            }
        }
    }


    /// Önceden ayrılmış bir bellek bloğunu serbest bırakır.
    /// Altta yatan bellek serbest bırakma işlemi `sahne64::memory::release` çağrısı ile yapılır.
    ///
    /// # Parametreler
    ///
    /// * `ptr`: Serbest bırakılacak bellek bloğunun başlangıç adresi (ham işaretçi).
    /// * `size`: Serbest bırakılacak bellek bloğunun boyutu (bayt cinsinden). *ÖNEMLİ*: Çağıran, doğru işaretçi ve boyut bilgisini sağlamalıdır.
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa `Ok(())`, hata oluşursa (örn. geçersiz pointer, yanlış boyut veya Sahne64 API hatası) `Err` ile bir `HBMError` döndürür.
    ///
    /// # Güvenlik
    ///
    /// Bu fonksiyon da `unsafe` blok kullanır (altta yatan `memory::release` çağrısı nedeniyle). Çağıran kişi
    /// `ptr` ve `size` parametrelerinin doğru olduğundan emin olmalıdır.
    ///
    /// # Örnek
    ///
    /// ```ignore // Örneklerin derlenebilirliği için Sahne64 crate'ine bağımlılık ve uygun bir ortam gerekir.
     use your_crate_name::HBMMemoryManager;
     use sahne64::memory;
    ///
     fn example_hbm_free(manager: &mut HBMMemoryManager, ptr: *mut u8, size: usize) {
     match manager.bellek_serbest_birak(ptr, size) {
         Ok(()) => {
              println!("Bellek başarıyla serbest bırakıldı.");
         }
         Err(hata) => {
              eprintln!("Bellek serbest bırakma hatası: {:?}", hata);
         }
     }
     }
    /// ```
    pub fn bellek_serbest_birak(&mut self, ptr: *mut u8, size: usize) -> Result<(), HBMError> { // <-- Return Result<(), HBMError>
         // Null işaretçi kontrolü
        if ptr.is_null() {
            // Null pointer ile serbest bırakma genellikle bir hatadır
            return Err(HBMError::InvalidParameter("Null pointer ile serbest bırakma denemesi".into())); // HBMError variantı döndür
        }
        // Not: Burada boyut kontrolü (size'ın ayrılmış bloklardan birine ait olup olmadığı)
        // bu basit implementasyonda yapılmıyor. Gerçek bir mem yöneticisi bu bilgiyi tutmalıdır.

        // Altta yatan Sahne64 bellek serbest bırakmayı çağır
        // memory::free yeni Sahne64 API'sinde memory::release olarak adlandırıldı ve Result<(), SahneError> dönüyor
        match memory::release(ptr, size) { // <-- Changed function call name
            Ok(_) => {
                // Başarılı olursa kullanılan bellek miktarını güvenli bir şekilde azalt
                self.kullanilan_bellek = self.kullanilan_bellek.checked_sub(size).unwrap_or(0); // Güvenli azaltma, 0 altına düşmez
                Ok(()) // Başarıyı bildir
            }
            Err(e) => {
                // Sahne64 API hatasını HBMError'a çevir (From implementasyonu kullanılabilir)
                // Burada hata loglama da yapabilirsiniz, ancak Result döndürdüğümüz için çağıran ilgilenir.
                Err(HBMError::from(e)) // HBMError::MemoryOperationError(e) ile aynı
            }
        }
    }

    /// Şu anda bu yönetici tarafından takip edilen kullanılan bellek miktarını döndürür.
    pub fn kullanilan_bellek_miktari(&self) -> usize {
        self.kullanilan_bellek
    }

    /// Bu yöneticinin takip ettiği toplam HBM bellek boyutunu (limiti) döndürür.
    pub fn toplam_bellek_boyutu(&self) -> usize {
        self.hbm_bellek_boyutu
    }
}

// Örnek kullanım (kernel içinde bir test fonksiyonu olabilir)
#[cfg(feature = "std")] // Bu bölüm sadece standart kütüphane ile derlenirken aktif olur
fn main() {
    // Bu örnek std ortamında çalışacak şekilde yapılandırılmıştır.
    // no_std ortamında Sahne64'ün çıktı makroları veya özel bir çıktı mekanizması gerektirir.

    #[cfg(feature = "std")]
    std::println!("Sahne64 HBM Bellek Yöneticisi Örneği (std)");
    #[cfg(not(feature = "std"))]
    println!("Sahne64 HBM Bellek Yöneticisi Örneği (no_std)");


    let mut hbm_yonetici = HBMMemoryManager::new(2 * 1024 * 1024); // 2MB HBM simülasyonu limiti

    #[cfg(feature = "std")]
    std::println!("Toplam HBM Bellek Limiti: {} bayt", hbm_yonetici.toplam_bellek_boyutu());
    #[cfg(not(feature = "std"))]
    println!("Toplam HBM Bellek Limiti: {} bayt", hbm_yonetici.toplam_bellek_boyutu());

    #[cfg(feature = "std")]
    std::println!("Kullanılan HBM Bellek Miktarı (başlangıç): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());
    #[cfg(not(feature = "std"))]
    println!("Kullanılan HBM Bellek Miktarı (başlangıç): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());


    match hbm_yonetici.bellek_ayir(512) {
        Ok(ptr) => {
             #[cfg(feature = "std")]
            std::println!("512 bayt bellek ayrıldı (Sahne64 havuzundan), adres: {:?}", ptr);
             #[cfg(not(feature = "std"))]
            println!("512 bayt bellek ayrıldı (Sahne64 havuzundan), adres: {:?}", ptr);

             #[cfg(feature = "std")]
            std::println!("Kullanılan HBM Bellek Miktarı (sonra ayırma): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());
             #[cfg(not(feature = "std"))]
            println!("Kullanılan HBM Bellek Miktarı (sonra ayırma): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());


            // Belleği kullanma örneği (isteğe bağlı)
             ptr::write_bytes std::ptr::write_bytes yerine core::ptr::write_bytes kullanırız no_std için
            unsafe {
                  core::ptr::write_bytes(ptr, 0xAA, 512); // Ayrılan belleği 0xAA ile doldur - core::ptr modülünü import et
                  write_volatile(ptr, 0xAA); // Sadece ilk byte (volatile yazma örneği)
                  volatile yazma için core::ptr::{read_volatile, write_volatile} import edildi

                 // 512 byte'ı volatile olarak doldurmak için döngü gerekebilir
                 let mut current_ptr = ptr;
                 for i in 0..512 {
                     write_volatile(current_ptr.add(i), 0xAA);
                 }
                 #[cfg(feature = "std")]
                 std::println!("Ayrılan {} bayt belleğe yazıldı (volatile).", 512);
                 #[cfg(not(feature = "std"))]
                 println!("Ayrılan {} bayt belleğe yazıldı (volatile).", 512);

            }


            // Belleği serbest bırak
            match hbm_yonetici.bellek_serbest_birak(ptr, 512) { // Şimdi Result döner
                 Ok(()) => {
                     #[cfg(feature = "std")]
                     std::println!("512 bayt bellek serbest bırakıldı.");
                      #[cfg(not(feature = "std"))]
                     println!("512 bayt bellek serbest bırakıldı.");

                     #[cfg(feature = "std")]
                     std::println!("Kullanılan HBM Bellek Miktarı (sonra serbest bırakma): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());
                     #[cfg(not(feature = "std"))]
                     println!("Kullanılan HBM Bellek Miktarı (sonra serbest bırakma): {} bayt", hbm_yonetici.kullanilan_bellek_miktari());
                 }
                 Err(hata) => {
                      #[cfg(feature = "std")]
                     std::eprintln!("Bellek serbest bırakma hatası: {:?}", hata);
                      #[cfg(not(feature = "std"))]
                     eprintln!("Bellek serbest bırakma hatası: {:?}", hata);
                 }
            }

        }
        Err(hata) => {
             #[cfg(feature = "std")]
            std::eprintln!("Bellek ayırma hatası: {:?}", hata); // Şimdi HBMError döner
             #[cfg(not(feature = "std"))]
            eprintln!("Bellek ayırma hatası: {:?}", hata); // Şimdi HBMError döner
        }
    }

    #[cfg(feature = "std")]
    std::println!("\n--- Büyük Bellek Ayırma Hatası Testi ---");
    #[cfg(not(feature = "std"))]
    println!("\n--- Büyük Bellek Ayırma Hatası Testi ---");


    // Çok büyük bir ayırma isteği (HATA örneği - yerel limit aşılır)
    match hbm_yonetici.bellek_ayir(2 * 1024 * 1024 + 1) {
        Ok(_) => {
            // Bu kısma asla gelinmemeli çünkü yerel limit aşılacak
             #[cfg(feature = "std")]
            std::println!("Bu olmamalı! Çok büyük bellek ayırma başarılı oldu.");
             #[cfg(not(feature = "std"))]
            println!("Bu olmamalı! Çok büyük bellek ayırma başarılı oldu.");
        }
        Err(hata) => {
             #[cfg(feature = "std")]
            std::eprintln!("Büyük bellek ayırma hatası (beklenen hata): {:?}", hata); // Şimdi HBMError döner
             #[cfg(not(feature = "std"))]
            eprintln!("Büyük bellek ayırma hatası (beklenen hata): {:?}", hata); // Şimdi HBMError döner
        }
    }

    #[cfg(feature = "std")]
    std::println!("\n--- Null Pointer Serbest Bırakma Hatası Testi ---");
    #[cfg(not(feature = "std"))]
    println!("\n--- Null Pointer Serbest Bırakma Hatası Testi ---");

    // Null pointer ile serbest bırakma denemesi (HATA örneği)
    match hbm_yonetici.bellek_serbest_birak(ptr::null_mut(), 100) {
         Ok(()) => {
              #[cfg(feature = "std")]
             std::println!("Bu olmamalı! Null pointer serbest bırakma başarılı oldu.");
              #[cfg(not(feature = "std"))]
             println!("Bu olmamalı! Null pointer serbest bırakma başarılı oldu.");
         }
         Err(hata) => {
              #[cfg(feature = "std")]
             std::eprintln!("Null pointer serbest bırakma hatası (beklenen hata): {:?}", hata); // Şimdi HBMError döner
              #[cfg(not(feature = "std"))]
             eprintln!("Null pointer serbest bırakma hatası (beklenen hata): {:?}", hata); // Şimdi HBMError döner
         }
    }
}
