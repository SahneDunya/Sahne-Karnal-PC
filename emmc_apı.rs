#![no_std] // CustomOS standart kütüphaneye ihtiyaç duymayabilir.

// eMMC donanımının temel adresleri ve register tanımları (Örnek değerlerdir, gerçek donanıma göre değişir!)
const EMMC_BASE_ADDRESS: usize = 0x12345000;
const EMMC_CONTROL_REGISTER: usize = EMMC_BASE_ADDRESS + 0x00;
const EMMC_STATUS_REGISTER: usize = EMMC_BASE_ADDRESS + 0x04;
const EMMC_DATA_PORT: usize = EMMC_BASE_ADDRESS + 0x08;
const EMMC_COMMAND_REGISTER: usize = EMMC_BASE_ADDRESS + 0x0C;
const EMMC_ARGUMENT_REGISTER: usize = EMMC_BASE_ADDRESS + 0x10;
const EMMC_RESPONSE_REGISTER_0: usize = EMMC_BASE_ADDRESS + 0x14; // R1, R5, R6, R7 response part 1
const EMMC_RESPONSE_REGISTER_1: usize = EMMC_BASE_ADDRESS + 0x18; // R2 response part 1
const EMMC_RESPONSE_REGISTER_2: usize = EMMC_BASE_ADDRESS + 0x1C; // R2 response part 2
const EMMC_RESPONSE_REGISTER_3: usize = EMMC_BASE_ADDRESS + 0x20; // R2 response part 3

// eMMC komut kodları (Gerçek eMMC standardına göre daha doğru değerler)
const CMD_GO_IDLE_STATE: u32 = 0 << 0;
const CMD_ALL_SEND_CID: u32 = 2 << 0;
const CMD_SEND_RELATIVE_ADDR: u32 = 3 << 0;
const CMD_SET_DSR: u32 = 4 << 0;
const CMD_SELECT_CARD: u32 = 7 << 0;
const CMD_SEND_IF_COND: u32 = 8 << 0;
const CMD_SEND_CSD: u32 = 9 << 0;
const CMD_SEND_CID: u32 = 10 << 0;
const CMD_STOP_TRANSMISSION: u32 = 12 << 0;
const CMD_SET_BLOCKLEN: u32 = 16 << 0;
const CMD_READ_SINGLE_BLOCK: u32 = 17 << 0;
const CMD_READ_MULTIPLE_BLOCK: u32 = 18 << 0;
const CMD_WRITE_SINGLE_BLOCK: u32 = 24 << 0;
const CMD_WRITE_MULTIPLE_BLOCK: u32 = 25 << 0;
const CMD_APP_CMD: u32 = 55 << 0;
const ACMD_SD_SEND_OP_COND: u32 = 41 << 0;

// Komut bayrakları (Command flags)
const CMD_RESPONSE_NONE: u32 = 0 << 6;
const CMD_RESPONSE_SHORT: u32 = 1 << 6;
const CMD_RESPONSE_LONG: u32 = 2 << 6;
const CMD_CRC_CHECK_ON: u32 = 1 << 5;
const CMD_DATA_PRESENT: u32 = 1 << 4;
const CMD_READ: u32 = 0 << 3;
const CMD_WRITE: u32 = 1 << 3;

// eMMC status bitleri (Gerçek değerlere daha yakın)
const STATUS_READY_FOR_DATA: u32 = 1 << 0;
const STATUS_COMMAND_INHIBIT_CMD: u32 = 1 << 1;
const STATUS_COMMAND_INHIBIT_DAT: u32 = 1 << 2;
const STATUS_DATA_BUSY: u32 = 1 << 3;

// Hata enum'ı güncellendi
#[derive(Debug)]
pub enum EMMCError {
    Timeout,
    CommandFailed(u32, u32), // Komut kodu ve status
    DataTransferFailed,
    InvalidBlockSize(usize),
    InvalidBufferLength(usize, usize), // Beklenen ve gerçek buffer uzunluğu
    InitializationFailed,
    CardIdentificationFailed,
    CardSpecificDataFailed,
    Other(u32),
}

// Yardımcı fonksiyon: Belirtilen sayıda döngü boyunca bekler.
fn delay_cycles(cycles: u32) {
    for _ in 0..cycles {}
}

// Güvenli olmayan (unsafe) bloklar, doğrudan donanım erişimi gerektiğinde kullanılır.

// Belirtilen adresten 32-bit değer okur.
unsafe fn read_u32(address: usize) -> u32 {
    *(address as *const u32)
}

// Belirtilen adrese 32-bit değer yazar.
unsafe fn write_u32(address: usize, value: u32) {
    *(address as *mut u32) = value;
}

// eMMC kontrolcüsünü başlatır. Daha kapsamlı bir başlatma süreci içerir.
pub fn initialize() -> Result<(), EMMCError> {
    unsafe {
        // Reset komutu gönder (CMD_GO_IDLE_STATE)
        send_command_raw(CMD_GO_IDLE_STATE, 0, CMD_RESPONSE_NONE)?;
        delay_cycles(1000);

        // Voltaj aralığını kontrol etme (CMD_SEND_IF_COND) - Örnek argüman
        if send_command_raw(CMD_SEND_IF_COND, 0x1AA, CMD_RESPONSE_SHORT)? != 0x1AA {
            return Err(EMMCError::InitializationFailed);
        }
        delay_cycles(1000);

        // SDHC/SDXC kartlarını kontrol etme (ACMD41) - Uygulamaya özel komut
        let mut timeout = 10000;
        while timeout > 0 {
            send_command_raw(CMD_APP_CMD, 0, CMD_RESPONSE_SHORT)?; // Önce APP_CMD gönderilmeli
            if send_command_raw(ACMD_SD_SEND_OP_COND, 0x40000000, CMD_RESPONSE_SHORT)? & 0x80000000 != 0 {
                break;
            }
            delay_cycles(100);
            timeout -= 1;
        }
        if timeout == 0 {
            return Err(EMMCError::InitializationFailed);
        }
        delay_cycles(1000);

        // CID'yi al (CMD_ALL_SEND_CID)
        let _cid = get_cid()?;
        delay_cycles(1000);

        // Kartı seç (CMD_SELECT_CARD) - Şu anda varsayılan 0 adresi kullanılıyor, gerçekte RCA alınmalı
        if send_command_raw(CMD_SELECT_CARD, 0, CMD_RESPONSE_SHORT)? != 0 {
            return Err(EMMCError::InitializationFailed);
        }
        delay_cycles(1000);

        Ok(())
    }
}

// eMMC'ye ham komut gönderir ve cevabı döndürür.
unsafe fn send_command_raw(command: u32, argument: u32, response_type: u32) -> Result<u32, EMMCError> {
    // Komut göndermeden önce kontrolcünün hazır olup olmadığını kontrol etme
    let mut timeout = 1000;
    while timeout > 0 {
        if read_u32(EMMC_STATUS_REGISTER) & STATUS_COMMAND_INHIBIT_CMD == 0 {
            break;
        }
        delay_cycles(100);
        timeout -= 1;
    }
    if timeout == 0 {
        return Err(EMMCError::Timeout);
    }

    // Argument register'ına argümanı yaz
    write_u32(EMMC_ARGUMENT_REGISTER, argument);

    // Komut register'ına komutu yaz
    write_u32(EMMC_COMMAND_REGISTER, command | response_type | CMD_CRC_CHECK_ON); // CRC varsayılıyor

    // Komutun tamamlanmasını bekleme
    timeout = 1000;
    while timeout > 0 {
        if read_u32(EMMC_STATUS_REGISTER) & STATUS_COMMAND_INHIBIT_CMD == 0 {
            // Cevap tipine göre cevabı oku
            match response_type {
                CMD_RESPONSE_NONE => return Ok(0),
                CMD_RESPONSE_SHORT => return Ok(read_u32(EMMC_RESPONSE_REGISTER_0)),
                CMD_RESPONSE_LONG => return Ok(read_u32(EMMC_RESPONSE_REGISTER_0)), // Uzun cevaplar için henüz tam destek yok
                _ => return Err(EMMCError::Other(2)),
            }
        }
        delay_cycles(100);
        timeout -= 1;
    }
    Err(EMMCError::Timeout)
}

// eMMC'ye komut gönderir ve kısa cevap (R1) bekler.
fn send_command(command: u32, argument: u32) -> Result<u32, EMMCError> {
    unsafe {
        send_command_raw(command, argument, CMD_RESPONSE_SHORT)
    }
}

// eMMC'den CID (Card Identification) bilgisini alır.
pub fn get_cid() -> Result<[u32; 4], EMMCError> {
    unsafe {
        send_command_raw(CMD_ALL_SEND_CID, 0, CMD_RESPONSE_LONG)?;
        Ok([
            read_u32(EMMC_RESPONSE_REGISTER_0),
            read_u32(EMMC_RESPONSE_REGISTER_1),
            read_u32(EMMC_RESPONSE_REGISTER_2),
            read_u32(EMMC_RESPONSE_REGISTER_3),
        ])
    }
}

// eMMC'den CSD (Card Specific Data) bilgisini alır.
pub fn get_csd() -> Result<[u32; 4], EMMCError> {
    unsafe {
        // Önce kartı seçmemiz gerekebilir (eğer seçili değilse)
        // Bu örnekte kartın seçili olduğunu varsayıyoruz.
        send_command_raw(CMD_SEND_CSD, 0, CMD_RESPONSE_LONG)?;
        Ok([
            read_u32(EMMC_RESPONSE_REGISTER_0),
            read_u32(EMMC_RESPONSE_REGISTER_1),
            read_u32(EMMC_RESPONSE_REGISTER_2),
            read_u32(EMMC_RESPONSE_REGISTER_3),
        ])
    }
}

// eMMC'den tek bir blok okur.
pub fn read_single_block(block_number: u32, buffer: &mut [u8]) -> Result<(), EMMCError> {
    if buffer.len() != 512 { // Tipik blok boyutu
        return Err(EMMCError::InvalidBufferLength(512, buffer.len()));
    }

    send_command(CMD_SET_BLOCKLEN, 512)?;
    send_command(CMD_READ_SINGLE_BLOCK, block_number)?; // Blok adreslemesi genellikle blok numarasıdır

    unsafe {
        // Veri transferinin başlamasını bekleme
        let mut timeout = 1000;
        while timeout > 0 {
            if read_u32(EMMC_STATUS_REGISTER) & STATUS_READY_FOR_DATA != 0 {
                break;
            }
            delay_cycles(100);
            timeout -= 1;
        }
        if timeout == 0 {
            return Err(EMMCError::Timeout);
        }

        // Veriyi okuma
        for i in 0..512 / 4 {
            let data = read_u32(EMMC_DATA_PORT);
            buffer[i * 4] = (data & 0xFF) as u8;
            buffer[i * 4 + 1] = ((data >> 8) & 0xFF) as u8;
            buffer[i * 4 + 2] = ((data >> 16) & 0xFF) as u8;
            buffer[i * 4 + 3] = ((data >> 24) & 0xFF) as u8;
        }

        // Veri transferinin tamamlanmasını bekleme
        timeout = 1000;
        while timeout > 0 {
            if read_u32(EMMC_STATUS_REGISTER) & STATUS_DATA_BUSY == 0 {
                return Ok(());
            }
            delay_cycles(100);
            timeout -= 1;
        }
        Err(EMMCError::Timeout)
    }
}

// eMMC'ye tek bir blok yazar.
pub fn write_single_block(block_number: u32, buffer: &[u8]) -> Result<(), EMMCError> {
    if buffer.len() != 512 { // Tipik blok boyutu
        return Err(EMMCError::InvalidBufferLength(512, buffer.len()));
    }

    send_command(CMD_SET_BLOCKLEN, 512)?;
    send_command(CMD_WRITE_SINGLE_BLOCK, block_number)?; // Blok adreslemesi genellikle blok numarasıdır

    unsafe {
        // Veri transferinin başlamasını bekleme
        let mut timeout = 1000;
        while timeout > 0 {
            if read_u32(EMMC_STATUS_REGISTER) & STATUS_READY_FOR_DATA != 0 {
                break;
            }
            delay_cycles(100);
            timeout -= 1;
        }
        if timeout == 0 {
            return Err(EMMCError::Timeout);
        }

        // Veriyi yazma
        for i in 0..512 / 4 {
            let data = (buffer[i * 4] as u32) |
                        ((buffer[i * 4 + 1] as u32) << 8) |
                        ((buffer[i * 4 + 2] as u32) << 16) |
                        ((buffer[i * 4 + 3] as u32) << 24);
            write_u32(EMMC_DATA_PORT, data);
        }

        // Veri transferinin tamamlanmasını bekleme
        timeout = 1000;
        while timeout > 0 {
            if read_u32(EMMC_STATUS_REGISTER) & STATUS_DATA_BUSY == 0 {
                return Ok(());
            }
            delay_cycles(100);
            timeout -= 1;
        }
        Err(EMMCError::Timeout)
    }
}

// Örnek bir fonksiyon: Kartın hazır olup olmadığını kontrol eder.
pub fn is_card_ready() -> bool {
    unsafe {
        read_u32(EMMC_STATUS_REGISTER) & STATUS_READY_FOR_DATA != 0
    }
}

// Örnek bir fonksiyon: Komut göndermeye hazır olup olmadığını kontrol eder.
pub fn is_command_ready() -> bool {
    unsafe {
        read_u32(EMMC_STATUS_REGISTER) & STATUS_COMMAND_INHIBIT_CMD == 0
    }
}

// Örnek bir fonksiyon: Veri transferi için hazır olup olmadığını kontrol eder.
pub fn is_data_ready() -> bool {
    unsafe {
        read_u32(EMMC_STATUS_REGISTER) & STATUS_COMMAND_INHIBIT_DAT == 0
    }
}