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
// Bu modül, kullanılan Elbrus çipindeki belirli USB kontrolcüsüne ait register
// adreslerini içermelidir. Elbrus'a özgü memory-mapped I/O adresleri için kılavuza bakın.
mod usb_registers {
    // Örnek USB kontrolcü temel adresi (Elbrus işlemcinize göre DEĞİŞİR!)
    pub const USB_CONTROLLER_BASE: usize = 0xFE000000; // Elbrus veya benzeri bir sistem için örnek adres

    // Örnek kontrol registerları (Elbrus işlemcinize göre DEĞİŞİR!)
    // Register offsetleri ve anlamları tamamen donanıma özgüdür.
    pub const REG_COMMAND: usize = 0x00;
    pub const REG_STATUS: usize = 0x04;
    pub const REG_ENDPOINT_CONTROL: usize = 0x08;
    pub const REG_ENDPOINT_STATUS: usize = 0x0C;
    pub const REG_BULK_OUT_DATA: usize = 0x10; // Örnek Bulk-OUT data FIFO/register adresi
    pub const REG_BULK_IN_DATA: usize = 0x20;  // Örnek Bulk-IN data FIFO/register adresi
    pub const REG_INTERRUPT_STATUS: usize = 0x30; // Örnek Interrupt Status Register
    pub const REG_INTERRUPT_ENABLE: usize = 0x34; // Örnek Interrupt Enable Register
    pub const REG_INTERRUPT_CLEAR: usize = 0x38;  // Örnek Interrupt Clear Register

    // Register değerlerindeki bitler ve maskeler (Elbrus işlemcinize göre DEĞİŞİR!)
    pub mod bits {
         pub const COMMAND_RESET: u32 = 1 << 0; // Örnek: Reset komut biti
         pub const STATUS_DEVICE_CONNECTED: u32 = 1 << 0; // Örnek: Aygıt bağlı biti
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
#[derive(Clone, Copy)] // Kopyalanabilir olması CBW oluştururken kolaylık sağlar
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
            dcbw_tag: 0, // Her yeni komut için benzersiz bir tag atanmalı
            dcbw_data_transfer_length: 0,
            bmcbw_flags: 0,
            bcbw_lun: 0,
            b_cbw_length: 0,
            cbw_cb: [0u8; 16],
        }
    }
     // Tag sayacı (Basit örnek, gerçekte daha güvenli yönetilmeli)
     static mut TAG_COUNTER: u32 = 0;

     pub fn new_with_tag() -> Self {
         let mut cbw = CommandBlockWrapper::new();
         unsafe {
             // unsafe block gerekli çünkü static mut kullanılıyor
             CommandBlockWrapper::TAG_COUNTER = CommandBlockWrapper::TAG_COUNTER.wrapping_add(1); // Tag'i artır (taşmaya dayanıklı)
             cbw.dcbw_tag = CommandBlockWrapper::TAG_COUNTER;
         }
         cbw
     }
}


// Command Status Wrapper (CSW) yapısı
// #[repr(C, packed)]
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)] // Debug trait'i kprintln! için gerekli
struct CommandStatusWrapper {
    dcsv_signature: u32,        // CSW İşareti (0x53425355 - "USBCSW")
    dcsv_tag: u32,              // Etiket (CBW'deki etiketle aynı olmalı)
    dcsv_data_residue: u32,     // Kalan veri uzunluğu (beklenen - aktarılan)
    bcs_status: u8,            // Komut durumu (MSC_STATUS_COMMAND_PASSED, vb.)
}

impl CommandStatusWrapper {
    pub fn new() -> Self {
        CommandStatusWrapper {
            dcsv_signature: 0, // CSW alındığında signature kontrol edilecek
            dcsv_tag: 0, // CSW alındığında CBW tag'i ile karşılaştırılacak
            dcsv_data_residue: 0,
            bcs_status: 0,
        }
    }
}

// USB cihazı tanıtıcı yapısı (Basic)
#[derive(Debug)]
struct DeviceDescriptor {
     bLength: u8, // Descriptor uzunluğu
     bDescriptorType: u8, // Descriptor tipi (0x01)
     bcdUSB: u16, // USB spesifikasyon versiyonu
     bDeviceClass: u8, // Aygıt Sınıfı
     bDeviceSubClass: u8, // Aygıt Alt Sınıfı
     bDeviceProtocol: u8, // Aygıt Protokolü
    b_max_packet_size0: u8, // Endpoint 0 için maks. paket boyutu
    id_vendor: u16, // Üretici ID
    id_product: u16, // Ürün ID
     bcdDevice: u16, // Aygıt Versiyonu
     iManufacturer: u8, // Üretici String Index
     iProduct: u8, // Ürün String Index
     iSerialNumber: u8, // Seri Numarası String Index
     bNumConfigurations: u8, // Konfigürasyon Sayısı

    // Örnek amaçlı sadece idVendor ve idProduct tutuluyor
    vendor_id: u16,
    product_id: u16,
    device_class: u8, // Ek bilgi için eklendi
    device_subclass: u8, // Ek bilgi için eklendi
    device_protocol: u8, // Ek bilgi için eklendi
}


// Kitle depolama cevabı yapısı (Şu an kullanılmıyor, CSW yeterli)
 #[derive(Debug)]
 struct MassStorageResponse {}


// ************************************************************************
// ÇEKİRDEK SEVİYESİ USB SÜRÜCÜ KODU
// ************************************************************************

// Okuma/yazma işlemleri için kullanılacak temel arabellek (kernel alanında)
// Dinamik bellek tahsisi (heap) kullanmaktan kaçınmak için statik bir arabellek.
// unsafe kullanımı gerektirir ve dikkatli yönetilmelidir.
static mut IO_BUFFER: [u8; 4096] = [0u8; 4096]; // Örnek 4KB arabellek (birden fazla sektör için yeterli olabilir)
                                                // Sektör boyutu genellikle 512, 2048 veya 4096 bayttır.


// Registerlara güvenli erişim için yardımcı fonksiyonlar (Volatile okuma/yazma)
// Bu fonksiyonlar, doğrudan bellek adreslerine erişirken derleyici optimizasyonlarını engeller.
// Elbrus'ta memory-mapped I/O için uygun register genişliğini kullanın (u32 veya u64).
#[inline(always)] // Her zaman inline yapmayı dene
unsafe fn read_register_u32(address: usize) -> u32 { // usize address input
    (address as *const u32).read_volatile() // Use const pointer for reading
}

#[inline(always)] // Her zaman inline yapmayı dene
unsafe fn write_register_u32(address: usize, value: u32) { // usize address input
    (address as *mut u32).write_volatile(value); // Use mut pointer for writing
}
// Diğer boyutlar (u8, u16, u64) için benzer fonksiyonlar eklenebilir.


// USB kontrolcüsünü başlatma fonksiyonu (DONANIMIZA ÖZEL!)
/// USB kontrolcüsünü donanımsal olarak etkinleştirir ve temel ayarlamaları yapar.
/// Bu fonksiyonun içeriği tamamen kullanılan USB donanımına bağımlıdır.
unsafe fn usb_controller_init() { // unsafe eklendi
    kprintln!("Elbrus USB kontrolcüsü başlatılıyor...");
    // TODO: DONANIMINIZA ÖZEL USB KONTROLCÜ BAŞLATMA KODUNU BURAYA YAZIN!
    // Örnek: USB kontrolcüyü etkinleştirme, saat frekanslarını ayarlama, PHY başlatma,
    // temel kesme ayarları (ancak detaylı işleme interrupt handler'da olmalı), vb.
    unsafe { // unsafe blok gerekli çünkü donanım registerlarına yazılıyor
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
        // Örneğin, bir kontrol registerına etkinleştirme biti yazma veya reset sinyali gönderme.
         let command_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_COMMAND);
         write_register_u32(command_reg_addr, bits::COMMAND_RESET); // USB kontrolcüyü resetle
        // Bir süre bekleme (reset işleminin tamamlanması için)
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
        // Reset bitini temizle (eğer yazarak resetleniyorsa)
         write_register_u32(command_reg_addr, 0); // Reset bitini temizle

         // Kontrolcüyü etkinleştirme (eğer reset ayrı bir bit ise)
          write_register_u32(command_reg_addr, bits::COMMAND_ENABLE_CONTROLLER); // Varsayımsal enable biti

        // ... diğer başlatma adımları ...

        // Not: Endpointlerin yapılandırılması genellikle aygıt bağlandıktan sonra yapılır.
    }
    kprintln!("Elbrus USB kontrolcüsü başlatma tamamlandı (Örnek).");
}


// USB aygıtını algılama fonksiyonu (BASİT ALGILAMA, GELİŞTİRİLEBİLİR)
/// USB portunda bir aygıtın bağlı olup olmadığını kontrol eder.
/// Bu fonksiyonun içeriği tamamen kullanılan USB donanımına bağımlıdır.
unsafe fn usb_device_detect() -> bool { // unsafe eklendi
    kprintln!("Elbrus USB aygıtı algılanıyor...");
    // TODO: USB aygıtı algılama mantığını buraya ekleyin.
    // Örnek: USB port durum registerlarını kontrol etme, aygıt bağlantı durumu bitini okuma, vb.
    unsafe { // unsafe blok gerekli çünkü donanım registerları okunuyor
         let status_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_STATUS);
        let status = read_register_u32(status_reg_addr);
        if (status & bits::STATUS_DEVICE_CONNECTED) != 0 { // Örnek: Aygıt bağlı bitini kontrol et
            kprintln!("Elbrus USB aygıtı algılandı.");
            return true;
        } else {
            kprintln!("Elbrus USB aygıtı algılanamadı.");
            return false;
        }
    }
}

// USB aygıtını yapılandırma (temel yapılandırma adımları)
/// Yeni bağlanan USB aygıtını adresleme, descriptor alma ve yapılandırma ayarlama adımlarını uygular.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn usb_configure_device() -> bool { // unsafe eklendi
    kprintln!("Elbrus USB aygıtı yapılandırılıyor...");

    // Not: Endpoint 0 (Kontrol Endpoint) genellikle aygıt sıfırlandıktan hemen sonra çalışmaya hazırdır.
    // Maksimum paket boyutu (EP0 Max Packet Size) genellikle 64 bayt varsayılır (ancak descriptor'dan teyit edilmeli).

    // Descriptor verisi için arabellek (statik arabellek veya caller'dan gelen arabellek kullanılmalı)
     let mut descriptor_buffer: [u8; 256] = [0u8; 256]; // Artık IO_BUFFER'ı kullanacağız veya arabellek dışarıdan gelecek.
    let descriptor_buffer_ptr = IO_BUFFER.as_mut_ptr(); // IO_BUFFER'ı kullan

    // 1. Device Descriptor'ı al
    // SETUP: 0x80 0x06 (GET_DESCRIPTOR) wValue=0x0100 (Device Desc Type=1, Index=0) wIndex=0 wLength=18
    let device_desc_len = 18;
    if device_desc_len > IO_BUFFER.len() { kprintln!("IO_BUFFER Device Descriptor için yeterli değil!"); return false; } // Basit kontrol

    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_IN, USB_REQ_GET_DESCRIPTOR, (DESC_TYPE_DEVICE as u16) << 8, 0, device_desc_len as u16, descriptor_buffer_ptr) {
        kprintln!("Device Descriptor alınamadı!");
        return false;
    }
    kprintln!("Device Descriptor alındı ({} bayt).", *descriptor_buffer_ptr); // İlk bayt uzunluk olmalı
    // TODO: Device Descriptor'ı ayrıştırıp idVendor, idProduct, bMaxPacketSize0 gibi bilgileri kullanabilirsiniz.
    // bMaxPacketSize0, Endpoint 0'ın maksimum paket boyutunu belirler ve kontrolcünüzün EP0'ı buna göre yapılandırılmalıdır.
    unsafe {
         let device_desc = descriptor_buffer_ptr as *const DeviceDescriptor;
         kprintln!("Device VendorID: {:04x}, ProductID: {:04x}", (*device_desc).vendor_id, (*device_desc).product_id);
         kprintln!("Device Class: {:02x}, SubClass: {:02x}, Protocol: {:02x}", (*device_desc).device_class, (*device_desc).device_subclass, (*device_desc).device_protocol);
    }


    // 2. Adresi ayarla (genellikle 1-127 arası bir adres atanır)
    // SETUP: 0x00 0x05 (SET_ADDRESS) wValue=Yeni Adres wIndex=0 wLength=0
    // Aygıt, SETUP aşaması tamamlandıktan sonra yeni adresi kullanmaya başlar.
    let new_address: u8 = 7; // Örnek adres
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_OUT, USB_REQ_SET_ADDRESS, new_address as u16, 0, 0, core::ptr::null_mut()) {
        kprintln!("Adres ayarlanamadı!");
        return false;
    }
    kprintln!("Adres ayarlandı: {}", new_address);
    // TODO: USB kontrolcü registerlarına aygıt adresini yazma (DONANIMIZA ÖZEL!)
    // Bu adım, kontrolcünün artık bu aygıtla belirtilen adresi kullanarak iletişim kurmasını sağlar.
    // Örneğin, kontrolcünüzün adres registerına `new_address` değerini yazmanız gerekebilir.

    // Kısa bir gecikme gerekebilir, aygıtın adres değiştirmesi için
    // TODO: Gecikme ekle (Örn: Yaklaşık 1-10 ms)
     unsafe { core::arch::asm!("nop"); } // Çok basit bir simülasyon gecikmesi


    // 3. Configuration Descriptor'ı al (ve tüm ilgili interface ve endpoint descriptor'larını)
    // Önce başlığı alıp toplam uzunluğu belirle, sonra tamamını al.
    // SETUP: 0x80 0x06 (GET_DESCRIPTOR) wValue=0x0200 (Config Desc Type=2, Index=0) wIndex=0 wLength=9 (Başlık uzunluğu)
    let config_desc_header_len = 9;
     if config_desc_header_len > IO_BUFFER.len() { kprintln!("IO_BUFFER Config Descriptor başlığı için yeterli değil!"); return false; } // Basit kontrol

    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_IN, USB_REQ_GET_DESCRIPTOR, (DESC_TYPE_CONFIGURATION as u16) << 8, 0, config_desc_header_len as u16, descriptor_buffer_ptr) { // İlk 9 baytı al (Configuration Descriptor başlığı)
        kprintln!("Configuration Descriptor başlığı alınamadı!");
        return false;
    }
    // Configuration Descriptor'ın toplam uzunluğunu descriptor'dan oku (Little-endian)
    let config_descriptor_length = unsafe {
        let len_ptr = descriptor_buffer_ptr.add(2) as *const u16;
        ptr::read_unaligned(len_ptr) // Descriptor packed olduğu için unaligned read gerekebilir
    };
    kprintln!("Config Descriptor başlığı alındı. Toplam uzunluk: {}", config_descriptor_length);

    if config_descriptor_length < 9 || config_descriptor_length > IO_BUFFER.len() as u16 { // Boyut kontrolü
         kprintln!("Geçersiz Configuration Descriptor uzunluğu ({}) veya IO_BUFFER yetersiz ({})!", config_descriptor_length, IO_BUFFER.len());
         return false;
    }

    // SETUP: 0x80 0x06 (GET_DESCRIPTOR) wValue=0x0200 wIndex=0 wLength=Toplam Uzunluk (Tamamı için)
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_IN, USB_REQ_GET_DESCRIPTOR, (DESC_TYPE_CONFIGURATION as u16) << 8, 0, config_descriptor_length, descriptor_buffer_ptr) { // Tam Configuration Descriptor'ı al
        kprintln!("Configuration Descriptor'ın tamamı alınamadı!");
        return false;
    }
    kprintln!("Configuration Descriptor alındı (Tamamı, {} bayt).", config_descriptor_length);
    // TODO: Configuration Descriptor'ı ve alt descriptor'ları (Interface, Endpoint) ayrıştırın.
    // Especially: MSC Interface (bInterfaceClass=0x08, bInterfaceSubClass=0x06, bInterfaceProtocol=0x50 for BOT)
    // BULK IN endpoint adresi (bEndpointAddress & 0x80), BULK OUT endpoint adresi (bEndpointAddress & 0x0F)
    // Endpointlerin maksimum paket boyutları.
    // Bu bilgiler, kontrolcünüzün Bulk IN/OUT endpointlerini doğru numaralar ve max paket boyutları ile yapılandırmak için kullanılır.
    // Endpoint adreslerini global değişkenlere kaydedebilirsiniz.
     unsafe { GLOBAL_BULK_IN_EP = ..., GLOBAL_BULK_OUT_EP = ... };


    // 4. Yapılandırmayı ayarla (Genellikle 1. yapılandırma kullanılır, descriptor'dan alınmalı)
    // SETUP: 0x00 0x09 (SET_CONFIGURATION) wValue=Yapılandırma Değeri (genellikle 1) wIndex=0 wLength=0
    let configuration_value: u8 = 1; // Örnek değer, descriptor'dan alınmalı
    if !usb_control_request(REQ_TYPE_STANDARD_DEVICE_OUT, USB_REQ_SET_CONFIGURATION, configuration_value as u16, 0, 0, core::ptr::null_mut()) {
        kprintln!("Yapılandırma ayarlanamadı!");
        return false;
    }
    kprintln!("Yapılandırma ayarlandı: {}", configuration_value);
    // TODO: Yapılandırma ayarlandıktan sonra yapılması gereken donanımsal ayarlar (DONANIMIZA ÖZEL!)
    // Endpointlerin donanımınızda etkinleştirilmesi ve doğru FIFO'lara/register setlerine bağlanması.
     write_register_u32(EP1_CONTROL_REG, ENABLE_BULK_IN | MAX_PACKET_SIZE_64);

    kprintln!("Elbrus USB aygıtı yapılandırması tamamlandı.");
    true // Başarılı
}


// Kontrol endpoint üzerinden USB kontrol isteği gönderme
// Bu fonksiyon, SETUP paketini oluşturup kontrol endpoint'e göndermeli, veri transferini yönetmeli ve STATUS aşamasını tamamlamalıdır.
// Çoğunlukla polleme tabanlı bir yaklaşımla donanım registerlarını kullanarak yapılır.
// GERÇEKTE: Çok daha karmaşık bir state machine ve interrupt/DMA kullanımı gerektirir.
/// # Güvenlik
/// Unsafe'dir, donanım registerlarına doğrudan erişir, veri arabelleği geçerli olmalıdır.
unsafe fn usb_control_request(request_type: u8, request: u8, value: u16, index: u16, length: u16, data_buffer: *mut u8) -> bool { // unsafe eklendi
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

    unsafe { // unsafe block gerekli çünkü donanım registerlarına yazılıyor
        // TODO: SETUP paketini donanım kontrolcünüzün kontrol endpoint'e ait register/FIFO'suna yazın. (DONANIMIZA ÖZEL!)
        // Örnek: Kontrol endpoint OUT FIFO'suna yazma register adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
        let ep0_out_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_OUT_DATA); // Placeholder, EP0 data reg/FIFO address

        // SETUP paketini yaz (polleme tabanlı örnek)
        let setup_slice = slice::from_raw_parts(setup_packet.as_ptr(), 8);
        // Kontrolcünüzün yazma arayüzüne bağlı olarak byte byte veya kelime kelime yazılabilir.
        for i in 0..8 {
             write_register_u32(ep0_out_data_reg_addr, setup_slice[i] as u32); // ÖRNEK: Her baytı 32-bit register'a yaz (ÇOK BASİT)
        }
        // TODO: Kontrolcüye SETUP paketinin gönderildiğini bildirin (DONANIMIZA ÖZEL!). Örneğin, bir komut registerı yazarak.

        // TODO: SETUP transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
        // Örneğin, bir durum registerındaki bitin set olmasını bekleyin.
         while (read_register_u32(status_reg) & SETUP_TRANSFER_DONE_BIT) == 0 { core::hint::spin_loop(); }
    }


    // 2. DATA Aşaması (wLength > 0 ise)
    if length > 0 {
        // Yöne göre DATA transferi (IN veya OUT)
        if (request_type & CBW_FLAG_DIRECTION_IN) != 0 { // Device to Host (IN)
             // TODO: Kontrol endpoint IN FIFO'sundan 'length' bayt veriyi 'data_buffer'a okuyun. (DONANIMIZA ÖZEL!)
             // Örnek: Kontrol endpoint IN FIFO'sundan okuma register adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
             let ep0_in_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_IN_DATA); // Placeholder, EP0 data reg/FIFO address

             unsafe { // unsafe block gerekli
                 let data_slice = slice::from_raw_parts_mut(data_buffer, length as usize);
                 for i in 0..length as usize {
                     // Kontrolcünüzün okuma arayüzüne bağlı olarak byte byte veya kelime kelime okunabilir.
                      data_slice[i] = read_register_u32(ep0_in_data_reg_addr) as u8; // ÖRNEK: 32-bit registerdan 1 bayt oku
                 }
             }
             // TODO: DATA IN transferinin tamamlanmasını bekleyin (Polleme veya interrupt).

        } else { // Host to Device (OUT)
             // TODO: 'data_buffer'dan 'length' bayt veriyi kontrol endpoint OUT FIFO'suna yazın. (DONANIMIZA ÖZEL!)
             // Örnek: Kontrol endpoint OUT FIFO'suna yazma register adresi aynı EP0 OUT data registerı olabilir.
             let ep0_out_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_OUT_DATA); // Placeholder

             unsafe { // unsafe block gerekli
                  let data_slice = slice::from_raw_parts(data_buffer, length as usize);
                 for i in 0..length as usize {
                     write_register_u32(ep0_out_data_reg_addr, data_slice[i] as u32); // ÖRNEK: Her baytı 32-bit register'a yaz
                 }
             }
             // TODO: DATA OUT transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
        }
         // TODO: DATA aşamasında ACK/NAK/STALL durumlarını yönetin.
    }


    // 3. STATUS Aşaması (Karşı yöne 0 uzunluklu paket gönderilir/alınır)
    // DATA aşaması IN ise STATUS aşaması OUT, DATA aşaması OUT ise STATUS aşaması IN'dir.
    if (request_type & CBW_FLAG_DIRECTION_IN) != 0 { // DATA IN ise STATUS OUT
         // TODO: Kontrol endpoint OUT'a 0 uzunluklu STATUS paketini gönderin. (DONANIMIZA ÖZEL!)
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
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn msc_reset() -> bool { // unsafe eklendi
    kprintln!("MSC Reset isteği gönderiliyor...");
    // MSC Reset isteği kontrol endpoint üzerinden sınıf-özel bir istektir.
     bmRequestType = 0x21 (Class | Interface | HostToDevice), bRequest=0xFF (MSC_REQ_RESET), wValue=0, wIndex=Interface#, wLength=0
    // wIndex genellikle MSC arayüz numarasını içerir. Bu örnekte 0 varsayıyoruz.
    if usb_control_request(REQ_TYPE_CLASS_INTERFACE_OUT, MSC_REQ_RESET, 0, 0, 0, core::ptr::null_mut()) { // Interface isteği (0x21), Alıcı Arayüzü (Interface)
        kprintln!("MSC Reset isteği gönderildi.");
        // TODO: Reset sonrası gerekli donanımsal bekleme veya durum kontrolü (DONANIMA ÖZEL!)
        // USB spesifikasyonu reset sonrası en az 10ms bekleme önerir.
        // Örnek: Gecikme fonksiyonunu çağır
         delay_ms(10);
        return true;
    } else {
        kprintln!("MSC Reset isteği BAŞARISIZ!");
        return false;
    }
}

// Maksimum LUN sayısını alma isteği
/// MSC Bulk-Only Transport Get_Max_LUN isteğini gönderir ve Max LUN sayısını alır.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn msc_get_max_lun() -> u8 { // unsafe eklendi
    kprintln!("Maksimum LUN sayısı alınıyor...");
    // MSC Get_Max_LUN isteği kontrol endpoint üzerinden sınıf-özel bir istektir.
    // bmRequestType = 0xA1 (Class | Interface | DeviceToHost), bRequest=0xFE (MSC_REQ_GET_MAX_LUN), wValue=0, wIndex=Interface#, wLength=1
    let mut lun_buffer: [u8; 1] = [0];
    // wIndex genellikle MSC arayüz numarasını içerir. Bu örnekte 0 varsayıyoruz.
    if usb_control_request(REQ_TYPE_CLASS_INTERFACE_IN, MSC_REQ_GET_MAX_LUN, 0, 0, 1, lun_buffer.as_mut_ptr()) { // Aygıttan veri alımı (0xA1), Alıcı Arayüzü (Interface)
        let max_lun = lun_buffer[0];
        kprintln!("Maksimum LUN sayısı alındı: {}", max_lun);
        // USB spesifikasyonu LUN değerinin 0-15 arasında olmasını gerektirir.
        if max_lun > 15 {
             kprintln!("Uyarı: Beklenmeyen Max LUN değeri: {} (0-15 arası olmalı)", max_lun);
             // Bu durumda bir hata işleme stratejisi benimsemek gerekebilir.
        }
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
unsafe fn msc_send_cbw(cbw: &CommandBlockWrapper) -> bool { // unsafe eklendi
    kprintln!("CBW gönderiliyor. Komut: {:02x}, Data Len: {}", cbw.cbw_cb[0], cbw.dcbw_data_transfer_length);

    // **BASİTLEŞTİRİLMİŞ & POLLEME TABANLI ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**

    // TODO: Bulk-OUT endpoint'e CBW gönderme mantığını buraya ekleyin.
    // Bu fonksiyon, CBW yapısını bellekte doğru şekilde oluşturmanız,
    // Bulk-OUT endpoint'e göndermeniz (donanım registerlarını/FIFOlara yazarak),
    // veri transferini yönetmeniz ve gerekli durum kontrollerini yapmanız gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    // Varsayım: Bulk-OUT endpoint'e veri yazma registerı/FIFO adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
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
         // Daha doğru bir yaklaşım: eğer FIFO 32-bit ise 4 baytı birleştirip yazmak veya donanımın DMA kullanmasını sağlamak.
         // volatile yazma:
         (bulk_out_data_reg_addr as *mut u8).write_volatile(cbw_bytes[i]); // ÖRNEK: 1 baytı volatile yaz (ÇOK BASİT)
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
unsafe fn msc_receive_csw(csw: &mut CommandStatusWrapper) -> bool { // unsafe eklendi
    kprintln!("CSW bekleniyor...");

    // **BASİTLEŞTİRİLMİŞ & POLLEME TABANLI ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**

    // TODO: Bulk-IN endpoint'ten CSW alma mantığını buraya ekleyin.
    // Bu fonksiyon, Bulk-IN endpoint'ten veri okumanız (donanım registerlarını/FIFOlardan),
    // CSW yapısını gelen verilere göre doldurmanız ve gerekli durum kontrollerini yapmanız gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    // Varsayım: Bulk-IN endpoint'ten veri okuma registerı/FIFO adresi (DONANIMINIZA GÖRE DEĞİŞİR!)
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
        // csw_bytes[i] = read_register_u32(bulk_in_data_reg_addr) as u8; // ÖRNEK: 32-bit registerdan 1 bayt oku (ÇOK BASİT)
         // Daha doğru bir yaklaşım: eğer FIFO 32-bit ise 4 baytı birleştirip okumak veya donanımın DMA kullanmasını sağlamak.
        csw_bytes[i] = (bulk_in_data_reg_addr as *const u32).read_volatile() as u8; // ÖRNEK: 1 baytı volatile oku (ÇOK BASİT)
    }
    // TODO: CSW transferinin tamamlanmasını bekleyin (Polleme veya interrupt).
    // Örneğin, endpoint durum registerındaki bir bitin set olmasını bekleyin.
     while (read_register_u32(ep_status_reg) & RX_DONE_BIT) == 0 { core::hint::spin_loop(); }


     // CSW İmzası ve Tag kontrolü (protokole uygunluk için)
     if csw.dcsv_signature.swap_bytes() != CSW_SIGNATURE { // Elbrus big-endian ise swap gerekebilir
         kprintln!("Hata: CSW İmzası Yanlış! Beklenen: {:x}, Alınan: {:x}", CSW_SIGNATURE, csw.dcsv_signature.swap_bytes());
         // TODO: Faz hatası (Phase Error) durumunu yönetin.
         return false; // İmza yanlışsa başarısız
     }
     // TODO: CSW tag'inin son gönderilen CBW tag'i ile eşleştiğini kontrol edin.


    // TODO: Hata yönetimi (NAK, STALL gibi durumlar) ve zaman aşımları.

    kprintln!("CSW alındı (Simüle). Durum: {:02x}, Kalan: {}", csw.bcs_status, csw.dcsv_data_residue);
    true // Örnek başarı durumu
}


// MSC komutu gönderme, veri transferi ve CSW alma (Temel MSC işlem akışı)
/// Bir MSC CBW gönderir, isteğe bağlı veri transferini (OUT veya IN) yönetir
/// ve ardından CSW'yi alır.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir. Veri arabelleği geçerli olmalıdır.
unsafe fn msc_send_command(cbw: &mut CommandBlockWrapper, data_buffer: *mut u8, data_length: u32) -> bool { // data_buffer ve length eklendi
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
    // CBW'deki transfer_length = data_length olmalı.
    if data_length > 0 {
        if (cbw.bmcbw_flags & CBW_FLAG_DIRECTION_IN) != 0 { // Cihazdan hosta (IN)
            // Veriyi data_buffer'a oku
             kprintln!("Veri alımı bekleniyor ({} bayt)...", data_length);
             if !msc_bulk_in_transfer(data_buffer, data_length) { // Okuma fonksiyonu
                 kprintln!("Veri alımı başarısız!");
                 // TODO: Hata yönetimi (Faz hatası, STALL vb.)
                 return false;
             }
             kprintln!("Veri alımı tamamlandı.");

        } else { // Host to Device (OUT)
            // Veriyi data_buffer'dan yaz
             kprintln!("Veri gönderimi bekleniyor ({} bayt)...", data_length);
             if !msc_bulk_out_transfer(data_buffer, data_length) { // Yazma fonksiyonu
                 kprintln!("Veri gönderimi başarısız!");
                 // TODO: Hata yönetimi (Faz hatası, STALL vb.)
                 return false;
             }
             kprintln!("Veri gönderimi tamamlandı.");
        }
    }


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
unsafe fn msc_test_unit_ready() -> bool { // unsafe eklendi
    kprintln!("Test Unit Ready komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new_with_tag(); // Tag'i otomatik al
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri yok, durum bilgisi alacağız (IN durumu)
    cbw.b_cbw_length = 6; // SCSI komut uzunluğu (Test Unit Ready 6 bayttır)
    cbw.cbw_cb[0] = MSC_CMD_TEST_UNIT_READY; // Test Unit Ready komutu

    // CBW gönder, Veri yok, CSW al
    msc_send_command(&mut cbw, core::ptr::null_mut(), 0) // Veri arabelleği null, uzunluk 0
}


// MSC Inquiry komutu (Aygıt bilgilerini alma)
/// Aygıtın temel MSC/SCSI bilgilerini (Vendor ID, Product ID, Versiyon vb.) alır
/// ve belirtilen arabelleğe yazar.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir. Veri arabelleği geçerli ve yeterli büyüklükte olmalıdır.
unsafe fn msc_inquiry(data_buffer: *mut u8, buffer_length: u32) -> bool { // data_buffer ve length eklendi
    kprintln!("Inquiry komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new_with_tag(); // Tag'i otomatik al
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri alacağız (IN)
    let inquiry_data_len: u32 = 36; // Inquiry verisi için beklenen uzunluk (standart)
    cbw.dcbw_data_transfer_length = inquiry_data_len; // Okunacak toplam veri uzunluğu
    cbw.b_cbw_length = 6; // SCSI komut uzunluğu
    cbw.cbw_cb[0] = MSC_CMD_INQUIRY; // Inquiry komutu
    cbw.cbw_cb[1] = 0; // LUN (Bits 7-5), EVPD (Bit 1) - Usually 0
    cbw.cbw_cb[2] = 0; // Page Code
    cbw.cbw_cb[3] = 0; // Reserved
    // Allocation Length (beklenen veri uzunluğu, Big-endian)
    let allocation_length = buffer_length.min(inquiry_data_len); // Sağlanan arabellek uzunluğu kadar iste
    cbw.cbw_cb[4] = ((allocation_length >> 8) & 0xFF) as u8;
    cbw.cbw_cb[5] = (allocation_length & 0xFF) as u8;
    // ... Geri kalan 10 bayt 0 ...

    // CBW gönder, Veri al, CSW al
    if msc_send_command(&mut cbw, data_buffer, allocation_length) {
         kprintln!("Inquiry CBW/CSW/Data başarılı.");
         // Okunan veriyi (data_buffer'dan) ayrıştırıp kullanabilirsiniz.
         let inquiry_result_bytes = slice::from_raw_parts(data_buffer, allocation_length as usize);
         kprintln!("Inquiry verisi (ilk {} bayt):", 8.min(allocation_length as usize));
         for i in 0..8.min(allocation_length as usize) {
             kprint!("{:02x} ", inquiry_result_bytes[i]);
         }
         kprintln!("");
         // TODO: Inquiry verisini ayrıştır (VendorID, ProductID vb.)

         // CSW'deki residue kontrol edilebilir.
          if csw.dcsv_data_residue != (inquiry_data_len - allocation_length) { ... }

         return true; // Komut, veri ve CSW başarılıysa true dön
    } else {
        kprintln!("Inquiry komutu başarısız!");
        return false;
    }
}


// MSC Read Capacity (10) komutu (Kapasite bilgisini alma)
/// Aygıtın toplam sektör sayısını ve sektör boyutunu alır
/// ve belirtilen arabelleğe yazar (8 bayt).
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir. Veri arabelleği geçerli ve en az 8 bayt olmalıdır.
unsafe fn msc_read_capacity_10(data_buffer: *mut u8, buffer_length: u32) -> bool { // data_buffer ve length eklendi
    kprintln!("Read Capacity (10) komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new_with_tag(); // Tag'i otomatik al
    cbw.bmcbw_flags = CBW_FLAG_DIRECTION_IN; // Veri alacağız (IN)
    let capacity_data_len: u32 = 8;  // Read Capacity (10) verisi 8 bayt uzunluğunda
    cbw.dcbw_data_transfer_length = capacity_data_len; // Okunacak toplam veri uzunluğu
    cbw.b_cbw_length = 10; // SCSI komut uzunluğu
    cbw.cbw_cb[0] = MSC_CMD_READ_CAPACITY_10; // Read Capacity (10) komutu
    // ... Geri kalan 9 bayt 0 (Reserved/LUN) ...

    if buffer_length < capacity_data_len {
         kprintln!("Hata: Kapasite verisi için arabellek yeterli değil (min {} bayt)! Sağlanan {}", capacity_data_len, buffer_length);
         return false;
    }


    // CBW gönder, Veri al, CSW al
    if msc_send_command(&mut cbw, data_buffer, capacity_data_len) {
        kprintln!("Read Capacity (10) CBW/CSW/Data başarılı.");
         // Okunan veriyi (data_buffer'dan) ayrıştırıp kullanabilirsiniz.
         let capacity_result_bytes = slice::from_raw_parts(data_buffer, capacity_data_len as usize);
        kprintln!("Read Capacity (10) verisi (8 bayt):");
         for i in 0..capacity_data_len as usize {
             kprint!("{:02x} ", capacity_result_bytes[i]);
         }
         kprintln!("");
         // Örnek ayrıştırma (Big-endian veya Little-endian - donanıma ve aygıta göre değişir):
         // Elbrus big-endian olabilir, bu durumda swap_bytes gerekebilir.
         let last_lba = u32::from_be_bytes([capacity_result_bytes[0], capacity_result_bytes[1], capacity_result_bytes[2], capacity_result_bytes[3]]);
         let sector_size = u32::from_be_bytes([capacity_result_bytes[4], capacity_result_bytes[5], capacity_result_bytes[6], capacity_result_bytes[7]]);
         kprintln!("Son LBA: {}, Sektör Boyutu: {}", last_lba, sector_size);

        return true; // Komut, veri ve CSW başarılıysa true dön
    } else {
        kprintln!("Read Capacity (10) komutu başarısz!");
        return false;
    }
}

// MSC Read (10) komutu (Sektör okuma)
/// Belirtilen LBA'dan belirtilen sayıda bloğu okur
/// ve belirtilen arabelleğe yazar.
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir. Veri arabelleği geçerli ve yeterli büyüklükte olmalıdır.
unsafe fn msc_read_10(lba: u32, block_size: u32, block_count: u16, data_buffer: *mut u8, buffer_length: u32) -> bool { // data_buffer ve length eklendi
    kprintln!("Read (10) komutu gönderiliyor. LBA: {}, Blok Sayısı: {}, Blok Boyutu: {}", lba, block_count, block_size);
    let transfer_length = block_size * block_count as u32; // Toplam transfer uzunluğu (u32)

    // Sağlanan arabellek transfer için yeterli mi kontrol et
     if buffer_length < transfer_length {
         kprintln!("Hata: Read transfer uzunluğu ({}) sağlanan arabellek boyutundan ({}) büyük!", transfer_length, buffer_length);
         return false;
     }
     if data_buffer.is_null() && transfer_length > 0 { // Veri okunacak ama arabellek yoksa hata
          kprintln!("Hata: Veri okunacak ancak data_buffer NULL!");
          return false;
     }


    let mut cbw = CommandBlockWrapper::new_with_tag(); // Tag'i otomatik al
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


    // CBW gönder, Veri al, CSW al
    if msc_send_command(&mut cbw, data_buffer, transfer_length) {
         kprintln!("Read (10) CBW/CSW/Data başarılı. Okunan {} bayt.", transfer_length);

         // Okunan veriyi (data_buffer'dan) ayrıştırıp kullanabilirsiniz.
         let read_data_slice = slice::from_raw_parts(data_buffer, transfer_length as usize);
         kprintln!("Okunan veri (ilk {} bayt - ÖRNEK):", 16.min(transfer_length as usize)); // data_buffer'daki veriyi örnek olarak gösteriyoruz
         for i in 0..16.min(transfer_length as usize) {
             kprint!("{:02x} ", read_data_slice[i]);
         }
         kprintln!(" ...");

        return true; // Komut, veri ve CSW başarılıysa true dön
    } else {
        kprintln!("Read (10) komutu başarısız!");
        return false;
    }
}

// TODO: MSC Write (10) komutu (Sektör yazma) unsafe fn msc_write_10(lba: u32, block_size: u32, block_count: u16, data_buffer: *const u8, buffer_length: u32) -> bool

// TODO: msc_bulk_in_transfer(buffer: *mut u8, length: u32) unsafe fn
// TODO: msc_bulk_out_transfer(buffer: *const u8, length: u32) unsafe fn
// Bu fonksiyonlar Bulk endpoint'lere veri yazma/okuma işlemini donanım registerları/FIFO'lar aracılığıyla yapmalıdır.
// Gerçekte çok karmaşık, polleme, interrupt veya DMA tabanlı olabilirler.


// USB Cihazı Bağlantı İşleyicisi (handle_usb_device_connected)
// Bu fonksiyon, bir USB aygıtının bağlandığı donanımsal bir olay (kesme)
// algılandığında çekirdek tarafından çağrılmalıdır.
// Şu anki kod polleme tabanlı, bu yüzden bu fonksiyon doğrudan çağrılmıyor.
fn handle_usb_device_connected(descriptor: DeviceDescriptor) { // descriptor parametresi eklendi
    kprintln!("USB cihazı bağlandı.");

    // Cihazı numaralandır ve yapılandır (USB protokolüne göre)
    // read_device_descriptor() zaten configure_device içinde çağrılıyor.
    // handle_usb_device_connected'a descriptor'ın tamamı veya bir kısmı parametre olarak gelebilir.

    kprintln!("Cihaz Tanıtıcısı: Vendor=0x{:X}, Product=0x{:X}",
                descriptor.vendor_id, descriptor.product_id);
    kprintln!("Cihaz Sınıfı: {:02x}, Alt Sınıf: {:02x}, Protokol: {:02x}",
                descriptor.device_class, descriptor.device_subclass, descriptor.device_protocol);


    // Eğer cihaz bir USB sürücüsü ise (Class=0x08)
    if descriptor.device_class == 0x08 { // Sadece sınıf koduna bakmak basit bir yöntemdir.
         kprintln!("Bu bir USB kitle depolama aygıtı (Sınıf: 0x08).");
        // USB sürücüsü ile iletişimi başlat
        unsafe { initiate_mass_storage_communication(); } // unsafe çağrı
    } else {
         kprintln!("Bu bir kitle depolama aygıtı DEĞİL (Sınıf: {:02x}).", descriptor.device_class);
    }
}

// USB aygıtının tanıtıcı bilgisini oku (Güncellenmiş)
// Bu fonksiyonun içeriği tamamen kullanılan USB donanımına bağımlıdır.
// usb_configure_device fonksiyonu bu logic'in bir kısmını zaten içeriyor.
// Bu fonksiyon artık sadece configure_device'dan descriptor ayrıştırma kısmını simüle ediyor.
unsafe fn read_device_descriptor() -> DeviceDescriptor { // unsafe eklendi
     kprintln!("Device Descriptor okunuyor (Simüle)...");
    // Gerçek okuma işlemi usb_control_request fonksiyonu ile yapılır.
    // usb_configure_device fonksiyonunda bu zaten yapılıyor.
    // Burada sadece örnek bir descriptor yapısı dönülüyor.
     let descriptor_buffer_ptr = IO_BUFFER.as_mut_ptr(); // IO_BUFFER'da olduğunu varsayalım

     // TODO: usb_control_request çağrısı ile Device Descriptor'ı IO_BUFFER'a okuyun.
     // Bu kısım usb_configure_device içinde zaten yapılıyor.
     // Bu fonksiyon artık bu okuma işini yapmayacak, sadece okunduğunu varsayıp ayrıştıracak.

     // ÖRNEK AYRIŞTIRMA (IO_BUFFER'da Device Descriptor olduğunu varsayarak)
     let device_desc_bytes = slice::from_raw_parts(descriptor_buffer_ptr, 18); // Device Descriptor 18 bayt

     let descriptor = DeviceDescriptor {
         b_max_packet_size0: device_desc_bytes[7],
         id_vendor: u16::from_le_bytes([device_desc_bytes[8], device_desc_bytes[9]]),
         id_product: u16::from_le_bytes([device_desc_bytes[10], device_desc_bytes[11]]),
         device_class: device_desc_bytes[5],
         device_subclass: device_desc_bytes[6],
         device_protocol: device_desc_bytes[7], // Note: This is bMaxPacketSize0 in standard descriptor, Protocol is byte 7
                                                // Standard descriptor: bLength (0), bDescriptorType (1), bcdUSB (2-3), bDeviceClass (4), bDeviceSubClass (5), bDeviceProtocol (6), bMaxPacketSize0 (7)
                                                // Corrected:
                                                device_class: device_desc_bytes[4],
                                                device_subclass: device_desc_bytes[5],
                                                device_protocol: device_desc_bytes[6],
                                                b_max_packet_size0: device_desc_bytes[7],
     };
     kprintln!("Device Descriptor Ayrıştırıldı (Simüle).");
     descriptor
}

// Cihaz tanıtıcısına göre kitle depolama cihazı olup olmadığını kontrol et (Artık sınıf kodunu kullanıyor)
/// Verilen Device Descriptor'a göre aygıtın bir MSC aygıtı olup olmadığını kontrol eder.
/// Genellikle Device Class (0x08) ve SubClass/Protocol alanlarına bakılır.
fn is_mass_storage_device(descriptor: &DeviceDescriptor) -> bool {
    // Bulk-Only Transport (BOT) için Class=0x08, SubClass=0x06, Protocol=0x50 beklenir.
    // Ancak sadece Class=0x08 kontrolü de yapılabilir.
    // Bu örnekte sadece Class'a bakıyoruz.
     descriptor.device_class == 0x08
      && descriptor.device_subclass == 0x06 // Daha detaylı kontrol
      && descriptor.device_protocol == 0x50 // Daha detaylı kontrol
}

// Kitle depolama cihazı ile iletişimi başlat (Güncellenmiş)
/// MSC aygıtı ile temel iletişim akışını başlatır (Reset, Get Max LUN, Inquiry, Read Capacity).
/// # Güvenlik
/// Unsafe'dir, alt fonksiyonlar donanıma doğrudan erişir.
unsafe fn initiate_mass_storage_communication() { // unsafe eklendi
    kprintln!("Elbrus Kitle depolama iletişimi başlatılıyor.");

    // Bulk-Only Transport (BOT) protokolü adımları
    // 1. MSC Reset gönder
    if !msc_reset() {
        kprintln!("MSC Reset BAŞARISIZ!");
        return;
    }
    kprintln!("MSC Reset gönderildi.");
    // TODO: Reset sonrası bekleme (minimum 10ms)
    unsafe { core::arch::asm!("nop"); } // Simülasyon gecikmesi

    // 2. Get Max LUN al
    let max_lun = msc_get_max_lun();
    kprintln!("Maksimum LUN sayısı: {}", max_lun);
    // TODO: Eğer max_lun > 0 ise, her LUN için Inquiry, Read Capacity vb. yapabilirsiniz.
    // Bu örnekte sadece LUN 0 ile devam ediyoruz.

    // 3. Inquiry komutu gönder (Aygıt bilgilerini al)
    // Inquiry verisi IO_BUFFER'e okunacak.
    if !msc_inquiry(IO_BUFFER.as_mut_ptr(), IO_BUFFER.len() as u32) { // IO_BUFFER ve boyutu geçildi
         kprintln!("Inquiry BAŞARISIZ!");
         // TODO: Kurtarma işlemi
         return;
    }
    kprintln!("Inquiry başarılı.");
    // TODO: IO_BUFFER'daki Inquiry verisini ayrıştırıp kullanın.


    // 4. Read Capacity (10) komutu gönder (Kapasite bilgisini al)
    // Kapasite verisi IO_BUFFER'e okunacak (8 bayt).
    if !msc_read_capacity_10(IO_BUFFER.as_mut_ptr(), IO_BUFFER.len() as u32) { // IO_BUFFER ve boyutu geçildi
         kprintln!("Read Capacity (10) BAŞARISIZ!");
         // TODO: Kurtarma işlemi
         return;
    }
    kprintln!("Read Capacity (10) başarılı.");
    // TODO: IO_BUFFER'daki Kapasite verisini ayrıştırıp Son LBA ve Sektör Boyutunu kaydedin.
     unsafe { GLOBAL_SECTOR_SIZE = ..., GLOBAL_LAST_LBA = ... };

    kprintln!("Elbrus Kitle depolama başlatma tamamlandı.");

    // Örnek: Ayrıştırılan sektör boyutu ile ilk sektörü okuma
    let assumed_sector_size = 512; // Örnek varsayım, Read Capacity'den alınmalı
    let lba_to_read = 0;
    let block_count_to_read = 1; // 1 sektör okuma

     // IO_BUFFER sektörü okumak için yeterli mi?
     if (assumed_sector_size * block_count_to_read as u32) as usize > IO_BUFFER.len() {
         kprintln!("IO_BUFFER sektör okuma için yeterli değil! {} > {}", (assumed_sector_size * block_count_to_read as u32), IO_BUFFER.len());
     } else {
        // Read (10) komutu gönder
        if msc_read_10(lba_to_read, assumed_sector_size, block_count_to_read, IO_BUFFER.as_mut_ptr(), IO_BUFFER.len() as u32) { // Unsafe çağrı, IO_BUFFER kullanıldı
             kprintln!("Sektör okuma başarılı! (LBA {}, {} bayt)", lba_to_read, assumed_sector_size * block_count_to_read as u32);
             // TODO: Okunan sektörü (IO_BUFFER) Sahne64 blok katmanına veya filesystem'e sağlayabilirsiniz.
              block_device_layer::write_block(0, IO_BUFFER.as_ptr(), assumed_sector_size);
        } else {
             kprintln!("Sektör okuma BAŞARISIZ! (LBA {})", lba_to_read);
        }
     }


}

// Kitle depolama komutu gönder (Bu fonksiyon artık veri transferini de yönetiyor)
// Daha önce ayrı fonksiyonlardı, şimdi birleştirildi.
// CBW gönderir, data_length > 0 ise veri transferini yönetir, CSW alır.
// # Güvenlik
// Unsafe'dir, donanıma doğrudan erişir. data_buffer geçerli olmalıdır.
 unsafe fn send_mass_storage_command(opcode: u8, lba: u32, length: u16) { ... } // Kaldırıldı, send_command kullanılıyor

// Kitle depolama cevabı al (Bu fonksiyon artık sadece CSW alıyor)
// Daha önce ayrı fonksiyonlardı, şimdi send_command içinde çağrılıyor.
 unsafe fn receive_mass_storage_response() -> MassStorageResponse { ... } // Kaldırıldı, msc_receive_csw kullanılıyor

// Kitle depolama verisi al (Bu fonksiyon artık sadece Bulk IN transferini yapıyor)
// Daha önce dinamik vec! kullanıyordu, şimdi static IO_BUFFER veya caller'dan gelen buffer kullanacak.
 unsafe fn receive_mass_storage_data(length: usize) -> Vec<u8> { ... } // Kaldırıldı, msc_bulk_in_transfer veya msc_read_10/msc_write_10 kullanılıyor.


// TODO: msc_bulk_in_transfer(buffer: *mut u8, length: u32) unsafe fn
/// Bulk IN endpoint üzerinden belirtilen arabelleğe veri okur.
/// # Güvenlik
/// Unsafe'dir, donanıma doğrudan erişir. Arabellek geçerli ve yeterli büyüklükte olmalıdır.
unsafe fn msc_bulk_in_transfer(buffer: *mut u8, length: u32) -> bool {
    kprintln!("Bulk IN transferi başlatılıyor ({} bayt)...", length);
    // TODO: Bulk-IN endpoint'ten 'length' bayt veriyi 'buffer'a okuma mantığını buraya ekleyin. (DONANIMIZA ÖZEL!)
    // Bu, donanımınızın Bulk-IN data register/FIFO'sundan okuma döngüsü veya DMA transferi olabilir.
    let bulk_in_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_IN_DATA); // Örnek adres
    let data_slice = slice::from_raw_parts_mut(buffer, length as usize);

    for i in 0..length as usize {
        // Kontrolcünüzün okuma arayüzüne bağlı olarak byte byte veya kelime kelime okunabilir.
         data_slice[i] = (bulk_in_data_reg_addr as *const u32).read_volatile() as u8; // ÖRNEK: 32-bit registerdan 1 bayt oku (ÇOK BASİT)
    }

    // TODO: Transferin tamamlanmasını ve başarı durumunu kontrol edin (Polleme veya interrupt).

     kprintln!("Bulk IN transferi tamamlandı (Simüle).");
    true // Örnek başarı durumu
}

// TODO: msc_bulk_out_transfer(buffer: *const u8, length: u32) unsafe fn
/// Bulk OUT endpoint üzerinden belirtilen arabellekten veri yazar.
/// # Güvenlik
/// Unsafe'dir, donanıma doğrudan erişir. Arabellek geçerli ve yeterli büyüklükte olmalıdır.
unsafe fn msc_bulk_out_transfer(buffer: *const u8, length: u32) -> bool {
    kprintln!("Bulk OUT transferi başlatılıyor ({} bayt)...", length);
    // TODO: Bulk-OUT endpoint'e 'length' bayt veriyi 'buffer'dan yazma mantığını buraya ekleyin. (DONANIMIZA ÖZEL!)
    // Bu, donanımınızın Bulk-OUT data register/FIFO'suna yazma döngüsü veya DMA transferi olabilir.
    let bulk_out_data_reg_addr = usb_registers::USB_CONTROLLER_BASE.wrapping_add(usb_registers::REG_BULK_OUT_DATA); // Örnek adres
    let data_slice = slice::from_raw_parts(buffer, length as usize);

    for i in 0..length as usize {
         // Kontrolcünüzün yazma arayüzüne bağlı olarak byte byte veya kelime kelime yazılabilir.
         (bulk_out_data_reg_addr as *mut u8).write_volatile(data_slice[i]); // ÖRNEK: 1 baytı volatile yaz (ÇOK BASİT)
    }

    // TODO: Transferin tamamlanmasını ve başarı durumunu kontrol edin (Polleme veya interrupt).

     kprintln!("Bulk OUT transferi tamamlandı (Simüle).");
    true // Örnek başarı durumu
}


// ************************************************************************
// ÇEKİRDEK GİRİŞ NOKTASI ve TEST KODU (Örnek amaçlı)
// ************************************************************************

// Bu fonksiyon, linker script tarafından çağrılan çekirdek giriş noktasıdır.
// no_main kullanıldığı için varsayılan main fonksiyonu çağrılmaz.
#[no_mangle]
pub extern "C" fn _start() -> ! {
     unsafe block gerekli çünkü usb_driver_init unsafe
    unsafe {
         // Sahne64 konsol makrolarının std dışı ortamda çalışması için gerekli
         // ilk ayarlar burada veya platform başlangıcında yapılmalıdır.
         // Örnekte kprintln! Sahne64 makrolarını kullanıyor (varsayım).
        kprintln!("srcio_elbrus.rs çekirdek örneği başladı! (Elbrus)");

        usb_driver_init(); // USB kontrolcüsünü başlat (unsafe)

        // TODO: Kısa bir gecikme (VBus dengelemesi vb. için)
         core::arch::asm!("nop"); // Çok basit bir simülasyon gecikmesi

        if usb_device_detect() { // USB aygıtı algılandı mı? (unsafe)
             // Aygıt algılandığında, configure_device'ı çağırarak numaralandırma yap.
            unsafe {  configure_device unsafe
                if usb_configure_device() { // USB aygıtını yapılandır
                    kprintln!("Elbrus USB aygıtı başarıyla yapılandırıldı.");

                    // Yapılandırma sonrası temel MSC iletişimini başlat
                     initiate_mass_storage_communication(); // unsafe
                } else {
                    kprintln!("Elbrus USB aygıtı yapılandırma BAŞARISIZ!");
                }
            }  unsafe block for configure_device

        } else {
            kprintln!("Elbrus USB aygıtı başlangıçta algılanamadı.");
        }

        kprintln!("srcio_elbrus.rs çekirdek örneği tamamlandı.");
    }  unsafe block for usb_driver_init and usb_device_detect

    loop {} // Sonsuz döngü (çekirdek _start fonksiyonundan dönmemeli)
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

// Redundant main fonksiyonu kaldırıldı çünkü no_main kullanılıyor ve _start giriş noktası.
 #[no_mangle]
 pub extern "C" fn main() {
     _start(); // Call the actual entry point
}
