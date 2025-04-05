#![no_std]
use core::arch::asm;

// MPU kayıtlarını ayarlamak için fonksiyon (64-bit uyumlu)
pub fn configure_mpu(index: usize, addr: u64, cfg: u8) {
    unsafe {
        // 64-bit adres kullanımı ve uygun register kullanımı (LoongArch özel register isimleri kontrol edilmeli)
        asm!(
            "csrwr mpuaddr{index}, {addr}", // LoongArch için MPU adres register'ı yazma komutu (kontrol edilmeli)
            index = in(reg) index,
            addr = in(reg) addr,
            options(nostack, preserves_flags)
        );
        asm!(
            "csrwr mpucfg{index}, {cfg}",   // LoongArch için MPU yapılandırma register'ı yazma komutu (kontrol edilmeli)
            index = in(reg) index,
            cfg = in(reg) cfg,
            options(nostack, preserves_flags)
        );
    }
}

// MPU yapılandırma sabitleri (daha okunabilir) - LOONGARCH ÖZEL DEĞERLER KONTROL EDİLMELİDİR
const MPU_A_TOR: u8 = 1 << 3; // Top of Range (Alan Üstü) - LOONGARCH DEĞERİ KONTROL EDİLMELİ
const MPU_M_RW: u8 = 2 << 1; // Read/Write (Okuma/Yazma) - LOONGARCH DEĞERİ KONTROL EDİLMELİ
const MPU_L: u8 = 1 << 0;    // Lock (Kilit) - LOONGARCH DEĞERİ KONTROL EDİLMELİ

// Basitleştirilmiş MPU başlatma fonksiyonu - Tek Örnek
pub fn init_mpu_simple_example() {
    // Örnek: RAM bölgesi için MPU yapılandırması (MPU0)
    let ram_start: u64 = 0x8000_0000;
    let ram_size: u64 = 1024 * 1024; // 1MB
    let ram_end = ram_start.wrapping_add(ram_size);

    // MPU0 yapılandırması: Top of Range, Read/Write erişim, Kilitli
    let mpu0_cfg = MPU_A_TOR | MPU_M_RW | MPU_L;
    configure_mpu(0, ram_end, mpu0_cfg);

    // Sadece MPU0 bölgesi yapılandırıldı.
    // Diğer MPU bölgeleri (MPU1, MPU2, MPU3...) yapılandırılmadı ve varsayılan durumda.
}