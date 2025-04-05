#![no_std]
use core::arch::asm;
use core::panic::PanicInfo;

// Sayfa boyutu (PowerPC için yaygın değer)
pub const PAGE_SIZE: usize = 4096; // 4KB

// Sayfa Tablosu Giriş Bayrakları (PowerPC 603e referansına göre)
pub const PTE_VALID: u32 = 0x80000000;    // Geçerli
pub const PTE_READ: u32 = 0x40000000;     // Okunabilir
pub const PTE_WRITE: u32 = 0x20000000;    // Yazılabilir
pub const PTE_EXECUTE: u32 = 0x10000000;  // Çalıştırılabilir
pub const PTE_USER: u32 = 0x08000000;     // Kullanıcı modu erişimi
pub const PTE_GUARDED: u32 = 0x04000000;  // Korumalı sayfa
pub const PTE_CACHE_INHIBITED: u32 = 0x02000000; // Önbelleğe almayı engelle
pub const PTE_MEMORY_COHERENT: u32 = 0x01000000; // Bellek tutarlılığı gerekli

// Sayfa tablosu girişini oluştur
pub fn pte_create(ppn: u32, flags: u32) -> u32 {
    (ppn << 12) | flags
}

// Statik sayfa tablosu (1. seviye, 4MB adres alanı için 1024 giriş)
static mut PAGE_TABLE: [u32; 1024] = [0; 1024];

// MMU'yu başlat (tek seviyeli sayfa tablosu örneği)
pub unsafe fn init_mmu() {
    // Sayfa tablosu adresini al ve hizala (4KB sınırına)
    let page_table_address = PAGE_TABLE.as_ptr() as usize;
    assert_eq!(page_table_address % PAGE_SIZE, 0, "Sayfa tablosu hizalanmamış!");

    // Sayfa tablosu fiziksel adresini PPN'ye dönüştür (sayfa çerçeve numarası)
    let page_table_ppn = (page_table_address >> 12) as u32;

    // Kimlik eşleştirmesi (0MB - 4MB) ve 3GB - 3GB+4MB arası için örnek eşleştirmeler
    // İlk 4MB'lık sanal adres alanını fiziksel adres alanının ilk 4MB'ına eşleştir
    for i in 0..1024 { // 1024 giriş, her biri 4KB'lık sayfaları temsil eder (4MB toplam)
        let physical_page_number = i as u32; // Fiziksel sayfa numarası (kimlik eşleştirme için)
        PAGE_TABLE[i] = pte_create(
            physical_page_number,
            PTE_VALID | PTE_READ | PTE_WRITE | PTE_EXECUTE | PTE_USER, // Örnek bayraklar
        );
    }
    // Ek olarak, 3GB - 3GB+4MB sanal adres aralığını fiziksel 4MB-8MB arasına eşleyelim (örnek)
    for i in 0..1024 {
        let virtual_page_number = (3 * 1024 * 1024 / PAGE_SIZE + i) as u32; // 3GB + i. sayfa
        let physical_page_number = (1024 + i) as u32; // Fiziksel 4MB + i. sayfa (1024 sayfa = 4MB)
        PAGE_TABLE[virtual_page_number as usize] = pte_create(
            physical_page_number,
            PTE_VALID | PTE_READ | PTE_WRITE | PTE_EXECUTE | PTE_USER, // Örnek bayraklar
        );
    }


    // SDR1 register'ını sayfa tablosu adresi ile ayarla
    asm!("mtsdr1 {}, {}", in(reg) page_table_address);

    // MMU'yu etkinleştir (PowerPC 603e için MAS0 register'ı ile örnek)
    // MAS0[MMUON] bitini (bit 0) 1 yaparak MMU'yu etkinleştiririz.
    // Diğer MAS0 ayarları varsayılan değerlerinde bırakılmıştır (örnek için basitleştirme).
    asm!("mtmsr 0x00008000"); // Sadece SR[IS] ve SR[DS] bitlerini 1 yapıyoruz, MMUON için yeterli olabilir.
                                // Tam MMU etkinleştirme adımları ve diğer MAS register ayarları
                                // işlemci modeline ve gereksinimlere göre değişir.
                                // Daha detaylı bilgi için PowerPC mimari referansına bakılmalıdır.


    // TLB'yi temizle (PowerPC'ye özel komut - örnek olarak 'tlbclr' - işlemciye göre değişebilir)
    // TLB temizleme komutu mimariye özgüdür ve her PowerPC işlemcisinde aynı olmayabilir.
    // Bu örnek sadece bir gösterimdir ve gerçek sistemde doğru komutun kullanıldığından emin olunmalıdır.
    asm!("tlbclr"); // Tüm TLB girişlerini temizle (genel bir komut, işlemciye göre değişebilir)


    // Diğer MMU ayarlarını yap (cache, koruma vb. - bu örnekte basitleştirildi)
    // ... (Örneğin, MAS1, MAS2, MAS3 register'ları cache ve koruma ayarları için kullanılabilir)
}

// Verilen sanal adresi fiziksel adrese çevir (tek seviyeli sayfa tablosu örneği)
pub fn translate_address(virtual_address: u32) -> Option<u32> {
    // 1. Sayfa tablosu indeksini hesapla (sanal adresin yüksek 10 bitini al)
    let page_table_index = (virtual_address >> 22) & 0x3FF; // 10 bit maske (1024 giriş)

    // 2. Sayfa tablosu girişini al
    let pte = unsafe { PAGE_TABLE[page_table_index as usize] };

    // 3. Geçerlilik bayrağını kontrol et
    if (pte & PTE_VALID) == 0 {
        return None; // Geçersiz sayfa tablosu girişi, çevirme başarısız
    }

    // 4. Fiziksel sayfa numarasını (PPN) PTE'den çıkar
    let physical_page_number = (pte >> 12) & 0xFFFFF; // 20 bit PPN maskesi (4GB fiziksel adres alanı varsayımı)

    // 5. Sayfa içi offset'i (sanal adresin düşük 12 biti) al
    let page_offset = virtual_address & 0xFFF; // 12 bit maske (4KB sayfa boyutu)

    // 6. Fiziksel adresi oluştur
    let physical_address = (physical_page_number << 12) | page_offset;

    Some(physical_address) // Fiziksel adresi döndür
}


// Panik durumunda sonsuz döngüye gir (no_std ortamı için)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}