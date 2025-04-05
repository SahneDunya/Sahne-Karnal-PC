use acpi::{AcpiHandler, AcpiTable, PhysicalMapping};
use core::ptr::NonNull;
use spin::Mutex;
use log::{debug, error, info};

pub struct MyAcpiHandler;

impl AcpiHandler for MyAcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        // Fiziksel bölgeyi eşlemek için işletim sistemine özgü kodu uygulayın
        // Bu örnekte, basitçe fiziksel adresi sanal adrese dönüştürüyoruz
        let virtual_address = physical_address as *mut T;
        PhysicalMapping::new(physical_address, NonNull::new(virtual_address), size, size, Self)
    }

    fn unmap_physical_region<T>(&self, region: PhysicalMapping<Self, T>) {
        // Fiziksel bölgenin eşlemesini kaldırmak için işletim sistemine özgü kodu uygulayın
        // Bu örnekte, hiçbir şey yapmıyoruz
    }
}

pub struct AcpiState {
    pub acpi_tables: Option<acpi::AcpiTables<MyAcpiHandler>>,
}

impl AcpiState {
    pub fn new() -> AcpiState {
        AcpiState { acpi_tables: None }
    }

    pub fn initialize(&mut self, rsdp_address: usize) {
        let handler = MyAcpiHandler;
        match unsafe { acpi::AcpiTables::from_rsdp(handler, rsdp_address) } {
            Ok(tables) => {
                self.acpi_tables = Some(tables);
                info!("ACPI tabloları başarıyla ayrıştırıldı");
            }
            Err(e) => {
                error!("ACPI tabloları ayrıştırılamadı: {:?}", e);
            }
        }
    }

    pub fn power_off(&self) {
        if let Some(tables) = &self.acpi_tables {
            if let Ok(fadt) = tables.find_table::<acpi::fadt::Fadt>() {
                if let Some(s5_addr) = fadt.s5_address {
                    unsafe {
                        // S5 ACPI nesnesini yürütmek için işletim sistemine özgü kodu uygulayın
                        // Bu örnekte, basitçe S5 adresini günlüğe kaydediyoruz
                        info!("S5 adresi: {:?}", s5_addr);
                    }
                } else {
                    error!("S5 nesnesi bulunamadı");
                }
            } else {
                error!("FADT tablosu bulunamadı");
            }
        } else {
            error!("ACPI tabloları başlatılmadı");
        }
    }
}

pub static ACPI_STATE: Mutex<AcpiState> = Mutex::new(AcpiState::new());

pub fn initialize_acpi(rsdp_address: usize) {
    let mut state = ACPI_STATE.lock();
    state.initialize(rsdp_address);
}

pub fn power_off() {
    let state = ACPI_STATE.lock();
    state.power_off();
}