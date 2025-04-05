#![no_std]
use core::ptr::{write_volatile, read_volatile};

// PLIC taban adresi (sabit)
const PLIC_BASE: usize = 0x0C000000;

// PLIC kayıt adresleri için offsetler (daha okunaklı ve organize)
const PRIORITY_BASE_OFFSET: usize = 0x0;
const ENABLE_BASE_OFFSET: usize = 0x2000; // Etkinleştirme kayıtları 0x2000 adresinden başlar (tipik PLIC)

// Hart (donanım iş parçacığı) ID'si. Genellikle 0'dan başlar.
const HART_ID: usize = 0;

pub struct Plic {
    base_address: usize,
    hart_id: usize,
}

impl Plic {
    pub fn new(base_address: usize, hart_id: usize) -> Self {
        Plic {
            base_address,
            hart_id,
        }
    }

    pub fn init(&self) {
        // PLIC genel yapılandırması (şu anda örnek için boş, gerçek sistemde gerekebilir)
        // Örneğin, tüm öncelikleri varsayılan değerlere ayarlamak gibi
        // ...
        self.disable_all_interrupts(); // Başlangıçta tüm kesmeleri devre dışı bırakmak iyi bir uygulamadır.
    }

    pub fn set_priority(&self, interrupt_number: usize, priority: u8) {
        // Kesme önceliğini ayarlayın.
        // Öncelik değeri genellikle 0 (en düşük) ile 7 (en yüksek) arasındadır.
        let priority_address = self.base_address + PRIORITY_BASE_OFFSET + interrupt_number * 4; // Her kesme için 4 byte
        unsafe {
            write_volatile(priority_address as *mut u32, priority as u32); // u32 olarak yazıyoruz çünkü kayıtlar genellikle 32 bit
        }
    }

    pub fn enable_interrupt(&self, interrupt_number: usize) {
        // Belirli bir kesmeyi etkinleştirin.
        let enable_address = self.get_enable_address();
        let enable_bit_mask = 1 << (interrupt_number % 32); // Her kayıt 32 kesmeyi kontrol eder.

        unsafe {
            let current_enable_value = read_volatile(enable_address as *const u32);
            write_volatile(
                enable_address as *mut u32,
                current_enable_value | enable_bit_mask, // Mevcut değer ile OR işlemi yaparak biti ayarlayın.
            );
        }
    }

    pub fn disable_interrupt(&self, interrupt_number: usize) {
        // Belirli bir kesmeyi devre dışı bırakın.
        let enable_address = self.get_enable_address();
        let disable_bit_mask = !(1 << (interrupt_number % 32)); // Devre dışı bırakmak için maskeyi tersleyin.

        unsafe {
            let current_enable_value = read_volatile(enable_address as *const u32);
            write_volatile(
                enable_address as *mut u32,
                current_enable_value & disable_bit_mask, // Mevcut değer ile AND işlemi yaparak biti temizleyin.
            );
        }
    }

    pub fn disable_all_interrupts(&self) {
        // Tüm kesmeleri devre dışı bırakır (belirli bir hart için).
        let enable_address = self.get_enable_address();
        unsafe {
            write_volatile(enable_address as *mut u32, 0x00000000); // Etkinleştirme kaydına 0 yazarak tüm bitleri temizleyin.
        }
    }


    // Yardımcı fonksiyon: Etkinleştirme kayıt adresini hesaplar (hart ID'sine göre)
    fn get_enable_address(&self) -> usize {
        self.base_address + ENABLE_BASE_OFFSET + 0x100 * self.hart_id // Her hart için 0x100 offset
    }


    // ... diğer PLIC fonksiyonları (örneğin kesme talebi okuma, tamamlama vb.)
}


// Örnek Kullanım (tek örnek gösterimi)
pub fn main() {
    let plic = Plic::new(PLIC_BASE, HART_ID); // PLIC örneği oluştur

    plic.init(); // PLIC'i başlat (şu an sadece tüm kesmeleri devre dışı bırakıyor)

    // 1 numaralı kesmenin önceliğini 7 (en yüksek) olarak ayarla.
    plic.set_priority(1, 7);

    // 1 numaralı kesmeyi etkinleştir.
    plic.enable_interrupt(1);

    // ... burada kesmeler gerçekleştiğinde işlenecek kod olabilir ...
}