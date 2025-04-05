#![no_std]
#![allow(dead_code)]
#![allow(unused_imports)]

// SdError ve SdCardApi tanımlarını buraya kopyalayın
#[derive(Debug)]
pub enum SdError {
    InitializationFailed,
    ReadError,
    WriteError,
    InvalidAddress,
    CardNotPresent,
    Other(String),
}

// SD kart API yapısı
pub struct SdCardApi {
    driver: SdDriver,
    card_capacity_blocks: u32, // Kart kapasitesini saklamak için
}

impl SdCardApi {
    // Yeni bir SD kart API örneği oluşturur
    pub fn new() -> Self {
        SdCardApi {
            driver: SdDriver::new(),
            card_capacity_blocks: 0, // Başlangıçta bilinmiyor
        }
    }

    // SD kartı başlatır
    pub fn init(&mut self) -> Result<(), SdError> {
        println!("SD kart başlatılıyor...");
        match self.driver.init() {
            Ok(_) => {
                println!("SD sürücüsü başarıyla başlatıldı.");
                // Burada kart kapasitesini okuma mantığı eklenebilir.
                // Şu anlık örnek bir değer atayalım.
                self.card_capacity_blocks = 4096;
                Ok(())
            }
            Err(e) => {
                println!("SD sürücüsü başlatma hatası: {:?}", e);
                Err(SdError::InitializationFailed) // Düşük seviyeli hatayı yüksek seviyeli hataya çevir
            }
        }
    }

    // Belirtilen adresten (blok numarasından) veri okur.
    pub fn read_block(&self, address: u32, buffer: &mut [u8], block_size: usize) -> Result<(), SdError> {
        if address >= self.card_capacity_blocks {
            return Err(SdError::InvalidAddress);
        }
        if buffer.len() != block_size {
            return Err(SdError::Other("Yanlış arabellek boyutu".to_string()));
        }
        if block_size != 512 {
            return Err(SdError::Other("Şu anda sadece 512 byte'lık blok boyutları destekleniyor.".to_string()));
        }

        println!("Blok okunuyor: Adres={}, Boyut={}", address, block_size);
        match self.driver.read_block(address, buffer) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("SD sürücüsü okuma hatası: {:?}", e);
                Err(SdError::ReadError) // Düşük seviyeli hatayı yüksek seviyeli hataya çevir
            }
        }
    }

    // Belirtilen adrese (blok numarasına) veri yazar.
    pub fn write_block(&self, address: u32, data: &[u8], block_size: usize) -> Result<(), SdError> {
        if address >= self.card_capacity_blocks {
            return Err(SdError::InvalidAddress);
        }
        if data.len() != block_size {
            return Err(SdError::Other("Yanlış veri boyutu".to_string()));
        }
        if block_size != 512 {
            return Err(SdError::Other("Şu anda sadece 512 byte'lık blok boyutları destekleniyor.".to_string()));
        }

        println!("Blok yazılıyor: Adres={}, Boyut={}", address, block_size);
        match self.driver.write_block(address, data) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("SD sürücüsü yazma hatası: {:?}", e);
                Err(SdError::WriteError) // Düşük seviyeli hatayı yüksek seviyeli hataya çevir
            }
        }
    }

    // SD kartın kapasitesini blok sayısında döndürür.
    pub fn get_card_capacity_blocks(&self) -> u32 {
        println!("SD kart kapasitesi (blok sayısı) alınıyor...");
        self.card_capacity_blocks
    }

    // SD kartın var olup olmadığını kontrol eder.
    pub fn is_card_present(&self) -> bool {
        println!("SD kart varlığı kontrol ediliyor...");
        // Düşük seviyeli sürücüde böyle bir metot olmayabilir,
        // bu bilgi donanım seviyesinde farklı şekillerde alınabilir.
        // Şimdilik her zaman takılı olduğunu varsayalım.
        true
    }
}

// *** İKİNCİ KODUNUZ BURAYA GELECEK ***
const SD_CONTROLLER_BASE_ADDRESS: usize = 0x12345678;

#[repr(C)]
struct SdControllerRegisters {
    data_port: u32,
    command_port: u32,
    status_port: u32,
    interrupt_enable: u32,
    interrupt_status: u32,
    block_size: u32,
    block_count: u32,
    argument: u32,
    response: u32,
}

const CMD0_GO_IDLE_STATE: u8 = 0;
const CMD8_SEND_IF_COND: u8 = 8;
const CMD9_SEND_CSD: u8 = 9;
const CMD16_SET_BLOCKLEN: u8 = 16;
const CMD17_READ_SINGLE_BLOCK: u8 = 17;
const CMD24_WRITE_SINGLE_BLOCK: u8 = 24;
const CMD55_APP_CMD: u8 = 55;
const ACMD41_SD_SEND_OP_COND: u8 = 41;

const R1: u8 = 1;
const R2: u8 = 2;
const R3: u8 = 3;
const R7: u8 = 7;

const STATUS_DATA_READY: u32 = 1 << 0;
const STATUS_COMMAND_FINISHED: u32 = 1 << 1;
const STATUS_ERROR: u32 = 1 << 2;

const ERROR_TIMEOUT: u32 = 1;
const ERROR_CRC: u32 = 2;

unsafe fn read_register<T>(address: usize) -> &'static T {
    &*(address as *const T)
}

unsafe fn write_register<T>(address: usize, value: T) {
    *(address as *mut T) = value;
}

pub struct SdDriver {
    registers: &'static mut SdControllerRegisters,
}

impl SdDriver {
    pub fn new() -> Self {
        let registers = unsafe {
            &mut *(SD_CONTROLLER_BASE_ADDRESS as *mut SdControllerRegisters)
        };
        SdDriver { registers }
    }

    pub fn init(&mut self) -> Result<(), u32> {
        self.delay(1000);
        self.send_command(CMD0_GO_IDLE_STATE, 0)?;
        self.delay(100);
        let response = self.send_command(CMD8_SEND_IF_COND, 0x1AA)?;
        if (response & 0xFF) != 0xAA {
            return Err(ERROR_TIMEOUT);
        }
        for _ in 0..100 {
            self.send_command(CMD55_APP_CMD, 0)?;
            let response = self.send_command(ACMD41_SD_SEND_OP_COND, 1 << 30)?;
            if (response >> 31) == 1 {
                break;
            }
            self.delay(10);
        }
        self.send_command(CMD16_SET_BLOCKLEN, 512)?;
        Ok(())
    }

    pub fn read_block(&mut self, block_address: u32, buffer: &mut [u8]) -> Result<(), u32> {
        if buffer.len() < 512 {
            return Err(ERROR_TIMEOUT);
        }
        self.send_command(CMD17_READ_SINGLE_BLOCK, block_address)?;
        if !self.wait_for_status(STATUS_DATA_READY, 1000) {
            return Err(ERROR_TIMEOUT);
        }
        for i in 0..128 {
            let data = unsafe { read_register::<u32>(SD_CONTROLLER_BASE_ADDRESS + offset_of!(SdControllerRegisters, data_port)) };
            buffer[i * 4 + 0] = (data >> 0) as u8;
            buffer[i * 4 + 1] = (data >> 8) as u8;
            buffer[i * 4 + 2] = (data >> 16) as u8;
            buffer[i * 4 + 3] = (data >> 24) as u8;
        }
        if !self.wait_for_status(STATUS_COMMAND_FINISHED, 1000) {
            return Err(ERROR_TIMEOUT);
        }
        Ok(())
    }

    pub fn write_block(&mut self, block_address: u32, buffer: &[u8]) -> Result<(), u32> {
        if buffer.len() < 512 {
            return Err(ERROR_TIMEOUT);
        }
        self.send_command(CMD24_WRITE_SINGLE_BLOCK, block_address)?;
        self.delay(10);
        for i in 0..128 {
            let data = (buffer[i * 4 + 0] as u32) |
                        ((buffer[i * 4 + 1] as u32) << 8) |
                        ((buffer[i * 4 + 2] as u32) << 16) |
                        ((buffer[i * 4 + 3] as u32) << 24);
            unsafe { write_register(SD_CONTROLLER_BASE_ADDRESS + offset_of!(SdControllerRegisters, data_port), data) };
        }
        if !self.wait_for_status(STATUS_COMMAND_FINISHED, 5000) {
            return Err(ERROR_TIMEOUT);
        }
        Ok(())
    }

    fn send_command(&mut self, command: u8, argument: u32) -> Result<u32, u32> {
        unsafe { write_register(SD_CONTROLLER_BASE_ADDRESS + offset_of!(SdControllerRegisters, argument), argument) };
        unsafe { write_register(SD_CONTROLLER_BASE_ADDRESS + offset_of!(SdControllerRegisters, command_port), command as u32) };
        if !self.wait_for_status(STATUS_COMMAND_FINISHED, 1000) {
            return Err(ERROR_TIMEOUT);
        }
        let response = unsafe { read_register::<u32>(SD_CONTROLLER_BASE_ADDRESS + offset_of!(SdControllerRegisters, response)) };
        Ok(response)
    }

    fn wait_for_status(&self, status_mask: u32, timeout_ms: u32) -> bool {
        for _ in 0..timeout_ms {
            let status = unsafe { read_register::<u32>(SD_CONTROLLER_BASE_ADDRESS + offset_of!(SdControllerRegisters, status_port)) };
            if (status & status_mask) == status_mask {
                return true;
            }
            self.delay(1);
        }
        false
    }

    fn delay(&self, milliseconds: u32) {
        for _ in 0..(milliseconds * 1000) {
            unsafe {
                core::ptr::read_volatile(&0);
            }
        }
    }
}

#[macro_export]
macro_rules! offset_of {
    ($struct:path, $field:tt) => {{
        let dummy = core::mem::MaybeUninit::<$struct>::uninit();
        let ptr = dummy.as_ptr();
        let field_ptr = core::ptr::addr_of!((*ptr).$field);
        (field_ptr as usize) - (ptr as usize)
    }};
}