#![no_std]
use core::ptr::{read_volatile, write_volatile};

// Framebuffer adresi ve boyutları (ÖRNEK - GERÇEK PLATFORMA GÖRE DEĞİŞİR!)
const FRAMEBUFFER_ADDRESS: usize = 0x80000000; // Örnek Framebuffer adresi
const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;
const BYTES_PER_PIXEL: usize = 4;

pub struct SifiveGpu {
    fb_address: usize,
    width: usize,
    height: usize,
    pitch: usize,
}

impl SifiveGpu {
    pub fn new() -> Self {
        SifiveGpu {
            fb_address: FRAMEBUFFER_ADDRESS,
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            pitch: SCREEN_WIDTH * BYTES_PER_PIXEL,
        }
    }

    pub fn clear(&mut self, color: u32) {
        let fb = self.fb_address as *mut u32;
        let size = self.width * self.height;
        for i in 0..size {
            unsafe {
                write_volatile(fb.add(i), color);
            }
        }
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let offset = y * (self.pitch / BYTES_PER_PIXEL) + x;
            let fb = self.fb_address as *mut u32;
            unsafe {
                write_volatile(fb.add(offset), color);
            }
        }
    }

    // İyileştirme 1: Piksel okuma fonksiyonu eklendi.
    pub fn read_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            let offset = y * (self.pitch / BYTES_PER_PIXEL) + x;
            let fb = self.fb_address as *mut u32;
            unsafe {
                read_volatile(fb.add(offset))
            }
        } else {
            0 // Sınırların dışında ise varsayılan renk (örneğin siyah) döndürülebilir.
        }
    }
}