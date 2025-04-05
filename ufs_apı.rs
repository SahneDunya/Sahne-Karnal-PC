#[derive(Debug)]
pub enum UfsError {
    OpenError,
    ReadError,
    WriteError,
    CloseError,
    InvalidAddress,
    Other(String),
}

// UFS aygıtını temsil eden temel yapı
pub struct UfsDevice {
    // Gerçek bir sistemde, bu yapı UFS aygıtına özel bilgileri içerecektir.
    // Örneğin, temel adres, aygıt boyutu vb.
    device_path: String, // Simülasyon için aygıt yolu
    is_open: bool,
    device_size: u64, // Aygıtın boyutu
}

impl UfsDevice {
    // Yeni bir UFS aygıtı örneği oluşturur.
    // Boyut bilgisi de parametre olarak alınabilir.
    pub fn new(path: &str, size: u64) -> Self {
        UfsDevice {
            device_path: path.to_string(),
            is_open: false,
            device_size: size,
        }
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
        // Gerçek bir sistemde, bu noktada UFS çipine düşük seviyeli okuma komutları
        // gönderilecektir. Veriler doğrudan belleğe veya sağlanan arabelleğe okunacaktır.
        println!(
            "UFS aygıtından okunuyor: adres={}, boyut={}",
            address,
            buffer.len()
        );
        // Simülasyon: Arabelleği bazı örnek verilerle dolduruyoruz.
        for (i, byte) in buffer.iter_mut().enumerate() {
            if address + i as u64 < self.device_size {
                *byte = (address + i as u64) as u8; // Basit bir desen
            } else {
                // Adresin sınırların ötesine geçmesini engelle
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
        // Gerçek bir sistemde, bu noktada UFS çipine düşük seviyeli yazma komutları
        // gönderilecektir. Veriler UFS yongasına yazılacaktır.
        println!(
            "UFS aygıtına yazılıyor: adres={}, boyut={}",
            address,
            buffer.len()
        );
        println!("Yazılacak veri (ilk 8 bayt): {:?}", &buffer[..buffer.len().min(8)]);
        // Simülasyon: Yazma işlemini başarılı olarak kabul ediyoruz.
        Ok(buffer.len().min((self.device_size - address) as usize))
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
    // Bu bilgi genellikle aygıtın kendisinden veya yapılandırmasından alınır.
    pub fn get_size(&self) -> u64 {
        self.device_size
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
}