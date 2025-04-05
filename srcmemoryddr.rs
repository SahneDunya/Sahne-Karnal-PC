#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

use crate::{memory, SahneError};
use core::ptr::{read_volatile, write_volatile};

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
    MemoryAllocationError(SahneError),
}

impl From<SahneError> for DDRError {
    fn from(error: SahneError) -> Self {
        DDRError::MemoryAllocationError(error)
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
    pub fn new(memory_size_mb: usize) -> Result<Self, DDRError> {
        let memory_size = memory_size_mb * 1024 * 1024; // MB'tan bayta dönüştür
        match memory::allocate(memory_size) {
            Ok(ptr) => Ok(DDRMemoryController {
                ddr_type: None,
                memory_size,
                memory_ptr: ptr,
                is_initialized: false,
            }),
            Err(e) => Err(DDRError::from(e)),
        }
    }

    /// DDR Bellek Yöneticisini belirli bir DDR türü için başlatır.
    pub fn init(&mut self, ddr_type: DDRType) -> Result<(), DDRError> {
        match ddr_type {
            DDRType::DDR3 => {
                println!("DDR3 Bellek Yöneticisi başlatılıyor...");
                // DDR3'e özgü başlatma işlemleri burada yapılabilir.
                // Örneğin, zamanlama parametrelerini ayarlamak gibi.
            }
            DDRType::DDR4 => {
                println!("DDR4 Bellek Yöneticisi başlatılıyor...");
                // DDR4'e özgü başlatma işlemleri burada yapılabilir.
            }
            DDRType::DDR5 => {
                println!("DDR5 Bellek Yöneticisi başlatılıyor...");
                // DDR5'e özgü başlatma işlemleri burada yapılabilir.
            }
        }
        self.ddr_type = Some(ddr_type);
        self.is_initialized = true;
        println!("{:?} Bellek Yöneticisi başarıyla başlatıldı.", ddr_type);
        Ok(())
    }

    /// Bellek yöneticisi kapatıldığında belleği serbest bırakır.
    fn deinit(&mut self) -> Result<(), DDRError> {
        if self.is_initialized {
            match self.ddr_type {
                Some(ddr_type) => println!("{:?} Bellek Yöneticisi kapatılıyor...", ddr_type),
                None => println!("Bellek Yöneticisi kapatılıyor..."),
            }
            // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
            let result = unsafe { memory::free(self.memory_ptr, self.memory_size) };
            self.is_initialized = false;
            self.memory_ptr = core::ptr::null_mut();
            self.ddr_type = None;
            result.map_err(DDRError::from)
        } else {
            Ok(())
        }
    }
}

impl Drop for DDRMemoryController {
    fn drop(&mut self) {
        let _ = self.deinit(); // Hata durumunda bile drop çağrılmalı
    }
}

impl DDRMemoryController {
    /// Belirtilen adresten bayt okur.
    pub fn read_byte(&self, address: usize) -> Result<u8, DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        if address >= self.memory_size {
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        unsafe { Ok(read_volatile(self.memory_ptr.add(address))) }
    }

    /// Belirtilen adrese bir bayt yazar.
    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        if address >= self.memory_size {
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        unsafe { write_volatile(self.memory_ptr.add(address), value) };
        Ok(())
    }

    /// Belirtilen adresten 32-bit (4 bayt) okur.
    pub fn read_u32(&self, address: usize) -> Result<u32, DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        if address + 4 > self.memory_size {
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
        unsafe {
            let ptr = self.memory_ptr.add(address) as *const u32;
            Ok(read_volatile(ptr))
        }
    }

    /// Belirtilen adrese 32-bit (4 bayt) yazar.
    pub fn write_u32(&mut self, address: usize, value: u32) -> Result<(), DDRError> {
        if !self.is_initialized {
            return Err(DDRError::NotInitialized);
        }
        if address + 4 > self.memory_size {
            return Err(DDRError::AddressOutOfRange(address));
        }
        // Güvenli olmayan (unsafe) bir işlem çünkü ham pointer ile çalışıyoruz.
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
}

// --- Örnek Kullanım ---
#[cfg(feature = "std")]
fn main() {
    println!("Sahne64 DDR Bellek Yöneticisi Örneği");

    // 16MB boyutunda bir DDR Bellek Yöneticisi oluştur
    match DDRMemoryController::new(16) {
        Ok(mut ddr_controller) => {
            // DDR4 olarak başlat
            match ddr_controller.init(DDRType::DDR4) {
                Ok(_) => println!("DDR Bellek Yöneticisi başarıyla başlatıldı."),
                Err(e) => {
                    eprintln!("DDR Bellek Yöneticisi başlatma hatası: {:?}", e);
                    return;
                }
            }

            // Belleğe veri yazma (32-bit)
            let address_to_write = 0x1000; // Örnek adres
            let data_to_write: u32 = 0x12345678;
            match ddr_controller.write_u32(address_to_write, data_to_write) {
                Ok(_) => println!("0x{:X} adresine 0x{:X} değeri yazıldı.", address_to_write, data_to_write),
                Err(e) => eprintln!("Yazma hatası: {:?}", e),
            }

            // Bellekten veri okuma (32-bit)
            let address_to_read = 0x1000; // Aynı adres
            match ddr_controller.read_u32(address_to_read) {
                Ok(read_data) => println!("0x{:X} adresinden okunan değer: 0x{:X}", address_to_read, read_data),
                Err(e) => eprintln!("Okuma hatası: {:?}", e),
            }

            // Bellek türünü ve boyutunu al
            if let Some(ddr_type) = ddr_controller.get_ddr_type() {
                println!("Kullanılan DDR Türü: {:?}", ddr_type);
            }
            println!("Toplam Bellek Boyutu: {} bayt", ddr_controller.get_memory_size());

            // --- Hata senaryoları ---
            println!("\n--- Hata Senaryoları ---");

            // Adres aralığı dışında okuma yapmaya çalışma
            let invalid_address = ddr_controller.get_memory_size(); // Bellek boyutunun dışında bir adres
            match ddr_controller.read_byte(invalid_address) {
                Ok(_) => println!("Bu olmamalı! Başarılı okuma beklenmiyordu."),
                Err(e) => eprintln!("Adres aralığı dışı okuma hatası alındı: {:?}", e), // Bu bekleniyor
            }

            // Bellek yöneticisi scope dışına çıktığında `drop` metodu çağrılacak ve bellek serbest bırakılacaktır.
        }
        Err(e) => {
            eprintln!("DDR Bellek Yöneticisi oluşturma hatası: {:?}", e);
        }
    }
}

#[cfg(not(feature = "std"))]
mod print {
    use core::fmt;
    use core::fmt::Write;

    struct Stdout;

    impl fmt::Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // Burada gerçek çıktı mekanizmasına (örneğin, bir UART sürücüsüne) erişim olmalı.
            // Bu örnekte, çıktı kaybolacaktır çünkü gerçek bir çıktı yok.
            // Gerçek bir işletim sisteminde, bu kısım donanıma özel olacaktır.
            Ok(())
        }
    }

    #[macro_export]
    macro_rules! print {
        ($($arg:tt)*) => ({
            let mut stdout = $crate::print::Stdout;
            core::fmt::write(&mut stdout, core::format_args!($($arg)*)).unwrap();
        });
    }

    #[macro_export]
    macro_rules! println {
        () => ($crate::print!("\n"));
        ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
    }
}