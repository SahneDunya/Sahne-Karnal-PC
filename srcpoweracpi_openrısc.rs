use crate::acpi::{AcpiHandler, GenericAddress};

pub struct OpenRiscAcpiHandler;

impl AcpiHandler for OpenRiscAcpiHandler {
    unsafe fn read_physical_address<T>(&self, address: GenericAddress) -> T {
        let physical_address = address.address as *const T;
        core::ptr::read_volatile(physical_address)
    }

    unsafe fn write_physical_address<T>(&mut self, address: GenericAddress, value: T) {
        let physical_address = address.address as *mut T;
        core::ptr::write_volatile(physical_address, value);
    }
}

// Örnek kullanım
fn main() {
    let mut handler = OpenRiscAcpiHandler;

    // Örnek bir GenericAddress yapısı oluşturun
    let address = GenericAddress {
        space_id: 0, // Sistem Hafıza Alanı
        bit_width: 32,
        bit_offset: 0,
        access_width: 32,
        address: 0x12345678, // Örnek fiziksel adres
    };

    // Fiziksel adresten bir değer okuyun
    let value: u32 = unsafe { handler.read_physical_address(address) };
    println!("Okunan değer: {}", value);

    // Fiziksel adrese bir değer yazın
    unsafe { handler.write_physical_address(address, 0xABCDEF01); }
    println!("Değer yazıldı.");
}