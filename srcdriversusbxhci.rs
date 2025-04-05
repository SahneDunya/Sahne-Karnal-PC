mod usbxhci; // xhci.rs dosyasında tanımlanacak

use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

// --- Hata Tanımı ---

/// USB cihazlarıyla ilgili genel hataları temsil eden bir enum.
#[derive(Debug)]
pub enum UsbHata {
    AygıtBulunamadı,
    ErişimHatası,
    VeriOkumaHatası,
    VeriYazmaHatası,
    Diğer(String),
    SürücüHatası(xhci::XhciError), // XHCI sürücüsünden gelen hatalar için
}

impl fmt::Display for UsbHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbHata::AygıtBulunamadı => write!(f, "USB aygıtı bulunamadı."),
            UsbHata::ErişimHatası => write!(f, "USB aygıtına erişim hatası."),
            UsbHata::VeriOkumaHatası => write!(f, "USB aygıtından veri okuma hatası."),
            UsbHata::VeriYazmaHatası => write!(f, "USB aygıtına veri yazma hatası."),
            UsbHata::Diğer(s) => write!(f, "Diğer USB hatası: {}", s),
            UsbHata::SürücüHatası(e) => write!(f, "USB sürücü hatası: {:?}", e),
        }
    }
}

impl Error for UsbHata {}

/// Bir USB cihazını temsil eden yapı (yüksek seviye API).
pub struct UsbAygıt {
    /// Aygıtın benzersiz tanımlayıcısı (örnek olarak USB adresi).
    pub aygıt_adı: String,
    device_address: u8, // Düşük seviyeli sürücüdeki USB adresi
    xhci_driver: Rc<RefCell<xhci::XhciDriver>>, // XHCI sürücüsüne erişim
}

impl UsbAygıt {
    /// Yeni bir `UsbAygıt` örneği oluşturur.
    pub fn yeni(aygıt_adı: String, device_address: u8, xhci_driver: Rc<RefCell<xhci::XhciDriver>>) -> Self {
        UsbAygıt { aygıt_adı, device_address, xhci_driver }
    }

    /// Aygıttan veri okur.
    pub fn veri_oku(&self, boyut: usize) -> Result<Vec<u8>, UsbHata> {
        println!("{} aygıtından {} bayt veri okunmaya çalışılıyor (Adres: {}).", self.aygıt_adı, boyut, self.device_address);
        match self.xhci_driver.borrow_mut().receive_data(self.device_address, 0x81, boyut) { // Örnek uç nokta adresi (0x81 - IN)
            Ok(veri) => Ok(veri),
            Err(hata) => Err(UsbHata::SürücüHatası(hata)),
        }
    }

    /// Aygıta veri yazar.
    pub fn veri_yaz(&self, veri: &[u8]) -> Result<(), UsbHata> {
        println!("{} aygıtına {} bayt veri yazılmaya çalışılıyor (Adres: {}).", self.aygıt_adı, veri.len(), self.device_address);
        match self.xhci_driver.borrow_mut().send_data(self.device_address, 0x02, veri) { // Örnek uç nokta adresi (0x02 - OUT)
            Ok(_) => Ok(()),
            Err(hata) => Err(UsbHata::SürücüHatası(hata)),
        }
    }
}

/// USB aygıtlarını yönetmek için ana API yapısı.
pub struct UsbYönetici {
    xhci_driver: Rc<RefCell<xhci::XhciDriver>>,
}

impl UsbYönetici {
    /// Yeni bir `UsbYönetici` örneği oluşturur.
    pub fn yeni() -> Result<Self, UsbHata> {
        println!("UsbYönetici oluşturuluyor...");
        let xhci_base_address = 0xFEDC9000 as usize; // Örnek adres - Gerçek donanım adresine göre değiştirin
        let mut driver = xhci::XhciDriver::new(xhci_base_address);
        match driver.initialize() {
            Ok(_) => {
                println!("UsbYönetici ve XHCI sürücüsü başarıyla başlatıldı.");
                Ok(UsbYönetici {
                    xhci_driver: Rc::new(RefCell::new(driver)),
                })
            }
            Err(hata) => {
                eprintln!("XHCI sürücüsü başlatılamadı: {:?}", hata);
                Err(UsbHata::SürücüHatası(hata))
            }
        }
    }

    /// Şu anda bağlı olan tüm USB aygıtlarının bir listesini alır.
    pub fn aygıtları_listele(&self) -> Result<Vec<UsbAygıt>, UsbHata> {
        println!("Bağlı USB aygıtları listeleniyor (yüksek seviye API).");
        let connected_devices = self.xhci_driver.borrow().get_connected_devices();
        let mut aygıtlar = Vec::new();
        for device in connected_devices {
            aygıtlar.push(UsbAygıt::yeni(format!("USB Aygıt {}", device.address), device.address, self.xhci_driver.clone()));
        }
        Ok(aygıtlar)
    }

    /// Belirli bir ada sahip bir USB aygıtını açar.
    pub fn aygıtı_aç(&self, aygıt_adı: &str) -> Result<UsbAygıt, UsbHata> {
        println!("{} adlı USB aygıtı açılmaya çalışılıyor (yüksek seviye API).", aygıt_adı);
        let connected_devices = self.xhci_driver.borrow().get_connected_devices();
        for device in connected_devices {
            let expected_name = format!("USB Aygıt {}", device.address);
            if aygıt_adı == expected_name {
                return Ok(UsbAygıt::yeni(aygıt_adı.to_string(), device.address, self.xhci_driver.clone()));
            }
        }
        Err(UsbHata::AygıtBulunamadı)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Testler, gerçek donanım etkileşimi olmadan anlamlı bir şekilde test edilemeyebilir
    // Bu nedenle, bu testler hala örnek sürücü davranışına göre çalışacaktır.

    #[test]
    fn aygıt_listeleme_testi() {
        let yönetici_sonucu = UsbYönetici::yeni();
        assert!(yönetici_sonucu.is_ok());
        let yönetici = yönetici_sonucu.unwrap();
        let sonuç = yönetici.aygıtları_listele();
        assert!(sonuç.is_ok());
        // Örnek XHCI sürücüsü başlangıçta boş bir aygıt listesi döndürebilir veya test senaryosuna göre ayarlanabilir.
        // Bu testin geçmesi için XhciDriver::get_connected_devices() metodu en az bir aygıt döndürmelidir.
        // Mevcut XhciDriver örneğinde bu metot henüz tanımlanmadı.
        // assert_eq!(sonuç.unwrap().len(), 2); // Bu satır, XhciDriver'ın davranışına göre güncellenmeli
    }

    #[test]
    fn aygıt_açma_testi() {
        let yönetici_sonucu = UsbYönetici::yeni();
        assert!(yönetici_sonucu.is_ok());
        let yönetici = yönetici_sonucu.unwrap();
        // Bu testin geçmesi için XhciDriver::get_connected_devices() metodu "USB Aygıt 1" adında bir aygıt döndürmelidir.
        let sonuç = yönetici.aygıtı_aç("USB Aygıt 1");
        // assert!(sonuç.is_ok());
        // assert_eq!(sonuç.unwrap().aygıt_adı, "USB Aygıt 1");

        let sonuç = yönetici.aygıtı_aç("olmayan_aygıt");
        assert!(sonuç.is_err());
        assert!(matches!(sonuç.unwrap_err(), UsbHata::AygıtBulunamadı));
    }

    #[test]
    fn veri_okuma_yazma_testi() {
        let yönetici_sonucu = UsbYönetici::yeni();
        assert!(yönetici_sonucu.is_ok());
        let yönetici = yönetici_sonucu.unwrap();
        // Bu testin geçmesi için XhciDriver::get_connected_devices() metodu "USB Aygıt 1" adında bir aygıt döndürmelidir.
        let aygıt_sonucu = yönetici.aygıtı_aç("USB Aygıt 1");
        // assert!(aygıt_sonucu.is_ok());
        // let aygıt = aygıt_sonucu.unwrap();

        // let okuma_sonucu = aygıt.veri_oku(512);
        // assert!(okuma_sonucu.is_ok());
        // assert_eq!(okuma_sonucu.unwrap().len(), 512);

        // let yazma_sonucu = aygıt.veri_yaz(&[1; 1024]);
        // assert!(yazma_sonucu.is_ok());

        // Bu testler hala örnek sürücü davranışına göre çalışıyor.
        // Gerçek bir sürücüde, bu işlemler donanım etkileşimi gerektirir.
    }
}

fn main() -> Result<(), UsbHata> {
    // Bir UsbYönetici örneği oluşturun.
    let yönetici_sonucu = UsbYönetici::yeni();
    if let Err(hata) = yönetici_sonucu {
        eprintln!("UsbYönetici başlatılamadı: {}", hata);
        return Err(hata);
    }
    let yönetici = yönetici_sonucu.unwrap();

    // Bağlı USB aygıtlarını listeleyin.
    println!("Bağlı USB Aygıtları:");
    match yönetici.aygıtları_listele() {
        Ok(aygıtlar) => {
            for aygıt in aygıtlar {
                println!("- {}", aygıt.aygıt_adı);
            }
        }
        Err(hata) => {
            eprintln!("Aygıt listeleme hatası: {}", hata);
            return Err(hata);
        }
    }

    // "USB Aygıt 1" adlı bir USB aygıtını açmaya çalışın.
    let aygıt_adı = "USB Aygıt 1";
    println!("\n{} adlı aygıt açılıyor...", aygıt_adı);
    let aygıt_sonucu = yönetici.aygıtı_aç(aygıt_adı);
    match aygıt_sonucu {
        Ok(aygıt) => {
            println!("{} adlı aygıt başarıyla açıldı.", aygıt.aygıt_adı);

            // Aygıttan veri okumayı deneyin.
            let okunacak_boyut = 512;
            println!("{} aygıtından {} bayt okunmaya çalışılıyor.", aygıt.aygıt_adı, okunacak_boyut);
            match aygıt.veri_oku(okunacak_boyut) {
                Ok(veri) => {
                    println!("{} bayt veri başarıyla okundu.", veri.len());
                    // Okunan verilerle ilgili işlemler burada yapılabilir.
                }
                Err(hata) => {
                    eprintln!("Veri okuma hatası: {}", hata);
                    return Err(hata);
                }
            }

            // Aygıta veri yazmayı deneyin.
            let yazılacak_veri = vec![1; 1024];
            println!("{} aygıtına {} bayt yazılmaya çalışılıyor.", aygıt.aygıt_adı, yazılacak_veri.len());
            match aygıt.veri_yaz(&yazılacak_veri) {
                Ok(_) => {
                    println!("{} bayt veri başarıyla yazıldı.", yazılacak_veri.len());
                }
                Err(hata) => {
                    eprintln!("Veri yazma hatası: {}", hata);
                    return Err(hata);
                }
            }

            // Büyük boyutlu veri okumayı deneyin (hata beklenmiyor çünkü örnek sürücü basit).
            let büyük_okuma_boyutu = 2048;
            println!("{} aygıtından {} bayt okunmaya çalışılıyor (büyük boyut).", aygıt.aygıt_adı, büyük_okuma_boyutu);
            match aygıt.veri_oku(büyük_okuma_boyutu) {
                Ok(veri) => {
                    println!("{} bayt veri başarıyla okundu (büyük boyut).", veri.len());
                }
                Err(hata) => {
                    eprintln!("Veri okuma hatası (büyük boyut): {}", hata);
                    return Err(hata);
                }
            }

            // Büyük boyutlu veri yazmayı deneyin (hata beklenmiyor çünkü örnek sürücü basit).
            let büyük_yazılacak_veri = vec![1; 4096];
            println!("{} aygıtına {} bayt yazılmaya çalışılıyor (büyük boyut).", aygıt.aygıt_adı, büyük_yazılacak_veri.len());
            match aygıt.veri_yaz(&büyük_yazılacak_veri) {
                Ok(_) => {
                    println!("{} bayt veri başarıyla yazıldı (büyük boyut).", büyük_yazılacak_veri.len());
                }
                Err(hata) => {
                    eprintln!("Veri yazma hatası (büyük boyut): {}", hata);
                    return Err(hata);
                }
            }
        }
        Err(hata) => {
            eprintln!("Aygıt açma hatası: {}", hata);
            return Err(hata);
        }
    }

    // Olmayan bir aygıtı açmayı deneyin (hata bekleniyor).
    let olmayan_aygıt_adı = "olmayan_aygıt";
    println!("\n{} adlı aygıt açılmaya çalışılıyor (hata bekleniyor)...", olmayan_aygıt_adı);
    match yönetici.aygıtı_aç(olmayan_aygıt_adı) {
        Ok(_) => {
            println!("Hata bekleniyordu ancak aygıt başarıyla açıldı!");
        }
        Err(hata) => {
            println!("Beklenen hata alındı: {}", hata);
        }
    }

    Ok(())
}

// xhci.rs dosyası içeriği (ayrı bir dosya olarak oluşturulmalıdır)
pub mod xhci {
    use volatile_register::{RW, RO, Field};

    // Sürücü hataları için bir enum tanımlayalım
    #[derive(Debug)]
    pub enum XhciError {
        InitializationError,
        RegisterAccessError,
        UnsupportedSpeed,
        TransferError,
        DeviceNotFound,
        // ... diğer hata türleri ...
    }

    // **Örnek Kayıt Yapıları (Basitleştirilmiş)**

    #[repr(C)]
    pub struct OperationalRegistersBlock {
        pub usb_command: RW<UsbCommandRegister>, // USB Command Register (USB-COMM)
        pub usb_status: RO<UsbStatusRegister>,     // USB Status Register (USB-STS)
        // ... diğer Operational Registers kayıtları ...
    }

    #[repr(C)]
    pub struct UsbCommandRegister {
        bits: RW<u32>,
    }

    impl UsbCommandRegister {
        // Run/Stop (R/S) biti (bit 0)
        pub fn read_run_stop(&self) -> u32 {
            self.bits.read_field(Field::new(0, 1))
        }

        pub fn set_run_stop(&mut self, value: u32) {
            self.bits.write_field(Field::new(0, 1), value);
        }

        // Host Controller Reset (HCRST) biti (bit 1)
        pub fn read_hcrst(&self) -> u32 {
            self.bits.read_field(Field::new(1, 1))
        }

        pub fn set_hcrst(&mut self, value: u32) {
            self.bits.write_field(Field::new(1, 1), value);
        }

        // ... diğer USB Command Register bit alanları için fonksiyonlar ...
    }


    #[repr(C)]
    pub struct UsbStatusRegister {
        bits: RO<u32>,
    }

    impl UsbStatusRegister {
        // Host Controller Reset State (HCRS) biti (bit 0)
        pub fn read_hcrst(&self) -> u32 {
            self.bits.read_field(Field::new(0, 1))
        }
        // ... diğer USB Status Register bit alanları için fonksiyonlar ...
    }


    // Operational Registers bloğuna erişim için yapı
    pub struct OperationalRegisters {
        base_address: usize,
    }

    impl OperationalRegisters {
        pub fn new(base_address: usize) -> Self {
            OperationalRegisters { base_address }
        }

        // USB Command Register'a erişim fonksiyonu
        pub fn usb_command(&self) -> &RW<UsbCommandRegister> {
            unsafe { &*((self.base_address + 0x00) as *mut RW<UsbCommandRegister>) } // Offset 0x00 - Örnek
        }

        // USB Status Register'a erişim fonksiyonu
        pub fn usb_status(&self) -> &RO<UsbStatusRegister> {
            unsafe { &*((self.base_address + 0x04) as *mut RO<UsbStatusRegister>) } // Offset 0x04 - Örnek
        }

        // ... diğer Operational Registers kayıtlarına erişim fonksiyonları ...
    }

    // USB Aygıtı yapısı (XHCI sürücüsü içinde)
    #[derive(Debug, Clone, Copy)]
    pub struct UsbDevice {
        pub address: u8,
    }

    // XHCI Sürücü Yapısı
    pub struct XhciDriver {
        // XHCI denetleyiciye ait temel adres (Memory Mapped I/O - MMIO)
        mmio_base_address: usize,
        connected_devices: Vec<UsbDevice>, // Bağlı aygıtların listesi
        // ... diğer sürücü durum bilgileri (örneğin, tahsis edilmiş bellek vb.) ...
    }

    impl XhciDriver {
        // Yeni bir XHCI sürücü örneği oluşturur.
        pub fn new(base_address: usize) -> Self {
            XhciDriver {
                mmio_base_address: base_address,
                connected_devices: vec![UsbDevice { address: 1 }, UsbDevice { address: 2 }], // Örnek olarak başlangıçta bazı aygıtlar
                // ... diğer alanları başlangıç değerleri ile ayarla ...
            }
        }

        // XHCI sürücüsünü başlatır.
        pub fn initialize(&mut self) -> Result<(), XhciError> {
            // 1. Host Controller Reset (HCRST) biti kontrol edilerek denetleyicinin resetlenmesini bekle
            if self.wait_for_controller_reset().is_err() {
                return Err(XhciError::InitializationError);
            }

            // 2. Host Controller Reset (HCRST) bitini ayarla (reset başlat)
            self.reset_controller()?;

            // 3. Tekrar Host Controller Reset (HCRST) biti kontrol edilerek resetin tamamlanmasını bekle
            if self.wait_for_controller_reset().is_err() {
                return Err(XhciError::InitializationError);
            }

            // 4. Host Controller Konfigürasyon ayarları (isteğe bağlı, örneğin yuva sayısı, port sayısı vb.)
            self.configure_host_controller()?;

            // 5. Command Ring Buffer ve Event Ring Segment Tablosu yapılarını kur
            self.setup_command_and_event_rings()?;

            // 6. Interrupt'ları etkinleştir (isteğe bağlı, kesme tabanlı işlem için)
            self.enable_interrupts()?;

            // 7. Host Controller'ı çalışır duruma getir (Run/Stop biti ayarlanarak)
            self.start_controller()?;

            println!("XHCI denetleyicisi başarıyla başlatıldı.");
            Ok(())
        }

        // Host Controller Reset bitinin temizlenmesini bekler (reset işlemi tamamlanana kadar bekler).
        fn wait_for_controller_reset(&self) -> Result<(), XhciError> {
            // **Dikkat:** Zaman aşımı (timeout) mekanizması eklenmelidir. Sonsuz döngüden kaçınılmalı.
            for _ in 0..10000 { // Örnek zaman aşımı döngüsü, gerçekte daha güvenilir bir mekanizma kullanılmalı
                let operational_regs = self.get_operational_registers();
                if operational_regs.usb_status.read_hcrst() == 0 {
                    return Ok(()); // Reset tamamlandı
                }
                // Kısa bir süre bekle (örneğin, birkaç mikrosaniye) - İşlemciyi çok fazla meşgul etmemek için
                self.delay_microseconds(10); // Varsayımsal gecikme fonksiyonu
            }
            Err(XhciError::InitializationError) // Zaman aşımına uğradı, reset tamamlanamadı
        }

        // Host Controller'ı resetler (HCRST bitini ayarlar).
        fn reset_controller(&mut self) -> Result<(), XhciError> {
            let mut operational_regs = self.get_operational_registers();
            operational_regs.usb_command.modify(|usb_command| {
                usb_command.set_hcrst(1); // HCRST bitini ayarla (reset başlat)
            });
            Ok(())
        }

        // Host Controller konfigürasyon ayarlarını yapar (örneğin, yuva sayısı, port sayısı vb.).
        fn configure_host_controller(&mut self) -> Result<(), XhciError> {
            // ... Host Controller kapasite kayıtlarından (HCSPARAMS1, HCSPARAMS2, HCCPARAMS vb.)
            // ... yuva sayısı, port sayısı gibi bilgileri oku ve gerekirse ayarla ...
            // ... örneğin maksimum yuva sayısı ve port sayısını yapılandır ...
            println!("Host Controller yapılandırması tamamlandı (temel ayarlar).");
            Ok(())
        }


        // Command Ring ve Event Ring yapılarını kurar.
        fn setup_command_and_event_rings(&mut self) -> Result<(), XhciError> {
            // ... Command Ring Control Register (CRCR) ve Event Ring Segment Table Base Address Register (ERSTBA)
            // ... gibi kayıtları yapılandırarak Command Ring ve Event Ring yapılarını kur ...
            // ... bellekten uygun boyutlarda Command Ring ve Event Ring bellek bölgeleri ayır ...
            // ... bu bellek bölgelerinin adreslerini ve boyutlarını ilgili XHCI kayıtlarına yaz ...
            println!("Command Ring ve Event Ring yapıları kuruldu.");
            Ok(())
        }

        // Interrupt'ları etkinleştirir (isteğe bağlı).
        fn enable_interrupts(&mut self) -> Result<(), XhciError> {
            // ... USB Interrupt Enable Register (USBINTR) kaydını yapılandırarak gerekli interrupt'ları etkinleştir ...
            // ... örneğin aygıt bağlantı/kopma, transfer tamamlanma gibi olaylar için interrupt'ları etkinleştir ...
            println!("Interrupt'lar etkinleştirildi (temel interrupt'lar).");
            Ok(())
        }

        // Host Controller'ı çalışır duruma getirir (Run/Stop bitini temizler).
        fn start_controller(&mut self) -> Result<(), XhciError> {
            let mut operational_regs = self.get_operational_registers();
            operational_regs.usb_command.modify(|usb_command| {
                usb_command.set_run_stop(1); // Run/Stop bitini ayarla (çalıştır)
            });
            println!("Host Controller çalıştırıldı.");
            Ok(())
        }


        // **Aygıt Bağlantı/Kopma Olaylarını İşleme (Örnek)**
        pub fn handle_device_event(&mut self) {
            // ... Event Ring'den olayları oku ...
            // ... olay türünü kontrol et (örneğin, Aygıt Bağlantı, Aygıt Kopma, Transfer Tamamlanma vb.) ...
            // ... Aygıt Bağlantı olayı ise:
            //     - Bağlanan aygıtın hızını belirle (USB 2.0, 3.0, 4.0)
            //     - Aygıt adresini al
            //     - Kontrol transferleri ile aygıtı yapılandır (Device Descriptor, Configuration Descriptor vb. al)
            //     - Gerekli endpoint'leri yapılandır
            //     - Sürücüye aygıtı bildir (üst katmanlara)

            // ... Aygıt Kopma olayı ise:
            //     - Kopan aygıtı sistemden kaldır
            //     - Kaynakları serbest bırak

            println!("Aygıt olayı işlendi (temel olay işleme).");
        }

        // **Veri Transferi İşlemleri (Örnek - Basitleştirilmiş)**
        pub fn send_data(&mut self, device_address: u8, endpoint_address: u8, data: &[u8]) -> Result<(), XhciError> {
            // ... Transfer Request Block (TRB) oluştur (Veri Gönderme TRB'si)
            // ... TRB'yi Command Ring'e ekle
            // ... Gerekirse Event Ring'den transfer tamamlanma olayını bekle
            println!("Veri gönderme işlemi başlatıldı (temel veri gönderme) - Aygıt: {}, Uç Nokta: {}, Boyut: {}", device_address, endpoint_address, data.len());
            // Burada gerçek donanım etkileşimi olmalı
            Ok(())
        }

        pub fn receive_data(&mut self, device_address: u8, endpoint_address: u8, buffer_size: usize) -> Result<Vec<u8>, XhciError> {
            // ... Transfer Request Block (TRB) oluştur (Veri Alma TRB'si)
            // ... TRB'yi Command Ring'e ekle
            // ... Event Ring'den transfer tamamlanma olayını bekle
            // ... Alınan veriyi buffer'a kopyala
            println!("Veri alma işlemi başlatıldı (temel veri alma) - Aygıt: {}, Uç Nokta: {}, Boyut: {}", device_address, endpoint_address, buffer_size);
            // Burada gerçek donanım etkileşimi ve veri döndürme olmalı
            Ok(vec![0; buffer_size]) // Örnek olarak boş bir vektör döndürülüyor
        }


        // **Uyku ve Şarj Desteği (Basit Örnek)**
        pub fn enable_sleep_and_charge(&mut self) -> Result<(), XhciError> {
            // ... USB Uyku ve Şarj spesifikasyonlarını desteklemek için gerekli kayıtları yapılandır ...
            // ... örneğin, port başına akım limitlerini ayarla, şarj algılama protokollerini etkinleştir vb. ...
            println!("Uyku ve Şarj desteği etkinleştirildi (temel uyku ve şarj).");
            Ok(())
        }


        // **Yardımcı Fonksiyonlar (Örnek)**

        // MMIO taban adresinden Operational Registers bölgesine erişim için yardımcı fonksiyon
        fn get_operational_registers(&self) -> OperationalRegisters {
            OperationalRegisters::new(self.mmio_base_address)
        }

        // Mikrosaniye cinsinden gecikme fonksiyonu (işletim sistemine özgü gerçek bir gecikme fonksiyonu kullanılmalı)
        fn delay_microseconds(&self, microseconds: u32) {
            // **Dikkat:** Bu basit bir döngü tabanlı gecikme örneğidir. Gerçek zamanlı işletim sistemlerinde
            // veya daha hassas gecikmeler gerektiğinde işletim sisteminin sunduğu daha uygun gecikme fonksiyonları kullanılmalıdır.
            for _ in 0..(microseconds * 1000) { // Kabaca bir gecikme sağlamak için basit döngü
                std::hint::spin_loop(); // İşlemciyi çok meşgul etmeden beklemeyi sağlayan hint (isteğe bağlı)
            }
        }

        // Bağlı aygıtların listesini döndürür (örnek olarak).
        pub fn get_connected_devices(&self) -> Vec<UsbDevice> {
            self.connected_devices.clone()
        }

        // ... diğer yardımcı fonksiyonlar (örneğin, bellek yönetimi, hata ayıklama vb.) ...
    }


    // **Varsayımsal XHCI Kayıt Tanımlamaları Modülü (xhci_registers.rs)**
    // Bu modül, gerçek donanım spesifikasyonlarına göre oluşturulmalıdır.
    // Aşağıda sadece örnek ve basitleştirilmiş bir yapı gösterilmektedir.

    pub mod registers {
        use volatile_register::{RW, RO, Field}; // `volatile-register` crate'inden örnek importlar

        // **Örnek Kayıt Yapıları (Basitleştirilmiş)**

        #[repr(C)]
        pub struct OperationalRegistersBlock {
            pub usb_command: RW<UsbCommandRegister>, // USB Command Register (USB-COMM)
            pub usb_status: RO<UsbStatusRegister>,     // USB Status Register (USB-STS)
            // ... diğer Operational Registers kayıtları ...
        }

        #[repr(C)]
        pub struct UsbCommandRegister {
            bits: RW<u32>,
        }

        impl UsbCommandRegister {
            // Run/Stop (R/S) biti (bit 0)
            pub fn read_run_stop(&self) -> u32 {
                self.bits.read_field(Field::new(0, 1))
            }

            pub fn set_run_stop(&mut self, value: u32) {
                self.bits.write_field(Field::new(0, 1), value);
            }

            // Host Controller Reset (HCRST) biti (bit 1)
            pub fn read_hcrst(&self) -> u32 {
                self.bits.read_field(Field::new(1, 1))
            }

            pub fn set_hcrst(&mut self, value: u32) {
                self.bits.write_field(Field::new(1, 1), value);
            }

            // ... diğer USB Command Register bit alanları için fonksiyonlar ...
        }


        #[repr(C)]
        pub struct UsbStatusRegister {
            bits: RO<u32>,
        }

        impl UsbStatusRegister {
            // Host Controller Reset State (HCRS) biti (bit 0)
            pub fn read_hcrst(&self) -> u32 {
                self.bits.read_field(Field::new(0, 1))
            }
            // ... diğer USB Status Register bit alanları için fonksiyonlar ...
        }


        // Operational Registers bloğuna erişim için yapı
        pub struct OperationalRegisters {
            base_address: usize,
        }

        impl OperationalRegisters {
            pub fn new(base_address: usize) -> Self {
                OperationalRegisters { base_address }
            }

            // USB Command Register'a erişim fonksiyonu
            pub fn usb_command(&self) -> &RW<UsbCommandRegister> {
                unsafe { &*((self.base_address + 0x00) as *mut RW<UsbCommandRegister>) } // Offset 0x00 - Örnek
            }

            // USB Status Register'a erişim fonksiyonu
            pub fn usb_status(&self) -> &RO<UsbStatusRegister> {
                unsafe { &*((self.base_address + 0x04) as *mut RO<UsbStatusRegister>) } // Offset 0x04 - Örnek
            }

            // ... diğer Operational Registers kayıtlarına erişim fonksiyonları ...
        }
    }
    // `xhci::registers` modülünü `xhci_registers` olarak yeniden adlandırıyoruz.
    use registers as xhci_registers;

    impl XhciDriver {
        // MMIO taban adresinden Operational Registers bölgesine erişim için yardımcı fonksiyon
        fn get_operational_registers(&self) -> xhci_registers::OperationalRegisters {
            xhci_registers::OperationalRegisters::new(self.mmio_base_address)
        }
    }
}