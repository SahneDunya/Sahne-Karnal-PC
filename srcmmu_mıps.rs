#![no_std]

// Sayfa boyutu (4KB)
pub const PAGE_SIZE: usize = 4096;

// Sayfa tablosu giriş bayrakları (MIPS benzeri)
pub const PTE_V: u32 = 1 << 0; // Geçerli (Valid)

// Sayfa tablosu girişini oluştur
pub fn pte_create(ppn: u32, flags: u32) -> u32 {
    (ppn << 12) | flags // Fiziksel Sayfa Numarası (PPN) ve bayrakları birleştir
}

// Bellek Yönetim Birimini (MMU) başlat
pub fn init_mmu() {
    // 1. Seviye sayfa tablosu için statik bellek alanı (4KB * 1024 = 4MB)
    static mut PAGE_TABLE: [u32; 1024] = [0; 1024];

    // Sayfa tablosunun başlangıç adresini al
    let page_table_address = unsafe { &PAGE_TABLE as *const _ as u32 };

    // *** Kimlik Eşleştirmesi (Identity Mapping) Yapılandırması ***
    // İlk 4MB'lık bellek bölgesini (1024 sayfa) kimliğe eşle
    // Sanal adres 0x0 - 0x3FFFFF (4MB) -> Fiziksel adres 0x0 - 0x3FFFFF (4MB)
    for i in 0..1024 {
        // Her sayfa için Fiziksel Sayfa Numarası (PPN), sayfa indeksine eşittir (kimlik eşleştirmesi)
        let physical_page_number = i as u32;
        // Sayfa tablosu girişini oluştur: PPN ve Geçerli bayrağı
        unsafe {
            PAGE_TABLE[i] = pte_create(physical_page_number, PTE_V); // Sadece 'Geçerli' bayrağı ayarlı
        }
    }

    // TLB'yi temizle (MIPS'te farklı bir mekanizma olabilir, bu sadece kavramsal bir örnektir)
    unsafe {
        asm!("mtc0 $0, $8\n\t"); // Index register'ı temizle (Kavramsal TLB temizleme)
        asm!("tlbwi\n\t");         // TLB'ye yaz (Kavramsal TLB temizleme)
    }

    // Sayfa tablosunun adresini MMU'nun sayfa tablosu taban adres kayıtçısına yükle
    // (MIPS'te bu işlem farklı bir kayıtçıya yapılabilir, bu örnek sadece kavramsal)
    unsafe {
        asm!("mtc0 $1, $26", in(reg) page_table_address); // Sayfa tablosu taban adresini ayarla (Kavramsal)
    }
}

// Sanal adresi fiziksel adrese çevir (basit kimlik eşleştirmesi örneği için)
pub fn translate_address(virtual_address: u32) -> Option<u32> {
    // *** DONANIM BAĞIMLI KOD BURAYA GELECEK (Gerçek sayfa tablosu yürüyüşü daha karmaşık olabilir) ***
    // Bu örnek, tek seviyeli sayfa tablosu ve kimlik eşleştirmesi varsayar.

    if virtual_address < 4 * 1024 * 1024 { // İlk 4MB bölge içinde mi? (Kimlik eşleştirmesi aralığı)
        // Kimlik eşleştirmesi: Sanal adres = Fiziksel adres
        Some(virtual_address)
    } else {
        // Kimlik eşleştirmesi aralığı dışında, dönüşüm yok (veya farklı bir eşleme olabilir)
        None // Bu örnekte, eşlenmemiş adresler için 'None' dönüyoruz.
    }
}