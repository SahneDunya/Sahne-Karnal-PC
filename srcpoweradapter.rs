pub mod poweradapter {
    use crate::{arch, kernel, SahneError};

    /// Desteklenen adaptör türlerini tanımlayan enum
    #[derive(Debug, PartialEq)]
    pub enum AdapterType {
        Usb,
        DcBarrel,
        Unknown, // Bilinmeyen veya algılanamayan adaptör türü
        None,    // Adaptör bağlı değil
    }

    /// Güç adaptörünün bağlantı durumunu gösteren enum
    #[derive(Debug, PartialEq)]
    pub enum PowerStatus {
        Connected,    // Adaptör bağlı ve güç sağlıyor
        Disconnected, // Adaptör bağlı değil
        LowVoltage,   // Düşük voltaj algılandı (isteğe bağlı)
        Error,        // Bir hata durumu oluştu
    }

    /// `PowerAdapterManager` yapısı, güç adaptörü yönetimini merkezileştirir.
    pub struct PowerAdapterManager {
        current_adapter_type: AdapterType,
        current_power_status: PowerStatus,
        // Gerekirse, adaptörle ilgili diğer bilgileri (voltaj, akım vb.) burada saklayabiliriz.
    }

    impl PowerAdapterManager {
        /// Yeni bir `PowerAdapterManager` örneği oluşturur.
        pub fn new() -> Self {
            PowerAdapterManager {
                current_adapter_type: AdapterType::None,
                current_power_status: PowerStatus::Disconnected,
            }
        }

        /// Adaptör türünü algıla.
        ///
        /// Bu fonksiyon, sistemdeki düşük seviyeli arayüzleri kullanarak
        /// bağlı adaptörün türünü algılamaya çalışır.
        pub fn detect_adapter_type(&mut self) {
            // **Sahne64'e Özgü Algılama Mantığı (Düşük Seviyeli Kod):**

            // Burada kernel modülündeki ioctl sistem çağrısını kullanarak
            // güç yönetimi aygıt sürücüsüne özel komutlar göndereceğiz.
            // Aygıt sürücüsünün uygun şekilde implemente edildiği varsayılmaktadır.

            // Güç yönetimi aygıtının dosya tanımlayıcısını (fd) almamız gerekebilir.
            // Bu bilgi sistemde sabit olabilir veya başka bir yolla elde edilebilir.
            const POWER_MANAGER_FD: u64 = 4; // Örnek dosya tanımlayıcısı

            match kernel::ioctl(POWER_MANAGER_FD, arch::POWER_MANAGER_IOCTL_GET_USB_CONNECTED, 0) {
                Ok(result) => {
                    if result > 0 {
                        self.current_adapter_type = AdapterType::Usb;
                    } else {
                        match kernel::ioctl(POWER_MANAGER_FD, arch::POWER_MANAGER_IOCTL_GET_DC_CONNECTED, 0) {
                            Ok(result) => {
                                if result > 0 {
                                    self.current_adapter_type = AdapterType::DcBarrel;
                                } else {
                                    match kernel::ioctl(POWER_MANAGER_FD, arch::POWER_MANAGER_IOCTL_GET_ANY_POWER_CONNECTED, 0) {
                                        Ok(result) => {
                                            if result > 0 {
                                                self.current_adapter_type = AdapterType::Unknown;
                                            } else {
                                                self.current_adapter_type = AdapterType::None;
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[poweradapter] Herhangi bir güç algılama hatası: {:?}", e);
                                            self.current_adapter_type = AdapterType::Unknown; // Hata durumunda bilinmeyen olarak işaretleyebiliriz
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[poweradapter] DC barrel algılama hatası: {:?}", e);
                                // USB algılandıysa onu koruyalım, aksi takdirde bilinmeyen yapalım
                                if self.current_adapter_type != AdapterType::Usb {
                                    self.current_adapter_type = AdapterType::Unknown;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[poweradapter] USB algılama hatası: {:?}", e);
                    // DC barrel algılamayı deneyelim
                    match kernel::ioctl(POWER_MANAGER_FD, arch::POWER_MANAGER_IOCTL_GET_DC_CONNECTED, 0) {
                        Ok(result) => {
                            if result > 0 {
                                self.current_adapter_type = AdapterType::DcBarrel;
                            } else {
                                self.current_adapter_type = AdapterType::Unknown;
                            }
                        }
                        Err(e) => {
                            eprintln!("[poweradapter] DC barrel algılama hatası: {:?}", e);
                            self.current_adapter_type = AdapterType::Unknown;
                        }
                    }
                }
            }

            // Adaptör türü algılandığında güç durumunu da güncelle
            self.update_power_status();
        }


        /// Mevcut güç durumunu güncelle.
        ///
        /// Bu fonksiyon, mevcut adaptör türüne ve sistemdeki güç durumuna göre
        /// `current_power_status` alanını günceller.
        fn update_power_status(&mut self) {
            const POWER_MANAGER_FD: u64 = 4; // Örnek dosya tanımlayıcısı

            match self.current_adapter_type {
                AdapterType::Usb | AdapterType::DcBarrel | AdapterType::Unknown => {
                    match kernel::ioctl(POWER_MANAGER_FD, arch::POWER_MANAGER_IOCTL_GET_POWER_AVAILABLE, 0) {
                        Ok(result) => {
                            if result > 0 {
                                match kernel::ioctl(POWER_MANAGER_FD, arch::POWER_MANAGER_IOCTL_GET_VOLTAGE_OK, 0) {
                                    Ok(result) => {
                                        if result > 0 {
                                            self.current_power_status = PowerStatus::Connected;
                                        } else {
                                            self.current_power_status = PowerStatus::LowVoltage; // Düşük voltaj
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("[poweradapter] Voltaj kontrol hatası: {:?}", e);
                                        self.current_power_status = PowerStatus::Error;
                                    }
                                }
                            } else {
                                self.current_power_status = PowerStatus::Disconnected; // Güç yok
                            }
                        }
                        Err(e) => {
                            eprintln!("[poweradapter] Güç durumu kontrol hatası: {:?}", e);
                            self.current_power_status = PowerStatus::Error;
                        }
                    }
                }
                AdapterType::None => {
                    self.current_power_status = PowerStatus::Disconnected; // Adaptör yoksa güç yok
                }
            }
        }

        /// Mevcut adaptör türünü döndürür.
        pub fn get_adapter_type(&self) -> &AdapterType {
            &self.current_adapter_type
        }

        /// Mevcut güç durumunu döndürür.
        pub fn get_power_status(&self) -> &PowerStatus {
            &self.current_power_status
        }

        /// Güç adaptörü bilgisini metin olarak döndürür (debug/log amaçlı).
        pub fn get_power_adapter_info(&self) -> String {
            format!(
                "Adaptör Türü: {:?}, Güç Durumu: {:?}",
                self.current_adapter_type, self.current_power_status
            )
        }
    }

    // **Harici Sistem Fonksiyonları (Sahne64'e Özgü - Artık Kullanılmıyor)**
    //
    // Bu fonksiyonlar önceki örnekte placeholder olarak tanımlanmıştı.
    // Artık kernel::ioctl sistem çağrısı kullanıldığı için bunlara gerek kalmadı.

    // /// Sistemde USB güç adaptörünün algılanıp algılanmadığını kontrol eder.
    // fn is_usb_power_detected() -> bool { ... }

    // /// Sistemde DC barrel jak güç adaptörünün algılanıp algılanmadığını kontrol eder.
    // fn is_dc_barrel_power_detected() -> bool { ... }

    // /// Sistemde herhangi bir güç adaptörünün (USB veya DC barrel) algılanıp algılanmadığını kontrol eder.
    // fn is_any_power_detected() -> bool { ... }

    // /// Sistemde güç kaynağının mevcut olup olmadığını kontrol eder.
    // fn is_power_available() -> bool { ... }

    // /// Güç voltaj seviyesinin kabul edilebilir aralıkta olup olmadığını kontrol eder (isteğe bağlı).
    // fn check_voltage_level_ok() -> bool { ... }


    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::kernel::{self as mock_kernel, SahneError};
        use crate::arch;

        // Basit bir mock kernel modülü (gerçek testler için daha gelişmiş bir yapı gerekebilir)
        mod mock_kernel {
            use super::*;
            use std::cell::RefCell;

            thread_local! {
                static USB_CONNECTED: RefCell<bool> = RefCell::new(false);
                static DC_CONNECTED: RefCell<bool> = RefCell::new(false);
                static ANY_CONNECTED: RefCell<bool> = RefCell::new(false);
                static POWER_AVAILABLE: RefCell<bool> = RefCell::new(false);
                static VOLTAGE_OK: RefCell<bool> = RefCell::new(false);
            }

            pub fn set_usb_connected(value: bool) {
                USB_CONNECTED.with(|v| *v.borrow_mut() = value);
            }

            pub fn set_dc_connected(value: bool) {
                DC_CONNECTED.with(|v| *v.borrow_mut() = value);
            }

            pub fn set_any_connected(value: bool) {
                ANY_CONNECTED.with(|v| *v.borrow_mut() = value);
            }

            pub fn set_power_available(value: bool) {
                POWER_AVAILABLE.with(|v| *v.borrow_mut() = value);
            }

            pub fn set_voltage_ok(value: bool) {
                VOLTAGE_OK.with(|v| *v.borrow_mut() = value);
            }

            pub fn ioctl(fd: u64, request: u64, arg: u64) -> Result<i64, SahneError> {
                if fd == 4 { // Örnek POWER_MANAGER_FD
                    match request {
                        arch::POWER_MANAGER_IOCTL_GET_USB_CONNECTED => USB_CONNECTED.with(|v| Ok(if *v.borrow() { 1 } else { 0 })),
                        arch::POWER_MANAGER_IOCTL_GET_DC_CONNECTED => DC_CONNECTED.with(|v| Ok(if *v.borrow() { 1 } else { 0 })),
                        arch::POWER_MANAGER_IOCTL_GET_ANY_POWER_CONNECTED => ANY_CONNECTED.with(|v| Ok(if *v.borrow() { 1 } else { 0 })),
                        arch::POWER_MANAGER_IOCTL_GET_POWER_AVAILABLE => POWER_AVAILABLE.with(|v| Ok(if *v.borrow() { 1 } else { 0 })),
                        arch::POWER_MANAGER_IOCTL_GET_VOLTAGE_OK => VOLTAGE_OK.with(|v| Ok(if *v.borrow() { 1 } else { 0 })),
                        _ => Err(SahneError::InvalidParameter),
                    }
                } else {
                    Err(SahneError::InvalidFileDescriptor)
                }
            }
        }

        #[test]
        fn test_power_adapter_manager_creation() {
            let power_manager = PowerAdapterManager::new();
            assert_eq!(power_manager.get_adapter_type(), &AdapterType::None);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::Disconnected);
        }

        #[test]
        fn test_detect_adapter_type_usb() {
            mock_kernel::set_usb_connected(true);
            mock_kernel::set_power_available(true);
            mock_kernel::set_voltage_ok(true);

            let mut power_manager = PowerAdapterManager::new();
            power_manager.detect_adapter_type();

            assert_eq!(power_manager.get_adapter_type(), &AdapterType::Usb);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::Connected);
        }

        #[test]
        fn test_detect_adapter_type_dc_barrel() {
            mock_kernel::set_usb_connected(false);
            mock_kernel::set_dc_connected(true);
            mock_kernel::set_power_available(true);
            mock_kernel::set_voltage_ok(true);

            let mut power_manager = PowerAdapterManager::new();
            power_manager.detect_adapter_type();

            assert_eq!(power_manager.get_adapter_type(), &AdapterType::DcBarrel);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::Connected);
        }

        #[test]
        fn test_detect_adapter_type_unknown() {
            mock_kernel::set_usb_connected(false);
            mock_kernel::set_dc_connected(false);
            mock_kernel::set_any_connected(true);
            mock_kernel::set_power_available(true);
            mock_kernel::set_voltage_ok(true);

            let mut power_manager = PowerAdapterManager::new();
            power_manager.detect_adapter_type();

            assert_eq!(power_manager.get_adapter_type(), &AdapterType::Unknown);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::Connected);
        }

        #[test]
        fn test_detect_adapter_type_none() {
            mock_kernel::set_usb_connected(false);
            mock_kernel::set_dc_connected(false);
            mock_kernel::set_any_connected(false);

            let mut power_manager = PowerAdapterManager::new();
            power_manager.detect_adapter_type();

            assert_eq!(power_manager.get_adapter_type(), &AdapterType::None);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::Disconnected);
        }

        #[test]
        fn test_power_status_low_voltage() {
            mock_kernel::set_usb_connected(true);
            mock_kernel::set_power_available(true);
            mock_kernel::set_voltage_ok(false);

            let mut power_manager = PowerAdapterManager::new();
            power_manager.detect_adapter_type();

            assert_eq!(power_manager.get_adapter_type(), &AdapterType::Usb);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::LowVoltage);
        }

        #[test]
        fn test_power_status_disconnected() {
            mock_kernel::set_usb_connected(true);
            mock_kernel::set_power_available(false);

            let mut power_manager = PowerAdapterManager::new();
            power_manager.detect_adapter_type();

            assert_eq!(power_manager.get_adapter_type(), &AdapterType::Usb);
            assert_eq!(power_manager.get_power_status(), &PowerStatus::Disconnected);
        }
    }
}

// Örnek bir konsol fonksiyonu (isteğe bağlı)
#[allow(dead_code)]
pub fn clear_screen() {
    // Düşük seviyeli konsol temizleme kodu buraya gelebilir.
    // Örneğin, belirli bir bellek adresine kontrol karakterleri yazılabilir.
    // Bu tamamen donanıma ve konsol sürücüsüne özgüdür.
    println!("[console] Ekran temizleme işlemi (gerçek implementasyon donanıma özeldir)");
}

#[allow(dead_code)]
pub fn print_string(s: &str) {
    // Düşük seviyeli string yazdırma kodu buraya gelebilir.
    // Örneğin, karakterleri doğrudan video belleğine veya bir UART portuna yazılabilir.
    // Bu tamamen donanıma ve konsol sürücüsüne özgüdür.
    println!("[console] Yazdırılıyor: {}", s);
}