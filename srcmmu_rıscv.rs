#![no_std]
use core::arch::asm;

// Sayfa boyutu (4KB)
pub const PAGE_SIZE: usize = 4096;

// Sayfa tablosu giriş bayrakları (RISC-V)
pub const PTE_V: u64 = 1 << 0; // Geçerli
pub const PTE_R: u64 = 1 << 1; // Okunabilir
pub const PTE_W: u64 = 1 << 2; // Yazılabilir
pub const PTE_X: u64 = 1 << 3; // Çalıştırılabilir

// Sayfa tablosu girişi oluşturma fonksiyonu
pub fn pte_create(ppn: u64, flags: u64) -> u64 {
    (ppn << 10) | flags
}

// MMU'yu başlatma fonksiyonu (basitleştirilmiş tek seviyeli örnek)
pub fn init_mmu() {
    // Statik olarak sayfa tablosu tanımla (4KB boyutunda, 512 giriş)
    static mut PAGE_TABLE: [u64; 512] = [0; 512];

    // Sayfa tablosunun fiziksel adresini al
    let page_table_address = unsafe { &PAGE_TABLE as *const _ as u64 };

    // Tek bir 4MB bölge için kimlik eşleştirmesi yap (0x0 - 0x400000)
    // İlk sayfa tablosu girişi (indeks 0), ilk 4MB sanal adresi eşleyecek
    unsafe {
        // Sayfa tablosunun PPN'sini (Fiziksel Sayfa Numarası) hesapla (12 bit sağa kaydır)
        let page_table_ppn = page_table_address >> 12;

        // İlk girişi oluştur: Sayfa tablosunun kendisi için geçerli, okunabilir, yazılabilir ve çalıştırılabilir izinleri ver
        // Bu, basit bir örnekte, sayfa tablosunun bellekte nerede olduğunu ve erişim izinlerini tanımlar.
        PAGE_TABLE[0] = pte_create(page_table_ppn, PTE_V | PTE_R | PTE_W | PTE_X);
    }

    // satp (Supervisor Address Translation and Protection) register'ını yapılandır
    unsafe {
        // Sv39 modunu (8) ve sayfa tablosunun fiziksel adresini satp'ye yaz
        let satp_value = 8 << 60 | (page_table_address >> 12);
        asm!("csrw satp, {}", in(reg) satp_value);
    }

    // TLB'yi (Translation Lookaside Buffer) temizle (MMU etkinleştirmesi için gerekli)
    unsafe {
        asm!("sfence.vma zero, zero");
    }
}

// Sanal adresi fiziksel adrese çevirme fonksiyonu (basitleştirilmiş örnek)
pub fn translate_address(virtual_address: u64) -> Option<u64> {
    // Bu çok basit örnek sadece kimlik eşleştirmesi varsayar ve gerçek bir çevirme yapmaz.
    // Gerçek bir sistemde, sayfa tablosunda yürüyerek fiziksel adres bulunur.
    // Bu örnekte, sanal adresin doğrudan fiziksel adres olduğunu varsayıyoruz (kimlik eşleştirmesi).
    Some(virtual_address)
}

// Örnek kullanım (MMU'yu başlat ve bir adresi çevirmeye çalış)
fn main() {
    init_mmu(); // MMU'yu başlat

    let virtual_address: u64 = 0x12345; // Örnek bir sanal adres

    if let Some(physical_address) = translate_address(virtual_address) {
        println!("Sanal adres 0x{:x} fiziksel adrese 0x{:x} çevrildi.", virtual_address, physical_address);
    } else {
        println!("Adres çevirme başarısız oldu.");
    }
}