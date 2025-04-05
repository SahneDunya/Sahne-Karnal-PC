use core::ptr::{read_volatile, write_volatile};

/// M-PCle işlemleri sırasında oluşabilecek hataları temsil eder.
#[derive(Debug)]
pub enum Error {
    /// Geçersiz adres hatası.
    InvalidAddress(usize),
    /// Okuma/Yazma hatası.
    IOError,
    /// Donanım başlatma hatası.
    InitializationError,
    /// Diğer hatalar.
    Other(String),
}

/// M-PCle işlemleri için sonuç türü.
pub type Result<T> = core::result::Result<T, Error>;

/// M-PCle cihazının temel adresini tanımlar (CustomOS'a özel).
pub const MPCLE_BASE_ADDRESS: usize = 0xYOUR_MPCLE_BASE_ADDRESS; // Gerçek adresi buraya yazın!

/// M-PCle aygıtının kontrol kaydının ofseti (CustomOS'a özel).
pub const MPCLE_CONTROL_OFFSET: usize = 0x00;

/// M-PCle aygıtının veri kaydının ofseti (CustomOS'a özel).
pub const MPCLE_DATA_OFFSET: usize = 0x04;

/// M-PCle aygıtını temsil eden yapı.
pub struct MpcleDevice {
    base_address: usize,
}

impl MpcleDevice {
    /// Yeni bir M-PCle aygıtı örneği oluşturur.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn new() -> Result<Self> {
        // Burada donanım başlatma işlemleri yapılabilir (CustomOS'a özel).
        // Örneğin, kontrol kaydına bir başlangıç değeri yazılabilir.
        let device = Self {
            base_address: MPCLE_BASE_ADDRESS,
        };

        // Başlatma başarılıysa Ok döner.
        Ok(device)
    }

    /// Belirtilen ofsetteki 8-bitlik değeri okur.
    ///
    /// # Arguments
    ///
    /// * `offset`: Temel adrese göre okunacak ofset.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn read8(&self, offset: usize) -> Result<u8> {
        let address = self.base_address + offset;
        // Burada adresin geçerli olup olmadığı kontrol edilebilir (CustomOS'a özel).
        if address < self.base_address || address >= self.base_address + 0x1000 { // Örnek bir sınır
            return Err(Error::InvalidAddress(address));
        }
        Ok(read_volatile(address as *const u8))
    }

    /// Belirtilen ofsetteki 16-bitlik değeri okur.
    ///
    /// # Arguments
    ///
    /// * `offset`: Temel adrese göre okunacak ofset.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn read16(&self, offset: usize) -> Result<u16> {
        let address = self.base_address + offset;
        if address % 2 != 0 { // 16-bitlik okumalar için hizalama kontrolü
            return Err(Error::InvalidAddress(address));
        }
        Ok(read_volatile(address as *const u16))
    }

    /// Belirtilen ofsetteki 32-bitlik değeri okur.
    ///
    /// # Arguments
    ///
    /// * `offset`: Temel adrese göre okunacak ofset.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn read32(&self, offset: usize) -> Result<u32> {
        let address = self.base_address + offset;
        if address % 4 != 0 { // 32-bitlik okumalar için hizalama kontrolü
            return Err(Error::InvalidAddress(address));
        }
        Ok(read_volatile(address as *const u32))
    }

    /// Belirtilen ofsetteki 8-bitlik değeri yazar.
    ///
    /// # Arguments
    ///
    /// * `offset`: Temel adrese göre yazılacak ofset.
    /// * `value`: Yazılacak 8-bitlik değer.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn write8(&self, offset: usize, value: u8) -> Result<()> {
        let address = self.base_address + offset;
        if address < self.base_address || address >= self.base_address + 0x1000 { // Örnek bir sınır
            return Err(Error::InvalidAddress(address));
        }
        write_volatile(address as *mut u8, value);
        Ok(())
    }

    /// Belirtilen ofsetteki 16-bitlik değeri yazar.
    ///
    /// # Arguments
    ///
    /// * `offset`: Temel adrese göre yazılacak ofset.
    /// * `value`: Yazılacak 16-bitlik değer.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn write16(&self, offset: usize, value: u16) -> Result<()> {
        let address = self.base_address + offset;
        if address % 2 != 0 { // 16-bitlik yazmalar için hizalama kontrolü
            return Err(Error::InvalidAddress(address));
        }
        write_volatile(address as *mut u16, value);
        Ok(())
    }

    /// Belirtilen ofsetteki 32-bitlik değeri yazar.
    ///
    /// # Arguments
    ///
    /// * `offset`: Temel adrese göre yazılacak ofset.
    /// * `value`: Yazılacak 32-bitlik değer.
    ///
    /// # Güvensiz
    ///
    /// Bu fonksiyon güvensizdir çünkü doğrudan donanım adresleriyle etkileşim kurar.
    pub unsafe fn write32(&self, offset: usize, value: u32) -> Result<()> {
        let address = self.base_address + offset;
        if address % 4 != 0 { // 32-bitlik yazmalar için hizalama kontrolü
            return Err(Error::InvalidAddress(address));
        }
        write_volatile(address as *mut u32, value);
        Ok(())
    }

    // Diğer M-PCle özel fonksiyonları buraya eklenebilir.
    // Örneğin, interrupt yönetimi, DMA işlemleri vb.
}

#[cfg(test)]
mod tests {
    use super::*;
}