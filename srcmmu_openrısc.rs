#![no_std]

use core::arch::asm;

// Sayfa boyutu (OpenRISC için tipik olarak 4KB)
pub const PAGE_SIZE: usize = 4096;

// Sayfa tablosu giriş bayrakları (OpenRISC spesifikasyonuna göre)
pub const PTE_VALID: u32 = 1 << 0; // Geçerli
pub const PTE_READ: u32 = 1 << 1; // Okunabilir
pub const PTE_WRITE: u32 = 1 << 2; // Yazılabilir
// ... diğer bayraklar (örneğin, PTE_EXECUTE, PTE_USER, PTE_DIRTY, PTE_ACCESSED)

// Sayfa tablosu girişini oluştur
pub fn pte_create(ppn: u32, flags: u32) -> u32 {
    (ppn << 12) | flags // PPN (Fiziksel Sayfa Numarası) ve bayrakları birleştir
}

// MMU'yu başlat
pub fn init_mmu() {
    // 1. Seviye sayfa tablosu için bellek ayır (statik olarak)
    static mut PAGE_TABLE: [u32; 1024] = [0; 1024]; // 4KB * 1024 giriş = 4MB boyutunda sayfa tablosu (Örnek boyut, 4GB adres alanını kapsar eğer 4KB sayfalar kullanılıyorsa ve tek seviyeli sayfa tablosu)

    let page_table_address = unsafe { &PAGE_TABLE as *const _ as u32 };

    // Kimlik eşleştirmesi için ilk 1MB'ı yapılandır (0x00000000 - 0x000FFFFF aralığı)
    // Bu örnek, 4KB'lık sayfaları ve tek seviyeli sayfa tablosunu varsayar.
    // İlk 256 sayfa tablosu girişi (PTE), ilk 1MB fiziksel belleği kimlik olarak eşleyecek.
    for i in 0..256 { // İlk 1MB için 256 sayfa (1MB / 4KB = 256)
        unsafe {
            PAGE_TABLE[i] = pte_create(i as u32, PTE_VALID | PTE_READ | PTE_WRITE);
        }
    }

    // Sayfa tablosu adresini MMU kontrol register'ına yükle
    unsafe {
        // OR1K_MMU_CONTROL register adresi (örnek olarak 0x100 verilmişti, gerçek değere göre değiştirilmeli)
        asm!("mtsr {:r}, or1k_mmu_control", in(reg) page_table_address); // 'or1k_mmu_control' yerine gerçek register adı veya adresi kullanılmalı.
    }

    // MMU'yu etkinleştir (OpenRISC'de farklı olabilir, OR1K_SR_MMU bitini ayarlayarak yapılır)
    unsafe {
        // SR register'ı oku
        let mut sr_value: u32;
        asm!("mfsr {:r}", out(reg) sr_value);

        // SR_MMU bitini ayarla (OpenRISC mimarisine özgü)
        sr_value |= (1 << 11); // SR_MMU biti genellikle 11. bit pozisyonundadır (kontrol etmeniz gerekebilir)

        // Güncellenmiş SR değerini geri yaz
        asm!("mtsr {:r}, sr", in(reg) sr_value);
    }
}

// Sanal adresi fiziksel adrese çevir (basit bir örnek sayfa tablosu yürüyüşü ile)
pub fn translate_address(virtual_address: u32) -> Option<u32> {
    // 1. Sayfa tablosu temel adresini MMU kontrol register'ından al (veya statik olarak saklanan adresi kullan)
    static mut PAGE_TABLE: [u32; 1024] = [0; 1024]; // Statik sayfa tablosu referansı (init_mmu içinde aynı tabloya referans verdiğimizi varsayıyoruz)
    let page_table_base_address = unsafe { &PAGE_TABLE as *const _ as u32 };

    // 2. Sanal adresten sayfa tablosu indeksini çıkar (üst 10 bit, eğer 4KB sayfalar ve tek seviyeli tablo ise)
    let page_table_index = (virtual_address >> 12) & 0x3FF; // 0x3FF = 1023 (10 bit maskesi)

    // 3. Sayfa tablosu girişini (PTE) sayfa tablosundan oku
    let pte_address = page_table_base_address + (page_table_index * 4); // Her PTE 4 byte boyutunda
    let pte = unsafe { *(pte_address as *const u32) };

    // 4. PTE'nin geçerli bitini kontrol et
    if (pte & PTE_VALID) == 0 {
        return None; // Geçersiz sayfa
    }

    // 5. PTE'den fiziksel sayfa numarasını (PPN) çıkar
    let ppn = (pte >> 12) & 0xFFFFF; // 20 bit PPN (örnek olarak, gerçek OpenRISC'e göre değişebilir)

    // 6. Fiziksel adresi oluştur (PPN ve sanal adresin sayfa içi offset'ini birleştir)
    let page_offset = virtual_address & 0xFFF; // Sayfa içi offset (12 bit maskesi, 4KB sayfalar için)
    let physical_address = (ppn << 12) | page_offset;

    Some(physical_address)
}