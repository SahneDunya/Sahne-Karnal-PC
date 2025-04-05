#![no_std]
use core::arch::asm;

// OpenRISC MPU (Bellek Koruma Birimi) kayıtlarını ayarlamak için fonksiyon
pub fn configure_mpu_region(index: usize, base_addr: u32, size: u32, ctrl: u32) {
    unsafe {
        // MPU Bölge Kontrol Kaydı (MPU_RCR) ayarlanır
        asm!(
            "mtspr r{rcr}, {index}", // Hangi bölgeyi yapılandıracağımızı seçiyoruz (index)
            rcr = const 960, // MPU_RCR'nin SPR numarası (Örnek olarak 960, OpenRISC mimari referansına bakılmalı)
            index = in(reg) index,
            options(nostack, preserves_flags)
        );

        // MPU Bölge Başlangıç Adresi Kaydı (MPU_RBAR) ayarlanır
        asm!(
            "mtspr r{rbar}, {addr}", // Bölge başlangıç adresini ayarlıyoruz
            rbar = const 961, // MPU_RBAR'nin SPR numarası (Örnek olarak 961, OpenRISC mimari referansına bakılmalı)
            addr = in(reg) base_addr,
            options(nostack, preserves_flags)
        );

        // MPU Bölge Boyutu ve Kontrol Kaydı (MPU_RRCR) ayarlanır
        asm!(
            "mtspr r{rrcr}, {config}", // Bölge boyutu ve kontrol ayarlarını yapılandırıyoruz
            rrcr = const 962, // MPU_RRCR'nin SPR numarası (Örnek olarak 962, OpenRISC mimari referansına bakılmalı)
            config = in(reg) (size | ctrl), // Boyut ve kontrol baytlarını birleştiriyoruz
            options(nostack, preserves_flags)
        );
    }
}

// MPU yapılandırma sabitleri (daha okunabilir) - Örnek değerler
const MPU_REGION_SIZE_1MB: u32 = 0b0010_0000; // 1MB bölge boyutu (Örnek değer, OpenRISC MPU dokümantasyonuna bakılmalı)
const MPU_REGION_ENABLE: u32 = 1 << 0;      // Bölgeyi etkinleştir
const MPU_REGION_CACHEABLE: u32 = 1 << 1;   // Bölgeyi önbelleklenebilir yap (Örnek özellik)
const MPU_REGION_BUFFERABLE: u32 = 1 << 2;  // Bölgeyi tamponlanabilir yap (Örnek özellik)
const MPU_REGION_EXECUTE_DISABLE: u32 = 1 << 3; // Bölgeden kod yürütmeyi devre dışı bırak (Örnek özellik)
const MPU_REGION_WRITE_PROTECT: u32 = 1 << 4; // Bölgeyi yazmaya karşı koru (Örnek özellik)
const MPU_REGION_READ_PROTECT: u32 = 1 << 5;  // Bölgeyi okumaya karşı koru (Örnek özellik)

// Basitleştirilmiş MPU başlatma fonksiyonu - Tek Örnek
pub fn init_mpu_simple_example() {
    // Örnek: RAM bölgesi için MPU yapılandırması (MPU Bölge 0)
    let ram_start: u32 = 0x10000000; // Örnek RAM başlangıç adresi (OpenRISC adres haritasına göre değişebilir)
    let ram_size_config = MPU_REGION_SIZE_1MB; // 1MB bölge boyutu

    // MPU Bölge 0 yapılandırması:
    // 1MB boyutunda, başlangıç adresi ram_start, etkin, önbelleklenebilir, tamponlanabilir,
    // yürütme devre dışı, okuma/yazma erişimine izin ver
    let mpu0_ctrl = MPU_REGION_ENABLE | MPU_REGION_CACHEABLE | MPU_REGION_BUFFERABLE | MPU_REGION_EXECUTE_DISABLE;
    configure_mpu_region(0, ram_start, ram_size_config, mpu0_ctrl);

    // Sadece MPU Bölge 0 yapılandırıldı.
    // Diğer MPU bölgeleri (MPU Bölge 1, MPU Bölge 2, MPU Bölge 3...) yapılandırılmadı ve varsayılan durumda.
}