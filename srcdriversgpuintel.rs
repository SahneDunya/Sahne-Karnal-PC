#![no_std]
use core::ptr::{read_volatile, write_volatile};

// MMIO adresleri (ÖRNEK - GERÇEK GPU'YA GÖRE DEĞİŞİR!)
const MMIO_BASE: usize = 0xF0000000; // Örnek MMIO taban adresi
const FB_BASE: usize = MMIO_BASE + 0x2000000; // Örnek Framebuffer adresi
const FB_SIZE: usize = 1024 * 768 * 4; // Örnek Framebuffer boyutu (1024x768, 32bpp)

// GMADR (Graphics Memory Address Register) gibi önemli kayıtlar
const DISPLAY_CONTROL: usize = MMIO_BASE + 0x6000; // Örnek Display Control register adresi - Ekranı etkinleştirmek/devre dışı bırakmak için
const FBCONFIG: usize = MMIO_BASE + 0x4000; // Örnek Framebuffer Configuration Register - Framebuffer başlangıç adresini ayarlamak için

pub struct IntelGpu {
    mmio_base: usize,
    fb_base: usize,
    fb_size: usize,
    width: usize,
    height: usize,
}

impl IntelGpu {
    pub fn new() -> Self {
        IntelGpu {
            mmio_base: MMIO_BASE,
            fb_base: FB_BASE,
            fb_size: FB_SIZE,
            width: 1024,
            height: 768,
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // ÇOK TEMEL BAŞLATMA (GERÇEK GPU İÇİN ÇOK DAHA FAZLASI GEREKLİ!)
        unsafe {
            // Framebuffer başlangıç adresini yapılandır. (Örnek)
            self.write_register(FBCONFIG, self.fb_base as u32)?;

            // Ekranı etkinleştir. (Çok basit bir örnek. Gerçekte daha fazla ayar gerekebilir.)
            self.write_register(DISPLAY_CONTROL, 0x1)?; // Örnek değer: 0x1 ekranı etkinleştir anlamına gelebilir.
        }
        Ok(())
    }

    pub fn clear_fb(&mut self, color: u32) -> Result<(), &'static str> {
        let fb = self.fb_base as *mut u32;
        for i in 0..(self.fb_size / 4) {
            unsafe {
                self.write_volatile(fb.add(i), color)?;
            }
        }
        Ok(())
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: u32) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            return Err("Pixel coordinates out of bounds");
        }
        let offset = y * self.width + x;
        let fb = self.fb_base as *mut u32;
        unsafe {
            self.write_volatile(fb.add(offset), color)?;
        }
        Ok(())
    }

    // Güvenli register yazma işlemi
    unsafe fn write_register(&self, reg: usize, value: u32) -> Result<(), &'static str> {
        // Örnek olarak MMIO bölgesinin sınırları içinde olup olmadığını kontrol et
        if reg < self.mmio_base || reg >= self.mmio_base + 0x10000 {
            return Err("Invalid register address");
        }
        write_volatile(reg as *mut u32, value);
        Ok(())
    }

    // Güvenli volatile yazma işlemi
    unsafe fn write_volatile(&self, ptr: *mut u32, value: u32) -> Result<(), &'static str> {
        if ptr.is_null() {
            return Err("Null pointer");
        }
        write_volatile(ptr, value);
        Ok(())
    }
}

// Örnek kullanım
fn main() -> Result<(), &'static str> {
    let mut gpu = IntelGpu::new();
    gpu.init()?;
    gpu.clear_fb(0x0000FF00)?; // Ekranı yeşil ile temizle (0x00RRGGBB formatında)
    // İsteğe bağlı olarak tek bir piksel yazılabilir, ancak bu örnekte sadece ekranı temizliyoruz.
    // gpu.write_pixel(10, 10, 0x00FF0000)?; // (10, 10) koordinatına kırmızı piksel çiz
    Ok(())
}