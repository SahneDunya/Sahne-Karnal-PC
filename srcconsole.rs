#![no_std]
use core::fmt;
use core::fmt::Write;
use crate::{fs, kernel}; // Sahne64 modüllerine erişim

// USB CDC-ACM için gerekli yapılar (Sahne64 sistem çağrılarını kullanacak şekilde güncellendi)
pub struct UsbSerial {
    fd: Option<u64>, // Dosya tanımlayıcısı (file descriptor)
    // usb_base_address: usize, // USB denetleyici base adresi - Artık doğrudan erişilmiyor
    // tx_endpoint_address: u8, // İletim endpoint adresi - Artık doğrudan yönetilmiyor
    // rx_endpoint_address: u8, // Alım endpoint adresi - Artık doğrudan yönetilmiyor
    // ... diğer gerekli donanım kaynakları ve konfigürasyon bilgileri ...
}

impl UsbSerial {
    pub const fn new() -> Self {
        Self {
            fd: None,
            // usb_base_address: 0, // Artık kullanılmıyor
            // tx_endpoint_address: 0, // Artık kullanılmıyor
            // rx_endpoint_address: 0, // Artık kullanılmıyor
        }
    }

    // USB üzerinden veri gönderme fonksiyonu (Sahne64 sistem çağrısını kullanacak şekilde güncellendi)
    fn usb_send(&self, data: &[u8]) {
        if let Some(fd) = self.fd {
            match fs::write(fd, data) {
                Ok(bytes_written) => {
                    if bytes_written != data.len() {
                        println!("Uyarı: Tüm veri yazılamadı. Yazılan: {}, Toplam: {}", bytes_written, data.len());
                    }
                }
                Err(e) => {
                    println!("USB yazma hatası: {:?}", e);
                }
            }
        } else {
            println!("Hata: USB seri portu henüz başlatılmadı.");
        }
    }

    // USB Denetleyici ve CDC-ACM Sürücüsü Başlatma (Sahne64 sistem çağrısını kullanacak şekilde güncellendi)
    pub fn init_usb_hardware(&mut self) {
        println!("USB Konsolu Açılıyor...");
        match fs::open("/dev/ttyUSB0", fs::O_RDWR) { // Örnek cihaz yolu
            Ok(fd) => {
                self.fd = Some(fd);
                println!("USB Konsolu Açıldı! Dosya tanımlayıcısı: {}", fd);

                // İsteğe bağlı: Baud hızı gibi ayarları yapılandırmak için ioctl kullanılabilir
                // match kernel::ioctl(fd, /* İlgili IOCTL isteği */, /* Ayar argümanı */) {
                //     Ok(_) => println!("USB ayarları yapılandırıldı."),
                //     Err(e) => eprintln!("USB ayarları yapılandırılırken hata: {:?}", e),
                // }
            }
            Err(e) => {
                println!("USB Konsolu açılırken hata: {:?}", e);
                println!("Hata Detayı: {:?}", e);
            }
        }
    }
}


// Konsol yapısı (değişiklik yok)
pub struct Console {
    usb_serial: &'static mut UsbSerial, // Mutable referans
}

impl Console {
    pub const fn new(usb_serial: &'static mut UsbSerial) -> Self {
        Self { usb_serial }
    }
}

// Konsola string yazdırma fonksiyonu (değişiklik yok - artık usb_send daha detaylı)
pub fn console_print(console: &Console, s: &str) {
    console.usb_serial.usb_send(s.as_bytes());
}

// Formatlı çıktı için Write trait uygulaması (değişiklik yok)
impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        console_print(self, s);
        Ok(())
    }
}

// Makrolar (değişiklik yok)
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!(crate::console::CONSOLE, $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => (print!("\r\n"));
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!(crate::console::CONSOLE, $($arg)*);
        print!("\r\n");
    })
}

// Örnek kullanım (Sahne64'e özgü cihaz yolu kullanılmalı!)
// **Cihaz yolu işletim sistemine göre değişir!**
pub static mut USB_SERIAL: UsbSerial = UsbSerial::new(); // Mutable static
pub static mut CONSOLE: Console = Console::new(unsafe { &mut USB_SERIAL }); // Mutable static referans

// Başlatma fonksiyonu (iyileştirilmiş - donanım başlatma fonksiyonunu çağırır)
pub fn init() {
    println!("Sahne64 USB Konsolu Başlatılıyor...");
    unsafe {
        USB_SERIAL.init_usb_hardware(); // Donanım başlatma fonksiyonunu çağır
    }
    println!("Sahne64 USB Konsolu Başlatıldı!");
}