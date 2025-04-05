#![no_std]
use core::fmt::Debug;

#[derive(Debug)]
pub enum BlockDeviceError {
    IoError,
    UnsupportedOperation,
    OutOfBounds,
    Other(u32) // Diğer hatalar için
}

pub type Result<T> = core::result::Result<T, BlockDeviceError>;

/// Blok cihazları için temel arayüz.
pub trait BlockDevice: Debug {
    /// Cihazdan veri okur.
    ///
    /// # Parametreler
    ///
    /// * `buf`: Verinin okunacağı arabellek.
    /// * `offset`: Okunacak verinin başlangıç ofseti (bayt cinsinden).
    fn read(&self, buf: &mut [u8], offset: u64) -> Result<()>;

    /// Cihaza veri yazar.
    ///
    /// # Parametreler
    ///
    /// * `buf`: Yazılacak veriyi içeren arabellek.
    /// * `offset`: Yazılacak verinin başlangıç ofseti (bayt cinsinden).
    fn write(&self, buf: &[u8], offset: u64) -> Result<()>;

    /// Cihazın boyutunu bayt cinsinden döndürür.
    fn size(&self) -> Result<u64> {
        Err(BlockDeviceError::UnsupportedOperation) // Varsayılan olarak desteklenmiyor
    }

    /// Blok boyutunu bayt cinsinden döndürür.
    fn block_size(&self) -> Result<u32> {
        Ok(512) // Varsayılan blok boyutu 512 bayt
    }

    /// Cihazın adını döndürür.
    fn name(&self) -> &str;
}