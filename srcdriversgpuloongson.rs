use core::ptr;

// Loongson GPU temel adresleri
const LOONGSON_GPU_BASE: usize = 0x80000000; // Örnek adres
const LOONGSON_GPU_CONTROL: usize = LOONGSON_GPU_BASE + 0x00;
const LOONGSON_GPU_FRAMEBUFFER: usize = LOONGSON_GPU_BASE + 0x1000;

// GPU kontrol register sabitleri
const LOONGSON_GPU_ENABLE: u32 = 1; // Daha okunabilir ve yaygın kullanım
const LOONGSON_GPU_DISABLE: u32 = 0;

pub struct LoongsonGPU {
    control_reg: *mut u32, // Kontrol register'ına doğrudan erişim
    framebuffer: *mut u32,
}

impl LoongsonGPU {
    pub fn new() -> Self {
        LoongsonGPU {
            control_reg: LOONGSON_GPU_CONTROL as *mut u32, // Kontrol register'ını başlat
            framebuffer: LOONGSON_GPU_FRAMEBUFFER as *mut u32,
        }
    }

    pub fn enable(&self) {
        unsafe {
            ptr::write_volatile(self.control_reg, LOONGSON_GPU_ENABLE); // Doğrudan control_reg kullanılıyor
        }
    }

    pub fn disable(&self) {
        unsafe {
            ptr::write_volatile(self.control_reg, LOONGSON_GPU_DISABLE); // Doğrudan control_reg kullanılıyor
        }
    }

    pub fn clear_screen(&self, color: u32) {
        let framebuffer_size = 1024 * 768;
        unsafe {
            // Daha performanslı temizleme için write_bytes kullanılıyor eğer mümkünse, veya döngü
            let framebuffer_slice = core::slice::from_raw_parts_mut(self.framebuffer as *mut u8, framebuffer_size * 4); // u32 * 4 = u8 boyutu
            let color_bytes = color.to_le_bytes(); // Little-endian byte sırası
            for i in 0..(framebuffer_size * 4) {
                framebuffer_slice[i] = color_bytes[i % 4]; // Her byte'ı renk byte'ları ile doldur
            }
        }
    }
}

// Örnek kullanım
fn main() {
    let gpu = LoongsonGPU::new();

    gpu.enable(); // GPU'yu etkinleştir

    gpu.clear_screen(0x0000FF00); // Ekranı yeşil yap

    // ... diğer GPU işlemleri ...

    gpu.disable(); // GPU'yu devre dışı bırak
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_enable_disable() {
        let gpu = LoongsonGPU::new();
        gpu.enable();
        // GPU'nun etkin olup olmadığını kontrol et (gerçek donanım gerektirir)
        gpu.disable();
        // GPU'nun devre dışı olup olmadığını kontrol et (gerçek donanım gerektirir)
    }

    #[test]
    fn test_clear_screen() {
        let gpu = LoongsonGPU::new();
        gpu.enable();
        gpu.clear_screen(0x000000FF); // Mavi renk
        // Ekranın temizlendiğini kontrol et (gerçek donanım gerektirir)
        gpu.disable();
    }
}