#![no_std]
use core::arch::asm;

// RISC-V standart kesme bitleri (64 bit uyumlu)
const MIE_BIT: usize = 1 << 3;     // Machine Interrupt Enable
const MTIE_BIT: usize = 1 << 7;    // Machine Timer Interrupt Enable

extern "C" {
    static IVT: u8; // Kesme vektör tablosu (başlangıç adresi)
}

pub fn init() {
    unsafe {
        // 1. MTVEC'i ayarla (Kesme Vektör Tablosu Adresi)
        let ivt_address = &IVT as *const u8 as usize;
        asm!("csrw mtvec, {0}", in(reg) ivt_address);

        // 2. Makine Modu Zamanlayıcı Kesmesini Etkinleştir
        asm!("csrrs mie, {0}, zero", in(reg) MTIE_BIT);

        // 3. Genel Kesmeleri Etkinleştir (MIE biti ile)
        asm!("csrrs mstatus, {0}, zero", in(reg) MIE_BIT);
    }
    // exception::init(); // İstisna işleyicisini başlat (Bu örnekte basitleştirme adına çıkarıldı)
}

// Örnek Makine Modu Zamanlayıcı Kesme İşleyicisi
#[no_mangle]
pub extern "C" fn MachineTimer_interrupt_handler() {
    // Zamanlayıcı kesmesi gerçekleştiğinde yapılacak işlemler buraya gelir.
    // Örneğin, bir sayacı artırabilir, bir görevi tetikleyebilir veya durum bilgisini güncelleyebilirsiniz.

    // ÖNEMLİ: Zamanlayıcı kesme bayrağını temizleyin (gerekirse DONANIMA ÖZGÜ işlem).
    // RISC-V zamanlayıcı kesmeleri genellikle otomatik olarak temizlenir (mtime ve mtimecmp karşılaştırması ile).
    // Ancak, bazı donanımlarda manuel temizleme gerekebilir.
    // Referans kılavuzunuzu kontrol edin.

    // Örnek olarak basit bir işlem:
    static mut TIMER_COUNT: usize = 0;
    unsafe {
        TIMER_COUNT += 1;
        // İsteğe bağlı: TIMER_COUNT değerini bir yere yazdırabilir veya loglayabilirsiniz.
        // Örneğin, bir UART üzerinden veya bir hata ayıklama arayüzü aracılığıyla.
    }

    // İleri Düzey (Gerekirse): Bir sonraki zamanlayıcı kesmesini ayarlamak için mtimecmp değerini güncelleyebilirsiniz.
    // Bu, periyodik zamanlayıcı kesmeleri oluşturmanıza olanak tanır.
}