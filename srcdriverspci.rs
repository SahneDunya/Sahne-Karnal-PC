// src/driver/pci.rs

use pci_api::{
    constants, CustomOsPciConfigReader, PciAddress as PciApiAddress, PciConfigReader,
    PciConfigWriter, PciDevice as PciApiDevice,
};

// PCI cihazının temel bilgilerini tutan bir yapı.
#[derive(Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
}

impl PciDevice {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        PciDevice {
            bus,
            device,
            function,
            vendor_id: 0, // Başlangıçta bilinmiyor
            device_id: 0, // Başlangıçta bilinmiyor
        }
    }

    // PCI yapılandırma alanından 16-bitlik bir değer okur.
    // Gerçek bir uygulamada, bu işlem donanım erişim mekanizmasını kullanır
    // (örneğin, I/O portları 0xCFC ve 0xCF8).
    pub fn read_config_u16(&self, offset: u8, config_reader: &CustomOsPciConfigReader) -> Result<u16, pci_api::PciError> {
        let pci_address = PciApiAddress::new(self.bus, self.device, self.function);
        config_reader.read_u16(pci_address, offset as u16)
    }

    // PCI yapılandırma alanına 32-bitlik bir değer yazar.
    // Benzer şekilde, gerçek bir uygulamada donanım erişimi gereklidir.
    pub fn write_config_u32(
        &self,
        offset: u8,
        value: u32,
        config_writer: &CustomOsPciConfigReader,
    ) -> Result<(), pci_api::PciError> {
        let pci_address = PciApiAddress::new(self.bus, self.device, self.function);
        config_writer.write_u32(pci_address, offset as u16, value)
    }

    // Cihazın Vendor ID'sini okur.
    pub fn get_vendor_id(&mut self, config_reader: &CustomOsPciConfigReader) -> Result<u16, pci_api::PciError> {
        self.read_config_u16(constants::PCI_VENDOR_ID as u8, config_reader)
    }

    // Cihazın Device ID'sini okur.
    pub fn get_device_id(&mut self, config_reader: &CustomOsPciConfigReader) -> Result<u16, pci_api::PciError> {
        self.read_config_u16(constants::PCI_DEVICE_ID as u8, config_reader)
    }

    // Cihazı etkinleştirir (örneğin, Command Register'daki bits'leri ayarlar).
    pub fn enable_device(&self, config_writer: &CustomOsPciConfigReader) -> Result<(), pci_api::PciError> {
        // Command Register'ın offset'i 0x04'tür.
        // Memory Access Enable ve Bus Master Enable bitlerini ayarlıyoruz.
        let command_register_offset = constants::PCI_COMMAND as u8;
        let enable_bits: u16 = 0b0000_0110; // Memory Access Enable | Bus Master Enable
        self.write_config_u32(command_register_offset, enable_bits as u32, config_writer)?;
        println!("PCI Cihazı Etkinleştirildi: {:?}", self);
        Ok(())
    }

    // Base Address Register'larını (BAR) okur.
    pub fn get_base_address_registers(&self, config_reader: &CustomOsPciConfigReader) -> Result<[u64; 6], pci_api::PciError> {
        let mut bars = [0; 6];
        for i in 0..6 {
            let offset = constants::PCI_BAR0 as u16 + i * 4;
            let low = config_reader.read_u32(PciApiAddress::new(self.bus, self.device, self.function), offset)?;
            bars[i] = low as u64; // Basitlik için 32-bit BAR'ları varsayıyoruz.
                                    // Gerçekte, 64-bit BAR'ları doğru şekilde ele almanız gerekir.
        }
        println!("BAR'lar Okundu: {:?}", bars);
        Ok(bars)
    }

    // Bir bellek eşlemeli BAR'ı eşlemeye çalışır.
    pub fn map_bar(&self, bar_index: usize, config_reader: &CustomOsPciConfigReader) -> Result<Option<*mut u8>, pci_api::PciError> {
        let bars_result = self.get_base_address_registers(config_reader)?;
        if bar_index < 6 {
            let bar_value = bars_result[bar_index];
            if (bar_value & 0x1) == 0 { // Bellek BAR'ı
                let address = bar_value & !0xF; // Alt bitleri maskele
                println!(
                    "Bellek BAR'ı Eşleniyor ({}): Adres = 0x{:x}",
                    bar_index, address
                );
                // **DİKKAT**: Gerçek bir çekirdekte, burada sanal adrese eşleme
                // işlemleri yapılmalıdır. Bu örnekte sadece adresi döndürüyoruz.
                Ok(Some(address as *mut u8))
            } else {
                println!("BAR {} bir I/O portudur, bellek değil.", bar_index);
                Ok(None)
            }
        } else {
            println!("Geçersiz BAR indeksi: {}", bar_index);
            Ok(None)
        }
    }

    // Örnek olarak cihaza veri yazma fonksiyonu (çok basitleştirilmiş).
    pub fn write_to_device(&self, address_offset: usize, value: u8, config_reader: &CustomOsPciConfigReader) -> Result<(), pci_api::PciError> {
        if let Ok(Some(base_address)) = self.map_bar(0, config_reader) {
            // **DİKKAT**: Güvenli olmayan (unsafe) bir blok içinde doğrudan bellek
            // adresine yazıyoruz. Gerçek bir sürücüde, bu çok dikkatli yapılmalı
            // ve erişim izinleri kontrol edilmelidir.
            let device_address = unsafe { base_address.add(address_offset) };
            unsafe { *device_address = value };
            println!(
                "Cihaza Yazıldı: Adres Ofseti = 0x{:x}, Değer = 0x{:x}",
                address_offset, value
            );
            Ok(())
        } else {
            println!("BAR eşlenemedi, cihaza yazılamıyor.");
            Ok(())
        }
    }
}

// PCI cihazlarını keşfetmek için bir fonksiyon (çok basitleştirilmiş).
// Gerçek bir sistemde, bu işlem PCI veri yolunu taramayı ve cihazları
// yapılandırma alanlarını okuyarak tanımlamayı içerir.
pub fn discover_pci_devices(config_reader: &CustomOsPciConfigReader) -> Result<Vec<PciDevice>, pci_api::PciError> {
    let mut devices = Vec::new();
    // Örnek olarak, bazı varsayılan adreslerde cihazların olduğunu varsayıyoruz.
    // Gerçekte, bu döngü tüm olası PCI veri yollarını, cihazlarını ve fonksiyonlarını taramalıdır.
    for bus in 0..1 {
        for device in 0..32 {
            for function in 0..8 {
                let mut pci_device = PciDevice::new(bus, device, function);
                match pci_device.get_vendor_id(config_reader) {
                    Ok(vendor) => {
                        // Geçerli bir Vendor ID'si (0xFFFF değilse) cihazın varlığını gösterir.
                        if vendor != 0xFFFF {
                            match pci_device.get_device_id(config_reader) {
                                Ok(_) => {
                                    println!("PCI Cihazı Bulundu: {:?}", pci_device);
                                    devices.push(pci_device);
                                    // Fonksiyon 0 olmayan cihazlar için diğer fonksiyonları kontrol etmeyebiliriz
                                    // (çok fonksiyonlu cihazlar için).
                                    let header_type_offset = 0x0E;
                                    match config_reader.read_u8(
                                        PciApiAddress::new(bus, device, function),
                                        header_type_offset,
                                    ) {
                                        Ok(header_type) => {
                                            if function == 0 && (header_type & 0x80) == 0 {
                                                break; // Tek fonksiyonlu cihaz
                                            }
                                        }
                                        Err(_) => {
                                            // Header type okuma hatası, devam et
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Device ID okuma hatası, devam et
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Vendor ID okuma hatası, bu adreste cihaz yok
                    }
                }
            }
        }
    }
    Ok(devices)
}

// Örnek sürücü fonksiyonu.
pub fn pci_driver_main() -> Result<(), pci_api::PciError> {
    println!("PCI Sürücüsü Başlatılıyor...");
    let config_reader = CustomOsPciConfigReader::new();
    let mut found_devices = discover_pci_devices(&config_reader)?;

    if let Some(mut device) = found_devices.pop() {
        println!("İşlenecek Bir Cihaz Bulundu: {:?}", device);
        device.enable_device(&config_reader)?;
        let bars = device.get_base_address_registers(&config_reader)?;
        println!("BAR'lar: {:?}", bars);

        if let Ok(Some(mapped_address)) = device.map_bar(0, &config_reader) {
            println!("BAR 0 Eşlendi: {:?}", mapped_address);
            device.write_to_device(0x1000, 0xAA, &config_reader)?; // Örnek yazma işlemi
        }
    } else {
        println!("Herhangi bir PCI cihazı bulunamadı.");
    }
    Ok(())
}