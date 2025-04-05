#![no_std] // Standart kütüphaneye bağımlılığı kaldırır.

// NVMe ile ilgili sabitler
pub mod constants {
    pub const NVME_CONTROLLER_BAR: u64 = 0xE0000000; // Örnek: NVMe kontrol cihazının Base Address Register (BAR) adresi
    pub const NVME_CAP_OFFSET: u64 = 0x00;        // Controller Capabilities (CAP) kaydının ofseti
    pub const NVME_VS_OFFSET: u64 = 0x08;         // Version (VS) kaydının ofseti
    pub const NVME_CC_OFFSET: u64 = 0x14;         // Controller Configuration (CC) kaydının ofseti
    pub const NVME_STS_OFFSET: u64 = 0x1C;        // Controller Status (STS) kaydının ofseti
    // ... diğer NVMe sabitleri ...
}

// NVMe ile ilgili veri yapıları
pub mod structures {
    #[repr(C)]
    pub struct NvmeControllerCapabilities {
        pub mqes: u16,         // Maximum Queue Entries Supported
        pub cqr: u16,          // Contiguous Queues Required
        pub ams: u8,           // Arbitration Mechanism Supported
        pub reserved0: u8,
        pub to: u16,           // Timeout
        pub dstrd: u8,         // Doorbell Stride
        pub reserved1: [u8; 25],
        pub css: u8,           // Command Set Supported
        pub reserved2: [u8; 23],
    }

    #[repr(C)]
    pub struct NvmeVersion {
        pub vmjr: u16,         // Version Major
        pub vmnr: u16,         // Version Minor
        pub mpsmin: u8,        // Minimum Memory Page Size
        pub mpsmax: u8,        // Maximum Memory Page Size
        pub reserved: [u8; 3],
        pub intrd_msix: u8,    // Interrupt Reporting Method
    }

    #[repr(C)]
    pub struct NvmeControllerConfiguration {
        pub en: u8,           // Enable
        pub aei: u8,          // Abort Execution Enable
        pub tccp: u8,         // Tail Check Completion Policy
        pub shn: u8,          // Shutdown Notification
        pub iosqes: u8,       // I/O Submission Queue Entry Size
        pub iocqes: u8,       // I/O Completion Queue Entry Size
        pub reserved: u16,
        pub dbtbs: u32,        // Doorbell Buffer Base Address
        pub reserved2: [u8; 4],
    }

    #[repr(C)]
    pub struct NvmeControllerStatus {
        pub rdy: u8,          // Ready
        pub cfs: u8,          // Controller Fatal Status
        pub reserved: [u8; 2],
        pub sc: u16,           // Status Code
        pub sct: u8,          // Status Code Type
        pub reserved2: u8,
        pub sqes: u8,          // Submission Queue Entry Size
        pub cqes: u8,          // Completion Queue Entry Size
        pub reserved3: [u8; 2],
        pub pciid: u16,        // PCI Vendor ID
        pub reserved4: [u8; 2],
        pub ss: u8,            // Subsystem Vendor ID
        pub reserved5: [u8; 3],
        pub rn: u8,            // Reboot Needed
        pub reserved6: [u8; 3],
    }

    // ... diğer NVMe veri yapıları (komutlar, kuyruklar vb.) ...
}

// Düşük seviyeli NVMe API fonksiyonları
pub mod low_level {
    use crate::constants::*;
    use crate::structures::*;

    // Güvenli olmayan (unsafe) işlemler gerektiren düşük seviyeli okuma fonksiyonu
    // Bu fonksiyonun CustomOS'un bellek erişim mekanizmasına uygun şekilde implemente edilmesi gerekir.
    unsafe fn read_volatile_u32(address: u64) -> u32 {
        *(address as *const u32)
    }

    unsafe fn read_volatile_u64(address: u64) -> u64 {
        *(address as *const u64)
    }

    // NVMe kontrol cihazının yeteneklerini (Capabilities) okur.
    pub unsafe fn identify_controller_capabilities() -> NvmeControllerCapabilities {
        let addr = NVME_CONTROLLER_BAR + NVME_CAP_OFFSET;
        let raw_value = read_volatile_u64(addr);
        NvmeControllerCapabilities {
            mqes: (raw_value & 0xFFFF) as u16,
            cqr: ((raw_value >> 16) & 0x1) as u16,
            ams: ((raw_value >> 17) & 0x7) as u8,
            reserved0: ((raw_value >> 20) & 0xF) as u8,
            to: ((raw_value >> 24) & 0xFFFF) as u16,
            dstrd: ((raw_value >> 40) & 0x3) as u8,
            reserved1: [0; 25], // Geri kalanını sıfırla
            css: ((raw_value >> 63) & 0x1) as u8,
            reserved2: [0; 23], // Geri kalanını sıfırla
        }
    }

    // NVMe kontrol cihazının versiyonunu okur.
    pub unsafe fn identify_controller_version() -> NvmeVersion {
        let addr = NVME_CONTROLLER_BAR + NVME_VS_OFFSET;
        let raw_value = read_volatile_u32(addr);
        NvmeVersion {
            vmjr: (raw_value >> 16) as u16,
            vmnr: raw_value as u16,
            mpsmin: 0, // Bu alanlar genellikle farklı bir yerden okunur veya hesaplanır.
            mpsmax: 0,
            reserved: [0; 3],
            intrd_msix: 0,
        }
    }

    // NVMe kontrol cihazını etkinleştirir.
    pub unsafe fn enable_controller() {
        let addr = NVME_CONTROLLER_BAR + NVME_CC_OFFSET;
        let mut config = read_volatile_u32(addr);
        config |= 0x1; // EN (Enable) bitini ayarla
        *(addr as *mut u32) = config;
    }

    // NVMe kontrol cihazının hazır olup olmadığını kontrol eder.
    pub unsafe fn is_controller_ready() -> bool {
        let addr = NVME_CONTROLLER_BAR + NVME_STS_OFFSET;
        (read_volatile_u32(addr) & 0x1) != 0
    }

    // ... diğer düşük seviyeli NVMe fonksiyonları (kuyruk oluşturma, komut gönderme vb.) ...
}

// Yüksek seviyeli NVMe API fonksiyonları (düşük seviyeli fonksiyonları kullanır)
pub mod high_level {
    use crate::low_level::*;
    use crate::structures::*;
    use core::fmt::Debug;

    #[derive(Debug)]
    pub struct ControllerInfo {
        pub capabilities: NvmeControllerCapabilities,
        pub version: NvmeVersion,
    }

    pub unsafe fn get_controller_info() -> ControllerInfo {
        let capabilities = identify_controller_capabilities();
        let version = identify_controller_version();
        ControllerInfo { capabilities, version }
    }

    pub unsafe fn initialize_nvme() -> Result<(), &'static str> {
        // Kontrol cihazını etkinleştir
        enable_controller();

        // Kontrol cihazının hazır olmasını bekle (uzun sürebilir, bu yüzden dikkatli olun)
        for _ in 0..100000 { // Örnek bir bekleme döngüsü
            if is_controller_ready() {
                return Ok(());
            }
            // Burada kısa bir gecikme eklenebilir (CustomOS'a özgü olmalı)
        }
        Err("NVMe kontrol cihazı zamanında hazır olmadı.")
    }

    // ... diğer yüksek seviyeli NVMe fonksiyonları ...
}

// Örnek kullanım (bu genellikle ayrı bir yerde olur, örneğin bir çekirdek modülünde)
// #[cfg(test)] // Eğer test ortamınız varsa
fn main() {
    // Güvenli olmayan (unsafe) bir blok içinde çalıştırılması gerekir çünkü düşük seviyeli donanım erişimi içerir.
    unsafe {
        match high_level::initialize_nvme() {
            Ok(_) => {
                println!("NVMe kontrol cihazı başarıyla başlatıldı.");
                let controller_info = high_level::get_controller_info();
                println!("NVMe Kontrol Cihazı Bilgileri: {:?}", controller_info);
            }
            Err(e) => {
                eprintln!("NVMe başlatma hatası: {}", e);
            }
        }
    }
}
mod custom_os {
}