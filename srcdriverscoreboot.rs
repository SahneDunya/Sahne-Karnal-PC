use core::ptr;

// Coreboot'un sağladığı yapıların ve fonksiyonların tanımları
#[repr(C)]
#[derive(Debug)] // For potential debugging
pub struct CorebootTable {
    pub header: Header,
    pub mtrr_base: u32,
    pub mtrr_size: u32,
    pub acpi_rsdp_addr: u32, // Örnek ek alan: ACPI RSDP adresi
    // ... diğer alanlar
}

#[repr(C)]
#[derive(Debug)] // For potential debugging
pub struct Header {
    pub signature: u32,
    pub revision: u16,
    pub header_size: u16,
    pub checksum: u32,
    pub oem_id: [u8; 6], // Örnek ek alan: OEM ID
    // ... diğer alanlar
}

// Coreboot tablosunun adresini bulma fonksiyonu
/// Coreboot tablosunu belirli bir adres aralığında arar.
///
/// # Döndürür
///
/// Eğer Coreboot tablosu bulunursa `Some(&'static CorebootTable)`, aksi takdirde `None` döner.
pub fn find_coreboot_table(start_addr_usize: usize, end_addr_usize: usize) -> Option<&'static CorebootTable> {
    // Adres aralığını 16 byte adımlarla tarayarak Coreboot tablosunu bul
    for addr in (start_addr_usize..=end_addr_usize).step_by(16) {
        // Adresi pointer'a dönüştür
        let table_ptr = addr as *const CorebootTable;

        // Güvenli olmayan blok içinde pointer'ı dereferans et
        // Dikkat: Bu işlem güvenli olmayabilir, adresin geçerli olduğundan emin olunmalı.
        let table = unsafe {
            &*table_ptr
        };

        // Coreboot tablosunun imzasını kontrol et
        if table.header.signature == 0x424F4F54 { // "BOOT" imzası (ASCII karşılığı)
            // İmzayı bulduk, tabloyu döndür
            return Some(table);
        }
        // Eğer imza eşleşmezse, döngü devam eder ve sonraki adrese bakar.
    }

    // Belirtilen aralıkta Coreboot tablosu bulunamadı.
    None
}

// Coreboot'tan MTRR bilgilerini okuma fonksiyonu
/// Coreboot tablosundan MTRR temel adresini ve boyutunu okur.
///
/// # Parametreler
///
/// * `table`: Geçerli bir `CorebootTable` referansı.
///
/// # Döndürür
///
/// MTRR temel adresi ve boyutunu bir tuple olarak döner `(mtrr_base, mtrr_size)`.
pub fn read_mtrr_info(table: &CorebootTable) -> (u32, u32) {
    (table.mtrr_base, table.mtrr_size)
}

// Örnek kullanım fonksiyonu
/// Coreboot tablosunu bulma ve MTRR bilgilerini okuma örneği.
pub fn example_usage() {
    // Coreboot tablosunun bilinen adres aralığı (örnek olarak F0000h - FFFFFh)
    let start_addr = 0x000F0000;
    let end_addr = 0x000FFFFF;

    println!("Coreboot table searching in range: 0x{:X} - 0x{:X}", start_addr, end_addr);

    // Coreboot tablosunu ara
    match find_coreboot_table(start_addr, end_addr) {
        Some(coreboot_table) => {
            println!("Coreboot table found at: 0x{:X}!", coreboot_table as *const _ as usize);
            println!("Coreboot table header: {:?}", coreboot_table.header); // Debug özelliği ile header içeriğini yazdır
            let (mtrr_base, mtrr_size) = read_mtrr_info(coreboot_table);
            println!("MTRR base: 0x{:X}, size: 0x{:X}", mtrr_base, mtrr_size);
            println!("ACPI RSDP Address: 0x{:X}", coreboot_table.acpi_rsdp_addr); // Örnek ek alanı yazdır
        }
        None => {
            println!("Coreboot table not found in the specified range.");
        }
    }
}

// Ana fonksiyon (isteğe bağlı, sadece kütüphane olarak da kullanılabilir)
fn main() {
    example_usage();
}