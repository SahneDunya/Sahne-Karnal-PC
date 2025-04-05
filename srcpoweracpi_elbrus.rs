use core::ptr;

// ACPI tablolarının temel adresini tutan yapı
#[repr(C, packed)]
struct RSDP {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

// ACPI tablolarını bulmak için ana fonksiyon
fn find_rsdp() -> Option<&'static RSDP> {
    // EBDA (Genişletilmiş BIOS Veri Alanı) ve BIOS ROM'unda RSDP'yi ara
    let ebda_address = 0x40E as *const u16;
    let ebda = unsafe { ptr::read_volatile(ebda_address) } as usize;

    for address in (ebda..ebda + 1024).step_by(16) {
        let rsdp = address as *const RSDP;
        if unsafe { (*rsdp).signature == *b"RSD PTR " } {
            return Some(unsafe { &*rsdp });
        }
    }

    for address in (0xE0000..0x100000).step_by(16) {
        let rsdp = address as *const RSDP;
        if unsafe { (*rsdp).signature == *b"RSD PTR " } {
            return Some(unsafe { &*rsdp });
        }
    }

    None
}

// Örnek kullanım
fn main() {
    if let Some(rsdp) = find_rsdp() {
        println!("RSDP bulundu: {:?}", rsdp);
        // RSDP'yi kullanarak diğer ACPI tablolarını bulabilirsiniz.
    } else {
        println!("RSDP bulunamadı.");
    }
}