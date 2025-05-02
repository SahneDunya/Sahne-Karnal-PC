#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz
#![no_main] // Rust'ın varsayılan giriş noktasını (main) kullanmıyoruz

use core::ptr;      // Pointer işlemleri için
use core::mem;      // Bellek işlemleri için (örneğin size_of)
use core::slice;    // Slice işlemleri için
use core::fmt::Write; // Yazma trait'i için

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile; // <-- Added import

// Sahne64 konsol makrolarını kullanabilmek için (çıktı/loglama amaçlı)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
 use sahne64::{println, eprintln}; // Örnek import eğer macro publicse

// Çıktı makroları (Sahne64 console makrolarını kullanacak şekilde ayarlandı)
// Eğer 'std' feature etkinse std::println! kullanılır.
// Eğer 'std' feature etkin değilse (no_std), Sahne64 crate'inden gelen println! kullanılır.
#[cfg(feature = "std")]
macro_rules! kprintln {
    () => (std::println!());
    ($($arg:tt)*) => (std::println!($($arg)*));
}
#[cfg(not(feature = "std"))]
macro_rules! kprintln {
    () => (println!()); // Varsayım: Sahne64 println! makrosu
    ($($arg:tt)*) => (println!($($arg)*)); // Varsayım: Sahne64 println! makrosu
}

#[cfg(feature = "std")]
macro_rules! kprint {
    ($($arg:tt)*) => (std::print!($($arg)*));
}
#[cfg(not(feature = "std"))]
macro_rules! kprint {
    ($($arg:tt)*) => (print!($($arg)*)); // Varsayım: Sahne64 print! makrosu
}


// ************************************************************************
// DONANIM BAĞIMLI BÖLÜM (Aşağıdaki değerler ve yapılar donanımınıza göre değişir!)
// ************************************************************************

// USB Kontrolcü Register Adresleri (ÖRNEK DEĞERLER, DOĞRU DEĞİLLER!)
// Bu modül, kullanılan ARM çipindeki belirli USB kontrolcüsüne ait register
// adreslerini içermelidir.
mod usb_registers {
    // Örnek USB kontrolcü temel adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
    // Bu adres, bellek eşlemeli (memory-mapped) registerların başlangıcıdır.
    pub const USB_CONTROLLER_BASE: usize = 0xABC00000;

    // Örnek kontrol registerları (DONANIMINIZA GÖRE DEĞİŞİR!)
    // Volatile sarmalayıcı ile bellek adresi olarak tanımlamak daha kullanışlı olabilir.
     pub const USB_CONTROL_REG: Volatile<u32> = Volatile::new(USB_CONTROLLER_BASE as *mut u32);
     pub const USB_STATUS_REG: Volatile<u32> = Volatile::new((USB_CONTROLLER_BASE + 0x04) as *mut u32);
     pub const USB_DATA_PORT: Volatile<u8> = Volatile::new((USB_CONTROLLER_BASE + 0x10) as *mut u8);
    // ... diğer registerlar ...

    // Veya sadece offsetleri tanımlayıp base adrese ekleyebilirsiniz.
    pub const REG_CONTROL: usize = 0x00;
    pub const REG_STATUS: usize = 0x04;
    pub const REG_ENDPOINT_CONTROL: usize = 0x08;
    pub const REG_ENDPOINT_STATUS: usize = 0x0C;
    pub const REG_BULK_OUT_DATA: usize = 0x10; // Örnek Bulk-OUT data FIFO/register
    pub const REG_BULK_IN_DATA: usize = 0x20;  // Örnek Bulk-IN data FIFO/register
    pub const REG_INTERRUPT_STATUS: usize = 0x30; // Örnek Interrupt Status Register
    pub const REG_INTERRUPT_ENABLE: usize = 0x34; // Örnek Interrupt Enable Register
    pub const REG_INTERRUPT_CLEAR: usize = 0x38;  // Örnek Interrupt Clear Register

    // Register değerlerindeki bitler ve maskeler (DONANIMINIZA GÖRE DEĞİŞİR!)
    pub mod bits {
         pub const STATUS_DEVICE_CONNECTED: u32 = 1 << 0; // Örnek: Aygıt bağlı biti
         pub const CONTROL_ENABLE_CONTROLLER: u32 = 1 << 0; // Örnek: Kontrolcüyü etkinleştirme biti
         // ... diğer bitler ...
    }
}
use usb_registers::bits; // Bitlere kolay erişim


// USB Endpoint Tanımları (ÖRNEK DEĞERLER, DONANIMINIZA VE CİHAZA GÖRE DEĞİŞİR!)
// Bu numaralar, USB protokolünde kullanılan endpoint adresleridir.
mod usb_endpoints {
    pub const CONTROL_ENDPOINT: u8 = 0;   // Kontrol endpoint (Genellikle çift yönlü, adres 0)
    pub const BULK_OUT_ENDPOINT: u8 = 1;  // Veri göndermek için bulk endpoint (Host -> Device yönü)
    pub const BULK_IN_ENDPOINT: u8 = 2;   // Veri almak için bulk endpoint (Device -> Host yönü)
    // Endpoint numaraları (1-15) cihazın descriptorlarında belirtilir ve
    // kontrolcünüzdeki belirli FIFOlara veya register setlerine maplenir.
}

// ************************************************************************
// USB PROTOKOLÜ İLE İLGİLİ SABİTLER VE YAPILAR (Temel MSC Bulk-Only Transport)
// ************************************************************************

mod usb_protocol_constants {
    // USB Standart İstek Tipleri (bmRequestType baytının bit alanları)
     b7: Data Transfer Direction (0=Host to Device, 1=Device to Host)
     b6..5: Type (0=Standard, 1=Class, 2=Vendor, 3=Reserved)
     b4..0: Recipient (0=Device, 1=Interface, 2=Endpoint, 3=Other)
    pub const REQ_TYPE_STANDARD_DEVICE_IN: u8 = 0x80; // 1000 0000
    pub const REQ_TYPE_STANDARD_DEVICE_OUT: u8 = 0x00; // 0000 0000
    pub const REQ_TYPE_STANDARD_INTERFACE_IN: u8 = 0x81; // 1000 0001
    pub const REQ_TYPE_STANDARD_INTERFACE_OUT: u8 = 0x01; // 0000 0001
    pub const REQ_TYPE_STANDARD_ENDPOINT_IN: u8 = 0x82; // 1000 0010
    pub const REQ_TYPE_STANDARD_ENDPOINT_OUT: u8 = 0x02; // 0000 0010
    pub const REQ_TYPE_CLASS_INTERFACE_IN: u8 = 0xA1;  // 1010 0001 (MSC Get_Max_LUN)
    pub const REQ_TYPE_CLASS_INTERFACE_OUT: u8 = 0x21; // 0010 0001 (MSC Reset)
    // ... diğer tipler ...

    // USB Standart İstek Kodları (bRequest baytı)
    pub const USB_REQ_GET_STATUS: u8 = 0x00;
    pub const USB_REQ_CLEAR_FEATURE: u8 = 0x01;
    pub const USB_REQ_SET_FEATURE: u8 = 0x03;
    pub const USB_REQ_SET_ADDRESS: u8 = 0x05;
    pub const USB_REQ_GET_DESCRIPTOR: u8 = 0x06;
    pub const USB_REQ_SET_DESCRIPTOR: u8 = 0x07;
    pub const USB_REQ_GET_CONFIGURATION: u8 = 0x08;
    pub const USB_REQ_SET_CONFIGURATION: u8 = 0x09;
    pub const USB_REQ_GET_INTERFACE: u8 = 0x0A;
    pub const USB_REQ_SET_INTERFACE: u8 = 0x0B;
    pub const USB_REQ_SYNCH_FRAME: u8 = 0x0C;
    // ... diğer standart istekler ...

    // USB Descriptor Tipleri (wValue'nun düşük baytı için)
    pub const DESC_TYPE_DEVICE: u8 = 0x01;
    pub const DESC_TYPE_CONFIGURATION: u8 = 0x02;
    pub const DESC_TYPE_STRING: u8 = 0x03;
    pub const DESC_TYPE_INTERFACE: u8 = 0x04;
    pub const DESC_TYPE_ENDPOINT: u8 = 0x05;
    pub const DESC_TYPE_DEVICE_QUALIFIER: u8 = 0x06;
    // ... diğer descriptor tipleri ...

    // MSC Sınıfına Özel İstekler (Bulk-Only Transport spesifikasyonundan)
    pub const MSC_REQ_RESET: u8 = 0xFF;
    pub const MSC_REQ_GET_MAX_LUN: u8 = 0xFE;

    // MSC Komut Kodları (SCSI komutları, CBW'nin command_block[0] baytı için)
    // Çok temel MSC için gerekli bazı komutlar
    pub const MSC_CMD_TEST_UNIT_READY: u8 = 0x00; // Cihazın hazır olup olmadığını kontrol et
    pub const MSC_CMD_REQUEST_SENSE: u8 = 0x03; // Hata detaylarını al
    pub const MSC_CMD_INQUIRY: u8 = 0x12; // Aygıt temel bilgilerini al (Vendor ID, Product ID, sürüm vb.)
    pub const MSC_CMD_READ_CAPACITY_10: u8 = 0x25; // Aygıtın toplam sektör sayısını ve sektör boyutunu al
    pub const MSC_CMD_READ_10: u8 = 0x28; // Belirtilen LBA'dan veri oku
    pub const MSC_CMD_WRITE_10: u8 = 0x2A; // Belirtilen LBA'ya veri yaz
    // ... diğer SCSI komutları (Mode Sense, Start Stop Unit vb.) ...

    // MSC Durum Kodları (CSW'nin status baytı için)
    pub const MSC_STATUS_COMMAND_PASSED: u8 = 0x00; // Komut başarılı
    pub const MSC_STATUS_COMMAND_FAILED: u8 = 0x01; // Komut başarısız
    pub const MSC_STATUS_PHASE_ERROR: u8 = 0x02;   // Faz hatası (beklenenden farklı veri transferi)

    // CBW/CSW İşaretleri (CBW'nin flags baytı için)
    pub const CBW_FLAG_DIRECTION_OUT: u8 = 0x00; // Hosttan cihaza veri gönderme (OUT)
    pub const CBW_FLAG_DIRECTION_IN: u8 = 0x80;  // Cihazdan hosta veri alma (IN)

    // CBW İmzası ("USBCBW")
    pub const CBW_SIGNATURE: u32 = 0x43425355; // Little-endian byte sırası (55 53 42 43)

    // CSW İmzası ("USBCSW")
    pub const CSW_SIGNATURE: u32 = 0x53425355; // Little-endian byte sırası (55 53 42 53)

    // CBW ve CSW boyutları (Bulk-Only Transport spesifikasyonundan)
    pub const CBW_SIZE: usize = 31;
    pub const CSW_SIZE: usize = 13;
}

use usb_protocol_constants::*; // Protokol sabitlerine kolay erişim


// Command Block Wrapper (CBW) yapısı
// #[repr(C, packed)]: C ABI uyumlu ve paketlenmiş bellek yerleşimi
#[repr(C, packed)]
struct CommandBlockWrapper {
    dcbw_signature: u32,        // CBW İşareti (0x43425355 - "USBCBW")
    dcbw_tag: u32,              // Etiket (Host tarafından oluşturulan, CSW ile eşleşir)
    dcbw_data_transfer_length: u32, // Veri transfer uzunluğu (bayt cinsinden)
    bmcbw_flags: u8,           // İşaretler (yön bilgisi - IN/OUT)
    bcbw_lun: u8,               // Logical Unit Number (LUN) - Genellikle 0
    b_cbw_length: u8, // Komut bloğu uzunluğu (1 ila 16 bayt)
    cbw_cb: [u8; 16],  // Komut bloğu (SCSI komutu ve parametreleri)
}

impl CommandBlockWrapper {
    pub fn new() -> Self {
        CommandBlockWrapper {
            dcbw_signature: CBW_SIGNATURE, // "USBCBW"
            dcbw_tag: 0x12345678,       // Örnek etiket (her CBW için farklı olabilir)
            dcbw_data_transfer_length: 0,
            bmcbw_flags: 0,
            bcbw_lun: 0,
            b_cbw_length: 0,
            cbw_cb: [0u8; 16],
        }
    }
}


// Command Status Wrapper (CSW) yapısı
// #[repr(C, packed)]
#[repr(C, packed)]
struct CommandStatusWrapper {
    dcsv_signature: u32,        // CSW İşareti (0x53425355 - "USBCSW")
    dcsv_tag: u32,              // Etiket (CBW'deki etiketle aynı olmalı)
    dcsv_data_residue: u32,     // Kalan veri uzunluğu (beklenen - aktarılan)
    bcs_status: u8,            // Komut durumu (MSC_STATUS_COMMAND_PASSED, vb.)
}

impl CommandStatusWrapper {
    pub fn new() -> Self {
        CommandStatusWrapper {
            dcsv_signature: CSW_SIGNATURE, // "USBCSW"
            dcsv_tag: 0, // CSW alındığında CBW tag'i ile doldurulacak
            dcsv_data_residue: 0,
            bcs_status: 0,
        }
    }
}


// ************************************************************************
// ÇEKİRDEK SEVİYESİ USB SÜRÜCÜ KODU
// ************************************************************************

// Okuma/yazma işlemleri için kullanılacak temel arabellek (kernel alanında)
// static mut kullanımı unsafe gerektirir ve dikkatli yönetilmelidir.
static mut IO_BUFFER: [u8; 512] = [0u8; 512]; // Örnek 512 baytlık arabellek (sektör boyutu)

// Registerlara güvenli erişim için yardımcı fonksiyonlar (Volatile okuma/yazma)
// Bu fonksiyonlar, doğrudan bellek adreslerine erişirken derleyici optimizasyonlarını engeller.
#[inline(always)] // Her zaman inline yapmayı dene
unsafe fn read_register_u32(address: usize) -> u32 { // usize address input
    (address as *const u32).read_volatile() // Use const pointer for reading
}

#[inline(always)] // Her zaman inline yapmayı dene
unsafe fn write_register_u32(address: usize, value: u32) { // usize address input
    (address as *mut u32).write_volatile(value); // Use mut pointer for writing
}
// Diğer boyutlar (u8, u16, u64) için benzer fonksiyonlar eklenebilir.


// USB kontrolcüsünü başlatma fonksiyonu (DONANIMA ÖZEL!)
/// USB kontrolcüsünü donanımsal olarak etkinleştirir ve temel ayarlamaları yapar.
/// Bu fonksiyonun içeriği tamamen kullanılan USB donanımına bağımlıdır.
fn usb_controller_init() {
    kprintln!("USB kontrolcüsü başlatılıyor...");
    // TODO: DONANIMINIZA ÖZEL USB KONTROLCÜ BAŞLATMA KODUNU BURAYA YAZIN!
    // Örnek: USB kontrolcüyü etkinleştirme, saat frekanslarını ayarlama, PHY başlatma, vb.
    unsafe {
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
        // Örneğin, bir kontrol registerına etkinleştirme biti yazma.
         let control_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_CONTROL);
         write_register_u32(control_reg_addr, bits::CONTROL_ENABLE_CONTROLLER); // USB kontrolcüyü etkinleştir
        // ... diğer başlatma adımları ...

        // Not: Endpointlerin yapılandırılması genellikle aygıt bağlandıktan sonra yapılır.
    }
    kprintln!("USB kontrolcüsü başlatma tamamlandı (Örnek).");
}


// USB aygıtını algılama fonksiyonu (BASİT ALGILAMA, GELİŞTİRİLEBİLİR)
/// USB portunda bir aygıtın bağlı olup olmadığını kontrol eder.
/// Bu fonksiyonun içeriği tamamen kullanılan USB donanımına bağımlıdır.
fn usb_device_detect() -> bool {
    kprintln!("USB aygıtı algılanıyor...");
    // TODO: USB aygıtı algılama mantığını buraya ekleyin.
    // Örnek: USB port durum registerlarını kontrol etme, aygıt bağlantı durumu bitini okuma, vb.
    unsafe {
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
         let status_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_STATUS);
        let status = read_register_u32(status_reg_addr);
        if (status & bits::STATUS_DEVICE_CONNECTED) != 0 { // Örnek: Aygıt bağlı bitini kontrol et
            kprintln!("USB aygıtı algılandı.");
            return true;
        } else {
            kprintln!("USB aygıtı algılanamadı.");
            return false;
        }
    }
}

// USB aygıtını yapılandırma (temel yapılandırma adımları)
/// Yeni bağlanan USB aygıtını adresleme, descriptor alma ve yapılandırma ayarlama adımlarını uygular.
fn usb_configure_device() -> bool {
    kprintln!("USB aygıtı yapılandırılıyor...");

    // Not: Endpoint 0 (Kontrol Endpoint) genellikle aygıt sıfırlandıktan hemen sonra çalışmaya hazırdır.
    // Maksimum paket boyutu (EP0 Max Packet Size) genellikle 64 bayt varsayılır (ancak descriptor'dan teyit edilmeli).

    // Descriptor verisi için arabellek
    let mut descriptor_buffer: [u8; 256] = [0u8; 256];


    // 1. Device Descriptor'ı al
    // SETUP: 0x80 0x06 (GET_DESCRIPTOR) wValue=0x0100 (Device Desc Type=1, Index=0) wIndex=0 wLength=18
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_IN, USB_REQ_GET_DESCRIPTOR, (DESC_TYPE_DEVICE as u16) << 8, 0, 18, descriptor_buffer.as_mut_ptr()) {
        kprintln!("Device Descriptor alınamadı!");
        return false;
    }
    kprintln!("Device Descriptor alındı ({} bayt).", descriptor_buffer[0]); // İlk bayt uzunluk olmalı
    // TODO: Device Descriptor'ı ayrıştırıp idVendor, idProduct, bMaxPacketSize0 gibi bilgileri kullanabilirsiniz.
    // bMaxPacketSize0, Endpoint 0'ın maksimum paket boyutunu belirler ve kontrolcünüzün EP0'ı buna göre yapılandırılmalıdır.


    // 2. Adresi ayarla (genellikle 1-127 arası bir adres atanır)
    // SETUP: 0x00 0x05 (SET_ADDRESS) wValue=Yeni Adres wIndex=0 wLength=0
    // Aygıt, SETUP aşaması tamamlandıktan sonra yeni adresi kullanmaya başlar.
    let new_address: u8 = 7; // Örnek adres
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_OUT, USB_REQ_SET_ADDRESS, new_address as u16, 0, 0, core::ptr::null_mut()) {
        kprintln!("Adres ayarlanamadı!");
        return false;
    }
    kprintln!("Adres ayarlandı: {}", new_address);
    // TODO: USB kontrolcü registerlarına aygıt adresini yazma (DONANIMA ÖZEL!)
    // Bu adım, kontrolcünün artık bu aygıtla belirtilen adresi kullanarak iletişim kurmasını sağlar.
    // Örneğin, kontrolcünüzün adres registerına `new_address` değerini yazmanız gerekebilir.

    // Kısa bir gecikme gerekebilir, aygıtın adres değiştirmesi için
    // TODO: Gecikme ekle (Örn: Yaklaşık 1-10 ms)
     unsafe { core::arch::asm!("nop"); } // Çok basit bir simülasyon gecikmesi


    // 3. Configuration Descriptor'ı al (ve tüm ilgili interface ve endpoint descriptor'larını)
    // Önce başlığı alıp toplam uzunluğu belirle, sonra tamamını al.
    // SETUP: 0x80 0x06 (GET_DESCRIPTOR) wValue=0x0200 (Config Desc Type=2, Index=0) wIndex=0 wLength=9 (Başlık uzunluğu)
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_IN, USB_REQ_GET_DESCRIPTOR, (DESC_TYPE_CONFIGURATION as u16) << 8, 0, 9, descriptor_buffer.as_mut_ptr()) { // İlk 9 baytı al (Configuration Descriptor başlığı)
        kprintln!("Configuration Descriptor başlığı alınamadı!");
        return false;
    }
    // Configuration Descriptor'ın toplam uzunluğunu descriptor'dan oku (Little-endian)
    let config_descriptor_length = unsafe {
        let len_ptr = descriptor_buffer.as_ptr().add(2) as *const u16;
        ptr::read_unaligned(len_ptr) // Descriptor packed olduğu için unaligned read gerekebilir
    };
    kprintln!("Config Descriptor başlığı alındı. Toplam uzunluk: {}", config_descriptor_length);

    if config_descriptor_length < 9 || config_descriptor_length > 256 { // Boyut kontrolü
         kprintln!("Geçersiz Configuration Descriptor uzunluğu: {}", config_descriptor_length);
         return false;
    }

    // SETUP: 0x80 0x06 (GET_DESCRIPTOR) wValue=0x0200 wIndex=0 wLength=Toplam Uzunluk (Tamamı için)
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_IN, USB_REQ_GET_DESCRIPTOR, (DESC_TYPE_CONFIGURATION as u16) << 8, 0, config_descriptor_length, descriptor_buffer.as_mut_ptr()) { // Tam Configuration Descriptor'ı al
        kprintln!("Configuration Descriptor'ın tamamı alınamadı!");
        return false;
    }
    kprintln!("Configuration Descriptor alındı (Tamamı, {} bayt).", config_descriptor_length);
    // TODO: Configuration Descriptor'ı ve alt descriptor'ları (Interface, Endpoint) ayrıştırın.
    // Especially: MSC Interface (bInterfaceClass=0x08, bInterfaceSubClass=0x06, bInterfaceProtocol=0x50 for BOT)
    // BULK IN endpoint adresi (bEndpointAddress & 0x80), BULK OUT endpoint adresi (bEndpointAddress & 0x0F)
    // Endpointlerin maksimum paket boyutları.
    // Bu bilgiler, kontrolcünüzün Bulk IN/OUT endpointlerini doğru numaralar ve max paket boyutları ile yapılandırmak için kullanılır.


    // 4. Yapılandırmayı ayarla (Genellikle 1. yapılandırma kullanılır, descriptor'dan alınmalı)
    // SETUP: 0x00 0x09 (SET_CONFIGURATION) wValue=Yapılandırma Değeri (genellikle 1) wIndex=0 wLength=0
    let configuration_value: u8 = 1; // Örnek değer
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_OUT, USB_REQ_SET_CONFIGURATION, configuration_value as u16, 0, 0, core::ptr::null_mut()) {
        kprintln!("Yapılandırma ayarlanamadı!");
        return false;
    }
    kprintln!("Yapılandırma ayarlandı: {}", configuration_value);
    // TODO: Yapılandırma ayarlandıktan sonra yapılması gereken donanımsal ayarlar (DONANIMA ÖZEL!)
    // Endpointlerin donanımınızda etkinleştirilmesi ve doğru FIFO'lara/register setlerine bağlanması.
     write_register(EP1_CONTROL_REG, ENABLE_BULK_IN | MAX_PACKET_SIZE_64);

    kprintln!("USB aygıtı yapılandırması tamamlandı.");
    true // Başarılı
}


// Kontrol endpoint üzerinden USB kontrol isteği gönderme (Örn: Descriptor alma, adres ayarlama)
// Bu fonksiyon, SETUP paketini oluşturup kontrol endpoint'e göndermeli, veri transferini yönetmeli ve STATUS aşamasını tamamlamalıdır.
// Çoğunlukla polleme tabanlı bir yaklaşımla donanım registerlarını kullanarak yapılır.
// GERÇEKTE: Çok daha karmaşık bir state machine ve interrupt/DMA kullanımı gerektirir.
fn usb_control_request(request_type: u8, request: u8, value: u16, index: u16, length: u16, data_buffer: *mut u8) -> bool {
    kprintln!("USB kontrol isteği: bmRequestType={:02x}, bRequest={:02x}, wValue={:04x}, wIndex={:04x}, wLength={}",
        request_type, request, value, index, length);

    // TODO: Kontrol endpoint üzerinden USB kontrol isteği gönderme mantığını buraya ekleyin.
    // **BASİTLEŞTİRİLMİŞ & POLLEME TABANLI ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**

    // 1. SETUP Paketini Oluştur ve Gönder (8 bayt)
    // SETUP paketi yapısı: bmRequestType (1), bRequest (1), wValue (2), wIndex (2), wLength (2)
    let mut setup_packet: [u8; 8] = [0u8; 8];
    setup_packet[0] = request_type;
    setup_packet[1] = request;
    // wValue (Little-endian)
    setup_packet[2] = (value & 0xFF) as u8;
    setup_packet[3] = (value >> 8) as u8;
    // wIndex (Little-endian)
    setup_packet[4] = (index & 0xFF) as u8;
    setup_packet[5] = (index >> 8) as u8;
    // wLength (Little-endian)
    setup_packet[6] = (length & 0xFF) as u8;
    setup_packet[7] = (length >> 8) as u8;

    unsafe {
        // TODO: SETUP paketini donanım kontrolcünüzün kontrol endpoint'e ait register/FIFO'suna yazın. (DONANIMA ÖZEL!)
        // Örnek: Kontrol endpoint OUT FIFO'suna yazma register adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
        let ep0_out_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_OUT_DATA); // Placeholder, EP0 data reg/FIFO address

        // SETUP paketini yaz (polleme tabanlı örnek)
        for i in 0..8 {
            // Kontrolcünüzün yazma arayüzüne bağlı olarak byte byte veya kelime kelime yazılabilir.
             write_register_u32(ep0_out_data_reg_addr, setup_packet[i] as u32); // Örnek: Her baytı 32-bit register'a yaz (ÇOK BASİT)
        }
        // TODO: Kontrolcüye SETUP paketinin gönderildiğini bildirin (DONANIMA ÖZEL!). Örneğin, bir komut registerı yazarak.

        // TODO: SETUP transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
        // Örneğin, bir durum registerındaki bitin set olmasını bekleyin.
         while (read_register_u32(status_reg) & SETUP_TRANSFER_DONE_BIT) == 0 { core::hint::spin_loop(); }
    }


    // 2. DATA Aşaması (wLength > 0 ise)
    if length > 0 {
        // Yöne göre DATA transferi (IN veya OUT)
        if (request_type & CBW_FLAG_DIRECTION_IN) != 0 { // Device to Host (IN)
             // TODO: Kontrol endpoint IN FIFO'sundan 'length' bayt veriyi 'data_buffer'a okuyun. (DONANIMA ÖZEL!)
             // Örnek: Kontrol endpoint IN FIFO'sundan okuma register adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
             let ep0_in_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_IN_DATA); // Placeholder, EP0 data reg/FIFO address

             unsafe {
                 for i in 0..length as usize {
                     // Kontrolcünüzün okuma arayüzüne bağlı olarak byte byte veya kelime kelime okunabilir.
                      let received_byte = read_register_u32(ep0_in_data_reg_addr) as u8; // Örnek: 32-bit registerdan 1 bayt oku
                      *data_buffer.add(i) = received_byte;
                 }
             }
             // TODO: DATA IN transferinin tamamlanmasını bekleyin (Polleme veya interrupt).

        } else { // Host to Device (OUT)
             // TODO: 'data_buffer'dan 'length' bayt veriyi kontrol endpoint OUT FIFO'suna yazın. (DONANIMA ÖZEL!)
             // Örnek: Kontrol endpoint OUT FIFO'suna yazma register adresi aynı EP0 OUT data registerı olabilir.
             let ep0_out_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_OUT_DATA); // Placeholder

             unsafe {
                  let data_slice = slice::from_raw_parts(data_buffer, length as usize);
                 for i in 0..length as usize {
                     write_register_u32(ep0_out_data_reg_addr, data_slice[i] as u32); // Örnek: Her baytı 32-bit register'a yaz
                 }
             }
             // TODO: DATA OUT transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
        }
         // TODO: DATA aşamasında ACK/NAK/STALL durumlarını yönetin.
    }


    // 3. STATUS Aşaması (Karşı yöne 0 uzunluklu paket gönderilir/alınır)
    // DATA aşaması IN ise STATUS aşaması OUT, DATA aşaması OUT ise STATUS aşaması IN'dir.
    if (request_type & CBW_FLAG_DIRECTION_IN) != 0 { // DATA IN ise STATUS OUT
         // TODO: Kontrol endpoint OUT'a 0 uzunluklu STATUS paketini gönderin. (DONANIMA ÖZEL!)
         // Genellikle sadece bir "transmit complete" veya benzeri bir durum bildirilir.
    } else { // DATA OUT ise STATUS IN
         // TODO: Kontrol endpoint IN'den 0 uzunluklu STATUS paketini alınmasını bekleyin. (DONANIM ÖZEL!)
         // Genellikle sadece bir "receive complete" veya benzeri bir durum beklenir.
    }
    // TODO: STATUS aşamasında ACK/NAK/STALL durumlarını yönetin.


    // İsteğin genel başarısını belirle (örnekte her zaman true dönüyor)
    // Gerçek uygulamada donanım durum registerlarına bakarak başarısızlıkları kontrol etmelisiniz.
     kprintln!("USB kontrol isteği tamamlandı (Simüle).");
    true // Örnek başarı durumu
}

// MSC Reset isteği gönderme
/// MSC Bulk-Only Transport Reset isteğini gönderir.
fn msc_reset() -> bool {
    kprintln!("MSC Reset isteği gönderiliyor...");
    // MSC Reset isteği kontrol endpoint üzerinden sınıf-özel bir istektir.
     bmRequestType = 0x21 (Class | Interface | HostToDevice), bRequest=0xFF (MSC_REQ_RESET), wValue=0, wIndex=Interface#, wLength=0
    // wIndex genellikle MSC arayüz numarasını içerir. Bu örnekte 0 varsayıyoruz.
    if usb_control_request(REQ_TYPE_CLASS_INTERFACE_OUT, MSC_REQ_RESET, 0, 0, 0, core::ptr::null_mut()) { // Interface isteği (0x21), Alıcı Arayüzü (Interface)
        kprintln!("MSC Reset isteği gönderildi.");
        // TODO: Reset sonrası gerekli donanımsal bekleme veya durum kontrolü (DONANIMA ÖZEL!)
        return true;
    } else {
        kprintln!("MSC Reset isteği BAŞARISIZ!");
        return false;
    }
}

// Maksimum LUN sayısını alma isteği
/// MSC Bulk-Only Transport Get_Max_LUN isteğini gönderir ve Max LUN sayısını alır.
fn msc_get_max_lun() -> u8 {
    kprintln!("Maksimum LUN sayısı alınıyor...");
    // MSC Get_Max_LUN isteği kontrol endpoint üzerinden sınıf-özel bir istektir.
     bmRequestType = 0xA1 (Class | Interface | DeviceToHost), bRequest=0xFE (MSC_REQ_GET_MAX_LUN), wValue=0, wIndex=Interface#, wLength=1
    let mut lun_buffer: [u8; 1] = [0];
    // wIndex genellikle MSC arayüz numarasını içerir. Bu örnekte 0 varsayıyoruz.
    if usb_control_request(REQ_TYPE_CLASS_INTERFACE_IN, MSC_REQ_GET_MAX_LUN, 0, 0, 1, lun_buffer.as_mut_ptr()) { // Aygıttan veri alımı (0xA1), Alıcı Arayüzü (Interface)
        let max_lun = lun_buffer[0];
        kprintln!("Maksimum LUN sayısı alındı: {}", max_lun);
        return max_lun;
    } else {
        kprintln!("Maksimum LUN sayısı alınamadı! Varsayılan 0 kullanılıyor.");
        // Hata durumunda 0 veya başka bir hata değeri dönebiliriz.
        return 0; // Varsayılan olarak 0 LUN kabul et (örnek)
    }
}


// CBW gönderme fonksiyonu (Bulk-OUT endpoint'e)
/// Verilen CBW yapısını Bulk-OUT endpoint üzerinden aygıta gönderir.
/// Bu fonksiyon, CBW'yi donanım registerları/FIFOlara yazarak gönderir ve transferin tamamlanmasını bekler.
/// # Güvenlik
/// Unsafe'dir, donanım registerlarına doğrudan erişir.
unsafe fn msc_send_cbw(cbw: &CommandBlockWrapper) -> bool {
    kprintln!("CBW gönderiliyor. Komut: {:02x}, Data Len: {}", cbw.cbw_cb[0], cbw.dcbw_data_transfer_length);

    // **BASİTLEŞTİRİLMİŞ & POLLEME TABANLI ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**

    // TODO: Bulk-OUT endpoint'e CBW gönderme mantığını buraya ekleyin.
    // Bu fonksiyon, CBW yapısını bellekte doğru şekilde oluşturmanız,
    // Bulk-OUT endpoint'e göndermeniz (donanım registerlarını/FIFOlara yazarak),
    // veri transferini yönetmeniz ve gerekli durum kontrollerini yapmanız gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    // Varsayım: Bulk-OUT endpoint'e veri yazma registerı/FIFO adresi (DONANIMIZA GÖRE DEĞİŞİR!)
    let bulk_out_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_OUT_DATA); // Örnek adres
    let cbw_ptr = cbw as *const CommandBlockWrapper as *const u8; // CBW yapısının byte pointerı
    let cbw_size = mem::size_of::<CommandBlockWrapper>(); // CBW boyutu (31 bayt)

    if cbw_size != CBW_SIZE {
         kprintln!("Hata: CBW yapısı boyutu ({}) beklenenden ({}) farklı!", cbw_size, CBW_SIZE);
         return false; // Boyut hatası
    }

    // CBW'yi Bulk-OUT endpoint FIFO'suna yaz (polleme tabanlı örnek)
    // Kontrolcünüzün yazma arayüzüne bağlı olarak byte byte veya kelime kelime yazılabilir.
    let cbw_bytes = slice::from_raw_parts(cbw_ptr, cbw_size);
    for i in 0..cbw_size {
          write_register_u32(bulk_out_data_reg_addr, cbw_bytes[i] as u32); // Örnek: Her baytı 32-bit register'a yaz (ÇOK BASİT)
         // Daha doğru bir yaklaşım: eğer FIFO 32-bit ise 4 baytı birleştirip yazmak.
         if (bulk_out_data_reg_addr as *mut u32).write_bytes(&cbw_bytes[i], 1).is_null() { // Örnek: 1 baytı volatile yaz
             kprintln!("Hata: CBW yazma başarısız oldu!");
             return false;
         }
    }
    // TODO: CBW transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
    // Örneğin, endpoint durum registerındaki bir bitin set olmasını bekleyin.
     while (read_register_u32(ep_status_reg) & TX_DONE_BIT) == 0 { core::hint::spin_loop(); }


    // TODO: Hata yönetimi (NAK, STALL, babble gibi durumlar) ve zaman aşımları.

    kprintln!("CBW gönderildi (Simüle).");
    true // Örnek başarı durumu
}


// CSW alma fonksiyonu (Bulk-IN endpoint'ten)
/// Bulk-IN endpoint üzerinden CSW yapısını aygıttan alır.
/// Bu fonksiyon, Bulk-IN endpoint'ten veri okuyarak CSW yapısını doldurur ve transferin tamamlanmasını bekler.
/// # Güvenlik
/// Unsafe'dir, donanım registerlarına doğrudan erişir.
unsafe fn msc_receive_csw(csw: &mut CommandStatusWrapper) -> bool {
    kprintln!("CSW bekleniyor...");

    // **BASİTLEŞTİRİLMİŞ & POLLEME TABANLI ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**

    // TODO: Bulk-IN endpoint'ten CSW alma mantığını burya ekleyin.
    // Bu fonksiyon, Bulk-IN endpoint'ten veri okumanız (donanım registerlarını/FIFOlardan),
    // CSW yapısını gelen verilere göre doldurmanız ve gerekli durum kontrollerini yapmanız gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    // Varsayım: Bulk-IN endpoint'ten veri okuma registerı/FIFO adresi (DONANIMIZA GÖRE DEĞİŞİR!)
    let bulk_in_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_IN_DATA); // Örnek adres
    let csw_ptr = csw as *mut CommandStatusWrapper as *mut u8; // CSW yapısının byte pointerı
    let csw_size = mem::size_of::<CommandStatusWrapper>(); // CSW boyutu (13 bayt)

     if csw_size != CSW_SIZE {
         kprintln!("Hata: CSW yapısı boyutu ({}) beklenenden ({}) farklı!", csw_size, CSW_SIZE);
         return false; // Boyut hatası
     }

    // CSW'yi Bulk-IN endpoint FIFO'sundan oku (polleme tabanlı örnek)
    // Kontrolcünüzün okuma arayüzüne bağlı olarak byte byte veya kelime kelime okunabilir.
    let csw_bytes = slice::from_raw_parts_mut(csw_ptr, csw_size);
    for i in 0..csw_size {
        // csw_bytes[i] = read_register_u32(bulk_in_data_reg_addr) as u8; // Örnek: 32-bit registerdan 1 bayt oku (ÇOK BASİT)
         // Daha doğru bir yaklaşım: eğer FIFO 32-bit ise 4 baytı birleştirip okumak.
        csw_bytes[i] = (bulk_in_data_reg_addr as *const u32).read_volatile() as u8; // Örnek: 1 baytı volatile oku
    }
    // TODO: CSW transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
    // Örneğin, endpoint durum registerındaki bir bitin set olmasını bekleyin.
     while (read_register_u32(ep_status_reg) & RX_DONE_BIT) == 0 { core::hint::spin_loop(); }


     // CSW İmzası ve Tag kontrolü (protokole uygunluk için)
     if csw.dcsv_signature != CSW_SIGNATURE {
         kprintln!("Hata: CSW İmzası Yanlış! Beklenen: {:x}, Alınan: {:x}", CSW_SIGNATURE, csw.dcsv_signature);
         // TODO: Faz hatası (Phase Error) durumunu yönetin.
         // return false; // İmza yanlışsa başarısız
     }
     // TODO: CSW tag'inin son gönderilen CBW tag'i ile eşleştiğini kontrol edin.


    // TODO: Hata yönetimi (NAK, STALL gibi durumlar) ve zaman aşımları.

    kprintln!("CSW alındı (Simüle). Durum: {:02x}, Kalan: {}", csw.bcs_status, csw.dcsv_data_residue);
    true // Örnek başarı durumu
}


// MSC komutu gönderme ve CSW alma (Temel MSC işlem akışı)
/// Bir MSC CBW gönderir, isteğe bağlı veri transferini (OUT veya IN) yönetir
/// ve ardından CSW'yi alır.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn msc_send_command(cbw: &mut CommandBlockWrapper) -> bool { // CSW alma bu fonksiyondan ayrıldı
     let mut csw = CommandStatusWrapper::new(); // Her komut için yeni CSW
     csw.dcsv_tag = cbw.dcbw_tag; // CSW tag'i CBW tag'i ile aynı olmalı

    if !msc_send_cbw(cbw) {
        kprintln!("CBW gönderme başarısız!");
        // TODO: Hata durumunda kurtarma (örneğin endpoint'leri resetleme).
        return false;
    }

    // TODO: VERİ AŞAMASI (Eğer transfer_length > 0 ise)
    // Yön bilgisine (cbw.bmcbw_flags) ve transfer_length'e göre Bulk IN veya Bulk OUT transferini yönetin.
    // Bu kısım Bulk transfer fonksiyonlarını çağırmalıdır. (Aşağıda örnekleri yok)
    // Örnek:
    
    if cbw.dcbw_data_transfer_length > 0 {
        if (cbw.bmcbw_flags & CBW_FLAG_DIRECTION_IN) != 0 { // Cihazdan hosta (IN)
             msc_bulk_in_transfer(IO_BUFFER.as_mut_ptr(), cbw.dcbw_data_transfer_length); // Okuma fonksiyonu
        } else { // Hosttan cihaza (OUT)
             msc_bulk_out_transfer(IO_BUFFER.as_ptr(), cbw.dcbw_data_transfer_length); // Yazma fonksiyonu
        }
        // TODO: Veri transferi sırasındaki hataları (STALL, NAK) ve faz hatalarını yönetin.
    }
    */


    // 3. CSW Aşaması
    if !msc_receive_csw(&mut csw) {
        kprintln!("CSW alma başarısız!");
        // TODO: Hata durumunda kurtarma (örneğin endpoint'leri resetleme).
        return false;
    }

    // CSW Durum kontrolü
    if csw.bcs_status == MSC_STATUS_COMMAND_PASSED {
        kprintln!("MSC Komutu başarılı. Durum: PASSED");
        return true;
    } else if csw.bcs_status == MSC_STATUS_COMMAND_FAILED {
        kprintln!("MSC Komutu durumu: FAILED. Sense isteği gönderilmeli.");
        // TODO: Sense isteği göndererek detaylı hata bilgisini alabilirsiniz (örnek dışı bırakıldı).
        // Hata durumunda genellikle false dönülür.
        return false;
    } else if csw.bcs_status == MSC_STATUS_PHASE_ERROR {
         kprintln!("MSC Komutu durumu: PHASE ERROR. Kurtarma işlemi yapılmalı.");
         // Faz hatası genellikle daha ciddi bir sorundur. Kurtarma (Reset, Endpoint reset) gerekebilir.
         return false;
    } else {
        kprintln!("MSC Komutu BAŞARISIZ. Beklenmeyen Durum: {:02x}", csw.bcs_status);
        return false;
    }
}


// MSC Test Unit Ready komutu (Aygıt hazır mı kontrolü)
/// Aygıtın MSC komutlarını kabul etmeye hazır olup olmadığını kontrol eder.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn msc_test_unit_ready() -> bool {
    kprintln!("Test Unit Ready komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new();
    cbw.dcbw_tag = 0x1234; // Örnek tag
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri yok, durum bilgisini alacağız (IN durumu)
    cbw.b_cbw_length = 6; // SCSI komut uzunluğu (Test Unit Ready 6 bayttır)
    cbw.cbw_cb[0] = MSC_CMD_TEST_UNIT_READY; // Test Unit Ready komutu

    msc_send_command(&mut cbw) // Sadece CBW gönder, CSW send_command içinde alınacak
}

// MSC Inquiry komutu (Aygıt bilgilerini alma)
/// Aygıtın temel MSC/SCSI bilgilerini (Vendor ID, Product ID, Versiyon vb.) alır.
/// Inquiry verisi IO_BUFFER'e okunur (simüle).
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn msc_inquiry() -> bool {
    kprintln!("Inquiry komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new();
    cbw.dcbw_tag = 0x5678; // Örnek tag
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri alacağız (IN)
    let inquiry_data_len: u32 = 36; // Inquiry verisi için beklenen uzunluk (standart)
    cbw.dcbw_data_transfer_length = inquiry_data_len; // Okunacak toplam veri uzunluğu
    cbw.b_cbw_length = 6; // SCSI komut uzunluğu
    cbw.cbw_cb[0] = MSC_CMD_INQUIRY; // Inquiry komutu
    cbw.cbw_cb[1] = 0; // LUN (Bits 7-5), EVPD (Bit 1) - Usually 0
    cbw.cbw_cb[2] = 0; // Page Code
    cbw.cbw_cb[3] = 0; // Reserved
    // Allocation Length (beklenen veri uzunluğu, Big-endian)
    cbw.cbw_cb[4] = ((inquiry_data_len >> 8) & 0xFF) as u8;
    cbw.cbw_cb[5] = (inquiry_data_len & 0xFF) as u8;
    // ... Geri kalan 10 bayt 0 ...

    // TODO: Inquiry verisini Bulk-IN endpoint'ten okuma (msc_send_command sonrası yapılacak)
    // Okunan veri IO_BUFFER'e yazılmalı. (Örnekte bu adım eksik, sadece CBW/CSW gönderiliyor)

    if msc_send_command(&mut cbw) { // Sadece CBW gönder, CSW send_command içinde alınacak
         kprintln!("Inquiry CBW/CSW başarılı.");
         // TODO: Bulk-IN endpoint'ten beklenen veri uzunluğu kadar veriyi oku ve IO_BUFFER'e yaz.
          msc_bulk_in_transfer(IO_BUFFER.as_mut_ptr(), inquiry_data_len); // Okuma fonksiyonu

         kprintln!("Inquiry verisi (ilk 8 bayt - ÖRNEK):"); // Sadece ilk 16 baytı örnek olarak gösteriyoruz
         // Okunan veriyi (IO_BUFFER'den) ayrıştırıp kullanabilirsiniz.
         let inquiry_result_bytes = slice::from_raw_parts(IO_BUFFER.as_ptr(), inquiry_data_len as usize); // Assume data was read into IO_BUFFER
         for i in 0..8.min(inquiry_data_len as usize) {
             kprint!("{:02x} ", inquiry_result_bytes[i]);
         }
         kprintln!("");
         return true; // Komut ve CSW başarılıysa true dön (veri okuma başarısı da kontrol edilmeli)
    } else {
        kprintln!("Inquiry komutu başarısız!");
        return false;
    }
}


// MSC Read Capacity (10) komutu (Kapasite bilgisini alma)
/// Aygıtın toplam sektör sayısını ve sektör boyutunu alır.
/// Kapasite verisi IO_BUFFER'e okunur (simüle).
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn msc_read_capacity_10() -> bool {
    kprintln!("Read Capacity (10) komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new();
    cbw.dcbw_tag = 0x9ABC; // Örnek tag
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri alacağız (IN)
    let capacity_data_len: u32 = 8;  // Read Capacity (10) verisi 8 bayt uzunluğunda
    cbw.dcbw_data_transfer_length = capacity_data_len; // Okunacak toplam veri uzunluğu
    cbw.b_cbw_length = 10; // SCSI komut uzunluğu
    cbw.cbw_cb[0] = MSC_CMD_READ_CAPACITY_10; // Read Capacity (10) komutu
    // ... Geri kalan 9 bayt 0 (Reserved/LUN) ...

    // TODO: Read Capacity verisini Bulk-IN endpoint'ten okuma (msc_send_command sonrası yapılacak)
    // Okunan veri IO_BUFFER'e yazılmalı. (Örnekte bu adım eksik, sadece CBW/CSW gönderiliyor)

    if msc_send_command(&mut cbw) { // Sadece CBW gönder, CSW send_command içinde alınacak
        kprintln!("Read Capacity (10) CBW/CSW başarılı.");
         // TODO: Bulk-IN endpoint'ten beklenen veri uzunluğu kadar veriyi oku ve IO_BUFFER'e yaz.
          msc_bulk_in_transfer(IO_BUFFER.as_mut_ptr(), capacity_data_len); // Okuma fonksiyonu

        kprintln!("Read Capacity (10) verisi (8 bayt - ÖRNEK):"); // Sadece 8 bayt
         // Okunan veriyi (IO_BUFFER'den) ayrıştırıp kullanabilirsiniz.
         // İlk 4 bayt: Son LBA (Big-endian), sonraki 4 bayt: Sektör Boyutu (Big-endian)
         let capacity_result_bytes = slice::from_raw_parts(IO_BUFFER.as_ptr(), capacity_data_len as usize); // Assume data was read into IO_BUFFER
         for i in 0..capacity_data_len as usize {
             kprint!("{:02x} ", capacity_result_bytes[i]);
         }
         kprintln!("");
         // Örnek ayrıştırma (Big-endian):
          let last_lba = u32::from_be_bytes([capacity_result_bytes[0], capacity_result_bytes[1], capacity_result_bytes[2], capacity_result_bytes[3]]);
          let sector_size = u33::from_be_bytes([capacity_result_bytes[4], capacity_result_bytes[5], capacity_result_bytes[6], capacity_result_bytes[7]]);
          kprintln!("Son LBA: {}, Sektör Boyutu: {}", last_lba, sector_size);

        return true; // Komut ve CSW başarılıysa true dön (veri okuma başarısı da kontrol edilmeli)
    } else {
        kprintln!("Read Capacity (10) komutu başarısız!");
        return false;
    }
}

// MSC Read (10) komutu (Sektör okuma)
/// Belirtilen LBA'dan belirtilen sayıda bloğu okur.
/// Okunan veri IO_BUFFER'e veya belirtilen bir arabelleğe yazılmalıdır.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir. Veri arabelleği geçerli olmalıdır.
unsafe fn msc_read_10(lba: u32, block_size: u32, block_count: u16, data_buffer: *mut u8) -> bool { // block_size u32 yapıldı
    kprintln!("Read (10) komutu gönderiliyor. LBA: {}, Blok Sayısı: {}, Blok Boyutu: {}", lba, block_count, block_size);
    let transfer_length = block_size * block_count as u32; // Toplam transfer uzunluğu (u32)

    // IO_BUFFER yeterli mi kontrol et (basit kontrol)
     if transfer_length > IO_BUFFER.len() as u32 {
         kprintln!("Hata: Read transfer uzunluğu ({}) IO_BUFFER boyutundan ({}) büyük!", transfer_length, IO_BUFFER.len());
         // Gerçekte dinamik bellek tahsisi veya başka bir arabellek yönetimi gerekir.
         return false;
     }

    let mut cbw = CommandBlockWrapper::new();
    cbw.dcbw_tag = 0xDBAF; // Örnek tag
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri alacağız (IN)
    cbw.dcbw_data_transfer_length = transfer_length; // Okunacak toplam veri uzunluğu
    cbw.b_cbw_length = 10; // SCSI komut uzunluğu (Read 10)
    cbw.cbw_cb[0] = MSC_CMD_READ_10; // Read (10) komutu
    cbw.cbw_cb[1] = 0; // Flags/LUN - Usually 0
    // LBA (Logical Block Address) - 4 bayt, Big-endian
    cbw.cbw_cb[2] = ((lba >> 24) & 0xFF) as u8;
    cbw.cbw_cb[3] = ((lba >> 16) & 0xFF) as u8;
    cbw.cbw_cb[4] = ((lba >> 8) & 0xFF) as u8;
    cbw.cbw_cb[5] = (lba & 0xFF) as u8;
    cbw.cbw_cb[6] = 0; // Reserved
    // Transfer Length (Block Count) - 2 bayt, Big-endian
    cbw.cbw_cb[7] = ((block_count >> 8) & 0xFF) as u8;
    cbw.cbw_cb[8] = (block_count & 0xFF) as u8;
    // ... Geri kalan 7 bayt 0 ...


    // TODO: Read verisini Bulk-IN endpoint'ten okuma (msc_send_command sonrası yapılacak)
    // Okunan veri `data_buffer`'a yazılmalı. (Örnekte bu adım eksik, sadece CBW/CSW gönderiliyor)

    if msc_send_command(&mut cbw) { // Sadece CBW gönder, CSW send_command içinde alınacak
         kprintln!("Read (10) CBW/CSW başarılı. Veri alımı bekleniyor ({} bayt).", transfer_length);

         // TODO: Bulk-IN endpoint'ten beklenen veri uzunluğu (transfer_length) kadar veriyi oku ve `data_buffer`'a yaz.
          msc_bulk_in_transfer(data_buffer, transfer_length); // Okuma fonksiyonu

         kprintln!("Okunan veri (ilk 16 bayt - ÖRNEK):"); // data_buffer'daki veriyi örnek olarak gösteriyoruz
         let read_data_slice = slice::from_raw_parts(data_buffer, transfer_length as usize); // Assume data was read into data_buffer
         for i in 0..16.min(transfer_length as usize) {
             kprint!("{:02x} ", read_data_slice[i]);
         }
         kprintln!(" ...");

        return true; // Komut ve CSW başarılıysa true dön (veri okuma başarısı da kontrol edilmeli)
    } else {
        kprintln!("Read (10) komutu başarısız!");
        return false;
    }
}

// TODO: msc_bulk_in_transfer(buffer: *mut u8, length: u32) unsafe fn
// TODO: msc_bulk_out_transfer(buffer: *const u8, length: u32) unsafe fn
// Bu fonksiyonlar Bulk endpoint'lere veri yazma/okuma işlemini donanım registerları/FIFO'lar aracılığıyla yapmalıdır.
// Gerçekte çok karmaşık, polleme, interrupt veya DMA tabanlı olabilirler.


// ************************************************************************
// ÇEKİRDEK GİRİŞ NOKTASI ve TEST KODU (Örnek amaçlı)
// ************************************************************************

// Bu fonksiyon, linker script tarafından çağrılan çekirdek giriş noktasıdır.
// no_main kullanıldığı için varsayılan main fonksiyonu çağrılmaz.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    kprintln!("srcio_arm.rs çekirdek örneği başladı!");

    // Sahne64 konsol makrolarının std dışı ortamda çalışması için gerekli
    // ilk ayarlar burada veya platform başlangıcında yapılmalıdır.
    // Örnekte kprintln! kendi KernelWriter'ını kullanıyor.

    usb_controller_init(); // USB kontrolcüsünü başlat
    // TODO: Kısa bir gecikme (VBus dengelemesi vb. için)
     unsafe { core::arch::asm!("nop"); } // Çok basit bir simülasyon gecikmesi


    if usb_device_detect() { // USB aygıtı algılandı mı?
        if usb_configure_device() { // USB aygıtını yapılandır
            kprintln!("USB aygıtı başarıyla yapılandırıldı.");

            unsafe { // MSC komutları unsafe
                if msc_reset() { // MSC Reset gönder
                    kprintln!("MSC Reset gönderildi.");
                    // TODO: Reset sonrası bekleme (minimum 10ms)
                     core::arch::asm!("nop"); // Simülasyon gecikmesi
                    if msc_get_max_lun() >= 0 { // Maksimum LUN sayısını al (veya 0 kabul et)
                         // Max LUN kontrolü (protokolde 0..15 arası olmalı)
                          if max_lun > 15 { kprintln!("Uyarı: Beklenmeyen Max LUN: {}", max_lun); }

                        if msc_test_unit_ready() { // Test Unit Ready komutu gönder
                            kprintln!("MSC Aygıt Hazır.");
                            // TODO: Aygıtın gerçekten hazır olması için birkaç kez deneme gerekebilir.
                            if msc_inquiry() { // Inquiry komutu gönder (aygıt bilgisi al)
                                 // Inquiry verisi IO_BUFFER'e okunur (simüle)
                                if msc_read_capacity_10() { // Read Capacity (10) komutu gönder (kapasite al)
                                     // Kapasite verisi IO_BUFFER'e okunur (simüle)
                                     // TODO: Okunan sektör boyutunu (sector_size) kullanın.
                                     let assumed_sector_size = 512; // Örnek: Sabit 512 bayt sektör boyutu varsayımı

                                    // Örnek olarak ilk sektörü okuma (LBA 0, 1 sektör, assumed_sector_size bayt sektör boyutu)
                                     // Okunan veri IO_BUFFER'e yazılacak (msc_read_10 içinde)
                                    if msc_read_10(0, assumed_sector_size, 1, IO_BUFFER.as_mut_ptr()) { // Unsafe çağrı
                                        kprintln!("Sektör okuma başarılı! (LBA 0, {} bayt)", assumed_sector_size);
                                        // TODO: Okunan sektörü (IO_BUFFER) Sahne64 blok katmanına veya filesystem'e sağlayabilirsiniz.
                                        block_device_layer::write_block(0, IO_BUFFER.as_ptr(), assumed_sector_size);
                                    } else {
                                        kprintln!("Sektör okuma BAŞARISIZ! (LBA 0)");
                                    }
                                } else {
                                    kprintln!("Read Capacity (10) BAŞARISIZ!");
                                }
                            } else {
                                kprintln!("Inquiry BAŞARISIZ!");
                            }
                        } else {
                            kprintln!("MSC Aygıt HAZIR DEĞİL! (Test Unit Ready başarısız)");
                        }
                    } else {
                        kprintln!("Maksimum LUN alınamadı veya geçersiz!");
                    }
                } else {
                    kprintln!("MSC Reset BAŞARISIZ!");
                }
            } // unsafe bloğu sonu (MSC komutları için)

        } else {
            kprintln!("USB aygıtı yapılandırma BAŞARISIZ!");
        }
    } else {
        kprintln!("USB aygıtı başlangıçta algılanamadı.");
    }

    kprintln!("srcio_arm.rs çekirdek örneği tamamlandı.");
    loop {} // Sonsuz döngü (çekirdek çekirdek_main fonksiyonundan dönmemeli)
}


// Panik işleyicisi (çekirdek panik durumunda çağrılır)
// PanicInfo'yu kullanarak hata bilgisini Sahne64 konsoluna yazdırır.
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Panik bilgisini Sahne64 konsol makrolarını kullanarak yazdır
    #[cfg(feature = "std")] std::eprintln!("KERNEL PANIC: {}", _info);
    #[cfg(not(feature = "std"))] eprintln!("KERNEL PANIC: {}", _info); // Varsayım: Sahne64 eprintln! makrosu

    // Sistem durdurma
    loop {} // Sonsuz döngü
}


// Redundant main fonksiyonu kaldırıldı çünkü no_main kullanılıyor ve kernel_main giriş noktası.
 #[no_mangle]
 pub extern "C" fn main() {
     kernel_main();
 }
