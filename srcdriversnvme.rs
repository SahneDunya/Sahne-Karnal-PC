#![no_std]
#![no_main]

// Bu örnek kod, API olmayan ve özel bir çekirdek için çok temel bir NVMe sürücüsünü göstermektedir.
// Gerçek bir sürücü çok daha karmaşık olacaktır ve hata işleme, kuyruk yönetimi, kesme işleme vb. içerecektir.

// Bu kodun çalışması için özel çekirdeğinizde uygun bellek yönetimi ve PCI erişimi mekanizmalarınızın olması gerekir.

// NVMe spesifikasyonundan bazı temel sabitler ve yapılar.
pub mod constants {
    pub const NVME_CONTROLLER_BAR: u64 = 0xF0000000; // Örnek: NVMe kontrol cihazının Base Address Register (BAR) adresi (Güncellendi)
    pub const NVME_CAP_OFFSET: u64 = 0x00;
    pub const NVME_VS_OFFSET: u64 = 0x08;
    pub const NVME_CC_OFFSET: u64 = 0x14;
    pub const NVME_STS_OFFSET: u64 = 0x1C;
    pub const NVME_AQA_OFFSET: u64 = 0x24;
    pub const NVME_ASQ_OFFSET: u64 = 0x28;
    pub const NVME_ACQ_OFFSET: u64 = 0x30;

    // NVMe Denetleyici Yapılandırma (CC) Alanı Bitleri
    pub const NVME_CC_EN: u32 = 1 << 0;
    pub const NVME_CC_CSS_NVM: u32 = 0 << 1; // NVM Komut Seti
    pub const NVME_CC_MPS_MASK: u32 = 0b1111 << 12;
    pub const NVME_CC_MPS_SHIFT: u32 = 12;

    // NVMe Denetleyici Durumu (STS) Alanı Bitleri (Önceki kodda CSTS olarak geçiyordu, düzeltildi)
    pub const NVME_STS_RDY: u32 = 1 << 0;
}

// NVMe ile ilgili veri yapıları
pub mod structures {
    #[repr(C)]
    pub struct NvmeControllerCapabilities {
        pub mqes: u16,
        pub cqr: u16,
        pub ams: u8,
        pub reserved0: u8,
        pub to: u16,
        pub dstrd: u8,
        pub reserved1: [u8; 25],
        pub css: u8,
        pub reserved2: [u8; 23],
    }

    #[repr(C)]
    pub struct NvmeVersion {
        pub vmjr: u16,
        pub vmnr: u16,
        pub mpsmin: u8,
        pub mpsmax: u8,
        pub reserved: [u8; 3],
        pub intrd_msix: u8,
    }

    #[repr(C)]
    pub struct NvmeControllerConfiguration {
        pub en: u8,
        pub aei: u8,
        pub tccp: u8,
        pub shn: u8,
        pub iosqes: u8,
        pub iocqes: u8,
        pub reserved: u16,
        pub dbtbs: u32,
        pub reserved2: [u8; 4],
    }

    #[repr(C)]
    pub struct NvmeControllerStatus {
        pub rdy: u8,
        pub cfs: u8,
        pub reserved: [u8; 2],
        pub sc: u16,
        pub sct: u8,
        pub reserved2: u8,
        pub sqes: u8,
        pub cqes: u8,
        pub reserved3: [u8; 2],
        pub pciid: u16,
        pub reserved4: [u8; 2],
        pub ss: u8,
        pub reserved5: [u8; 3],
        pub rn: u8,
        pub reserved6: [u8; 3],
    }

    // NVMe Komut Yapısı (çok temel)
    #[repr(C)]
    pub struct NvmeCommand {
        pub opcode: u8,
        pub flags: u8,
        pub cid: u16,
        pub nsid: u32,
        // Daha fazla alan eklenebilir (örneğin, okuma/yazma komutları için adres ve uzunluk)
        pub dptr: [u64; 2], // Veri İşaretçileri
        // ... diğer alanlar ...
    }

    // NVMe Tamamlama Kuyruğu Girişi Yapısı (çok temel)
    #[repr(C)]
    pub struct NvmeCompletion {
        pub sqid: u16,
        pub sqhd: u16,
        pub status: u16,
        // Diğer alanlar eklenebilir
    }
}

// Düşük seviyeli NVMe API fonksiyonları
pub mod low_level {
    use crate::constants::*;
    use crate::structures::*;
    use core::ptr::{read_volatile, write_volatile};

    // Güvenli olmayan (unsafe) işlemler gerektiren düşük seviyeli okuma fonksiyonu
    // Bu fonksiyonun CustomOS'un bellek erişim mekanizmasına uygun şekilde implemente edilmesi gerekir.
    unsafe fn read_volatile_u32(address: u64) -> u32 {
        *(address as *const u32)
    }

    unsafe fn read_volatile_u64(address: u64) -> u64 {
        *(address as *const u64)
    }

    unsafe fn write_volatile_u32(address: u64, value: u32) {
        *(address as *mut u32) = value;
    }

    unsafe fn write_volatile_u64(address: u64, value: u64) {
        *(address as *mut u64) = value;
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
        config |= NVME_CC_EN; // EN (Enable) bitini ayarla
        write_volatile_u32(addr, config);
    }

    // NVMe kontrol cihazının hazır olup olmadığını kontrol eder.
    pub unsafe fn is_controller_ready() -> bool {
        let addr = NVME_CONTROLLER_BAR + NVME_STS_OFFSET;
        (read_volatile_u32(addr) & NVME_STS_RDY as u32) != 0
    }

    // NVMe adresini verilen offset ile birlikte hesaplar.
    fn get_reg_addr(offset: u64) -> u64 {
        NVME_CONTROLLER_BAR.wrapping_add(offset)
    }

    // Belirtilen ofsetteki 32-bit değeri okur.
    pub unsafe fn read_reg32(offset: u64) -> u32 {
        read_volatile_u32(get_reg_addr(offset))
    }

    // Belirtilen ofsetteki 64-bit değeri okur.
    pub unsafe fn read_reg64(offset: u64) -> u64 {
        read_volatile_u64(get_reg_addr(offset))
    }

    // Belirtilen ofsetteki 32-bit değeri yazar.
    pub unsafe fn write_reg32(offset: u64, value: u32) {
        write_volatile_u32(get_reg_addr(offset), value);
    }

    // Belirtilen ofsetteki 64-bit değeri yazar.
    pub unsafe fn write_reg64(offset: u64, value: u64) {
        write_volatile_u64(get_reg_addr(offset), value);
    }
}

// Yüksek seviyeli NVMe API fonksiyonları (düşük seviyeli fonksiyonları kullanır)
pub mod high_level {
    use crate::low_level::*;
    use crate::structures::*;

    pub unsafe fn initialize_nvme() -> Result<(), &'static str> {
        // 1. Denetleyiciyi devre dışı bırak.
        let cc = low_level::read_reg32(super::constants::NVME_CC_OFFSET);
        low_level::write_reg32(super::constants::NVME_CC_OFFSET, cc & !(super::constants::NVME_CC_EN as u32));

        // 2. ASQ ve ACQ adreslerini ayarla.
        // Özel çekirdeğinizin bellek ayırma mekanizmasını kullanmanız gerekecektir.
        // Bu örnekte, basitçe statik adresler varsayıyoruz.
        const SUBMISSION_QUEUE_BASE: u64 = 0x1000;
        const COMPLETION_QUEUE_BASE: u64 = 0x2000;
        const QUEUE_SIZE: u16 = 31; // 0 tabanlı, bu yüzden 32 giriş

        low_level::write_reg64(super::constants::NVME_AQA_OFFSET, ((QUEUE_SIZE as u64) << 16) | (QUEUE_SIZE as u64));
        low_level::write_reg64(super::constants::NVME_ASQ_OFFSET, SUBMISSION_QUEUE_BASE);
        low_level::write_reg64(super::constants::NVME_ACQ_OFFSET, COMPLETION_QUEUE_BASE);

        // 3. Denetleyici Yapılandırmasını (CC) ayarla.
        // Bellek Sayfası Boyutu (MPS) ayarlanabilir. Örnek olarak 4KB (0) kullanıyoruz.
        let mps = 0; // log2(4096) - 12 = 0
        let new_cc = (super::constants::NVME_CC_EN as u32) | (super::constants::NVME_CC_CSS_NVM as u32) | (mps << super::constants::NVME_CC_MPS_SHIFT);
        low_level::write_reg32(super::constants::NVME_CC_OFFSET, new_cc);

        // 4. Denetleyicinin hazır olmasını bekle.
        for _ in 0..100000 { // Zaman aşımı eklemek iyi bir fikirdir
            if low_level::is_controller_ready() {
                return Ok(());
            }
            // Kısa bir gecikme eklenebilir (örneğin, core::hint::spin_loop());
        }

        Err("NVMe denetleyicisi başlatılamadı.")
    }

    // Basit bir okuma komutu gönderir.
    pub unsafe fn read_block(lba: u64, namespace_id: u32, buffer: *mut u8, block_size: u32) -> Result<(), &'static str> {
        use super::structures::NvmeCommand;
        use super::structures::NvmeCompletion;
        use super::constants::*;

        const SUBMISSION_QUEUE_BASE: u64 = 0x1000;
        const COMPLETION_QUEUE_BASE: u64 = 0x2000;

        // Komut kuyruğundan bir boşluk almanız gerekecektir.
        // Bu örnekte, basitçe ilk girişi kullanıyoruz.
        let command_ptr = SUBMISSION_QUEUE_BASE as *mut NvmeCommand;
        let completion_ptr = COMPLETION_QUEUE_BASE as *mut NvmeCompletion;

        let command = NvmeCommand {
            opcode: 0x02, // Okuma komutu
            flags: 0x00,
            cid: 0x1234, // Komut Kimliği
            nsid: namespace_id,
            dptr: [buffer as u64, 0], // Veri Tamponu İşaretçisi
            // Diğer alanlar (örneğin, okuma uzunluğu) ayarlanabilir
        };

        // Komutu gönder.
        *command_ptr = command;

        // Gönderme Kuyruğu Başlığını (SQHD) güncelle.
        let sqhd_offset = NVME_AQA_OFFSET + 4; // ASQ'dan sonraki 4 bayt
        let current_sqhd = low_level::read_reg32(sqhd_offset) as u16;
        low_level::write_reg32(sqhd_offset, (current_sqhd + 1) as u32);

        // Tamamlanmayı bekle.
        for _ in 0..100000 {
            let completion = *completion_ptr;
            if completion.cid == 0x1234 {
                if completion.status == 0 {
                    return Ok(());
                } else {
                    return Err("NVMe okuma komutu başarısız oldu.");
                }
            }
            // Kısa bir gecikme eklenebilir
        }

        Err("NVMe okuma komutu zaman aşımına uğradı.")
    }
}

// Özel çekirdeğinizin giriş noktası.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Güvenli olmayan blok içinde NVMe sürücüsünü başlat.
    unsafe {
        match high_level::initialize_nvme() {
            Ok(_) => {
                // Başlatma başarılı oldu. Şimdi okuma/yazma işlemleri yapabilirsiniz.
                // Örnek olarak, bir blok okuyalım.
                const BLOCK_SIZE: u32 = 512;
                let mut buffer = [0u8; BLOCK_SIZE as usize];
                let namespace_id = 1; // Genellikle 1
                let lba = 0; // İlk blok

                match high_level::read_block(lba, namespace_id, buffer.as_mut_ptr(), BLOCK_SIZE) {
                    Ok(_) => {
                        // Blok başarıyla okundu.
                        // Burada okunan verilerle bir şeyler yapabilirsiniz.
                        // Örneğin, çekirdek günlüğüne yazdırabilirsiniz.
                        // log!("NVMe'den okunan ilk blok: {:?}", buffer);
                    }
                    Err(e) => {
                        // log!("NVMe okuma hatası: {}", e);
                    }
                }
            }
            Err(e) => {
                // log!("NVMe başlatma hatası: {}", e);
            }
        }
    }

    // Özel çekirdeğinizin döngüsü veya kapanma mekanizması.
    loop {}
}

// Panik işleyicisi (çekirdeğiniz için uygun bir panik işleyicisi sağlamanız gerekir).
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}