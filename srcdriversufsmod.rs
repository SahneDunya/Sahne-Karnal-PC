#![no_std]
pub mod core;
pub mod uniprot;
pub mod mphy;
pub mod scsi;

use core::fmt;

#[derive(Debug)]
pub struct UfsDevice {
    pub uniprot_instance: uniprot::UniprotInstance,
    pub device_id: u8,
    pub ufs_version: u16, // UFS sürümünü sakla
    device_path: String,
    is_open: bool,
    device_size: u64,
}

impl UfsDevice {
    pub fn new(path: &str, size: u64, base_address: u64, device_id: u8, ufs_version: u16) -> Result<Self, UfsError> {
        // M-PHY ve UniPro başlatma
        let mphy_instance = mphy::MphyInstance::new(base_address).map_err(UfsError::MphyError)?;
        let uniprot_instance = uniprot::UniprotInstance::new(base_address, mphy_instance, ufs_version).map_err(UfsError::UniprotError)?;

        Ok(Self {
            uniprot_instance,
            device_id,
            ufs_version,
            device_path: path.to_string(),
            is_open: false,
            device_size: size,
        })
    }

    // UFS aygıtını açar.
    pub fn open(&mut self) -> Result<(), UfsError> {
        if self.is_open {
            return Ok(()); // Zaten açıksa sorun yok
        }
        // Gerçek bir sistemde, bu noktada aygıt sürücüsü ile iletişim kurulacak
        // ve aygıt kullanıma hazır hale getirilecektir.
        println!("UFS aygıtı açılıyor: {}", self.device_path);
        // Simülasyon için her zaman başarılı oluyoruz.
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
        if address % 512 != 0 || buffer.len() % 512 != 0 {
            return Err(UfsError::InvalidParameter); // Basitlik için blok hizalaması kontrolü
        }

        let lba = address / 512;
        let block_count = (buffer.len() / 512) as u32;

        let cdb = match self.ufs_version {
            0x0100..=0x0300 => scsi::ScsiCommandDescriptorBlock::read10(lba, block_count),
            0x0400 => scsi::ScsiCommandDescriptorBlock::read16(lba, block_count), // UFS 4.0 için 16 byte CDB
            _ => return Err(UfsError::UnsupportedUFSVersion),
        };

        self.uniprot_instance.execute_command(&cdb, buffer).map_err(|e| UfsError::UniprotError(e))?;
        Ok(buffer.len())
    }

    // UFS aygıtına belirtilen adrese veri yazar.
    pub fn write(&mut self, address: u64, buffer: &[u8]) -> Result<usize, UfsError> {
        if !self.is_open {
            return Err(UfsError::WriteError);
        }
        if address >= self.device_size {
            return Err(UfsError::InvalidAddress);
        }
        if address % 512 != 0 || buffer.len() % 512 != 0 {
            return Err(UfsError::InvalidParameter); // Basitlik için blok hizalaması kontrolü
        }

        let lba = address / 512;
        let block_count = (buffer.len() / 512) as u32;

        let cdb = match self.ufs_version {
            0x0100..=0x0300 => scsi::ScsiCommandDescriptorBlock::write10(lba, block_count),
            0x0400 => scsi::ScsiCommandDescriptorBlock::write16(lba, block_count), // UFS 4.0 için 16 byte CDB
            _ => return Err(UfsError::UnsupportedUFSVersion),
        };
        self.uniprot_instance.execute_command(&cdb, buffer).map_err(|e| UfsError::UniprotError(e))?;
        Ok(buffer.len())
    }

    // UFS aygıtını kapatır.
    pub fn close(&mut self) -> Result<(), UfsError> {
        if !self.is_open {
            return Ok(()); // Zaten kapalıysa sorun yok
        }
        // Gerçek bir sistemde, bu noktada aygıt sürücüsü ile iletişim kesilecek
        // ve aygıt serbest bırakılacaktır.
        println!("UFS aygıtı kapatılıyor: {}", self.device_path);
        // Simülasyon için her zaman başarılı oluyoruz.
        self.is_open = false;
        Ok(())
    }

    // UFS aygıtının boyutunu (bayt cinsinden) döndürür.
    pub fn get_size(&self) -> u64 {
        self.device_size
    }
}

#[derive(Debug)]
pub enum UfsError {
    OpenError,
    ReadError,
    WriteError,
    CloseError,
    InvalidAddress,
    UnsupportedUFSVersion,
    InvalidParameter,
    Other(String),
}

impl fmt::Display for UfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UfsError::OpenError => write!(f, "Aygıt açma hatası"),
            UfsError::ReadError => write!(f, "Aygıttan okuma hatası"),
            UfsError::WriteError => write!(f, "Aygıta yazma hatası"),
            UfsError::CloseError => write!(f, "Aygıt kapatma hatası"),
            UfsError::InvalidAddress => write!(f, "Geçersiz adres"),
            UfsError::UnsupportedUFSVersion => write!(f, "Desteklenmeyen UFS Versiyonu"),
            UfsError::InvalidParameter => write!(f, "Geçersiz parametre"),
            UfsError::Other(e) => write!(f, "Diğer hata: {}", e),
        }
    }
}

impl From<uniprot::UniprotError> for UfsError {
    fn from(error: uniprot::UniprotError) -> Self {
        match error {
            uniprot::UniprotError::MphyError(e) => UfsError::MphyError(e),
            other => UfsError::Other(format!("UniPro hatası: {:?}", other)),
        }
    }
}

impl From<mphy::MphyError> for UfsError {
    fn from(error: mphy::MphyError) -> Self {
        UfsError::MphyError(error)
    }
}

pub mod uniprot {
    use super::mphy;
    use core::fmt;

    #[derive(Debug)]
    pub struct UniprotInstance {
        base_address: u64,
        mphy_instance: mphy::MphyInstance,
        ufs_version: u16,
    }

    impl UniprotInstance {
        pub fn new(base_address: u64, mphy_instance: mphy::MphyInstance, ufs_version: u16) -> Result<Self, UniprotError> {
            Ok(Self {
                base_address,
                mphy_instance,
                ufs_version,
            })
        }

        pub fn execute_command(&mut self, cdb: &super::scsi::ScsiCommandDescriptorBlock, buffer: &mut [u8]) -> Result<(), UniprotError> {
            // Gerçek bir implementasyonda UniPro katmanına komut gönderme ve yanıt alma işlemleri burada yer alacaktır.
            // Bu örnekte sadece bir bilgilendirme mesajı yazdırıyoruz.
            println!("UniPro komutu yürütülüyor: {:?}", cdb);
            println!("Veri arabelleği boyutu: {}", buffer.len());
            // Simülasyon: Başarılı dönüş
            Ok(())
        }

        pub fn execute_command_write(&mut self, cdb: &super::scsi::ScsiCommandDescriptorBlock, buffer: &[u8]) -> Result<(), UniprotError> {
            // Gerçek bir implementasyonda UniPro katmanına komut gönderme ve yanıt alma işlemleri burada yer alacaktır.
            // Bu örnekte sadece bir bilgilendirme mesajı yazdırıyoruz.
            println!("UniPro yazma komutu yürütülüyor: {:?}", cdb);
            println!("Yazılacak veri arabelleği boyutu: {}", buffer.len());
            // Simülasyon: Başarılı dönüş
            Ok(())
        }
    }

    #[derive(Debug)]
    pub enum UniprotError {
        MphyError(mphy::MphyError),
        Other(String),
    }

    impl fmt::Display for UniprotError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                UniprotError::MphyError(e) => write!(f, "M-PHY hatası: {}", e),
                UniprotError::Other(e) => write!(f, "Diğer UniPro hatası: {}", e),
            }
        }
    }
}

pub mod mphy {
    use core::fmt;

    #[derive(Debug)]
    pub struct MphyInstance {
        base_address: u64,
    }

    impl MphyInstance {
        pub fn new(base_address: u64) -> Result<Self, MphyError> {
            Ok(Self { base_address })
        }
    }

    #[derive(Debug)]
    pub enum MphyError {
        InitializationError,
        LinkUpError,
        Other(String),
    }

    impl fmt::Display for MphyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MphyError::InitializationError => write!(f, "M-PHY başlatma hatası"),
                MphyError::LinkUpError => write!(f, "M-PHY bağlantı hatası"),
                MphyError::Other(e) => write!(f, "Diğer M-PHY hatası: {}", e),
            }
        }
    }
}

pub mod scsi {
    // Basit bir SCSI CDB tanımı
    #[derive(Debug)]
    pub enum ScsiCommandDescriptorBlock {
        Read10 { lba: u64, block_count: u32 },
        Write10 { lba: u64, block_count: u32 },
        Read16 { lba: u64, block_count: u32 },
        Write16 { lba: u64, block_count: u32 },
        Inquiry { lun: u8, allocation_length: u8 },
    }

    impl ScsiCommandDescriptorBlock {
        pub fn read10(lba: u64, block_count: u32) -> Self {
            ScsiCommandDescriptorBlock::Read10 { lba, block_count }
        }

        pub fn write10(lba: u64, block_count: u32) -> Self {
            ScsiCommandDescriptorBlock::Write10 { lba, block_count }
        }

        pub fn read16(lba: u64, block_count: u32) -> Self {
            ScsiCommandDescriptorBlock::Read16 { lba, block_count }
        }

        pub fn write16(lba: u64, block_count: u32) -> Self {
            ScsiCommandDescriptorBlock::Write16 { lba, block_count }
        }

        pub fn inquiry(lun: u8, allocation_length: u8) -> Self {
            ScsiCommandDescriptorBlock::Inquiry { lun, allocation_length }
        }
    }

    #[derive(Debug)]
    pub struct InquiryResponse {
        pub peripheral_qualifier: u8,
        pub peripheral_device_type: u8,
        pub removable_medium: bool,
        pub ansi_version: u8,
        pub oem_identification: [u8; 8],
        pub product_identification: [u8; 16],
        pub product_revision_level: [u8; 4],
        pub vendor_specific: [u8; 20],
        pub reserved: [u8; 3],
        pub additional_length: u8,
        // ... diğer alanlar
    }

    impl InquiryResponse {
        pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
            if data.len() < 36 {
                return Err("Inquiry response too short".to_string());
            }
            Ok(Self {
                peripheral_qualifier: (data[0] >> 5) & 0x07,
                peripheral_device_type: data[0] & 0x1F,
                removable_medium: (data[1] & 0x80) != 0,
                ansi_version: data[2],
                oem_identification: data[3..11].try_into().unwrap(),
                product_identification: data[11..27].try_into().unwrap(),
                product_revision_level: data[27..31].try_into().unwrap(),
                vendor_specific: data[31..51].try_into().unwrap(),
                reserved: data[51..54].try_into().unwrap(),
                additional_length: data[4],
                // ... diğer alanlar
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ufs_device_new() {
        let device_result = UfsDevice::new("/dev/test_ufs", 4096 * 1024, 0x10000000, 0, 0x0300);
        assert!(device_result.is_ok());
        let device = device_result.unwrap();
        assert_eq!(device.device_path, "/dev/test_ufs");
        assert_eq!(device.is_open, false);
        assert_eq!(device.device_size, 4096 * 1024);
        assert_eq!(device.ufs_version, 0x0300);
    }

    #[test]
    fn test_ufs_device_open_close() {
        let mut device = UfsDevice::new("/dev/test_ufs", 4096 * 1024, 0x10000000, 0, 0x0300).unwrap();
        assert!(!device.is_open);
        assert!(device.open().is_ok());
        assert!(device.is_open);
        assert!(device.close().is_ok());
        assert!(!device.is_open);
    }

    #[test]
    fn test_ufs_device_read_write() {
        let mut device = UfsDevice::new("/dev/test_ufs", 4096 * 1024, 0x10000000, 0, 0x0300).unwrap();
        assert!(device.open().is_ok());

        let address = 0;
        let mut read_buffer = [0u8; 512];
        let write_buffer = [0xAAu8; 512];

        assert!(device.write(address, &write_buffer).is_ok());
        assert!(device.read(address, &mut read_buffer).is_ok());
        assert_eq!(read_buffer, write_buffer);

        assert!(device.close().is_ok());
    }

    #[test]
    fn test_ufs_device_read_write_invalid_address() {
        let mut device = UfsDevice::new("/dev/test_ufs", 4096, 0x10000000, 0, 0x0300).unwrap();
        assert!(device.open().is_ok());
        let address = 4096;
        let mut buffer = [0u8; 512];
        assert!(device.read(address, &mut buffer).is_err());
        assert!(device.write(address, &buffer).is_err());
        assert!(device.close().is_ok());
    }

    #[test]
    fn test_ufs_device_read_write_closed() {
        let mut device = UfsDevice::new("/dev/test_ufs", 4096, 0x10000000, 0, 0x0300).unwrap();
        let address = 0;
        let mut buffer = [0u8; 512];
        assert!(device.read(address, &mut buffer).is_err());
        assert!(device.write(address, &buffer).is_err());
    }
}