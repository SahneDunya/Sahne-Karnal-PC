use crate::error::Error;
use crate::drivers::mmc::emmc_card::EmmcCard; // Assuming EmmcCard is in this module

pub struct MmcCore {
    // hosts: Vec<MmcHost>, // Removed MmcHost
    card: Option<EmmcCard>, // Store the detected eMMC card
}

impl MmcCore {
    pub fn new() -> Self {
        MmcCore { card: None }
    }

    // pub fn register_host(&mut self, host: MmcHost) { // Removed register_host
    //     self.hosts.push(host);
    // }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initialization is now handled in detect_cards
        Ok(())
    }

    pub fn detect_cards(&mut self) -> Result<Vec<EmmcCard>, Error> {
        let mut cards = Vec::new();
        match EmmcCard::new() {
            Some(card) => {
                self.card = Some(card);
                cards.push(self.card.take().unwrap()); // Move the card out of the Option
            }
            None => {
                // Kart bulunamadı
            }
        }
        Ok(cards)
    }

    pub fn read_block(&mut self, block_number: u32, buffer: &mut [u8]) -> Result<(), Error> {
        match &mut self.card {
            Some(card) => match card.read_block(block_number, buffer) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::CardError(format!("Blok okuma hatası: {:?}", e))),
            },
            None => Err(Error::CardError("Kart bulunamadı".to_string())),
        }
    }

    pub fn write_block(&mut self, block_number: u32, buffer: &[u8]) -> Result<(), Error> {
        match &mut self.card {
            Some(card) => match card.write_block(block_number, buffer) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::CardError(format!("Blok yazma hatası: {:?}", e))),
            },
            None => Err(Error::CardError("Kart bulunamadı".to_string())),
        }
    }

    pub fn capacity(&self) -> Option<u64> {
        self.card.as_ref().map(|card| card.capacity())
    }

    // Diğer MMC Core fonksiyonları buraya eklenebilir.
    // Örneğin:
    // - Kart yapılandırma işlemleri
    // - Hata yönetimi
}

// Örnek kullanım:
// Assuming 'EmmcCard' struct and its 'new' method are defined in 'emmc_card.rs'
// and are compatible with the 'emmc_api.rs' provided earlier.

fn main() -> Result<(), Error> {
    let mut mmc_core = MmcCore::new();

    // No need to register host explicitly anymore

    // mmc_core.init()?; // Initialization is done during detection

    let cards = mmc_core.detect_cards()?; // Kartları tespit et

    if cards.is_empty() {
        println!("Hiç kart bulunamadı.");
    } else {
        for mut card in cards {
            println!("Kart bulundu: {:?}", card.mmc);
            println!("Kart kapasitesi: {} bytes", card.capacity());

            // Örnek olarak bir blok okuma ve yazma işlemi (ihtiyaca göre düzenleyin)
            let block_number = 0;
            let mut read_buffer = [0u8; 512];
            let write_buffer = [0xAAu8; 512];

            println!("Blok yazılıyor...");
            match mmc_core.write_block(block_number, &write_buffer) {
                Ok(_) => println!("Blok yazma başarılı."),
                Err(e) => eprintln!("Blok yazma hatası: {:?}", e),
            }

            println!("Blok okunuyor...");
            match mmc_core.read_block(block_number, &mut read_buffer) {
                Ok(_) => {
                    println!("Blok okuma başarılı.");
                    // İstenirse okunan veriyi inceleyebilirsiniz.
                    // println!("Okunan veri: {:?}", &read_buffer[0..32]);
                }
                Err(e) => eprintln!("Blok okuma hatası: {:?}", e),
            }
        }
    }

    Ok(())
}

// MmcHost ve MmcCard struct'larının tanımları kaldırıldı.
// MmcCard struct'ı ve new fonksiyonu 'emmc_card.rs' dosyasında tanımlanmıştır.

// Error handling için daha detaylı bir Error enum'ı (güncellendi)
#[derive(Debug)]
pub enum Error {
    HostError,
    CardError(String), // CardError şimdi bir String mesaj içeriyor
    NotSupported,
    HostInitializationError(String),
    CardDetectionError(String),
    CardInitializationError(String),
    ParameterError(String),
    TimeoutError(String),
    CommunicationError(String),
    UnexpectedResponse(String),
    UnknownError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::HostError => write!(f, "Host Hatası"),
            Error::CardError(msg) => write!(f, "Kart Hatası: {}", msg),
            Error::NotSupported => write!(f, "Desteklenmiyor"),
            Error::HostInitializationError(msg) => write!(f, "Host Başlatma Hatası: {}", msg),
            Error::CardDetectionError(msg) => write!(f, "Kart Tespit Hatası: {}", msg),
            Error::CardInitializationError(msg) => write!(f, "Kart Başlatma Hatası: {}", msg),
            Error::ParameterError(msg) => write!(f, "Parametre Hatası: {}", msg),
            Error::TimeoutError(msg) => write!(f, "Zaman Aşımı Hatası: {}", msg),
            Error::CommunicationError(msg) => write!(f, "İletişim Hatası: {}", msg),
            Error::UnexpectedResponse(msg) => write!(f, "Beklenmeyen Cevap Hatası: {}", msg),
            Error::UnknownError(msg) => write!(f, "Bilinmeyen Hata: {}", msg),
        }
    }
}

impl std::error::Error for Error {}