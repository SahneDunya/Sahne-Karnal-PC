#![no_std]
use core::arch::asm;

// Sayfa boyutu (4KB) - Yaygın ve genellikle iyi bir seçim
pub const PAGE_SIZE: usize = 4096;

// *** ARMv7-A Mimarisi için Sayfa Tablosu Giriş Bayrakları ***
// Bu bayraklar ARMv7-A mimarisine özeldir ve farklı ARM mimarilerinde değişiklik gösterebilir.
// ARM ARMv7-A mimarisi Referans Kılavuzu'na (ARMv7-A Architecture Reference Manual) bakarak
// kullandığınız işlemciye uygun değerleri kontrol etmeniz önemlidir.
pub const PTE_VALID: u32 = 1 << 0;      // Geçerli giriş
pub const PTE_TYPE_PAGE: u32 = 1 << 1;  // Sayfa girişi (blok girişi yerine)
pub const PTE_AP_RW_PRW: u32 = 3 << 4;   // Ayrıcalıklı (kernel) ve ayrıcalıksız (user) için Okuma/Yazma erişimi
pub const PTE_AP_RW_PR: u32 = 2 << 4;    // Ayrıcalıklı için Okuma/Yazma, ayrıcalıksız için Okuma erişimi
pub const PTE_AP_RO_PR: u32 = 1 << 4;    // Ayrıcalıklı ve ayrıcalıksız için sadece Okuma erişimi
pub const PTE_S: u32 = 1 << 10;         // Paylaşım alanı (Shareable) - Çoklu çekirdek sistemlerde önemli
pub const PTE_nG: u32 = 1 << 11;        // Global değil (Non-global) - Genellikle yerel (per-process) eşleştirmeler için
pub const PTE_AF: u32 = 1 << 10;        // Erişim Bayrağı (Access Flag) - Sayfaya erişildiğinde donanım tarafından ayarlanır (yazılımlar için faydalı)
pub const PTE_TEX_NORMAL_NONCACHEABLE: u32 = 0 << 12; // Normal, önbelleğe alınamaz bellek (örn. aygıt belleği)
pub const PTE_TEX_NORMAL_CACHEABLE: u32 = 1 << 12;    // Normal, yazma-geri (write-back), yazma-ayırma (write-allocate) önbelleğe alınabilir bellek
pub const PTE_C: u32 = 1 << 2;           // Önbelleğe alınabilir (Cacheable)
pub const PTE_B: u32 = 1 << 3;           // Arabellekli (Bufferable) - Yazma arabelleklemesi için

// Sayfa tablosu girişi oluşturma fonksiyonu (ARMv7-A'ya uygun bayraklarla)
pub fn pte_create(ppn: u32, flags: u32) -> u32 {
    (ppn << 12) | flags | PTE_TYPE_PAGE | PTE_VALID // PPN, bayraklar, sayfa tipi ve geçerlilik biti birleştiriliyor
}

// MMU'yu başlatma fonksiyonu (2 seviyeli sayfa tablosu örneği)
pub fn init_mmu() {
    // *** 1. Seviye Sayfa Tablosu (L1 Page Table) ***
    // 1MB'lık bölümleri (section) yönetir. 4KB sayfa boyutunda, 4MB adres alanını kapsar.
    static mut L1_PAGE_TABLE: [u32; 4096] = [0; 4096]; // 4KB * 4096 = 16MB (L1 için yeterli)
    let l1_table_address = unsafe { &L1_PAGE_TABLE as *const _ as u32 };

    // *** 2. Seviye Sayfa Tablosu (L2 Page Table) ***
    // 4KB'lık sayfaları yönetir. Her L2 tablosu 1MB'lık bir L1 bölümünü alt bölümlere ayırır.
    static mut L2_PAGE_TABLE: [u32; 4096] = [0; 4096]; // Bir tane L2 tablosu örneği (daha fazlası gerekebilir)
    let l2_table_address = unsafe { &L2_PAGE_TABLE as *const _ as u32 };

    // *** Kimlik Eşleştirmesi (Identity Mapping) Örneği: 0x00000000 - 0x00400000 (4MB) ***
    // İlk 4MB'lık adres alanını (0x00000000 - 0x00400000) hem sanal hem de fiziksel olarak aynı adrese eşleştirelim.
    // Bu örnekte, çekirdek kod ve verisi gibi temel sistem bileşenlerini eşleştirmek için kullanılabilir.

    // 1. L1 Girişini Yapılandır (0. L1 girişi, 0x00000000 - 0x400000 adres aralığı için)
    // L1 tablosunun 0. girişi, 0x00000000 - 0x400000 (4MB) adres aralığını temsil eder.
    // Bu girişe 2. seviye sayfa tablosunun adresini ve gerekli bayrakları yazacağız.
    unsafe {
        L1_PAGE_TABLE[0] = (l2_table_address & 0xFFFF_FC00) | PTE_TYPE_PAGE | PTE_VALID;
        // L2 Tablo Adresi (2. seviye tablo adresinin 12. bit ve üzeri)  | Sayfa Tablosu Tipi | Geçerli
        // Not: & 0xFFFF_FC00  ile son 10 biti (offset) temizliyoruz, çünkü L2 tablo adresi 4KB hizalı olmalı.
    }

    // 2. L2 Girişlerini Yapılandır (0x00000000 - 0x00400000 aralığı için 4KB'lık sayfalar)
    // Her L2 girişi 4KB'lık bir sayfayı temsil eder. 4MB'lık alanı kapsamak için 1024 L2 girişi gereklidir (4MB / 4KB = 1024).
    for i in 0..1024 {
        let physical_address = (i * PAGE_SIZE) as u32; // Fiziksel adres (kimlik eşleştirmesi olduğu için sanal adresle aynı)
        let ppn = physical_address >> 12; // Fiziksel Sayfa Numarası (Physical Page Number)
        unsafe {
            L2_PAGE_TABLE[i] = pte_create(ppn, PTE_AP_RW_PRW | PTE_S | PTE_C | PTE_B | PTE_TEX_NORMAL_CACHEABLE);
            // PTE oluşturma fonksiyonu ile giriş oluşturuluyor.
            // Bayraklar: Okuma/Yazma, Paylaşılabilir, Önbelleklenebilir, Arabellekli, Normal Önbelleklenebilir Bellek
        }
    }

    // *** CP15 Kontrol Register'larını Yapılandırma (ARMv7-A) ***
    unsafe {
        // TTBR0 (Translation Table Base Register 0) - 0. adres alanı için (örn. çekirdek) sayfa tablosu adresi
        // TTBR0'a 1. seviye sayfa tablosunun fiziksel adresini yazıyoruz.
        asm!("mcr p15, 0, {}, c2, c0, 0", in(reg) l1_table_address); // TTBR0'ı ayarla

        // TTB Control Register (TTBCR) - Genellikle 0 olarak ayarlanır (yalnızca TTBR0 kullanılıyorsa)
        asm!("mcr p15, 0, {}, c2, c0, 2", in(reg) 0u32); // TTBCR'yi 0 yap

        // Control Register (SCTLR) - MMU, önbellekler vb. sistem özelliklerini kontrol eder.
        let mut sctlr: u32;
        asm!("mrc p15, 0, {}, c1, c0, 0", out(reg) sctlr); // Mevcut SCTLR değerini oku
        sctlr |= 1 << 0;        // M biti: MMU'yu etkinleştir
        sctlr |= 1 << 11;       // C biti: Veri önbelleğini etkinleştir (isteğe bağlı)
        sctlr |= 1 << 2;        // Z biti: Dallanma tahmini (branch prediction) etkinleştir (isteğe bağlı)
        asm!("mcr p15, 0, {}, c1, c0, 0", in(reg) sctlr); // Güncellenmiş SCTLR değerini yaz
    }

    // *** MMU'yu Etkinleştirme ve Senkronizasyon ***
    unsafe {
        asm!("isb"); // Instruction Synchronization Barrier - Talimat akışını temizle ve değişikliklerin geçerli olmasını sağla
        asm!("dsb"); // Data Synchronization Barrier    - Veri akışını temizle
        asm!("Invalidate TLB"); // TLB'yi (Translation Lookaside Buffer) temizle - Eski çevirmeleri geçersiz kıl
    }
}

// Sanal adresi fiziksel adrese çevirme fonksiyonu (2 seviyeli sayfa tablosu yürüyüşü - Page Table Walk)
pub fn translate_address(virtual_address: u32) -> Option<u32> {
    unsafe {
        let l1_table_address_reg: u32;
        asm!("mrc p15, 0, {}, c2, c0, 0", out(reg) l1_table_address_reg); // TTBR0'dan L1 tablo adresini oku
        let l1_table_address = l1_table_address_reg & 0xFFFF_C000; // Son 14 biti maskele (hizalama için)

        let l1_index = (virtual_address >> 20) & 0xFFF; // Sanal adresin yüksek 12 biti (L1 indeksi)
        let l1_entry_ptr = (l1_table_address + (l1_index * 4)) as *const u32; // L1 girişinin adresi
        let l1_entry = *l1_entry_ptr; // L1 girişini oku

        if (l1_entry & PTE_VALID) == 0 { // Geçerlilik kontrolü
            return None; // Geçersiz L1 girişi
        }

        let l2_table_address = l1_entry & 0xFFFF_FC00; // L2 tablo adresini L1 girişinden al
        let l2_index = (virtual_address >> 12) & 0xFF; // Sanal adresin sonraki 8 biti (L2 indeksi)
        let l2_entry_ptr = (l2_table_address + (l2_index * 4)) as *const u32; // L2 girişinin adresi
        let l2_entry = *l2_entry_ptr; // L2 girişini oku

        if (l2_entry & PTE_VALID) == 0 { // Geçerlilik kontrolü
            return None; // Geçersiz L2 girişi
        }

        let ppn = (l2_entry >> 12) & 0xFFFF_FFFF; // Fiziksel Sayfa Numarası (PPN)
        let page_offset = virtual_address & 0xFFF; // Sayfa içindeki offset (son 12 bit)
        let physical_address = (ppn << 12) | page_offset; // Fiziksel adresi oluştur

        Some(physical_address)
    }
}