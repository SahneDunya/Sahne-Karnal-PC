#![no_std]
#[allow(dead_code)] // Keep if needed, otherwise remove

// Use the Sahne64 crate's modules/types
// Assuming this file is part of a separate crate that depends on 'sahne64'
use sahne64::{memory, SahneError}; // <-- Changed import, added SahneError
use core::ptr; // Added for pointer operations like null_mut
use core::slice; // Keep for slice operations

// Need access to the custom print/eprintln macros from Sahne64's stdio_impl in no_std
// Assuming these are made available.
#[cfg(not(feature = "std"))] #[macro_use] extern crate sahne64; // Or specific imports if macros exported differently.
// Remove the local print import as we'll use Sahne64's
use crate::print::{print, println}; // <-- Removed

/// LPDDR Bellek Tipleri
#[derive(Debug, Clone, Copy)]
pub enum LpddrType {
    Lpddr3,
    Lpddr4,
    Lpddr5,
}

impl core::fmt::Display for LpddrType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LpddrType::Lpddr3 => write!(f, "LPDDR3"),
            LpddrType::Lpddr4 => write!(f, "LPDDR4"),
            LpddrType::Lpddr5 => write!(f, "LPDDR5"),
        }
    }
}

/// LPDDR Yapılandırma Parametreleri
#[derive(Debug, Clone)]
pub struct LpddrConfig {
    pub lpddr_type: LpddrType,
    pub clock_speed_mhz: u32, // Bellek saat hızı (MHz cinsinden)
    pub voltage_mv: u16,      // Çalışma gerilimi (mV cinsinden)
    // ... Diğer yapılandırma parametreleri (örneğin zamanlamalar, adresleme modları vb.) ...
}

impl LpddrConfig {
    pub fn new(lpddr_type: LpddrType, clock_speed_mhz: u32, voltage_mv: u16) -> Self {
        LpddrConfig {
            lpddr_type,
            clock_speed_mhz,
            voltage_mv,
        }
    }
}

/// LPDDR Bellek Yöneticisi Hataları
#[derive(Debug, Clone)]
pub enum LpddrError { // <-- Redefine LpddrError
    /// Başlatma veya bellek işlemi sırasında Sahne64 API hatası oluştu.
    SahneApiError(SahneError), // <-- Wrap SahneError
    /// Belirtilen işlem desteklenmiyor (örn. hizalanmamış erişim).
    UnsupportedOperation(&'static str),
    /// Erişim adresi bellek aralığı dışında veya geçersiz.
    InvalidAddress(u64), // Keep specific address
    /// Yeterli bellek yok (Sahne64 API'den veya başka bir nedenle).
    OutOfMemory, // <-- Simplify, don't include u64 code
    // Add other LPDDR-specific errors if needed
}

// <-- Implement From<SahneError> for LpddrError
impl From<SahneError> for LpddrError {
    fn from(error: SahneError) -> Self {
        match error {
            // Map specific SahneErrors if meaningful at this layer, otherwise use generic wrapper
            SahneError::OutOfMemory => LpddrError::OutOfMemory, // Map SahneError::OutOfMemory to LpddrError::OutOfMemory
            e => LpddrError::SahneApiError(e), // Wrap all other SahneErrors
        }
    }
}

impl core::fmt::Display for LpddrError { // <-- Update Display implementation
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LpddrError::SahneApiError(e) => write!(f, "Sahne64 API Hatası: {:?}", e), // Display inner SahneError
            LpddrError::UnsupportedOperation(msg) => write!(f, "Desteklenmeyen İşlem: {}", msg),
            LpddrError::InvalidAddress(addr) => write!(f, "Geçersiz Adres: 0x{:X}", addr),
            LpddrError::OutOfMemory => write!(f, "Bellek Yetersiz"), // No code needed here
        }
    }
}


/// LPDDR Bellek Yöneticisi
/// Bu yapı, Sahne64'ün temel bellek tahsis mekanizmasını kullanarak
/// bir bellek bloğu edinir ve bu bloğa LPDDR'a özgü (simüle edilmiş)
/// okuma/yazma işlemleri için bir arayüz sağlar.
pub struct LpddrMemoryManager {
    config: LpddrConfig,
    memory_ptr: *mut u8,
    memory_size_bytes: usize,
    // Add a flag to track if memory_ptr is valid/allocated (e.g., set to null_mut after release)
    is_allocated: bool,
}

impl LpddrMemoryManager {
    /// Yeni bir LPDDR Bellek Yöneticisi oluşturur ve belirtilen boyutta bellek ayırır.
    ///
    /// # Parametreler
    ///
    /// * `config`: Kullanılacak LPDDR yapılandırması.
    /// * `memory_size_bytes`: Yönetilecek toplam bellek boyutu (bayt cinsinden). Bu boyutta Sahne64'ten bellek talep edilir.
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa `Ok(Self)`, bellek ayırma başarısız olursa `Err(LpddrError)`.
    ///
    /// # Güvenlik
    ///
    /// `memory::allocate` çağrısı nedeniyle `unsafe` ortamda kullanılır. Döndürülen işaretçi
    /// `memory_ptr` alanında saklanır. Bu yapı, `Drop` uygulayarak belleği serbest bırakır.
    pub fn new(config: LpddrConfig, memory_size_bytes: usize) -> Result<Self, LpddrError> { // <-- Return Result<Self, LpddrError>
         if memory_size_bytes == 0 {
             // Sıfır boyutlu tahsis genellikle geçerli değildir veya None döner.
             // Burada bir hata dönebiliriz.
             return Err(LpddrError::InvalidAddress(0)); // Veya InvalidParameter
         }

        // memory::allocate yeni Sahne64 API'sine göre Result<*mut u8, SahneError> dönüyor
        match memory::allocate(memory_size_bytes) {
            Ok(ptr) => Ok(LpddrMemoryManager {
                config,
                memory_ptr: ptr,
                memory_size_bytes,
                is_allocated: true, // Bellek başarıyla ayrıldı
            }),
            Err(e) => {
                // Sahne64 API hatasını LpddrError'a çevir (From implementasyonu kullanılır)
                Err(LpddrError::from(e)) // LpddrError::SahneApiError(e) veya LpddrError::OutOfMemory
            }
        }
    }

    /// Belirtilen adresten veri okur (32-bit).
    /// Okuma işlemi, yönetici oluşturulurken ayrılan bellek bloğu üzerinden yapılır.
    ///
    /// # Parametreler
    ///
    /// * `address`: Okunacak adres (bu yöneticinin yönettiği blok içindeki offset olarak düşünülebilir).
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa okunan 32-bit değeri içeren `Ok`, hata oluşursa `Err(LpddrError)`.
    ///
    /// # Güvenlik
    ///
    /// Ham pointer ile çalışır. Adres aralığı ve hizalama kontrolü yapılır.
    pub fn read(&self, address: u64) -> Result<u32, LpddrError> {
        // Adres aralığı ve hizalama kontrolü
        // address u64 olduğu için memory_size_bytes'ın u64'e cast edilmesi gerekebilir veya address usize olmalı?
        // API tasarımında adresler u64 olarak tanımlanmış, kullanıma uyalım.
        if address >= self.memory_size_bytes as u64 {
            return Err(LpddrError::InvalidAddress(address));
        }
         // 32-bit okuma için 4 byte gerekir, adres + 4 bellek boyutu içinde olmalı.
         // address + 4 overflow yaparsa da hata kabul edilmeli.
        if address.checked_add(4).unwrap_or(u64::MAX) > self.memory_size_bytes as u64 { // Check bounds including size of access
            return Err(LpddrError::InvalidAddress(address));
        }
        if address % 4 != 0 { // 32-bit okuma örneği, hizalama kontrolü
            return Err(LpddrError::UnsupportedOperation("32-bit hizalanmamış okuma desteklenmiyor"));
        }

        // Ham pointer + offset hesapla
        let start_ptr = self.memory_ptr.wrapping_add(address as usize); // usize ekleme için

        // Güvenli olmayan (unsafe) blok: Pointer geçerli ve boyut içinde varsayılır.
        unsafe {
             // volatile okuma, donanım registerları veya bellek eşlemeli I/O için uygun olabilir.
             // Direkt RAM okuması için read_unaligned veya normal pointer okuması da kullanılabilir.
             // Orijinal kod copy_from_slice kullanıyordu, bu da unaligned okuma/yazma yapabilir.
             // volatile olmayan bir yaklaşım:
              let mut data_bytes: [u8; 4] = [0; 4];
              ptr::copy_nonoverlapping(start_ptr, data_bytes.as_mut_ptr(), 4);
              let data = u32::from_le_bytes(data_bytes);

            // volatile yaklaşım (orijinal kodun ruhuna daha yakın olabilir)
             let data = ptr::read_volatile(start_ptr as *const u32); // 32-bit volatile oku

             #[cfg(feature = "std")]
             std::println!("0x{:X} adresinden okunan değer: 0x{:X}", address, data);
             #[cfg(not(feature = "std"))]
             println!("0x{:X} adresinden okunan değer: 0x{:X}", address, data);

            Ok(data)
        }
    }

    /// Belirtilen adrese veri yazar (32-bit).
    /// Yazma işlemi, yönetici oluşturulurken ayrılan bellek bloğu üzerinden yapılır.
    ///
    /// # Parametreler
    ///
    /// * `address`: Yazılacak adres (bu yöneticinin yönettiği blok içindeki offset olarak düşünülebilir).
    /// * `data`: Yazılacak 32-bit değer.
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa `Ok(())`, hata oluşursa `Err(LpddrError)`.
    ///
    /// # Güvenlik
    ///
    /// Ham pointer ile çalışır. Adres aralığı ve hizalama kontrolü yapılır.
    pub fn write(&mut self, address: u64, data: u32) -> Result<(), LpddrError> {
         // Adres aralığı ve hizalama kontrolü
        if address >= self.memory_size_bytes as u64 {
            return Err(LpddrError::InvalidAddress(address));
        }
        if address.checked_add(4).unwrap_or(u64::MAX) > self.memory_size_bytes as u64 { // Check bounds including size of access
            return Err(LpddrError::InvalidAddress(address));
        }
        if address % 4 != 0 { // 32-bit yazma örneği, hizalama kontrolü
            return Err(LpddrError::UnsupportedOperation("32-bit hizalanmamış yazma desteklenmiyor"));
        }

         // Ham pointer + offset hesapla
        let start_ptr = self.memory_ptr.wrapping_add(address as usize); // usize ekleme için

        // Güvenli olmayan (unsafe) blok: Pointer geçerli ve boyut içinde varsayılır.
        unsafe {
            // volatile yazma, donanım registerları veya bellek eşlemeli I/O için uygun olabilir.
            // Direkt RAM yazması için write_unaligned veya normal pointer yazması da kullanılabilir.
             // Orijinal kod copy_nonoverlapping kullanıyordu, bu da unaligned okuma/yazma yapabilir.
             // volatile olmayan bir yaklaşım:
              let data_bytes = data.to_le_bytes();
              ptr::copy_nonoverlapping(data_bytes.as_ptr(), start_ptr, 4);

            // volatile yaklaşım (orijinal kodun ruhuna daha yakın olabilir)
            ptr::write_volatile(start_ptr as *mut u32, data); // 32-bit volatile yaz

             #[cfg(feature = "std")]
             std::println!("0x{:X} adresine 0x{:X} değeri yazıldı", address, data);
             #[cfg(not(feature = "std"))]
             println!("0x{:X} adresine 0x{:X} değeri yazıldı", address, data);

            Ok(())
        }
    }

    // Bellek serbest bırakma işlemini deinit gibi bir fonksiyona koyup Drop'tan çağırmak daha iyidir.
    // Bu Drop implementasyonunu ekleyerek memory::release çağrısını otomatikleştiriyoruz.
    fn deinit(&mut self) -> Result<(), LpddrError> {
         if self.is_allocated && !self.memory_ptr.is_null() { // Eğer bellek ayrılmışsa ve pointer null değilse
             let ptr = self.memory_ptr;
             let size = self.memory_size_bytes;

             // Belleği serbest bırak
             // memory::free yeni Sahne64 API'sinde memory::release olarak adlandırıldı ve Result<(), SahneError> dönüyor
             let result = memory::release(ptr, size); // <-- Changed function call name

             self.memory_ptr = ptr::null_mut(); // Pointer'ı null yap
             self.memory_size_bytes = 0; // Boyutu sıfırla
             self.is_allocated = false; // Ayrılmadı olarak işaretle

             // Sahne64 API hatasını LpddrError'a çevir
             result.map_err(LpddrError::from) // LpddrError::SahneApiError(e) veya LpddrError::OutOfMemory
         } else {
             // Zaten serbest bırakılmış veya hiç ayrılmamış
             Ok(())
         }
    }

    // ... Diğer bellek yönetimi fonksiyonları (örneğin burst okuma/yazma, yenileme vb.) ...
}

// <-- Add Drop implementation to automatically call deinit/release
impl Drop for LpddrMemoryManager {
    fn drop(&mut self) {
         // Drop içinde hata yoksayılır, sadece loglama yapılır
         if let Err(e) = self.deinit() {
              #[cfg(feature = "std")] std::eprintln!("UYARI: LPDDR Bellek Yöneticisi serbest bırakılırken hata: {:?}", e);
              #[cfg(not(feature = "std"))] eprintln!("UYARI: LPDDR Bellek Yöneticisi serbest bırakılırken hata: {:?}", e);
         }
    }
}


// Örnek Kullanım (no_std ortamında main fonksiyonu farklı olabilir)
#[cfg(feature = "std")]
fn main() {
    // Bu örnek std ortamında çalışacak şekilde yapılandırılmıştır.
    // no_std ortamında Sahne64'ün çıktı makroları veya özel bir çıktı mekanizması gerektirir.

    #[cfg(feature = "std")]
    std::println!("Sahne64 LPDDR Bellek Yöneticisi Örneği (std)");
    #[cfg(not(feature = "std"))]
    println!("Sahne64 LPDDR Bellek Yöneticisi Örneği (no_std)");


    // LPDDR4 yapılandırması oluştur
    let lpddr4_config = LpddrConfig::new(
        LpddrType::Lpddr4,
        1600, // 1600 MHz saat hızı
        1100, // 1.1V (1100 mV) gerilim
    );

    // LPDDR5 yapılandırması oluştur
    let lpddr5_config = LpddrConfig::new(
        LpddrType::Lpddr5,
        3200, // 3200 MHz saat hızı
        1050, // 1.05V (1050 mV) gerilim
    );

    // LPDDR4 bellek yöneticisi oluştur (1MB bellek alanı)
    #[cfg(feature = "std")]
    std::println!("\n--- LPDDR4 Yöneticisi Testi ---");
    #[cfg(not(feature = "std"))]
    println!("\n--- LPDDR4 Yöneticisi Testi ---");

    match LpddrMemoryManager::new(lpddr4_config.clone(), 1024 * 1024) { // clone config to use again
        Ok(mut lpddr4_manager) => {
            // LPDDR4 belleği başlat
            match lpddr4_manager.initialize() {
                Ok(_) => {
                     #[cfg(feature = "std")] std::println!("LPDDR4 bellek başlatma başarılı.");
                     #[cfg(not(feature = "std"))] println!("LPDDR4 bellek başlatma başarılı.");
                }
                Err(e) => {
                     #[cfg(feature = "std")] std::eprintln!("LPDDR4 bellek başlatma hatası: {}", e);
                     #[cfg(not(feature = "std"))] eprintln!("LPDDR4 bellek başlatma hatası: {}", e);
                    // Başlatma hatası kritikse burada `return` veya `panic` yapılabilir.
                }
            }

            let address_to_write: u64 = 0x1000; // Yazılacak adres
            let data_to_write: u32 = 0x12345678; // Yazılacak veri

            // LPDDR4 belleğe yazma
            match lpddr4_manager.write(address_to_write, data_to_write) {
                Ok(_) => {
                     #[cfg(feature = "std")] std::println!("LPDDR4 belleğe yazma başarılı.");
                     #[cfg(not(feature = "std"))] println!("LPDDR4 belleğe yazma başarılı.");
                }
                Err(e) => {
                     #[cfg(feature = "std")] std::eprintln!("LPDDR4 belleğe yazma hatası: {}", e);
                     #[cfg(not(feature = "std"))] eprintln!("LPDDR4 belleğe yazma hatası: {}", e);
                }
            }

            // LPDDR4 bellekten okuma
            match lpddr4_manager.read(address_to_write) {
                Ok(data) => {
                     #[cfg(feature = "std")] std::println!("LPDDR4 bellekten okunan veri: 0x{:X}", data);
                     #[cfg(not(feature = "std"))] println!("LPDDR4 bellekten okunan veri: 0x{:X}", data);
                }
                Err(e) => {
                     #[cfg(feature = "std")] std::eprintln!("LPDDR4 bellekten okuma hatası: {}", e);
                     #[cfg(not(feature = "std"))] eprintln!("LPDDR4 bellekten okuma hatası: {}", e);
                }
            }

            // Belleği serbest bırakma Drop tarafından otomatik olarak yapılacak
             #[cfg(feature = "std")] std::println!("LPDDR4 yöneticisi scope dışına çıkıyor, bellek otomatik serbest bırakılacak.");
             #[cfg(not(feature = "std"))] println!("LPDDR4 yöneticisi scope dışına çıkıyor, bellek otomatik serbest bırakılacak.");

        } // lpddr4_manager burada Drop edilir
        Err(e) => {
             #[cfg(feature = "std")] std::eprintln!("LPDDR4 bellek yöneticisi oluşturma hatası: {}", e);
             #[cfg(not(feature = "std"))] eprintln!("LPDDR4 bellek yöneticisi oluşturma hatası: {}", e);
        }
    }

    // LPDDR5 bellek yöneticisi oluştur (1MB bellek alanı)
    #[cfg(feature = "std")]
    std::println!("\n--- LPDDR5 Yöneticisi Testi ---");
    #[cfg(not(feature = "std"))]
    println!("\n--- LPDDR5 Yöneticisi Testi ---");

    match LpddrMemoryManager::new(lpddr5_config.clone(), 1024 * 1024) { // clone config to use again
        Ok(mut lpddr5_manager) => {
            // LPDDR5 belleği başlat
            match lpddr5_manager.initialize() {
                Ok(_) => {
                     #[cfg(feature = "std")] std::println!("LPDDR5 bellek başlatma başarılı.");
                     #[cfg(not(feature = "std"))] println!("LPDDR5 bellek başlatma başarılı.");
                }
                Err(e) => {
                     #[cfg(feature = "std")] std::eprintln!("LPDDR5 bellek başlatma hatası: {}", e);
                     #[cfg(not(feature = "std"))] eprintln!("LPDDR5 bellek başlatma hatası: {}", e);
                     // Başlatma hatası kritikse burada `return` veya `panic` yapılabilir.
                }
            }

            let address_to_write: u64 = 0x2000; // Yazılacak adres
            let data_to_write: u32 = 0x9ABCDEF0; // Yazılacak veri

            // LPDDR5 belleğe yazma
            match lpddr5_manager.write(address_to_write, data_to_write) {
                Ok(_) => {
                     #[cfg(feature = "std")] std::println!("LPDDR5 belleğe yazma başarılı.");
                     #[cfg(not(feature = "std"))] println!("LPDDR5 belleğe yazma başarılı.");
                }
                Err(e) => {
                     #[cfg(feature = "std")] std::eprintln!("LPDDR5 belleğe yazma hatası: {}", e);
                     #[cfg(not(feature = "std"))] eprintln!("LPDDR5 belleğe yazma hatası: {}", e);
                }
            }

            // LPDDR5 bellekten okuma
            match lpddr5_manager.read(address_to_write) {
                Ok(data) => {
                     #[cfg(feature = "std")] std::println!("LPDDR5 bellekten okunan veri: 0x{:X}", data);
                     #[cfg(not(feature = "std"))] println!("LPDDR5 bellekten okunan veri: 0x{:X}", data);
                }
                Err(e) => {
                     #[cfg(feature = "std")] std::eprintln!("LPDDR5 bellekten okuma hatası: {}", e);
                     #[cfg(not(feature = "std"))] eprintln!("LPDDR5 bellekten okuma hatası: {}", e);
                }
            }

            // Belleği serbest bırakma Drop tarafından otomatik olarak yapılacak
             #[cfg(feature = "std")] std::println!("LPDDR5 yöneticisi scope dışına çıkıyor, bellek otomatik serbest bırakılacak.");
             #[cfg(not(feature = "std"))] println!("LPDDR5 yöneticisi scope dışına çıkıyor, bellek otomatik serbest bırakılacak.");

        } // lpddr5_manager burada Drop edilir
        Err(e) => {
             #[cfg(feature = "std")] std::eprintln!("LPDDR5 bellek yöneticisi oluşturma hatası: {}", e);
             #[cfg(not(feature = "std"))] eprintln!("LPDDR5 bellek yöneticisi oluşturma hatası: {}", e);
        }
    }

    #[cfg(feature = "std")]
    std::println!("\n--- Sıfır Boyutlu Tahsis Testi ---");
    #[cfg(not(feature = "std"))]
    println!("\n--- Sıfır Boyutlu Tahsis Testi ---");

    // Sıfır boyutlu bellek yöneticisi oluşturma (HATA örneği)
    match LpddrMemoryManager::new(lpddr4_config.clone(), 0) {
        Ok(_) => {
            #[cfg(feature = "std")] std::println!("Bu olmamalı! Sıfır boyutlu yönetici başarılı oldu.");
            #[cfg(not(feature = "std"))] println!("Bu olmamalı! Sıfır boyutlu yönetici başarılı oldu.");
        }
        Err(e) => {
            #[cfg(feature = "std")] std::eprintln!("Sıfır boyutlu yönetici oluşturma hatası (beklenen hata): {}", e); // Şimdi LpddrError döner
            #[cfg(not(feature = "std"))] eprintln!("Sıfır boyutlu yönetici oluşturma hatası (beklenen hata): {}", e); // Şimdi LpddrError döner
        }
    }

     #[cfg(feature = "std")]
    std::println!("\n--- Adres Aralığı Dışı ve Hizalama Hatası Testleri ---");
    #[cfg(not(feature = "std"))]
    println!("\n--- Adres Aralığı Dışı ve Hizalama Hatası Testleri ---");

    // Geçici bir yönetici oluştur (hata testleri için)
    if let Ok(mut temp_manager) = LpddrMemoryManager::new(lpddr4_config.clone(), 1024) {
        // Adres aralığı dışı okuma
        match temp_manager.read(2048) {
            Ok(_) => {
                 #[cfg(feature = "std")] std::println!("Bu olmamalı! Adres aralığı dışı okuma başarılı oldu.");
                 #[cfg(not(feature = "std"))] println!("Bu olmamalı! Adres aralığı dışı okuma başarılı oldu.");
            }
            Err(e) => {
                 #[cfg(feature = "std")] std::eprintln!("Adres aralığı dışı okuma hatası (beklenen): {}", e);
                 #[cfg(not(feature = "std"))] eprintln!("Adres aralığı dışı okuma hatası (beklenen): {}", e);
            }
        }
         // Adres aralığı dışı yazma
        match temp_manager.write(2048, 0xDEADBEEF) {
            Ok(_) => {
                 #[cfg(feature = "std")] std::println!("Bu olmamalı! Adres aralığı dışı yazma başarılı oldu.");
                 #[cfg(not(feature = "std"))] println!("Bu olmamalı! Adres aralığı dışı yazma başarılı oldu.");
            }
            Err(e) => {
                 #[cfg(feature = "std")] std::eprintln!("Adres aralığı dışı yazma hatası (beklenen): {}", e);
                 #[cfg(not(feature = "std"))] eprintln!("Adres aralığı dışı yazma hatası (beklenen): {}", e);
            }
        }

        // Hizalanmamış okuma
        match temp_manager.read(0x1001) { // 0x1000 + 1
             Ok(_) => {
                  #[cfg(feature = "std")] std::println!("Bu olmamalı! Hizalanmamış okuma başarılı oldu.");
                  #[cfg(not(feature = "std"))] println!("Bu olmamalı! Hizalanmamış okuma başarılı oldu.");
             }
             Err(e) => {
                  #[cfg(feature = "std")] std::eprintln!("Hizalanmamış okuma hatası (beklenen): {}", e);
                  #[cfg(not(feature = "std"))] eprintln!("Hizalanmamış okuma hatası (beklenen): {}", e);
             }
         }

        // Hizalanmamış yazma
         match temp_manager.write(0x1001, 0xDEADBEEF) {
             Ok(_) => {
                  #[cfg(feature = "std")] std::println!("Bu olmamalı! Hizalanmamış yazma başarılı oldu.");
                  #[cfg(not(feature = "std"))] println!("Bu olmamalı! Hizalanmamış yazma başarılı oldu.");
             }
             Err(e) => {
                  #[cfg(feature = "std")] std::eprintln!("Hizalanmamış yazma hatası (beklenen): {}", e);
                  #[cfg(not(feature = "std"))] eprintln!("Hizalanmamış yazma hatası (beklenen): {}", e);
             }
         }

    } // temp_manager burada Drop edilir

}

// Panic handler burada kalabilir veya Sahne64 crate'inden sağlanabilir.
// Eğer bu component crate'in kendi başına panik yakalaması gerekiyorsa kalmalı,
// aksi halde Sahne64'ün genel panic handler'ı yeterlidir.
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Gerçek bir sistemde burada hata bilgisi loglanmalı ve sistem durdurulmalı/yeniden başlatılmalı.
     eprintln!("PANIC: {}", _info); // eprintln! makrosu scope'ta olmalı
    loop { core::hint::spin_loop(); }
}
