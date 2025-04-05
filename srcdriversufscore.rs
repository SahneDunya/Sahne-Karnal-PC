#![no_std]
use core::fmt;

// UFS Descriptor'ları

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DeviceDescriptor {
    pub wTotalLength: u16,
    pub bDescriptorType: u8,
    pub bcdUFSVersion: u16,
    pub bNumberOfLogicalUnits: u8,
    pub bUFSFeatures: u8,
    pub wManufacturerNameLength: u16,
    pub ManufacturerName: [u8; 256],
    pub wProductNameLength: u16,
    pub ProductName: [u8; 256],
    pub wSerialNumberLength: u16,
    pub SerialNumber: [u8; 256],
    // UFS 2.0 ve sonrası için ek alanlar (örnek)
    pub bConfigurationSetting: u8,
    pub bMaxNumberSupportedConfigurations: u8,
    // ... diğer alanlar
}


// UFS Hata Türleri
#[derive(Debug)]
pub enum DescriptorError {
    InvalidDescriptorType(u8),
    InvalidDescriptorLength(usize, usize), // (expected, actual)
    ParseError,
    UnsupportedUFSVersion(u16),
    InsufficientData(usize, usize), // (required, actual)
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescriptorError::InvalidDescriptorType(type_val) => {
                write!(f, "Geçersiz Descriptor Tipi: Beklenen 0x01, alınan 0x{:02X}", type_val)
            }
            DescriptorError::InvalidDescriptorLength(expected, actual) => {
                write!(f, "Geçersiz Descriptor Uzunluğu: Beklenen en az {} byte, alınan {} byte", expected, actual)
            }
            DescriptorError::ParseError => write!(f, "Descriptor ayrıştırma hatası"),
            DescriptorError::UnsupportedUFSVersion(version) => {
                write!(f, "Desteklenmeyen UFS Versiyonu: 0x{:04X}", version)
            }
            DescriptorError::InsufficientData(required, actual) => {
                write!(f, "Yetersiz Veri: En az {} byte gerekli, {} byte sağlandı", required, actual)
            }
        }
    }
}

// Sabitler (Descriptor Tipleri, UFS Versiyon Aralığı)
const DESCRIPTOR_TYPE_DEVICE: u8 = 0x01;
const UFS_VERSION_MIN: u16 = 0x0100; // UFS 1.0
const UFS_VERSION_MAX: u16 = 0x0400; // UFS 4.0
const UFS_VERSION_2_0: u16 = 0x0200; // UFS 2.0

// Yardımcı Fonksiyonlar

pub fn parse_device_descriptor(data: &[u8]) -> Result<DeviceDescriptor, DescriptorError> {
    // Minimum descriptor uzunluğu kontrolü (Temel alanlar için)
    if data.len() < 6 {
        return Err(DescriptorError::InsufficientData(6, data.len()));
    }

    // Toplam uzunluğu oku
    let total_length = u16::from_le_bytes([data[0], data[1]]);
    // Veri uzunluğunun toplam uzunluğa eşit veya daha büyük olduğunu kontrol et
    if data.len() < total_length as usize {
        return Err(DescriptorError::InvalidDescriptorLength(total_length as usize, data.len()));
    }

    // Descriptor tipini kontrol et (Device Descriptor için 0x01 olmalı)
    if data[2] != DESCRIPTOR_TYPE_DEVICE {
        return Err(DescriptorError::InvalidDescriptorType(data[2]));
    }

    // UFS Versiyonunu oku
    let ufs_version = u16::from_le_bytes([data[3], data[4]]);
    // Desteklenen UFS versiyon aralığını kontrol et (1.0 - 4.0)
    if ufs_version < UFS_VERSION_MIN || ufs_version > UFS_VERSION_MAX {
        return Err(DescriptorError::UnsupportedUFSVersion(ufs_version));
    }

    // DeviceDescriptor yapısını oluştur ve temel alanları doldur
    let mut descriptor = DeviceDescriptor {
        wTotalLength: total_length,
        bDescriptorType: data[2],
        bcdUFSVersion: ufs_version,
        bNumberOfLogicalUnits: data[5],
        bUFSFeatures: data[6],
        wManufacturerNameLength: u16::from_le_bytes([data[7], data[8]]),
        ManufacturerName: [0u8; 256],
        wProductNameLength: u16::from_le_bytes([data[9], data[10]]),
        ProductName: [0u8; 256],
        wSerialNumberLength: u16::from_le_bytes([data[11], data[12]]),
        SerialNumber: [0u8; 256],
        bConfigurationSetting: 0, // Varsayılan değerler
        bMaxNumberSupportedConfigurations: 0,
    };

    let mut offset = 13; // Okunan byte sayısı (Temel alanlardan sonraki ilk byte)

    macro_rules! read_string {
        ($field:ident, $len_field:ident) => {
            let len = descriptor.$len_field as usize;
            let required_len = offset + len;
            if data.len() < required_len {
                return Err(DescriptorError::InsufficientData(required_len, data.len()));
            }
            descriptor.$field[..len].copy_from_slice(&data[offset..offset + len]);
            offset += len;
        };
    }

    // Üretici Adı, Ürün Adı ve Seri Numarası stringlerini oku
    read_string!(ManufacturerName, wManufacturerNameLength);
    read_string!(ProductName, wProductNameLength);
    read_string!(SerialNumber, wSerialNumberLength);

    // UFS 2.0 ve sonrası için ek alanları ayrıştır (Örnek)
    if ufs_version >= UFS_VERSION_2_0 {
        if data.len() < offset + 2 {
            return Err(DescriptorError::InsufficientData(offset + 2, data.len()));
        }
        descriptor.bConfigurationSetting = data[offset];
        descriptor.bMaxNumberSupportedConfigurations = data[offset + 1];
        offset += 2;
    }

    Ok(descriptor)
}

// UFS Hata Türleri (Önceki koddan)
#[derive(Debug)]
pub enum UfsError {
    OpenError,
    ReadError,
    WriteError,
    CloseError,
    InvalidAddress,
    DescriptorError(DescriptorError), // Descriptor hatalarını kapsayan yeni bir varyant
    Other(String),
}

impl From<DescriptorError> for UfsError {
    fn from(error: DescriptorError) -> Self {
        UfsError::DescriptorError(error)
    }
}

// UFS aygıtını temsil eden temel yapı (Önceki koddan güncellendi)
pub struct UfsDevice {
    device_path: String, // Simülasyon için aygıt yolu
    is_open: bool,
    device_size: u64, // Aygıtın boyutu
    // Yeni alanlar: Descriptor bilgilerini saklamak için
    manufacturer: String,
    product_name: String,
    serial_number: String,
    ufs_version: u16,
}

impl UfsDevice {
    // Yeni bir UFS aygıtı örneği oluşturur (Descriptor verisinden)
    pub fn from_descriptor_data(path: &str, size: u64, data: &[u8]) -> Result<Self, UfsError> {
        let descriptor = parse_device_descriptor(data)?;
        let manufacturer = String::from_utf8_lossy(&descriptor.ManufacturerName[..descriptor.wManufacturerNameLength as usize]).into_owned();
        let product_name = String::from_utf8_lossy(&descriptor.ProductName[..descriptor.wProductNameLength as usize]).into_owned();
        let serial_number = String::from_utf8_lossy(&descriptor.SerialNumber[..descriptor.wSerialNumberLength as usize]).into_owned();

        Ok(UfsDevice {
            device_path: path.to_string(),
            is_open: false,
            device_size: size,
            manufacturer,
            product_name,
            serial_number,
            ufs_version: descriptor.bcdUFSVersion,
        })
    }

    // Yeni bir UFS aygıtı örneği oluşturur (Önceki ile aynı)
    pub fn new(path: &str, size: u64) -> Self {
        UfsDevice {
            device_path: path.to_string(),
            is_open: false,
            device_size: size,
            manufacturer: String::new(),
            product_name: String::new(),
            serial_number: String::new(),
            ufs_version: 0,
        }
    }

    // UFS aygıtını açar.
    pub fn open(&mut self) -> Result<(), UfsError> {
        if self.is_open {
            return Ok(()); // Zaten açıksa sorun yok
        }
        println!("UFS aygıtı açılıyor: {}", self.device_path);
        self.is_open = true;
        Ok(())
    }

    // UFS aygıtından belirtilen adresten veri okur.
    pub fn read(&self, address: u64, buffer: &mut [u8]) -> Result<usize, UfsError> {
        if !self.is_open {
            return Err(UfsError::ReadError);
        }
        if address >= self.device_size {
            return Err(UfsError::InvalidAddress);
        }
        println!(
            "UFS aygıtından okunuyor: adres={}, boyut={}",
            address,
            buffer.len()
        );
        for (i, byte) in buffer.iter_mut().enumerate() {
            if address + i as u64 < self.device_size {
                *byte = (address + i as u64) as u8; // Basit bir desen
            } else {
                break;
            }
        }
        Ok(buffer.len().min((self.device_size - address) as usize))
    }

    // UFS aygıtına belirtilen adrese veri yazar.
    pub fn write(&mut self, address: u64, buffer: &[u8]) -> Result<usize, UfsError> {
        if !self.is_open {
            return Err(UfsError::WriteError);
        }
        if address >= self.device_size {
            return Err(UfsError::InvalidAddress);
        }
        println!(
            "UFS aygıtına yazılıyor: adres={}, boyut={}",
            address,
            buffer.len()
        );
        println!("Yazılacak veri (ilk 8 bayt): {:?}", &buffer[..buffer.len().min(8)]);
        Ok(buffer.len().min((self.device_size - address) as usize))
    }

    // UFS aygıtını kapatır.
    pub fn close(&mut self) -> Result<(), UfsError> {
        if !self.is_open {
            return Ok(()); // Zaten kapalıysa sorun yok
        }
        println!("UFS aygıtı kapatılıyor: {}", self.device_path);
        self.is_open = false;
        Ok(())
    }

    // UFS aygıtının boyutunu (bayt cinsinden) döndürür.
    pub fn get_size(&self) -> u64 {
        self.device_size
    }

    // UFS aygıtının üreticisini döndürür.
    pub fn get_manufacturer(&self) -> &str {
        &self.manufacturer
    }

    // UFS aygıtının ürün adını döndürür.
    pub fn get_product_name(&self) -> &str {
        &self.product_name
    }

    // UFS aygıtının seri numarasını döndürür.
    pub fn get_serial_number(&self) -> &str {
        &self.serial_number
    }

    // UFS aygıtının versiyonunu döndürür.
    pub fn get_ufs_version(&self) -> u16 {
        self.ufs_version
    }
}

fn main() {
    let data: [u8; 54] = [
        0x36, 0x00, 0x01, 0x02, 0x00, 0x01, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00,
        0x54, 0x65, 0x73, 0x74, 0x00, 0x50, 0x72, 0x6f, 0x64, 0x00, 0x53, 0x65, 0x72, 0x69, 0x61, 0x6c, 0x00,
        0x01, 0x02, // UFS 2.0+ alanları
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    match UfsDevice::from_descriptor_data("/dev/test_ufs", 4096, &data) {
        Ok(device) => println!("{:#?}", device),
        Err(e) => println!("Error: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_device() {
        let device = UfsDevice::new("/dev/test_ufs", 1024);
        assert_eq!(device.device_path, "/dev/test_ufs");
        assert_eq!(device.is_open, false);
        assert_eq!(device.get_size(), 1024);
        assert_eq!(device.get_manufacturer(), "");
        assert_eq!(device.get_product_name(), "");
        assert_eq!(device.get_serial_number(), "");
        assert_eq!(device.get_ufs_version(), 0);
    }

    #[test]
    fn test_open_close_device() {
        let mut device = UfsDevice::new("/dev/test_ufs", 1024);
        assert!(!device.is_open);
        assert!(device.open().is_ok());
        assert!(device.is_open);
        assert!(device.open().is_ok()); // Açıkken tekrar açmak sorun olmamalı
        assert!(device.close().is_ok());
        assert!(!device.is_open);
        assert!(device.close().is_ok()); // Kapalıyken tekrar kapatmak sorun olmamalı
    }

    #[test]
    fn test_read_write_device() {
        let mut device = UfsDevice::new("/dev/test_ufs", 4096);
        assert!(device.open().is_ok());

        let read_address = 1024;
        let mut read_buffer = [0u8; 512];
        let read_result = device.read(read_address, &mut read_buffer);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), 512);
        for (i, byte) in read_buffer.iter().enumerate() {
            assert_eq!(*byte, (read_address + i as u64) as u8);
        }

        let write_address = 2048;
        let write_data = [0xAA, 0xBB, 0xCC, 0xDD];
        let write_result = device.write(write_address, &write_data);
        assert!(write_result.is_ok());
        assert_eq!(write_result.unwrap(), 4);

        assert!(device.close().is_ok());
    }

    #[test]
    fn test_read_write_closed_device() {
        let device = UfsDevice::new("/dev/test_ufs", 1024);
        let mut buffer = [0u8; 10];
        assert!(device.read(0, &mut buffer).is_err());
        assert!(device.write(0, &buffer).is_err());
    }

    #[test]
    fn test_read_write_invalid_address() {
        let mut device = UfsDevice::new("/dev/test_ufs", 1024);
        assert!(device.open().is_ok());
        let mut buffer = [0u8; 10];
        assert!(device.read(2048, &mut buffer).is_err());
        assert!(device.write(2048, &buffer).is_err());
        assert!(device.close().is_ok());
    }

    #[test]
    fn test_read_write_partial_end() {
        let mut device = UfsDevice::new("/dev/test_ufs", 10);
        assert!(device.open().is_ok());
        let mut read_buffer = [0u8; 20];
        let read_result = device.read(5, &mut read_buffer);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), 5);
        for i in 0..5 {
            assert_eq!(read_buffer[i], (5 + i) as u8);
        }

        let write_data = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let write_result = device.write(7, &write_data);
        assert!(write_result.is_ok());
        assert_eq!(write_result.unwrap(), 3);

        assert!(device.close().is_ok());
    }

    #[test]
    fn test_create_device_from_descriptor() {
        let data: [u8; 54] = [
            0x36, 0x00, 0x01, 0x02, 0x00, 0x01, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00,
            0x54, 0x65, 0x73, 0x74, 0x00, 0x50, 0x72, 0x6f, 0x64, 0x00, 0x53, 0x65, 0x72, 0x69, 0x61, 0x6c, 0x00,
            0x01, 0x02, // UFS 2.0+ alanları
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let device_result = UfsDevice::from_descriptor_data("/dev/ufs_from_desc", 8192, &data);
        assert!(device_result.is_ok());
        let device = device_result.unwrap();
        assert_eq!(device.get_device_path(), "/dev/ufs_from_desc");
        assert_eq!(device.get_size(), 8192);
        assert_eq!(device.get_manufacturer(), "Test");
        assert_eq!(device.get_product_name(), "Prod");
        assert_eq!(device.get_serial_number(), "Serial");
        assert_eq!(device.get_ufs_version(), 0x0200);
    }
}