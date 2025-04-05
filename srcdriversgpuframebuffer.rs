#![no_std]
use core::ptr::{write_volatile, read_volatile};

// **ÖNEMLİ**: Framebuffer adresi ve boyutları donanıma *özgü* olmalıdır.
// Bu örnek değerler *sanal* adreslerdir ve donanımınızın gerçek adreslerine
// eşlenmelidir. Yanlış yapılandırma çekirdek hatalarına yol açabilir.
const FRAMEBUFFER_ADDRESS: usize = 0x4000_0000; // Örnek sanal adres
const SCREEN_WIDTH: usize = 800;
const SCREEN_HEIGHT: usize = 600;
const BYTES_PER_PIXEL: usize = 4; // RGBA32 formatı (her piksel 4 bayt)

/// Framebuffer yapısı, ekran arabelleğini yönetir.
pub struct Framebuffer {
    address: usize,
    width: usize,
    height: usize,
    pitch: usize, // Satır başına bayt sayısı
}

impl Framebuffer {
    /// Yeni bir Framebuffer örneği oluşturur.
    pub fn new() -> Self {
        Framebuffer {
            address: FRAMEBUFFER_ADDRESS,
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            pitch: SCREEN_WIDTH * BYTES_PER_PIXEL, // Satır genişliği * piksel başına bayt
        }
    }

    /// Framebuffer'ı belirtilen renkle temizler.
    pub fn clear(&mut self, color: u32) {
        // Framebuffer'a güvenli olmayan (unsafe) bir şekilde erişiyoruz.
        let framebuffer = self.address as *mut u32;
        // Ekranın toplam piksel sayısını hesapla.
        let size = self.width * self.height;
        // Tüm pikselleri döngüyle gez ve belirtilen renkle doldur.
        for i in 0..size {
            unsafe {
                // **write_volatile**: Derleyicinin bu yazma işlemini optimize etmesini engeller.
                // Donanım регистрlerine erişirken bu önemlidir.
                write_volatile(framebuffer.add(i), color);
            }
        }
    }

    /// Belirtilen koordinata bir piksel yazar.
    pub fn write_pixel(&mut self, x: usize, y: usize, color: u32) {
        // Koordinatların ekran sınırları içinde olup olmadığını kontrol et.
        if x < self.width && y < self.height {
            // Pikselin framebuffer içindeki konumunu hesapla.
            // `pitch` satır başına bayt sayısını ifade eder.
            let offset = y * (self.pitch / BYTES_PER_PIXEL) + x;
            let framebuffer = self.address as *mut u32;
            unsafe {
                // Güvenli olmayan blok içinde pikseli yaz.
                write_volatile(framebuffer.add(offset), color);
            }
        }
        // Ekran sınırları dışındaki koordinatlar için işlem yapma (sessizce başarısız ol).
    }

    /// Belirtilen konum ve boyutlarda bir dikdörtgen çizer.
    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        // Dikdörtgenin her satırı için...
        for j in y..y + height {
            // ve her sütunu için...
            for i in x..x + width {
                // pikseli belirtilen renkle yaz.
                self.write_pixel(i, j, color);
            }
        }
    }

    /// Çok basit bir karakter yazma fonksiyonu (geliştirilmesi gerekir!).
    pub fn write_char(&mut self, x: usize, y: usize, character: char, color: u32) {
        // **UYARI**: Bu fonksiyon çok temeldir ve sadece örnek amaçlıdır.
        // Gerçek bir uygulama için font tabanlı bir çözüm gereklidir.

        // Örnek olarak, sadece basit bir "A" karakteri çiziyoruz.
        if character == 'A' {
            // "A" karakterinin basit piksel deseni.
            self.write_pixel(x, y, color);
            self.write_pixel(x + 1, y, color);
            self.write_pixel(x, y + 1, color);
            self.write_pixel(x + 2, y + 1, color);
            self.write_pixel(x, y + 2, color);
            self.write_pixel(x, y + 3, color);
            self.write_pixel(x + 2, y + 3, color);
        }
        // İstenirse diğer karakterler için benzer desenler eklenebilir.
        // Ancak bu yöntem pratik bir çözüm değildir.
    }

    /// Belirtilen koordinattaki pikselin rengini okur.
    pub fn read_pixel(&mut self, x: usize, y: usize) -> u32 {
        // Koordinatların geçerliliğini kontrol et.
        if x < self.width && y < self.height {
            // Pikselin framebuffer içindeki konumunu hesapla.
            let offset = y * (self.pitch / BYTES_PER_PIXEL) + x;
            let framebuffer = self.address as *mut u32;
            unsafe {
                // **read_volatile**: Derleyicinin bu okuma işlemini optimize etmesini engeller.
                // Donanım регистрlerinden okurken önemlidir.
                read_volatile(framebuffer.add(offset))
            }
        } else {
            // Geçersiz koordinatlar için varsayılan bir renk (siyah) döndür.
            0 // Siyah renk (RGBA formatında)
        }
    }
}