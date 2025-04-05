#![no_std]
use core::ptr::{write_volatile, read_volatile};

// Yerel APIC'in taban adresi. Genellikle sabittir ve MSR (Model Specific Register) aracılığıyla öğrenilir.
// Örnek bir adres (gerçek sistemde farklı olabilir, MSR'den okunmalıdır).
const LAPIC_BASE: usize = 0xFEE00000;

// Yerel APIC kayıt offsetleri (Intel Manuel'inden alınmıştır)
const LAPIC_ID_OFFSET: usize = 0x20;        // APIC ID Kaydı
const LAPIC_EOI_OFFSET: usize = 0xB0;       // EOI (End of Interrupt) Kaydı
const LAPIC_SVR_OFFSET: usize = 0x80;       // SVR (Spurious Vector Register) - Sahte Kesme Vektör Kaydı
const LAPIC_ICR_LOW_OFFSET: usize = 0x300;   // ICR (Interrupt Command Register) Düşük kısım - IPI göndermek için
const LAPIC_ICR_HIGH_OFFSET: usize = 0x310;  // ICR (Interrupt Command Register) Yüksek kısım - Hedef İşlemci Belirleme

pub struct LocalApic {
    base_address: usize,
}

impl LocalApic {
    pub fn new(base_address: usize) -> Self {
        LocalApic { base_address }
    }

    pub fn init(&self) {
        // Yerel APIC'i başlat (örneğin, sahte kesme vektörünü etkinleştir)
        self.enable_spurious_interrupt();
    }

    pub fn get_apic_id(&self) -> u32 {
        // Yerel APIC ID'sini oku
        let id_address = self.base_address + LAPIC_ID_OFFSET;
        unsafe { read_volatile(id_address as *const u32) }
    }

    pub fn send_eoi(&self) {
        // EOI (End of Interrupt) sinyali gönder. Kesme işleyicinin sonunda çağrılmalıdır.
        let eoi_address = self.base_address + LAPIC_EOI_OFFSET;
        unsafe { write_volatile(eoi_address as *mut u32, 0x0); /* Değer önemli değil, sadece yazma işlemi EOI sinyali gönderir */ }
    }

    pub fn enable_spurious_interrupt(&self) {
        // Sahte kesme vektörünü etkinleştir. Genellikle önerilen bir başlangıç yapılandırmasıdır.
        let svr_address = self.base_address + LAPIC_SVR_OFFSET;
        unsafe {
            let current_svr_value = read_volatile(svr_address as *const u32);
            write_volatile(svr_address as *mut u32, current_svr_value | 0x100 | 0xFF); // Vektör 0xFF ve APIC yazılımını etkinleştir (bit 8)
        }
    }


    pub fn send_ipi(&self, destination_apic_id: u32, vector: u8) {
        // İşlemciler arası kesme (IPI) gönder
        unsafe {
            // Önce yüksek kısmı ayarla (hedef APIC ID'si)
            write_volatile(
                (self.base_address + LAPIC_ICR_HIGH_OFFSET) as *mut u32,
                destination_apic_id << 24, // Hedef APIC ID'si yüksek 8 bitte (24-31)
            );

            // Sonra düşük kısmı ayarla (vektör ve diğer kontrol bitleri) - En son yazılmalıdır!
            write_volatile(
                (self.base_address + LAPIC_ICR_LOW_OFFSET) as *mut u32,
                (vector as u32) /* Vektör */ | 0x40000 /* Sabit teslimat modu */ | 0 /* Fiziksel hedefleme */,
            );
        }
    }

    // ... diğer Yerel APIC fonksiyonları (örneğin, zamanlayıcı yapılandırması, vb.) ...
}

// Örnek Kullanım
pub fn main() {
    let lapic = LocalApic::new(LAPIC_BASE);
    lapic.init();

    let my_apic_id = lapic.get_apic_id();
    // ... APIC ID'si ile ilgili işlemler ...

    // Başka bir işlemciye IPI gönder (örnek olarak APIC ID'si 1 olan işlemciye, vektör 0x50 ile)
    lapic.send_ipi(1, 0x50);

    // ... kesme işleme kodu ...
    // Kesme işleyicinin sonunda EOI gönderilmeli: lapic.send_eoi();
}