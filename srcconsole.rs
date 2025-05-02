#![no_std]
#[allow(dead_code)] // Keep if needed

use core::fmt;
use core::fmt::Write;
// Use Sahne64 resource, kernel (for control), SahneError, and Handle
use sahne64::{resource, kernel, SahneError, Handle}; // <-- Changed imports
// No need for crate::fs as it's replaced by sahne64::resource
// No direct need for crate::kernel here, resource::control wraps ioctl-like calls
// Need access to the custom print/eprintln macros from Sahne64's stdio_impl in no_std
// Assuming these are made available.

/// USB CDC-ACM seri portunu Sahne64 kaynak API'si üzerinden yöneten yapı.
pub struct UsbSerial {
    console_handle: Option<Handle>, // <-- Değişiklik 1: Dosya tanımlayıcısı yerine Handle
     usb_base_address: usize, // USB denetleyici base adresi - Artık doğrudan erişilmiyor (yorum satırı kalsın)
     tx_endpoint_address: u8, // İletim endpoint adresi - Artık doğrudan yönetilmiyor (yorum satırı kalsın)
     rx_endpoint_address: u8, // Alım endpoint adresi - Artık doğrudan yönetilmiyor (yorum satırı kalsın)
    // ... diğer gerekli donanım kaynakları ve konfigürasyon bilgileri ...
}

impl UsbSerial {
    /// Yeni bir `UsbSerial` örneği oluşturur (başlatılmamış).
    pub const fn new() -> Self {
        Self {
            console_handle: None, // Handle başlangıçta yok
             usb_base_address: 0, // Artık kullanılmıyor
             tx_endpoint_address: 0, // Artık kullanılmıyor
             rx_endpoint_address: 0, // Artık kullanılmıyor
        }
    }

    /// USB üzerinden veri gönderme fonksiyonu (Sahne64 resource::write sistem çağrısını kullanır).
    ///
    /// # Parametreler
    /// * `data`: Gönderilecek veri dilimi.
    ///
    /// # Not
    /// Başarısızlık durumunda hata mesajını loglar.
    fn usb_send(&self, data: &[u8]) {
        // Değişiklik 2: fd yerine console_handle kullan
        if let Some(handle) = self.console_handle.filter(|h| h.is_valid()) { // Handle geçerli mi kontrolü eklendi
            // Değişiklik 2: fs::write yerine resource::write kullan
            match resource::write(handle, data) {
                Ok(bytes_written) => {
                    if bytes_written != data.len() {
                        // Çıktı makroları için cfg ayarı
                        #[cfg(feature = "std")] std::println!("Uyarı: Tüm veri yazılamadı. Yazılan: {}, Toplam: {}", bytes_written, data.len());
                        #[cfg(not(feature = "std"))] println!("Uyarı: Tüm veri yazılamadı. Yazılan: {}, Toplam: {}", bytes_written, data.len());
                    }
                }
                Err(e) => {
                    // Çıktı makroları için cfg ayarı
                     #[cfg(feature = "std")] std::eprintln!("USB yazma hatası: {:?}", e);
                     #[cfg(not(feature = "std"))] eprintln!("USB yazma hatası: {:?}", e);
                }
            }
        } else {
            // Çıktı makroları için cfg ayarı
             #[cfg(feature = "std")] std::eprintln!("Hata: USB seri portu henüz başlatılmadı veya Handle geçersiz.");
             #[cfg(not(feature = "std"))] eprintln!("Hata: USB seri portu henüz başlatılmadı veya Handle geçersiz.");
        }
    }

    /// USB Denetleyici ve CDC-ACM Sürücüsü Başlatma (Sahne64 resource::acquire sistem çağrısını kullanır).
    ///
    /// # Parametreler
    /// * `device_resource_id`: USB CDC-ACM aygıtını temsil eden Sahne64 kaynak ID'si.
    ///
    /// # Not
    /// Başarısızlık durumunda hata mesajlarını loglar.
    pub fn init_usb_hardware(&mut self, device_resource_id: resource::ResourceId) { // <-- Add resource ID parameter
        // Çıktı makroları için cfg ayarı
         #[cfg(feature = "std")] std::println!("USB Konsolu '{}' Açılıyor...", device_resource_id);
         #[cfg(not(feature = "std"))] println!("USB Konsolu '{}' Açılıyor...", device_resource_id);


        // Değişiklik 3: fs::open yerine resource::acquire kullan
        // resource::acquire Result<Handle, SahneError> döner
        match resource::acquire(device_resource_id, resource::MODE_READ | resource::MODE_WRITE) {
            Ok(handle) => {
                self.console_handle = Some(handle);
                 // Çıktı makroları için cfg ayarı
                #[cfg(feature = "std")] std::println!("USB Konsolu Açıldı! Handle: {:?}", handle);
                #[cfg(not(feature = "std"))] println!("USB Konsolu Açıldı! Handle: {:?}", handle);


                // Değişiklik 4: Baud hızı gibi ayarları yapılandırmak için resource::control kullanılabilir
                // Bu kısım simüle ediliyor, gerçek istek kodları GPU sürücüsü gibi cihaza özgüdür.
                // Varsayımsal bir CONTROL_SET_BAUD_RATE isteği ve 115200 argümanı kullanalım.
                const CONTROL_SET_BAUD_RATE: u64 = 1; // Örnek kontrol isteği kodu
                let baud_rate: u64 = 115200; // Örnek baud hızı

                match resource::control(handle, CONTROL_SET_BAUD_RATE, baud_rate) {
                     Ok(result_val) => {
                         // Çıktı makroları için cfg ayarı
                         #[cfg(feature = "std")] std::println!("USB ayarları yapılandırıldı ({} baud). Kontrol sonucu: {}", baud_rate, result_val);
                         #[cfg(not(feature = "std"))] println!("USB ayarları yapılandırıldı ({} baud). Kontrol sonucu: {}", baud_rate, result_val);
                     }
                     Err(e) => {
                         // Çıktı makroları için cfg ayarı
                         #[cfg(feature = "std")] std::eprintln!("USB ayarları yapılandırılırken hata: {:?}", e);
                         #[cfg(not(feature = "std"))] eprintln!("USB ayarları yapılandırılırken hata: {:?}", e);
                     }
                }
            }
            Err(e) => {
                 // Çıktı makroları için cfg ayarı
                 #[cfg(feature = "std")] std::eprintln!("USB Konsolu '{}' açılırken hata: {:?}", device_resource_id, e);
                 #[cfg(not(feature = "std"))] eprintln!("USB Konsolu '{}' açılırken hata: {:?}", device_resource_id, e);
                 // Hata detayını ayrıca yazdırmaya gerek yok, {:?} SahneError'ı zaten detaylı gösterir.
                  #[cfg(feature = "std")] std::println!("Hata Detayı: {:?}", e); // Kaldırıldı
                  #[cfg(not(feature = "std"))] println!("Hata Detayı: {:?}", e); // Kaldırıldı
            }
        }
    }

    // Kaynak Handle'ını serbest bırakmak için bir deinitialization fonksiyonu (isteğe bağlı ancak önerilir)
    // Singleton statik durumunda Drop tam olarak kontrol edilemeyebilir, bu yüzden explicit deinit faydalı olabilir.
    pub fn deinit_usb_hardware(&mut self) -> Result<(), SahneError> {
        if let Some(handle) = self.console_handle.take() { // Handle'ı al ve None yap
            if handle.is_valid() {
                // Çıktı makroları için cfg ayarı
                #[cfg(feature = "std")] std::println!("USB Konsolu Kaynak Handle'ı serbest bırakılıyor: {:?}", handle);
                #[cfg(not(feature = "std"))] println!("USB Konsolu Kaynak Handle'ı serbest bırakılıyor: {:?}", handle);

                // resource::release Result<(), SahneError> döner
                resource::release(handle)
            } else {
                // Çıktı makroları için cfg ayarı
                #[cfg(feature = "std")] std::println!("USB Konsolu Handle zaten geçersizdi.");
                #[cfg(not(feature = "std"))] println!("USB Konsolu Handle zaten geçersizdi.");
                Ok(())
            }
        } else {
            // Çıktı makroları için cfg ayarı
            #[cfg(feature = "std")] std::println!("USB Konsolu Handle zaten None idi.");
            #[cfg(not(feature = "std"))] println!("USB Konsolu Handle zaten None idi.");
            Ok(()) // Zaten serbest bırakılmış gibi kabul et
        }
    }
}

// Console yapısı (değişiklik yok, sadece başvurduğu UsbSerial değişti)
/// Konsol çıktı arayüzünü sağlar.
/// İçsel olarak bir `UsbSerial` örneğini kullanarak çıktı verir.
pub struct Console {
    // Statik mutable referans yerine belki UnsafeCell içinde UsbSerial tutulabilir?
    // Ancak global static singleton deseni için &'static mut yaygın bir no_std hilesidir.
    usb_serial: &'static mut UsbSerial, // Mutable referans
}

impl Console {
    /// Yeni bir `Console` örneği oluşturur.
    /// # Güvenlik
    /// Sağlanan `usb_serial` referansının 'static ömürlü ve tekil mutable erişime sahip olduğundan emin olunmalıdır.
    pub const fn new(usb_serial: &'static mut UsbSerial) -> Self {
        Self { usb_serial }
    }
}

// Formatlı çıktı için Write trait uygulaması (değişiklik yok, altındaki usb_send değişti)
impl fmt::Write for Console {
    /// String dilimini konsola yazar.
    /// Dahili olarak `usb_serial.usb_send` fonksiyonunu kullanır.
    fn write_str(&mut self, s: &str) -> fmt::Result {
         // console_print fonksiyonunu çağırmak yerine doğrudan usb_serial kullanabiliriz
          console_print(self, s); // Kaldırıldı
         self.usb_serial.usb_send(s.as_bytes());
         Ok(()) // usb_send hatalarını loglar, Write trait'i Result dönmez
    }
}

// Makrolar (Değişiklik yok, CONSOLE global static'ini kullanıyorlar)
// Bu makroların kullanıldığı crate'te (bu dosya gibi) Sahne64 macro_use ayarının yapıldığı varsayılır.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
         core::fmt::Write trait'ine gerek yok, write! macro'su otomatik getirir
         use core::fmt::Write; // Kaldırıldı
        // crate::console::CONSOLE global static'ine yaz
        let _ = write!(crate::console::CONSOLE, $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => (print!("\r\n"));
    ($($arg:tt)*) => ({
          core::fmt::Write trait'ine gerek yok
         use core::fmt::Write; // Kaldırıldı
        // crate::console::CONSOLE global static'ine yaz
        let _ = write!(crate::console::CONSOLE, $($arg)*);
        print!("\r\n"); // Satır sonu ekle
    })
}

// Örnek kullanım ve başlatma
// Global static mutable örnekler. Sahne64 çalışma zamanı başlangıcında
// yalnızca bir kez başlatılmaları gerekmektedir.
// Güvenli olmayan (unsafe) kullanıma dikkat edilmelidir.
// console modülü genellikle bir kez init fonksiyonu ile başlatılır ve
// makrolar global statik üzerinden kullanılır.
pub static mut USB_SERIAL: UsbSerial = UsbSerial::new(); // Mutable static USB seri port örneği
pub static mut CONSOLE: Console = Console::new(unsafe { &mut USB_SERIAL }); // Mutable static Konsol örneği

/// Konsol sistemini başlatır.
/// USB seri donanımını Sahne64 kaynak API'sini kullanarak açar.
/// Bu fonksiyon sistem başlangıcında bir kez çağrılmalıdır.
pub fn init() {
    // Çıktı makroları henüz tam çalışmayabilir, ancak init_usb_hardware içinde loglama var.
    // Burada Sahne64'ün kendi çekirdek başlangıç çıktı mekanizması varsa kullanılabilir.
    // Şimdilik çıktı makrolarını kullanıyoruz, resource::acquire'dan sonra çalışacaktır.

     #[cfg(feature = "std")] std::println!("Sahne64 USB Konsolu Başlatılıyor...");
     #[cfg(not(feature = "std"))] println!("Sahne64 USB Konsolu Başlatılıyor...");


    unsafe {
        // Değişiklik 3: init_usb_hardware fonksiyonu artık resource ID parametresi alıyor
        // USB CDC-ACM aygıtının Sahne64 kaynak ID'si (örnek)
        let usb_cdc_resource_id = "sahne://device/ttyUSB0";
        USB_SERIAL.init_usb_hardware(usb_cdc_resource_id); // Donanım başlatma fonksiyonunu çağır
    }

    // init_usb_hardware içindeki çıktı, handle edindikten sonra görünür olacaktır.
    // Buradaki çıktı, handle edindiyse görünür, edinemediyse init_usb_hardware içindeki hata görünür.
     #[cfg(feature = "std")] std::println!("Sahne64 USB Konsolu Başlatma İsteği Gönderildi.");
     #[cfg(not(feature = "std"))] println!("Sahne64 USB Konsolu Başlatma İsteği Gönderildi.");

     // Not: Başlatmanın gerçekten başarılı olup olmadığını kontrol etmek için
     // init_usb_hardware fonksiyonunun bir Result döndürmesi daha iyi olabilir.
     // Şu an void döndürüyor ve hatayı içeride logluyor.
}

// Konsol sistemini kapatır (isteğe bağlı)
pub fn deinit() -> Result<(), SahneError> {
    #[cfg(feature = "std")] std::println!("Sahne64 USB Konsolu Kapatılıyor...");
    #[cfg(not(feature = "std"))] println!("Sahne64 USB Konsolu Kapatılıyor...");

    unsafe {
        // Deinit fonksiyonunu çağırarak Handle'ı serbest bırak
        USB_SERIAL.deinit_usb_hardware()
    }
    // Result döndürülür, çağıran hata olup olmadığını kontrol edebilir.
}
