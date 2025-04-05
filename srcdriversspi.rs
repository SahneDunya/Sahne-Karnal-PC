#![no_std]
use crate::platform;
use core::ptr::{read_volatile, write_volatile};

// SPI register adreslerinin ve bit alanlarının tanımlanması
// Bu, sihirli sayıları ortadan kaldırır ve kodu daha okunabilir yapar.
mod registers {
    pub const CONTROL_REGISTER_OFFSET: usize = 0x00;
    pub const DATA_OUT_REGISTER_OFFSET: usize = 0x04;
    pub const DATA_IN_REGISTER_OFFSET: usize = 0x08;
    pub const STATUS_REGISTER_OFFSET: usize = 0x0C;

    // Kontrol Register Bitleri (örnek olarak)
    pub const SPI_ENABLE_BIT: u32 = 0x01; // Örnek bit
    pub const SPI_DISABLE_BIT: u32 = 0x00; // Örnek bit

    // Status Register Bitleri (örnek olarak)
    pub const BUSY_FLAG_BIT: u8 = 0x80; // SPI meşgul bayrağı biti
}

// SPI yapılandırma parametreleri için struct
// Bu, SPI'yı farklı modlarda ve ayarlarda yapılandırmayı kolaylaştırır.
pub struct SpiConfig {
    pub clock_speed: u32, // Örnek yapılandırma parametresi
    pub mode: SpiMode,     // SPI modu (örneğin, Mode0, Mode1, Mode2, Mode3)
}

// SPI Modları için Enum
pub enum SpiMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
    // ... diğer modlar eklenebilir
}

pub struct Spi {
    base_address: usize,
    config: SpiConfig, // Yapılandırma parametrelerini saklar
}

impl Spi {
    // new fonksiyonu artık yapılandırma da alıyor
    pub const fn new(base_address: usize, config: SpiConfig) -> Self {
        Self {
            base_address,
            config,
        }
    }

    pub fn init(&mut self) {
        // SPI yapılandırması (platforma özgü olmalı ve yapılandırma parametrelerini kullanmalı)
        // Örnek:
        // let control_ptr = (self.base_address + registers::CONTROL_REGISTER_OFFSET) as *mut u32;
        // write_volatile(control_ptr, registers::SPI_ENABLE_BIT); // SPI'yı etkinleştir
        self.configure_spi(); // Yapılandırma fonksiyonunu çağır
    }

    fn configure_spi(&mut self) {
        // SPI yapılandırma detayları (platforma ve config'e özgü)
        // Bu fonksiyon, SPI modunu, hızını ve diğer parametreleri ayarlar.
        // Örnek olarak, sadece SPI'yı etkinleştiriyoruz. Gerçek bir uygulamada,
        // daha fazla yapılandırma adımı olacaktır (örneğin, mod ayarlama, hız bölme vb.).

        unsafe {
            let control_ptr = (self.base_address + registers::CONTROL_REGISTER_OFFSET) as *mut u32;
            write_volatile(control_ptr, registers::SPI_ENABLE_BIT); // SPI'yı etkinleştir
            // ... diğer yapılandırma adımları (örneğin mod ayarlama, hız bölme vb.) ...

            // Örnek olarak, SPI modunu ayarlıyoruz (config'den alarak)
            match self.config.mode {
                SpiMode::Mode0 => {
                    // Mode 0 yapılandırması
                    // ...
                },
                SpiMode::Mode1 => {
                    // Mode 1 yapılandırması
                    // ...
                },
                SpiMode::Mode2 => {
                    // Mode 2 yapılandırması
                    // ...
                },
                SpiMode::Mode3 => {
                    // Mode 3 yapılandırması
                    // ...
                },
            }

            // Örnek olarak, saat hızını ayarlıyoruz (config'den alarak)
            // let clock_ptr = ...; // Saat hızını ayarlayan register adresi
            // write_volatile(clock_ptr, self.config.clock_speed);
        }
    }


    pub fn transfer(&mut self, data: u8) -> u8 {
        // SPI üzerinden veri transferi
        // Bu kısım platforma özgüdür ve donanım register'larına erişimi içerir.
        unsafe {
            let data_out_ptr = (self.base_address + registers::DATA_OUT_REGISTER_OFFSET) as *mut u8;
            let data_in_ptr = (self.base_address + registers::DATA_IN_REGISTER_OFFSET) as *mut u8;
            let status_ptr = (self.base_address + registers::STATUS_REGISTER_OFFSET) as *mut u8;

            write_volatile(data_out_ptr, data);

            // Meşgul bayrağı kontrolü daha okunabilir bir sabit ile yapılıyor
            while (read_volatile(status_ptr) & registers::BUSY_FLAG_BIT) != 0 {
                // İşlem tamamlanana kadar bekle
                // İsteğe bağlı olarak buraya bir timeout mekanizması eklenebilir.
            }

            read_volatile(data_in_ptr)
        }
    }
}

// Örnek SPI yapılandırması
pub static mut SPI0: Spi = Spi::new(
    platform::SPI0,
    SpiConfig {
        clock_speed: 1_000_000, // 1 MHz örnek saat hızı
        mode: SpiMode::Mode0,     // Mode 0 örnek SPI modu
    }
);