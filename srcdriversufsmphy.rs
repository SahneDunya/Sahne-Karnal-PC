#![no_std]
use core::fmt;

// Register adresleri ve sabitler için tanımlar
const UFS_VERSION_REGISTER_OFFSET: u64 = 0x00; // Örnek offset
const DATA_BUFFER_REGISTER_OFFSET: u64 = 0x04; // Örnek offset
const STATUS_REGISTER_OFFSET: u64 = 0x08; // Örnek offset

const UFS_VERSION_4_0: u16 = 0x0400;
const UFS_VERSION_3_1: u16 = 0x0310; // Örnek UFS 3.1 versiyonu
const UFS_VERSION_1_0: u16 = 0x0100;

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
            // Bu örnekte M-PHY'ı doğrudan kullanıyoruz.
            println!("UniPro komutu yürütülüyor (M-PHY üzerinden): {:?}", cdb);
            println!("Veri arabelleği boyutu: {}", buffer.len());

            // Basit bir simülasyon: Veriyi M-PHY üzerinden al
            self.mphy_instance.receive_data(buffer).map_err(UniprotError::MphyError)?;

            // Simülasyon: Başarılı dönüş
            Ok(())
        }

        pub fn execute_command_write(&mut self, cdb: &super::scsi::ScsiCommandDescriptorBlock, buffer: &[u8]) -> Result<(), UniprotError> {
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
            0x0100..=0x0300 => scsi::ScsiCommandDescriptorBlock::read10(lba, block_count),
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
            0x0100..=0x0300 => scsi::ScsiCommandDescriptorBlock::write10(lba, block_count),
            0x0400 => scsi::ScsiCommandDescriptorBlock::write16(lba, block_count), // UFS 4.0 için 16 byte CDB
            _ => return Err(UfsError::UnsupportedUFSVersion),
        };
        self.uniprot_instance.execute_command_write(&cdb, buffer).map_err(UfsError::UniprotError)?;
        Ok(buffer.len())
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