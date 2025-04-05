#![no_std]
use core::ptr::{read_volatile, write_volatile};

// MMIO adresleri (ÖRNEK - GERÇEK GPU'YA GÖRE DEĞİŞİR!)
const MMIO_BASE: usize = 0xF0000000; // Örnek MMIO taban adresi
const FB_BASE: usize = MMIO_BASE + 0x100000; // Örnek Framebuffer adresi
const FB_SIZE: usize = 800 * 600 * 4; // Örnek Framebuffer boyutu
const GRBM_GFX_INDEX: usize = MMIO_BASE + 0x028; // Genel Kayıt Bloğu Yöneticisi GFX İndeksi

/// AMD GPU yapısı
pub struct AmdGpu {
    mmio_base: usize,
    fb_base: usize,
    fb_size: usize,
    width: usize,
    height: usize,
}

impl AmdGpu {
    /// Yeni bir AMD GPU örneği oluşturur.
    pub fn new(mmio_base: usize, fb_base: usize, fb_size: usize, width: usize, height: usize) -> Self {
        Self { // 'Self' kısaltması kullanıldı
            mmio_base,
            fb_base,
            fb_size,
            width,
            height,
        }
    }

    /// GPU'yu başlatır.
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Güvenli olmayan (unsafe) blok, doğrudan donanım kaydına yazma işlemi gerektiğinde kullanılır.
        unsafe {
            // GRBM_GFX_INDEX kaydına 0x00010000 değeri yazarak GPU'yu etkinleştirmeyi deniyoruz.
            // DİKKAT: Bu örnek bir değerdir ve gerçek bir GPU'da farklı bir değer gerekebilir
            // veya bu adres ve yöntem gerçek GPU'lar için geçerli olmayabilir.
            write_volatile(GRBM_GFX_INDEX as *mut u32, 0x00010000);
        }
        Ok(())
    }

    /// Framebuffer'ı belirli bir renkle temizler.
    pub fn clear_fb(&mut self, color: u32) -> Result<(), &'static str> {
        // Framebuffer'a yazma işlemi güvenli olmayan (unsafe) bir işlemdir.
        let fb = self.fb_base as *mut u32;
        let fb_size_u32 = self.fb_size / 4; // Döngüyü u32 cinsinden boyutlandırıyoruz.

        // Framebuffer belleğine döngü ile renk değerini yazıyoruz.
        for i in 0..fb_size_u32 {
            unsafe {
                // volatile yazma işlemi, derleyicinin bu yazma işlemini optimize etmesini engeller.
                write_volatile(fb.add(i), color);
            }
        }
        Ok(())
    }

    /// Framebuffer'da belirli bir pikseli belirli bir renkle boyar.
    pub fn write_pixel(&mut self, x: usize, y: usize, color: u32) -> Result<(), &'static str> {
        // Koordinatların framebuffer sınırları içinde olup olmadığını kontrol ediyoruz.
        if x >= self.width || y >= self.height {
            return Err("Pixel coordinates out of bounds"); // Daha açıklayıcı hata mesajı
        }

        // Pikselin framebuffer içindeki ofsetini hesaplıyoruz.
        let offset = y * self.width + x;
        let fb = self.fb_base as *mut u32;

        // Framebuffer'a yazma işlemi güvenli olmayan (unsafe) bir işlemdir.
        unsafe {
            // Belirtilen ofsetteki piksel adresine renk değerini yazıyoruz.
            write_volatile(fb.add(offset), color);
        }
        Ok(())
    }
}

// Örnek kullanım fonksiyonu
fn main() {
    // AmdGpu yapısının yeni bir örneğini oluşturuyoruz.
    let mut gpu = AmdGpu::new(MMIO_BASE, FB_BASE, FB_SIZE, 800, 600);

    // GPU'yu başlatmayı deniyoruz ve hata durumunda panik oluşturuyoruz.
    if let Err(e) = gpu.init() {
        panic!("GPU initialization failed: {}", e); // Daha açıklayıcı panik mesajı
    }

    // Framebuffer'ı yeşil renk ile temizlemeyi deniyoruz ve hata durumunda panik oluşturuyoruz.
    if let Err(e) = gpu.clear_fb(0x00FF00) { // Yeşil renk (0x00RRGGBB formatında)
        panic!("Failed to clear framebuffer: {}", e); // Daha açıklayıcı panik mesajı
    }

    // (100, 100) koordinatlarına kırmızı bir piksel yazmayı deniyoruz ve hata durumunda panik oluşturuyoruz.
    if let Err(e) = gpu.write_pixel(100, 100, 0xFF0000) { // Kırmızı renk (0x00RRGGBB formatında)
        panic!("Failed to write pixel: {}", e); // Daha açıklayıcı panik mesajı
    }

    // Programın sonuna gelindiğinde başarılı bir şekilde çalıştığını belirtmek için bir mesaj yazdırabiliriz.
    println!("GPU example completed successfully!");
}