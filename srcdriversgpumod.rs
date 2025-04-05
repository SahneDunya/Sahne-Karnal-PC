#![no_std]
pub mod framebuffer; // Framebuffer sürücüsü
// pub mod amd;       // AMD sürücüsü (ileride)
// pub mod intel;     // Intel sürücüsü (ileride)
// pub mod sifive;    // SiFive sürücüsü (ileride)
pub mod common;    // Ortak fonksiyonlar ve yapılar

use framebuffer::Framebuffer;

// GPU sürücüsü yapısı (şimdilik sadece framebuffer içeriyor).
pub struct GpuDriver {
    framebuffer: Framebuffer,
    // Diğer GPU sürücüleri ve bilgileri burada eklenebilir.
}

impl GpuDriver {
    pub fn new() -> Self {
        GpuDriver {
            framebuffer: Framebuffer::new(),
        }
    }

    pub fn init(&mut self) {
        // GPU'yu başlat. Şimdilik sadece framebuffer'ı temizliyoruz.
        self.framebuffer.clear(0x000000); // Siyah ekran
    }

    pub fn clear(&mut self, color: u32){
        self.framebuffer.clear(color);
    }

        pub fn write_pixel(&mut self, x: usize, y: usize, color: u32){
        self.framebuffer.write_pixel(x, y, color);
    }

    // İleride diğer GPU sürücüleri için başlatma ve kontrol fonksiyonları buraya eklenecek.
}

// GPU sürücüsünü başlatma fonksiyonu (çekirdek tarafından çağrılacak).
pub fn init_gpu() -> GpuDriver{
    let mut gpu_driver = GpuDriver::new();
    gpu_driver.init();
    gpu_driver
}

// Örnek kullanım (başka bir modülden):
// use gpu::init_gpu;
// let mut gpu = init_gpu();
// gpu.clear(0xFF0000); // Ekranı kırmızıya boyar