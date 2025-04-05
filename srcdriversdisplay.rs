#![no_std]
use crate::platform;

pub struct Framebuffer {
    address: usize,
    width: usize,
    height: usize,
    pitch: usize, // Satır genişliği (byte cinsinden)
}

impl Framebuffer {
    pub const fn new(address: usize, width: usize, height: usize, pitch: usize) -> Self {
        Self { address, width, height, pitch }
    }

    pub fn init(&mut self) {
        // Framebuffer başlatma (gerekirse)
        // Genellikle bir şey yapmaya gerek yoktur, çünkü BIOS/firmware tarafından ayarlanır.
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let offset = y * self.pitch + x * 4; // Her piksel 4 byte (32 bit - ARGB)
            unsafe {
                *((self.address + offset) as *mut u32) = color;
            }
        }
    }

    pub fn clear(&mut self, color: u32){
        for y in 0..self.height{
            for x in 0..self.width{
                self.write_pixel(x, y, color);
            }
        }
    }
}

pub static mut FRAMEBUFFER: Framebuffer = Framebuffer::new(platform::FRAMEBUFFER_ADDRESS, platform::FRAMEBUFFER_WIDTH, platform::FRAMEBUFFER_HEIGHT, platform::FRAMEBUFFER_PITCH);