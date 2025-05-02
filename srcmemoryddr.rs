#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Use the Sahne64 crate's modules/types
// Assuming this file is part of a separate crate that depends on 'sahne64'
use sahne64::{memory, SahneError}; // <-- Changed import
use core::ptr::{read_volatile, write_volatile};

// Need access to the custom print/eprint macros from Sahne64's stdio_impl in no_std
// Assuming these are made available in the build setup for this component crate.
 #[cfg(not(feature = "std"))] #[macro_use] extern crate sahne64; // Or specific imports if macros exported differently.


/// DDR Bellek Türlerini tanımlar. Şu anda DDR3, DDR4 ve DDR5 desteklenmektedir.
#[derive(Debug, Copy, Clone)]
pub enum DDRType {
    DDR3,
    DDR4,
    DDR5,
}

/// DDR Bellek Yöneticisi işlemleri sırasında oluşabilecek hataları tanımlar.
#[derive(Debug)]
pub enum DDRError {
    InitializationError(String),
    ReadError(String),
    WriteError(String),
    UnsupportedDDRType(DDRType),
    AddressOutOfRange(usize),
    /// Bellek kontrolcüsü henüz başlatılmadı.
    NotInitialized,
    MemoryOperationError(SahneError), // Renamed from MemoryAllocationError for broader use
}

impl From<SahneError> for DDRError {
    fn from(error: SahneError) -> Self {
        DDRError::MemoryOperationError(error) // Uses the renamed variant
    }
}

/// DDR Bellek Yöneticisi yapısı. Bu yapı, belirli bir DDR türünü yönetmek için kullanılır.
///
/// **Dikkat:** Bu örnek kod, DDR bellek kontrolcüsünün basitleştirilmiş bir modelidir.
/// Gerçek bir DDR kontrolcüsü çok daha karmaşık donanım etkileşimleri ve zamanlama
/// gereksinimleri içerir. Bu kod sadece kavramsal bir örnek olarak sunulmuştur ve
/// gerçek donanım üzerinde çalışması amaçlanmamıştır. Bu güncellenmiş versiyon,
/// Sahne64'ün bellek yönetimi sistem çağrılarını kullanmayı amaçlamaktadır.
pub struct DDRMemoryController {
    ddr_type: Option<DDRType>,
    memory_size: usize, // Bellek boyutu (bayt cinsinden)
    memory_ptr: *mut u8, // Belleği temsil eden bir pointer
    is_initialized: bool,
}

impl DDRMemoryController {
    /// Yeni bir DDR Bellek Yöneticisi örneği oluşturur ve belleği ayırır.
    ///
    /// Başlangıçta bellek yöneticisi başlatılmamıştır. `init` metodu ile başlatılmalıdır.
    /// Bellek ayırma sırasında bir Sahne64 API hatası oluşursa `DDRError::MemoryOperationError` döndürülür.
    pub fn new(memory_size_mb: usize) -> Result<Self, DDRError> {
        let memory_size = memory_size_mb * 1024 * 1024; // MB'tan bayta dönüştür
        // memory::allocate yeni Sahne64 API'sine göre Result<*mut u8, SahneError> dönüyor
        match memory::allocate(memory_size) { // <-- Correct function call based on new API
            Ok(ptr) => Ok(DDRMemoryController {
                ddr_type: None,
                memory_size,
                memory_ptr: ptr,
                is_initialized: false,
            }),
            Err(e) => Err(DDRError::from(e)), // SahneError, From trait ile DDRError'a çevriliyor
        }
    }

    /// DDR Bellek Yöneticisini belirli bir DDR türü için başlatır.
    /// Gerçek donanım başlatma işlemleri burada simüle edilir.
    pub fn init(&mut self, ddr_type: DDRType) -> Result<(), DDRError> {
        if self.is_initialized {
            // Zaten başlatılmışsa hata dönebilir veya yeniden başlatmaya izin verilebilir.
            // Bu örnekte basitlik adına yeniden başlatmaya izin verelim ama gerçekte daha dikkatli olmak gerekir.
              return Err(DDRError::InitializationError("Controller already initialized".into()));
        }

        // Simüle edilmiş başlatma mesajları
        #[cfg(feature = "std")]
        std::println!("{:?} Bellek Yöneticisi başlatılıyor...", ddr_type);
        #[cfg(not(feature = "std"))]
        println!("{:?} Bellek Yöneticisi başlatılıyor...", ddr_type);


        match ddr_type {
            DDRType::DDR3 => {
                // DDR3'e özgü başlatma işlemleri burada yapılabilir.
                // Örneğin, zamanlama parametrelerini ayarlamak gibi.
            }
            DDRType::DDR4 => {
                // DDR4'e özgü başlatma işlemleri burada yapılabilir.
            }
            DDRType::DDR5 => {
                // DDR5'e özgü başlatma işlemleri burada yapılabilir.
            }
        }
        self.ddr_type = Some(ddr_type);
        self.is_initialized = true;

        // Simüle edilmiş başarı mesajı
         #[cfg(feature = "std")]
        std::println!("{:?} Bellek Yöneticisi başarıyla başlatıldı.", ddr_type);
         #[cfg(not(feature = "std"))]
        println!("{:?} Bellek Yö yöneticisi başarıyla başlatıldı.", ddr_type);

        Ok(())
    }

    /// Bellek yöneticisi kapatıldığında belleği serbest bırakır.
    /// Dahili kullanımdır, genellikle Drop tarafından çağrılır.
    fn deinit(&mut self) -> Result<(), DDRError> {
        if self.is_initialized {
             #[cfg(feature = "std")]
            match self.ddr_type {
                Some(ddr_type) => std::println!("{:?} Bellek Yöneticisi kapatılıyor...", ddr_type),
                None => std::println!("Bellek Yöneticisi kapatılıyor..."), // init çağrılmamışsa
            }
             #[cfg(not(feature = "std"))]
             match self.ddr_type {
                Some(ddr_type) => println!("{:?} Bellek Yöneticisi kapatılıyor...", ddr_type),
                None => println!("Bellek Yöneticisi kapatılıyor..."), // init çağrılmamışsa
            }


            // Belleği Sahne64 API'sini kullanarak serbest bırak
            // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
            // <-- Değişiklik 2: memory::free yerine memory::release çağırıldı
            let result = memory::release(self.memory_ptr, self.memory_size);

            self.is_initialized = false;
            self.memory_ptr = core::ptr::null_mut(); // Serbest bırakılan pointer'ı null yap
            self.ddr_type = None;

            // Sahne64 API'sinden dönen Result<_, SahneError>'ı Result<_, DDRError>'a çevir
            result.map_err(DDRError::from)
        } else {
            // Zaten başlatılmamışsa bir şey yapmaya gerek yok
            Ok(())
        }
    }
}

impl Drop for DDRMemoryController {
    /// `DDRMemoryController` örneği düşürüldüğünde belleği serbest bırakır.
    /// Bu, Drop trait'inin gerektirdiği fonksiyondur. deinit'i çağırır ve
    /// deinit'teki olası hataları yoksayar (Drop içinde hata fırlatmak Rust prensiplerine aykırıdır).
    fn drop(&mut self) {
        // deinit'ten dönen Result'ı kontrol etmiyoruz, çünkü Drop içinde
        // paniklememeye çalışmalıyız. Ancak loglama yapabiliriz.
         if let Err(e) = self.deinit() {
             // Değişiklik 3: eprintln! kullanımı için std/no_std kontrolü
             #[cfg(not(feature = "std"))]
             {
                  // Sahne64 custom eprintln! macro'sunun scope'ta olduğunu varsayalım.
                  eprintln!("UYARI: DDR Bellek Yöneticisi serbest bırakılırken hata oluştu: {:?}", e);
             }
             #[cfg(feature = "std")]
             {
                 std::eprintln!("UYARI: DDR Bellek Yöneticisi serbest bırakılırken hata oluştu: {:?}", e);
             }
         }
    }
}

impl DDRMemoryController {
    /// Belirtilen adresten bayt okur.
    /// Okuma işlemi volatile olarak yapılır.
    pub fn read_byte(&self, address: usize) -> Result<u8, DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        if address >= self.memory_size {
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        // Pointer'ın geçerli ve boyut içinde olduğunu kontrol ettik.
        unsafe { Ok(read_volatile(self.memory_ptr.add(address))) }
    }

    /// Belirtilen adrese bir bayt yazar.
    /// Yazma işlemi volatile olarak yapılır.
    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        if address >= self.memory_size {
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        // Pointer'ın geçerli ve boyut içinde olduğunu kontrol ettik.
        unsafe { write_volatile(self.memory_ptr.add(address), value) };
        Ok(())
    }

    /// Belirtilen adresten 32-bit (4 bayt) okur.
    /// Okuma işlemi volatile olarak yapılır. Endianness dikkate alınmayabilir,
    /// gerçek donanım sürücüsünde endianness yönetimi önemlidir.
    pub fn read_u32(&self, address: usize) -> Result<u32, DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        // 32-bit okuma için 4 bayt alan gerekir.
        if address.checked_add(4).unwrap_or(usize::MAX) > self.memory_size { // Overflow check eklendi
             return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        // Pointer'ın geçerli ve boyut içinde olduğunu kontrol ettik.
        unsafe {
            let ptr = self.memory_ptr.add(address) as *const u32;
            Ok(read_volatile(ptr))
        }
    }

    /// Belirtilen adrese 32-bit (4 bayt) yazar.
    /// Yazma işlemi volatile olarak yapılır. Endianness dikkate alınmayabilir.
    pub fn write_u32(&mut self, address: usize, value: u32) -> Result<(), DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
         // 32-bit yazma için 4 bayt alan gerekir.
        if address.checked_add(4).unwrap_or(usize::MAX) > self.memory_size { // Overflow check eklendi
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        // Pointer'ın geçerli ve boyut içinde olduğunu kontrol ettik.
        unsafe {
            let ptr = self.memory_ptr.add(address) as *mut u32;
            write_volatile(ptr, value);
        }
        Ok(())
    }

    /// Bellek yöneticisinin şu anda hangi DDR türünü kullandığını döndürür.
    pub fn get_ddr_type(&self) -> Option<DDRType> {
        self.ddr_type
    }

    /// Bellek yöneticisinin yönettiği toplam bellek boyutunu bayt cinsinden döndürür.
    pub fn get_memory_size(&self) -> usize {
        self.memory_size
    }

    /// Ayrılan belleğin ham pointer'ını döndürür.
    /// DİKKAT: Bu pointer'ı kullanırken bellek güvenliğini sağlamak çağıranın sorumluluğundadır.
    pub fn as_ptr(&self) -> *mut u8 {
         self.memory_ptr
    }
}

// --- Örnek Kullanım ---
#[cfg(feature = "std")]
fn main() {
    // Bu örnek std ortamında çalışacak şekilde yapılandırılmıştır.
    // no_std ortamında Sahne64'ün çıktı makroları veya özel bir çıktı mekanizması gerektirir.

    #[cfg(feature = "std")]
    std::println!("Sahne64 DDR Bellek Yöneticisi Örneği (std)");
    #[cfg(not(feature = "std"))]
    println!("Sahne64 DDR Bellek Yöneticisi Örneği (no_std)");


    // 16MB boyutunda bir DDR Bellek Yöneticisi oluştur
    match DDRMemoryController::new(16) {
        Ok(mut ddr_controller) => {
            // DDR4 olarak başlat
            match ddr_controller.init(DDRType::DDR4) {
                Ok(_) => {
                     #[cfg(feature = "std")]
                    std::println!("DDR Bellek Yöneticisi başarıyla başlatıldı.");
                     #[cfg(not(feature = "std"))]
                     println!("DDR Bellek Yöneticisi başarıyla başlatıldı.");
                }
                Err(e) => {
                    #[cfg(feature = "std")]
                    std::eprintln!("DDR Bellek Yöneticisi başlatma hatası: {:?}", e);
                     #[cfg(not(feature = "std"))]
                     eprintln!("DDR Bellek Yöneticisi başlatma hatası: {:?}", e);
                    return;
                }
            }

            // Belleğe veri yazma (32-bit)
            let address_to_write = 0x1000; // Örnek adres
            let data_to_write: u32 = 0x12345678;
            match ddr_controller.write_u32(address_to_write, data_to_write) {
                Ok(_) => {
                    #[cfg(feature = "std")]
                    std::println!("0x{:X} adresine 0x{:X} değeri yazıldı.", address_to_write, data_to_write);
                     #[cfg(not(feature = "std"))]
                     println!("0x{:X} adresine 0x{:X} değeri yazıldı.", address_to_write, data_to_write);
                }
                Err(e) => {
                     #[cfg(feature = "std")]
                    std::eprintln!("Yazma hatası: {:?}", e);
                     #[cfg(not(feature = "std"))]
                     eprintln!("Yazma hatası: {:?}", e);
                }
            }

            // Bellekten veri okuma (32-bit)
            let address_to_read = 0x1000; // Aynı adres
            match ddr_controller.read_u32(address_to_read) {
                Ok(read_data) => {
                    #[cfg(feature = "std")]
                    std::println!("0x{:X} adresinden okunan değer: 0x{:X}", address_to_read, read_data);
                     #[cfg(not(feature = "std"))]
                     println!("0x{:X} adresinden okunan değer: 0x{:X}", address_to_read, read_data);
                }
                Err(e) => {
                     #[cfg(feature = "std")]
                    std::eprintln!("Okuma hatası: {:?}", e);
                     #[cfg(not(feature = "std"))]
                     eprintln!("Okuma hatası: {:?}", e);
                }
            }

            // Bellek türünü ve boyutunu al
            if let Some(ddr_type) = ddr_controller.get_ddr_type() {
                 #[cfg(feature = "std")]
                std::println!("Kullanılan DDR Türü: {:?}", ddr_type);
                 #[cfg(not(feature = "std"))]
                println!("Kullanılan DDR Türü: {:?}", ddr_type);
            }
             #[cfg(feature = "std")]
            std::println!("Toplam Bellek Boyutu: {} bayt", ddr_controller.get_memory_size());
             #[cfg(not(feature = "std"))]
            println!("Toplam Bellek Boyutu: {} bayt", ddr_controller.get_memory_size());

            // --- Hata senaryoları ---
             #[cfg(feature = "std")]
            std::println!("\n--- Hata Senaryoları ---");
             #[cfg(not(feature = "std"))]
            println!("\n--- Hata Senaryoları ---");


            // Adres aralığı dışında okuma yapmaya çalışma
            let invalid_address = ddr_controller.get_memory_size(); // Bellek boyutunun dışında bir adres
            match ddr_controller.read_byte(invalid_address) {
                Ok(_) => {
                     #[cfg(feature = "std")]
                    std::println!("Bu olmamalı! Başarılı okuma beklenmiyordu.");
                     #[cfg(not(feature = "std"))]
                     println!("Bu olmamalı! Başarılı okuma beklenmiyordu.");
                }
                Err(e) => {
                     #[cfg(feature = "std")]
                    std::eprintln!("Adres aralığı dışı okuma hatası alındı: {:?}", e); // Bu bekleniyor
                     #[cfg(not(feature = "std"))]
                     eprintln!("Adres aralığı dışı okuma hatası alındı: {:?}", e); // Bu bekleniyor
                }
            }
             // Adres aralığı dışında 32-bit okuma yapmaya çalışma (limit + 4)
            let invalid_address_u32 = ddr_controller.get_memory_size().checked_sub(3).unwrap_or(0); // Sondan 3 byte öncesi
            match ddr_controller.read_u32(invalid_address_u32) {
                Ok(_) => {
                     #[cfg(feature = "std")]
                    std::println!("Bu olmamalı! Başarılı 32-bit okuma beklenmiyordu.");
                     #[cfg(not(feature = "std"))]
                     println!("Bu olmamalı! Başarılı 32-bit okuma beklenmiyordu.");
                }
                Err(e) => {
                     #[cfg(feature = "std")]
                    std::eprintln!("Adres aralığı dışı 32-bit okuma hatası alındı: {:?}", e); // Bu bekleniyor
                     #[cfg(not(feature = "std"))]
                     eprintln!("Adres aralığı dışı 32-bit okuma hatası alındı: {:?}", e); // Bu bekleniyor
                }
            }


            // Bellek yöneticisi scope dışına çıktığında `drop` metodu çağrılacak ve bellek serbest bırakılacaktır.
        }
        Err(e) => {
             #[cfg(feature = "std")]
            std::eprintln!("DDR Bellek Yöneticisi oluşturma hatası: {:?}", e);
             #[cfg(not(feature = "std"))]
            eprintln!("DDR Bellek Yöneticisi oluşturma hatası: {:?}", e);
        }
    }
}
