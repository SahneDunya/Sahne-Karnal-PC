#![no_std]
pub mod core;
pub mod uniprot;
pub mod mphy;

use core::fmt;
use bitflags::bitflags;

// SCSI Komut Kodları
mod scsi_commands {
    pub const READ_10: u8 = 0x28;
    pub const WRITE_10: u8 = 0x2A;
    pub const READ_16: u8 = 0x88;
    pub const WRITE_16: u8 = 0x8A;
    pub const INQUIRY: u8 = 0x12;
}

// SCSI Komutları Enumu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScsiCommand {
    Read10,
    Write10,
    Read16,
    Write16,
    Inquiry,
    // Diğer komutlar eklenebilir
}

/// SCSI Komut Tanımlayıcı Blokları (CDB)
/// 16 byte maksimum boyut
#[derive(Debug, PartialEq, Eq)]
pub struct ScsiCommandDescriptorBlock {
    pub command: ScsiCommand,
    pub cdb: [u8; 16],
}

impl ScsiCommandDescriptorBlock {
    /// Ortak 10-byte komut oluşturma mantığı
    fn build_10_cmd(
        cmd: u8,
        command: ScsiCommand,
        lba: u32,
        block_count: u16,
    ) -> Self {
        let mut cdb = [0u8; 16];
        cdb[0] = cmd;
        cdb[2..6].copy_from_slice(&lba.to_be_bytes());
        cdb[7..9].copy_from_slice(&block_count.to_be_bytes());
        Self { command, cdb }
    }

    /// READ(10) komutu oluşturur
    pub fn read10(lba: u32, block_count: u16) -> Self {
        Self::build_10_cmd(
            scsi_commands::READ_10,
            ScsiCommand::Read10,
            lba,
            block_count,
        )
    }

    /// WRITE(10) komutu oluşturur
    pub fn write10(lba: u32, block_count: u16) -> Self {
        Self::build_10_cmd(
            scsi_commands::WRITE_10,
            ScsiCommand::Write10,
            lba,
            block_count,
        )
    }

    /// Ortak 16-byte komut oluşturma mantığı
    fn build_16_cmd(
        cmd: u8,
        command: ScsiCommand,
        lba: u64,
        block_count: u32,
    ) -> Self {
        let mut cdb = [0u8; 16];
        cdb[0] = cmd;
        cdb[2..10].copy_from_slice(&lba.to_be_bytes());
        cdb[10..14].copy_from_slice(&block_count.to_be_bytes());
        Self { command, cdb }
    }

    /// READ(16) komutu oluşturur (48-bit LBA)
    pub fn read16(lba: u64, block_count: u32) -> Self {
        Self::build_16_cmd(
            scsi_commands::READ_16,
            ScsiCommand::Read16,
            lba,
            block_count,
        )
    }

    /// WRITE(16) komutu oluşturur
    pub fn write16(lba: u64, block_count: u32) -> Self {
        Self::build_16_cmd(
            scsi_commands::WRITE_16,
            ScsiCommand::Write16,
            lba,
            block_count,
        )
    }

    /// INQUIRY komutu oluşturur
    pub fn inquiry(evpd: bool, page_code: u8, allocation_length: u16) -> Self {
        let mut cdb = [0u8; 16];
        cdb[0] = scsi_commands::INQUIRY;
        cdb[1] = evpd as u8;
        cdb[2] = page_code;
        cdb[3..5].copy_from_slice(&allocation_length.to_be_bytes());

        Self {
            command: ScsiCommand::Inquiry,
            cdb,
        }
    }
}

/// INQUIRY yanıt veri yapısı
#[derive(Debug, PartialEq, Eq)]
pub struct InquiryResponse {
    pub peripheral_device_type: u8,
    pub peripheral_qualifier: u8,
    pub version: u8,
    pub response_data_format: u8,
    pub additional_length: u8,
    pub flags: InquiryFlags,
    pub vendor_identification: [u8; 8],
    pub product_identification: [u8; 16],
    pub product_revision_level: [u8; 4],
    // Diğer alanlar...
}

/// INQUIRY yanıt bayrak bitleri
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InquiryFlags: u8 {
        const TPGS     = 0b10000000;
        const CMDQUE   = 0b01000000;
        const SFTRE    = 0b00100000;
        const LINKED   = 0b00010000;
        const SYNC     = 0b00001000;
        const WBUS16   = 0b00000100;
    }
}

impl InquiryResponse {
    /// Ham byte dizisinden yanıtı parse eder
    pub fn from_bytes(data: &[u8]) -> Result<Self, ScsiError> {
        // Minimum geçerli yanıt boyutu kontrolü
        if data.len() < 36 {
            return Err(ScsiError::InvalidResponse);
        }

        let flags = InquiryFlags::from_bits(data[3])
            .ok_or(ScsiError::InvalidResponse)?;

        Ok(Self {
            peripheral_device_type: data[0] & 0x1F,
            peripheral_qualifier: (data[0] >> 5) & 0x07,
            version: data[1],
            response_data_format: data[2],
            additional_length: data[4],
            flags,
            vendor_identification: data[8..16].try_into().unwrap(), // Already checked length, safe to unwrap
            product_identification: data[16..32].try_into().unwrap(), // Already checked length, safe to unwrap
            product_revision_level: data[32..36].try_into().unwrap(), // Already checked length, safe to unwrap
        })
    }
}

/// SCSI Hata Türleri
#[derive(Debug, PartialEq, Eq)]
pub enum ScsiError {
    InvalidCommand,
    InvalidResponse,
    InvalidParameter,
    Other(&'static str),
}

impl fmt::Display for ScsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand => write!(f, "Geçersiz SCSI komutu"),
            Self::InvalidResponse => write!(f, "Geçersiz yanıt formatı"),
            Self::InvalidParameter => write!(f, "Geçersiz parametre"),
            Self::Other(msg) => write!(f, "Hata: {}", msg),
        }
    }
}

// Hata dönüşümleri
impl From<&'static str> for ScsiError {
    fn from(value: &'static str) -> Self {
        Self::Other(value)
    }
}

pub mod scsi {
    pub use super::{
        ScsiCommand,
        ScsiCommandDescriptorBlock,
        InquiryResponse,
        ScsiError,
    };
}

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
        // M-PHY başlatma
        let mphy_instance = mphy::MphyInstance::new(base_address).map_err(UfsError::MphyError)?;
        // UniPro başlatma
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
        if address % 512 != 0 || buffer.len() % 512 != 0 {
            return Err(UfsError::InvalidParameter); // Basitlik için blok hizalaması kontrolü
        }

        let lba = address / 512;
        let block_count = (buffer.len() / 512) as u32;

        let cdb = match self.ufs_version {
            0x0100..=0x0300 => scsi::ScsiCommandDescriptorBlock::read10(lba as u32, block_count as u16),
            0x0400 => scsi::ScsiCommandDescriptorBlock::read16(lba, block_count), // UFS 4.0 için 16 byte CDB
            _ => return Err(UfsError::UnsupportedUFSVersion),
        };

        self.uniprot_instance.execute_command(&cdb, buffer).map_err(UfsError::UniprotError)?;
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
            0x0100..=0x0300 => scsi::ScsiCommandDescriptorBlock::write10(lba as u32, block_count as u16),
            0x0400 => scsi::ScsiCommandDescriptorBlock::write16(lba, block_count), // UFS 4.0 için 16 byte CDB
            _ => return Err(UfsError::UnsupportedUFSVersion),
        };
        self.uniprot_instance.execute_command_write(&cdb, buffer).map_err(UfsError::UniprotError)?;
        Ok(buffer.len())
    }

    pub fn inquiry(&mut self) -> Result<scsi::InquiryResponse, UfsError> {
        let mut inquiry_data = [0u8; 96];
        let cdb = scsi::ScsiCommandDescriptorBlock::inquiry(false, 0, inquiry_data.len() as u16);
        self.uniprot_instance.execute_command(&cdb, &mut inquiry_data)?;
        let response = scsi::InquiryResponse::from_bytes(&inquiry_data).map_err(UfsError::ScsiError)?;
        Ok(response)
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
    MphyError(mphy::MphyError),
    UniprotError(uniprot::UniprotError),
    ScsiError(scsi::ScsiError),
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
            UfsError::MphyError(e) => write!(f, "M-PHY hatası: {}", e),
            UfsError::UniprotError(e) => write!(f, "UniPro hatası: {}", e),
            UfsError::ScsiError(e) => write!(f, "SCSI hatası: {}", e),
            UfsError::Other(e) => write!(f, "Diğer hata: {}", e),
        }
    }
}

impl From<uniprot::UniprotError> for UfsError {
    fn from(error: uniprot::UniprotError) -> Self {
        match error {
            uniprot::UniprotError::MphyError(e) => UfsError::MphyError(e),
            other => UfsError::UniprotError(other),
        }
    }
}

impl From<mphy::MphyError> for UfsError {
    fn from(error: mphy::MphyError) -> Self {
        UfsError::MphyError(error)
    }
}

impl From<scsi::ScsiError> for UfsError {
    fn from(error: scsi::ScsiError) -> Self {
        UfsError::ScsiError(error)
    }
}

pub mod uniprot {
    use super::mphy;
    use super::scsi;
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

        pub fn execute_command(&mut self, cdb: &scsi::ScsiCommandDescriptorBlock, buffer: &mut [u8]) -> Result<(), UniprotError> {
            // Gerçek bir implementasyonda UniPro katmanına komut gönderme ve yanıt alma işlemleri burada yer alacaktır.
            // Bu örnekte M-PHY'ı doğrudan kullanıyoruz.
            println!("UniPro komutu yürütülüyor (M-PHY üzerinden): {:?}", cdb);
            println!("Veri arabelleği boyutu: {}", buffer.len());

            // Basit bir simülasyon: Veriyi M-PHY üzerinden al
            self.mphy_instance.receive_data(buffer).map_err(UniprotError::MphyError)?;

            // Simülasyon: Başarılı dönüş
            Ok(())
        }

        pub fn execute_command_write(&mut self, cdb: &scsi::ScsiCommandDescriptorBlock, buffer: &[u8]) -> Result<(), UniprotError> {
            // Gerçek bir implementasyonda UniPro katmanına komut gönderme ve yanıt alma işlemleri burada yer alacaktır.
            // Bu örnekte M-PHY'ı doğrudan kullanıyoruz.
            println!("UniPro yazma komutu yürütülüyor (M-PHY üzerinden): {:?}", cdb);
            println!("Yazılacak veri arabelleği boyutu: {}", buffer.len());

            // Basit bir simülasyon: Veriyi M-PHY üzerinden gönder
            self.mphy_instance.send_data(buffer).map_err(UniprotError::MphyError)?;

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

    // Register adresleri ve sabitler için tanımlar
    const UFS_VERSION_REGISTER_OFFSET: u64 = 0x00; // Örnek offset
    const DATA_BUFFER_REGISTER_OFFSET: u64 = 0x04; // Örnek offset
    const STATUS_REGISTER_OFFSET: u64 = 0x08; // Örnek offset

    const UFS_VERSION_4_0: u16 = 0x0400;
    const UFS_VERSION_3_1: u16 = 0x0310; // Örnek UFS 3.1 versiyonu
    const UFS_VERSION_1_0: u16 = 0x0100;

    #[derive(Debug)]
    pub struct MphyInstance {
        base_address: u64,
        ufs_version: u16,
    }

    impl MphyInstance {
        pub fn new(base_address: u64) -> Result<Self, MphyError> {
            // M-PHY başlatma işlemleri (register ayarları vb.) buraya gelecek.
            // UFS sürümünü donanımdan okumalıyız.
            let ufs_version = Self::read_ufs_version(base_address)?; // Donanımdan UFS versiyonunu oku
            match ufs_version {
                UFS_VERSION_1_0..=UFS_VERSION_4_0 => {
                    // UFS sürümüne özgü başlatma kodları buraya
                    if ufs_version == UFS_VERSION_4_0 {
                        Self::init_ufs_4_0(base_address)?;
                    } else {
                        Self::init_ufs(base_address, ufs_version)?;
                    }
                },
                _ => return Err(MphyError::UnsupportedUFSVersion),
            }

            Ok(Self {
                base_address,
                ufs_version,
            })
        }

        fn read_ufs_version(base_address: u64) -> Result<u16, MphyError> {
            // Gerçek donanım okuma işlemleri buraya gelecek
            // Register okuma işlemi daha detaylı ve hata kontrolleri ile birlikte
            let version = unsafe {
                let version_ptr = (base_address + UFS_VERSION_REGISTER_OFFSET) as *const u16;
                version_ptr.read_volatile() // volatile okuma, donanım register'ları için önemli
            };

            // Versiyon okuma hatası kontrolü (örnek olarak basit bir kontrol)
            if version == 0xFFFF { // Örnek hata değeri
                return Err(MphyError::InitializationError); // Daha spesifik bir hata türü düşünülebilir.
            }

            Ok(UFS_VERSION_4_0) // Örnek UFS 4.0 versiyonu (test için sabit değer)
            // Ok(version) // Gerçek donanım okuma durumunda bu satır kullanılmalı
        }

        fn init_ufs(base_address: u64, ufs_version: u16) -> Result<(), MphyError> {
            // UFS 1.0 - 3.1 başlatma işlemleri
            // Donanıma özgü register ayarlamaları buraya gelecek
            // Örnek olarak, bir register'a değer yazma
            unsafe {
                let status_reg_ptr = (base_address + STATUS_REGISTER_OFFSET) as *mut u32;
                status_reg_ptr.write_volatile(0x01); // Örnek bir başlatma ayarı
            }
            Ok(())
        }


        fn init_ufs_4_0(base_address: u64) -> Result<(), MphyError> {
            // UFS 4.0 başlatma işlemleri (Örnek HS-LSS ayarları vb.)
            // Donanıma özgü register ayarlamaları buraya gelecek
            // Örnek olarak, farklı bir register'a farklı bir değer yazma
            unsafe {
                let status_reg_ptr = (base_address + STATUS_REGISTER_OFFSET) as *mut u32;
                status_reg_ptr.write_volatile(0x03); // Farklı bir başlatma ayarı (UFS 4.0 için)
            }
            Ok(())
        }

        pub fn send_data(&self, data: &[u8]) -> Result<(), MphyError> {
            // Veri gönderme işlemleri buraya gelecek.
            match self.ufs_version {
                UFS_VERSION_1_0..=UFS_VERSION_3_1 => { // UFS 1.0 - 3.1
                    self.send_data_ufs_1_to_3(data)?;
                }
                UFS_VERSION_4_0 => { // UFS 4.0 (Örnek HS-LSS desteği)
                    self.send_data_ufs_4_0(data)?;
                }
                _ => return Err(MphyError::UnsupportedUFSVersion), // Bu durum new fonksiyonunda engellenmeli ancak yine de ekledik.
            }
            Ok(())
        }

        fn send_data_ufs_1_to_3(&self, data: &[u8]) -> Result<(), MphyError> {
            for byte in data {
                // Daha detaylı register erişimi ve hata kontrolü (örnek)
                unsafe {
                    let data_reg_ptr = (self.base_address + DATA_BUFFER_REGISTER_OFFSET) as *mut u8;
                    data_reg_ptr.write_volatile(*byte); // Veriyi data register'ına yaz
                }
                // Gerçekte, M-PHY register'larına doğru şekilde erişmek ve veri gönderme protokolünü uygulamak gerekir.
                // Burada, veri gönderme işlemi sırasında hata oluşup oluşmadığını kontrol etmek ve gerekirse MphyError döndürmek önemlidir.
            }
            Ok(())
        }

        fn send_data_ufs_4_0(&self, data: &[u8]) -> Result<(), MphyError> {
            // HS-LSS veya diğer UFS 4.0'a özgü veri gönderme mekanizmaları buraya
            for byte in data {
                // Benzer şekilde, UFS 4.0 için register erişimi ve hata kontrolü (örnek)
                unsafe {
                    let data_reg_ptr = (self.base_address + DATA_BUFFER_REGISTER_OFFSET) as *mut u8;
                    data_reg_ptr.write_volatile(*byte); // Veriyi data register'ına yaz
                }
                // Gerçekte, M-PHY register'larına doğru şekilde erişmek ve veri gönderme protokolünü uygulamak gerekir.
                // Burada, veri gönderme işlemi sırasında hata oluşup oluşmadığını kontrol etmek ve gerekirse MphyError döndürmek önemlidir.
            }
            Ok(())
        }


        pub fn receive_data(&self, buffer: &mut [u8]) -> Result<(), MphyError> {
            // Veri alma işlemleri buraya gelecek.
            match self.ufs_version {
                UFS_VERSION_1_0..=UFS_VERSION_3_1 => { // UFS 1.0 - 3.1
                    self.receive_data_ufs_1_to_3(buffer)?;
                }
                UFS_VERSION_4_0 => { // UFS 4.0 (Örnek HS-LSS desteği)
                    self.receive_data_ufs_4_0(buffer)?;
                }
                _ => return Err(MphyError::UnsupportedUFSVersion), // Bu durum new fonksiyonunda engellenmeli ancak yine de ekledik.
            }
            Ok(())
        }

        fn receive_data_ufs_1_to_3(&self, buffer: &mut [u8]) -> Result<(), MphyError> {
            for i in 0..buffer.len() {
                // Daha detaylı register erişimi ve hata kontrolü (örnek)
                unsafe {
                    let data_reg_ptr = (self.base_address + DATA_BUFFER_REGISTER_OFFSET) as *const u8;
                    buffer[i] = data_reg_ptr.read_volatile(); // Veriyi data register'ından oku
                }
                // Gerçekte, M-PHY register'larından doğru şekilde okumak ve veri alma protokolünü uygulamak gerekir.
                // Burada, veri alma işlemi sırasında hata oluşup oluşmadığını kontrol etmek ve gerekirse MphyError döndürmek önemlidir.
            }
            Ok(())
        }

        fn receive_data_ufs_4_0(&self, buffer: &mut [u8]) -> Result<(), MphyError> {
            // HS-LSS veya diğer UFS 4.0'a özgü veri alma mekanizmaları buraya
            for i in 0..buffer.len() {
                // Benzer şekilde, UFS 4.0 için register erişimi ve hata kontrolü (örnek)
                unsafe {
                    let data_reg_ptr = (self.base_address + DATA_BUFFER_REGISTER_OFFSET) as *const u8;
                    buffer[i] = data_reg_ptr.read_volatile(); // Veriyi data register'ından oku
                }
                // Gerçekte, M-PHY register'larından doğru şekilde okumak ve veri alma protokolünü uygulamak gerekir.
                // Burada, veri alma işlemi sırasında hata oluşup oluşmadığını kontrol etmek ve gerekirse MphyError döndürmek önemlidir.
            }
            Ok(())
        }

        // ... diğer M-PHY fonksiyonları (hız değiştirme, hata kontrolü vb.)
    }

    #[derive(Debug)]
    pub enum MphyError {
        Timeout,
        CrcError,
        InitializationError,
        TransmissionError,
        ReceptionError,
        UnsupportedUFSVersion,
        RegisterError, // Yeni hata türü: Register erişim hataları için
    }

    impl fmt::Display for MphyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MphyError::Timeout => write!(f, "Zaman aşımı"),
                MphyError::CrcError => write!(f, "CRC hatası"),
                MphyError::InitializationError => write!(f, "Başlatma Hatası"),
                MphyError::TransmissionError => write!(f, "Veri Gönderme Hatası"),
                MphyError::ReceptionError => write!(f, "Veri Alma Hatası"),
                MphyError::UnsupportedUFSVersion => write!(f, "Desteklenmeyen UFS Versiyonu"),
                MphyError::RegisterError => write!(f, "Register Erişim Hatası"), // Yeni hata mesajı
            }
        }
    }
}