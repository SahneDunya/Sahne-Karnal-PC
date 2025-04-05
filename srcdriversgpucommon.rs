#![no_std]
#[derive(Copy, Clone, Debug)]
pub enum ColorFormat {
    RGB888,   // 24 bit RGB
    RGBA8888, // 32 bit RGBA
    // Diğer formatlar eklenebilir (örneğin, RGB565, ARGB1555 vb.)
}

// Temel grafik yapıları
#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub position: Point,
    pub size: Size,
}

// Piksel verisini renk formatına göre u32'ye dönüştürme fonksiyonu
pub fn color_to_u32(r: u8, g: u8, b: u8, a: u8, format: ColorFormat) -> u32 {
    match format {
        ColorFormat::RGB888 => {
            (r as u32) << 16 | (g as u32) << 8 | (b as u32)
        }
        ColorFormat::RGBA8888 => {
            (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | (a as u32)
        }
    }
}

// Piksel verisini renk formatına göre byte dizisine dönüştürme fonksiyonu
pub fn color_to_bytes(r: u8, g: u8, b: u8, a: u8, format: ColorFormat) -> [u8; 4] {
    match format {
        ColorFormat::RGB888 => {
            [b, g, r, 0] // RGB formatında A değeri kullanılmaz. Örnek olarak 0 verildi.
        }
        ColorFormat::RGBA8888 => {
            [b, g, r, a]
        }
    }
}

// Ekran modu bilgisi
#[derive(Copy, Clone, Debug)]
pub struct DisplayMode {
    pub resolution: Size,
    pub color_format: ColorFormat,
    pub refresh_rate: u32, // Hz
}

// Yardımcı fonksiyon: iki değeri karşılaştırıp max olanı döndürür (Standart kütüphane kullanılarak iyileştirildi)
pub fn max(a: usize, b: usize) -> usize {
    std::cmp::max(a, b) // std::cmp::max kullanıldı
}

// Yardımcı fonksiyon: iki değeri karşılaştırıp min olanı döndürür (Standart kütüphane kullanılarak iyileştirildi)
pub fn min(a: usize, b: usize) -> usize {
    std::cmp::min(a, b) // std::cmp::min kullanıldı
}

fn main() {
    // Örnek kullanım
    let format = ColorFormat::RGBA8888;
    let renk_u32 = color_to_u32(255, 0, 0, 255, format); // Kırmızı renk (RGBA)
    let renk_bytes = color_to_bytes(255, 0, 0, 255, format); // Kırmızı renk (RGBA)

    println!("Renk (u32): {:?}", renk_u32); // Output: Renk (u32): 4294901760
    println!("Renk (bytes): {:?}", renk_bytes); // Output: Renk (bytes): [0, 0, 255, 255]

    let boyut = Size { width: 100, height: 50 };
    println!("Boyut: {:?}", boyut); // Output: Boyut: Size { width: 100, height: 50 }

    let nokta = Point { x: 10, y: 20 };
    println!("Nokta: {:?}", nokta); // Output: Nokta: Point { x: 10, y: 20 }

    let dikdortgen = Rect { position: nokta, size: boyut };
    println!("Dikdörtgen: {:?}", dikdortgen); // Output: Dikdörtgen: Rect { position: Point { x: 10, y: 20 }, size: Size { width: 100, height: 50 } }

    println!("Max(5, 10): {:?}", max(5, 10)); // Output: Max(5, 10): 10
    println!("Min(5, 10): {:?}", min(5, 10)); // Output: Min(5, 10): 5
}