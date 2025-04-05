#![no_std]
use core::ptr::{write_volatile, read_volatile};

// Sabitler için modüler bir yapı tanımlıyoruz
mod constants {
    // GIC Dağıtıcı ve CPU arayüzü taban adresleri (örnek değerler, gerçek sistemde ayarlanmalı)
    pub const GIC_DIST_BASE: usize = 0x0D000000;
    pub const GIC_CPU_BASE: usize = 0x0C000000;

    // GIC Dağıtıcı (Distributor) Kayıt Offsetleri
    pub const GICD_CTLR: usize = 0x000;        // Kontrol Kaydı
    pub const GICD_TYPER: usize = 0x004;       // Tip Kaydı
    pub const GICD_IGROUPR0: usize = 0x080;    // Kesme Gruplandırma Kayıtları
    pub const GICD_ISENABLER_BASE: usize = 0x100; // Kesme Etkinleştirme Ayar Kayıtları Başlangıcı
    pub const GICD_ICENABLER_BASE: usize = 0x180; // Kesme Etkinleştirme Temizleme Kayıtları Başlangıcı
    pub const GICD_IPRIORITYR_BASE: usize = 0x400; // Kesme Öncelik Kayıtları Başlangıcı

    // GIC CPU Arayüzü Kayıt Offsetleri
    pub const GICC_CTLR: usize = 0x00;   // Kontrol Kaydı
    pub const GICC_PMR: usize = 0x04;    // Öncelik Maskeleme Kaydı
    pub const GICC_IAR: usize = 0x0C;    // Kesme Onaylama Kaydı
    pub const GICC_EOIR: usize = 0x10;   // Kesme Bitirme Kaydı
    pub const GICC_HPPIR: usize = 0x14;  // En Yüksek Öncelikli Bekleyen Kesme Kaydı

    // Maksimum kesme sayısı (örnek değer, GIC tipine göre değişir)
    pub const MAX_INTERRUPT_COUNT: usize = 1020; // GICv2'de genellikle 1020 kesme desteklenir
}

// GIC yapılandırmasını temsil eden yapı
pub struct Gic {
    dist_base: usize,
    cpu_base: usize,
    hart_id: usize,
}

impl Gic {
    /// Yeni bir GIC örneği oluşturur
    pub const fn new(dist_base: usize, cpu_base: usize, hart_id: usize) -> Self {
        Self {
            dist_base,
            cpu_base,
            hart_id,
        }
    }

    /// GIC'i başlatır
    pub fn init(&self) {
        unsafe {
            // Dağıtıcıyı etkinleştir (Bit 0: Enable Group 0, Bit 1: Enable Group 1)
            write_volatile(
                (self.dist_base + constants::GICD_CTLR) as *mut u32,
                0x1,
            );

            // CPU arayüzünü etkinleştir
            write_volatile(
                (self.cpu_base + constants::GICC_CTLR) as *mut u32,
                0x1,
            );

            // Öncelik maskesini tüm kesmelere izin verecek şekilde ayarla (0xFF)
            write_volatile(
                (self.cpu_base + constants::GICC_PMR) as *mut u32,
                0xFF,
            );
        }
        self.disable_all_interrupts();
    }

    /// Belirli bir kesmenin önceliğini ayarlar
    pub fn set_priority(&self, interrupt_id: usize, priority: u8) {
        if interrupt_id >= constants::MAX_INTERRUPT_COUNT {
            return; // Hata: Geçersiz kesme ID'si
        }

        let offset = (interrupt_id / 4) * 4; // Her kayıt 4 kesme için öncelik tutar
        let shift = (interrupt_id % 4) * 8;  // Her kesme 8 bit öncelik kullanır
        let addr = self.dist_base + constants::GICD_IPRIORITYR_BASE + offset;

        unsafe {
            let current = read_volatile(addr as *const u32);
            let mask = !(0xFF << shift); // İlgili 8 biti temizle
            let new_value = (current & mask) | ((priority as u32) << shift);
            write_volatile(addr as *mut u32, new_value);
        }
    }

    /// Belirli bir kesmeyi etkinleştirir
    pub fn enable_interrupt(&self, interrupt_id: usize) {
        if interrupt_id >= constants::MAX_INTERRUPT_COUNT {
            return; // Hata: Geçersiz kesme ID'si
        }

        let reg_offset = (interrupt_id / 32) * 4; // Her kayıt 32 kesmeyi kontrol eder
        let bit_pos = interrupt_id % 32;
        let addr = self.dist_base + constants::GICD_ISENABLER_BASE + reg_offset;

        unsafe {
            let current = read_volatile(addr as *const u32);
            write_volatile(addr as *mut u32, current | (1 << bit_pos));
        }
    }

    /// Belirli bir kesmeyi devre dışı bırakır
    pub fn disable_interrupt(&self, interrupt_id: usize) {
        if interrupt_id >= constants::MAX_INTERRUPT_COUNT {
            return; // Hata: Geçersiz kesme ID'si
        }

        let reg_offset = (interrupt_id / 32) * 4;
        let bit_pos = interrupt_id % 32;
        let addr = self.dist_base + constants::GICD_ICENABLER_BASE + reg_offset;

        unsafe {
            write_volatile(addr as *mut u32, 1 << bit_pos); // ICENABLER'da 1 yazmak ilgili biti sıfırlar
        }
    }

    /// Tüm kesmeleri devre dışı bırakır
    pub fn disable_all_interrupts(&self) {
        unsafe {
            for i in (0..constants::MAX_INTERRUPT_COUNT).step_by(32) {
                let addr = self.dist_base + constants::GICD_ICENABLER_BASE + (i / 32) * 4;
                write_volatile(addr as *mut u32, 0xFFFFFFFF); // Tüm bitleri temizle
            }
        }
    }

    /// Bekleyen bir kesmeyi talep eder
    pub fn claim_interrupt(&self) -> u32 {
        unsafe { read_volatile((self.cpu_base + constants::GICC_IAR) as *const u32) }
    }

    /// Kesme işlemini tamamlar
    pub fn complete_interrupt(&self, interrupt_id: u32) {
        unsafe {
            write_volatile((self.cpu_base + constants::GICC_EOIR) as *mut u32, interrupt_id);
        }
    }
}

/// Örnek kullanım
pub fn main() {
    let gic = Gic::new(constants::GIC_DIST_BASE, constants::GIC_CPU_BASE, 0);
    gic.init();

    // Kesme 30'un önceliğini 64 olarak ayarla ve etkinleştir
    gic.set_priority(30, 64);
    gic.enable_interrupt(30);

    // Kesme işleme örneği
    let interrupt_id = gic.claim_interrupt();
    // Kesme işleme kodu burada yer alır
    gic.complete_interrupt(interrupt_id);
}