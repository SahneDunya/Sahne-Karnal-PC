#![no_std]
pub const MMIO_BASE: usize = 0xF0000000;

// Örnek bir kayıt yapısı
#[repr(C)] // C ile uyumlu bellek düzeni
pub struct DisplayControlRegister {
    pub enable: u32,       // 0. bit: Ekranı etkinleştir/devre dışı bırak
    pub mode: u32,         // 1-2. bitler: Ekran modu (örneğin, 00: 640x480, 01: 800x600)
    _reserved: u32,      // Ayrılmış bitler
}

// Display Control Register'ın MMIO adresi
pub const DISPLAY_CONTROL_ADDRESS: usize = MMIO_BASE + 0x100;

// Örnek bir başka kayıt (Framebuffer Adres Kaydı)
#[repr(C)]
pub struct FbAddressRegister {
    pub address: u32, // Framebuffer'ın başlangıç adresi
}

// Framebuffer Adres Kaydı'nın MMIO adresi
pub const FB_ADDRESS_ADDRESS: usize = MMIO_BASE + 0x200;

// Başka bir örnek kayıt (Interrupt Enable Register)
#[repr(C)]
pub struct InterruptEnableRegister {
    pub vblank_interrupt_enable: u32, // Dikey tarama kesmesini etkinleştir/devre dışı bırak
    pub other_interrupt_enable:u32, // Başka bir kesmeyi etkinleştir/devre dışı bırak
}

pub const INTERRUPT_ENABLE_ADDRESS: usize = MMIO_BASE + 0x300;


// Bit maskeleri (bit alanlarına erişmek için)
pub const DISPLAY_ENABLE_BIT: u32 = 1 << 0;  // 0x01
pub const DISPLAY_MODE_BITS: u32 = 3 << 1;    // 0x06 (1-2. bitler)
pub const VBLANK_INTERRUPT_ENABLE_BIT: u32 = 1 << 0; //0x01
pub const OTHER_INTERRUPT_ENABLE_BIT: u32 = 1 << 1; //0x02

// Ekran modları için sabitler
pub const DISPLAY_MODE_640x480: u32 = 0 << 1; // 0x00
pub const DISPLAY_MODE_800x600: u32 = 1 << 1; // 0x02

// İYİLEŞTİRİLMİŞ ÖRNEK KULLANIM:
fn main() {
    unsafe {
        // 1. Display Control Register'a güvenli olmayan (raw) bir işaretçi oluştur
        let display_control_ptr = DISPLAY_CONTROL_ADDRESS as *mut DisplayControlRegister;

        // 2. İşaretçiyi güvenli bir Rust referansına dönüştür (hala unsafe blok içinde)
        let display_control_ref = &mut *display_control_ptr;

        // 3. Referans aracılığıyla kayıt alanlarına eriş ve değerleri ayarla
        display_control_ref.enable = DISPLAY_ENABLE_BIT; // Ekranı etkinleştir
        display_control_ref.mode = DISPLAY_MODE_800x600; // Ekran modunu 800x600 olarak ayarla

        // İsteğe bağlı: Değerleri okuyarak ayarların yapıldığını doğrulayabilirsiniz
        let current_enable = display_control_ref.enable;
        let current_mode = display_control_ref.mode;

        // Sonuçları yazdır (sadece örnek amaçlı, gerçek bir senaryoda bu MMIO üzerinden okunur)
        println!("Ekran Etkinleştirme: {}", current_enable);
        println!("Ekran Modu: {}", current_mode);
    }
}