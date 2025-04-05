#![no_std]
use core::arch::asm;

// MPU yapılandırma sabitleri (ARMv7-M için)
const MPU_REGION_SIZE_1MB: u32 = 0x17 << 1; // 1MB bölge boyutu (2^(23+1) bayt)
const MPU_REGION_ENABLE: u32 = 1 << 0;      // Bölgeyi etkinleştir
const MPU_REGION_ACCESS_RW: u32 = 0x3 << 24; // Okuma/Yazma erişimi (AP: 011)
const MPU_REGION_TEX_S_CB: u32 = 0x0 << 19; // Normal bellek, Paylaşılabilir, Önbelleklenebilir, Tamponlanabilir

/// MPU bölge yapılandırma hatası
#[derive(Debug)]
pub enum MpuError {
    InvalidRegionIndex,
    InvalidSize,
    MisalignedBaseAddress,
}

/// MPU bölge boyutunu kontrol eden yardımcı fonksiyon
fn validate_size(size: u32) -> Result<u8, MpuError> {
    match size {
        32 => Ok(0x04 << 1),      // 32 bayt
        1_048_576 => Ok(0x17 << 1), // 1MB
        // Diğer boyutlar eklenebilir
        _ => Err(MpuError::InvalidSize),
    }
}

/// MPU bölge taban adresini doğrulayan yardımcı fonksiyon
fn validate_base_addr(base_addr: u32, size: u32) -> Result<(), MpuError> {
    if base_addr % size != 0 {
        Err(MpuError::MisalignedBaseAddress)
    } else {
        Ok(())
    }
}

/// MPU bölgesini yapılandıran fonksiyon (ARMv7-M için)
pub fn configure_mpu_region(
    index: usize,
    base_addr: u32,
    size: u32,
    attrib: u32,
) -> Result<(), MpuError> {
    // Bölge numarası kontrolü (ARMv7-M'de genellikle 8 bölge desteklenir)
    if index >= 8 {
        return Err(MpuError::InvalidRegionIndex);
    }

    // Boyut ve adres uyumluluğunu kontrol et
    validate_size(size)?;
    validate_base_addr(base_addr, size)?;

    unsafe {
        // MPU bölge taban adres kaydını ayarla (RBAR)
        asm!(
            "mcr p15, 0, {0}, c6, c2, 0", // RBAR yazma (ARMv7-M'de c2 kullanılır)
            in(reg) base_addr | (index as u32 & 0xF), // Taban adres + bölge numarası
            options(nostack, preserves_flags)
        );

        // MPU bölge boyut ve kontrol kaydını ayarla (RASR)
        asm!(
            "mcr p15, 0, {0}, c6, c1, 0", // RASR yazma (ARMv7-M'de c1 kullanılır)
            in(reg) attrib, // Öznitelik ve boyut bilgisi
            options(nostack, preserves_flags)
        );
    }
    Ok(())
}

/// MPU'yu basit bir örnekle başlatır (ARMv7-M)
pub fn init_mpu_simple_example_armv7m() -> Result<(), MpuError> {
    let ram_start: u32 = 0x2000_0000; // SRAM başlangıç adresi
    let ram_size: u32 = 1_048_576;    // 1MB

    // MPU Bölge 0: 1MB RAM, RW erişim, etkin
    let attrib = MPU_REGION_SIZE_1MB
        | MPU_REGION_ENABLE
        | MPU_REGION_ACCESS_RW
        | MPU_REGION_TEX_S_CB;

    configure_mpu_region(0, ram_start, ram_size, attrib)?;
    Ok(())
}

/// MPU'yu Cortex-M4/M7 için SCB üzerinden başlatır
#[cfg(target_arch = "arm")]
pub fn init_mpu_simple_example_cortex_m() -> Result<(), MpuError> {
    use cortex_m::peripheral::Peripherals;

    if let Some(mut peripherals) = Peripherals::take() {
        let mpu = &mut peripherals.MPU;

        // MPU'yu etkinleştir
        mpu.ctrl.modify(|r, w| {
            w.enable().set_bit(); // MPU'yu aç
            w.hfnmiena().clear_bit(); // Hard Fault NMI'yi devre dışı bırak (isteğe bağlı)
            w.privdefena().clear_bit() // Varsayılan arka plan bölgesini devre dışı bırak
        });

        // Bölge 0: 1MB RAM yapılandırması
        let ram_start: u32 = 0x2000_0000;
        let ram_size: u32 = 1_048_576;
        let attrib = MPU_REGION_SIZE_1MB
            | MPU_REGION_ENABLE
            | MPU_REGION_ACCESS_RW
            | MPU_REGION_TEX_S_CB;

        // Taban adres ve bölge numarası
        mpu.rbar.write(ram_start | 0); // Bölge 0
        mpu.rasr.write(attrib);

        // Bellek hata kesmelerini etkinleştir (isteğe bağlı)
        peripherals.SCB.shcsr.modify(|_, w| w.memfaultena().set_bit());

        Ok(())
    } else {
        Err(MpuError::InvalidRegionIndex) // Peripherals alınamazsa hata
    }
}

// Test modülü (simülasyon ortamı için)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_size() {
        assert!(validate_size(1_048_576).is_ok());
        assert!(validate_size(32).is_ok());
        assert!(validate_size(123).is_err());
    }

    #[test]
    fn test_validate_base_addr() {
        assert!(validate_base_addr(0x2000_0000, 1_048_576).is_ok());
        assert!(validate_base_addr(0x2000_0010, 1_048_576).is_err());
    }
}