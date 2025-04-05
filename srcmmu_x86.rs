#![no_std]
use core::arch::x86_64::{
    __readcr3,
    __writecr3,
    invlpg,
};
use core::ptr::addr_of_mut; // Daha güvenli statik mut erişimi için

// Sayfa boyutu (4KB)
pub const PAGE_SIZE: usize = 4096;

// Sayfa tablosu giriş bayrakları (x86_64) - Daha anlamlı isimler
pub const PTE_PRESENT: u64 = 1 << 0;   // Geçerli
pub const PTE_WRITABLE: u64 = 1 << 1;  // Yazılabilir
pub const PTE_USER_SUPERVISOR: u64 = 1 << 2; // Kullanıcı/Denetleyici (User/Supervisor)
// ... diğer bayraklar (Örneğin, PTE_PAGE_SIZE: Büyük sayfalar için)

// Sayfa tablosu girişini oluştur - Fonksiyonel yaklaşım
#[inline]
pub fn pte_create(physical_address: u64, flags: u64) -> u64 {
    physical_address | flags
}

// Sayfa tablosunu başlat (4 seviyeli sayfa tablosu)
pub fn init_mmu() {
    // Sayfa tabloları için statik mut diziler - Daha okunabilir isimler ve ayrı tanımlar
    static mut PML4_TABLE: [u64; 512] = [0; 512];
    static mut PDPT_TABLE: [u64; 512] = [0; 512];
    static mut PDT_TABLE: [u64; 512] = [0; 512];
    static mut PT_TABLE: [u64; 512] = [0; 512];

    // Tablo adreslerini daha güvenli bir şekilde al ve sabitlere ata
    let pml4_address = unsafe { addr_of_mut!(PML4_TABLE) as u64 };
    let pdpt_address = unsafe { addr_of_mut!(PDPT_TABLE) as u64 };
    let pdt_address = unsafe { addr_of_mut!(PDT_TABLE) as u64 };
    let pt_address = unsafe { addr_of_mut!(PT_TABLE) as u64 };

    // Kimlik eşleştirmesi için girişleri yapılandır (Örnek)
    unsafe {
        // PML4 -> PDPT
        PML4_TABLE[0] = pte_create(pdpt_address, PTE_PRESENT | PTE_WRITABLE);

        // PDPT -> PDT
        PDPT_TABLE[0] = pte_create(pdt_address, PTE_PRESENT | PTE_WRITABLE);

        // PDT -> PT
        PDT_TABLE[0] = pte_create(pt_address, PTE_PRESENT | PTE_WRITABLE);

        // PT -> Sayfa (Kimlik Eşleştirmesi - İlk 1GB)
        for i in 0..256 { // 1GB / 4KB = 256 sayfa
            PT_TABLE[i] = pte_create((i as u64) * PAGE_SIZE as u64, PTE_PRESENT | PTE_WRITABLE);
        }
    }

    // CR3 register'ını ayarla (PML4'ün fiziksel adresi)
    unsafe {
        __writecr3(pml4_address);
    }

    // Bellek yönetimini etkinleştir (CR0 register'ındaki PG bitini ayarlayarak)
    // **ÖNEMLİ İYİLEŞTİRME**: PG bitini ayarlamak için inline assembly kullanıyoruz.
    unsafe {
        asm!(
            "mov cr0, %rax",
            "or rax, 0x80000000", // PG bitini (31. bit) set et
            inout("rax") { core::arch::x86_64::__readcr0() } => _, // Mevcut CR0 değerini oku ve üzerine yaz
            options(nostack, preserves_flags)
        );
    }


    // TLB'yi temizle (isteğe bağlı) - Açıklama eklendi
    unsafe {
       invlpg(core::ptr::null_mut()); // Sanal adres 0 için TLB temizleme - Daha güvenli null pointer kullanımı
       // Not: `invlpg` talimatı, verilen sanal adresle eşleşen TLB girişlerini temizler.
       // `null_mut()` kullanmak genellikle tüm TLB'yi temizlemek için yeterlidir.
    }

    // İşlem tamamlandı mesajı (isteğe bağlı)
    // println!("MMU başlatıldı ve kimlik eşleştirmesi yapılandırıldı."); // Eğer `println!` kullanılabilirse
}


// Sanal adresi fiziksel adrese çevir (henüz tam olarak uygulanmadı) - Fonksiyon şimdilik aynı kalıyor
pub fn translate_address(virtual_address: u64) -> Option<u64> {
    // TODO: Sayfa tablosu yürüyüşünü burada gerçekleştir.
    // Bu fonksiyon, 4 seviyeli sayfa tablosunu kullanarak
    // verilen sanal adrese karşılık gelen fiziksel adresi bulmalıdır.

    // Şu an sadece kimlik eşleştirmesi yapıyor (GELİŞTİRİLECEK)
    Some(virtual_address)
}