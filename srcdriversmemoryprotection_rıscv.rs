#![no_std]
use core::arch::asm;

// PMP kayıtlarını ayarlamak için fonksiyon (64-bit uyumlu)
pub fn configure_pmp(index: usize, addr: u64, cfg: u8) {
    unsafe {
        // 64-bit adres kullanımı ve uygun register kullanımı
        asm!(
            "csrw pmpaddr{}, {}",
            in(reg) index,
            in(reg) addr,
            options(nostack, preserves_flags)
        );
        asm!(
            "csrw pmpcfg{}, {}",
            in(reg) index,
            in(reg) cfg,
            options(nostack, preserves_flags)
        );
    }
}

// PMP yapılandırma sabitleri (daha okunabilir)
const PMP_A_TOR: u8 = 1 << 3; // Top of Range
const PMP_M_RW: u8 = 2 << 1; // Read/Write
const PMP_L: u8 = 1 << 0;   // Lock

// Basitleştirilmiş PMP başlatma fonksiyonu - Tek Örnek
pub fn init_pmp_simple_example() {
    // Örnek: RAM bölgesi için PMP yapılandırması (PMP0)
    let ram_start: u64 = 0x8000_0000;
    let ram_size: u64 = 1024 * 1024; // 1MB
    let ram_end = ram_start.wrapping_add(ram_size);

    // PMP0 yapılandırması: Top of Range, Read/Write erişim, Kilitli
    let pmp0_cfg = PMP_A_TOR | PMP_M_RW | PMP_L;
    configure_pmp(0, ram_end, pmp0_cfg);

    // Sadece PMP0 bölgesi yapılandırıldı.
    // Diğer PMP bölgeleri (PMP1, PMP2, PMP3...) yapılandırılmadı ve varsayılan durumda.
}