#![no_std]
#![no_main]

// Bu dosya srcio_arm.rs

// Bu örnek kod, USB sürücüsü kullanarak (USB yığın depolama sınıfı - Mass Storage Class - MSC varsayımıyla)
// basit bir "srcio_arm.rs" dosyası için çekirdek seviyesinde (kendi çekirdeğinizde) bir örnektir.
// API kullanılmayacak ve Linux çekirdeği yerine kendi çekirdeğiniz için tasarlanmıştır.
// Donanım seviyesinde USB kontrolcüsü ve yığın depolama aygıtı ile etkileşime odaklanacaktır.

// ÖNEMLİ UYARI:
// Bu KESİNLİKLE BASİTLEŞTİRİLMİŞ bir örnektir ve gerçek dünya senaryoları için YETERLİ DEĞİLDİR.
// Gerçek bir USB sürücüsü ÇOK DAHA KARMAŞIKTIR, hata yönetimi, detaylı USB protokolü işleme,
// farklı USB kontrolcüleri ve cihaz türleri için destek, performans optimizasyonu, vb. içerir.
// Bu örnek sadece temel kavramları GÖSTERMEK için tasarlanmıştır.
// DONANIMINIZA ÖZEL REGİSTER ADRESLERİ VE USB KONTROLCÜ DETAYLARI İLE GÜNCELLENMESİ GEREKİR.

// Varsayımlar:
// 1. ARM mimarisi üzerinde çalışıyorsunuz.
// 2. USB kontrolcünüz bellek eşlemeli (memory-mapped) registerlara sahip.
// 3. USB Yığın Depolama Sınıfı (MSC) aygıtı ile iletişim kurulacak.
// 4. Temel seviyede hata yönetimi (örnek amaçlı).

// ************************************************************************
// DONANIM BAĞIMLI BÖLÜM (Aşağıdaki değerler ve yapılar donanımınıza göre değişir!)
// ************************************************************************

// USB Kontrolcü Register Adresleri (ÖRNEK DEĞERLER, DOĞRU DEĞİLLER!)
mod usb_registers {
    pub const USB_CONTROLLER_BASE: u32 = 0xABC00000; // USB kontrolcü temel adresi (DONANIMINIZA GÖRE DEĞİŞİR!)

    // Örnek kontrol registerları (DONANIMINIZA GÖRE DEĞİŞİR!)
    pub const USB_CONTROL_REG: u32 = USB_CONTROLLER_BASE + 0x00;
    pub const USB_STATUS_REG: u32 = USB_CONTROLLER_BASE + 0x04;
    pub const USB_ENDPOINT_CONTROL_REG: u32 = USB_CONTROLLER_BASE + 0x08;
    pub const USB_ENDPOINT_STATUS_REG: u32 = USB_CONTROLLER_BASE + 0x0C;
    // ... diğer registerlar ...
}

// USB Endpoint Tanımları (ÖRNEK DEĞERLER)
mod usb_endpoints {
    pub const CONTROL_ENDPOINT: u8 = 0;   // Kontrol endpoint (Endpoint 0 genellikle kontrol endpointidir)
    pub const BULK_IN_ENDPOINT: u8 = 1;   // Veri almak için bulk endpoint (DONANIMINIZA GÖRE BULK IN endpoint numarasını bulun!)
    pub const BULK_OUT_ENDPOINT: u8 = 2;  // Veri göndermek için bulk endpoint (DONANIMINIZA GÖRE BULK OUT endpoint numarasını bulun!)
}

// ************************************************************************
// USB PROTOKOLÜ İLE İLGİLİ SABİTLER VE YAPILAR (Temel MSC için yeterli)
// ************************************************************************

mod usb_protocol_constants {
    // USB Standart İstek Kodları
    pub const USB_REQUEST_GET_DESCRIPTOR: u8 = 0x06;
    pub const USB_REQUEST_SET_ADDRESS: u8 = 0x05;
    pub const USB_REQUEST_SET_CONFIGURATION: u8 = 0x09;

    // USB Descriptor Tipleri
    pub const USB_DESCRIPTOR_TYPE_DEVICE: u8 = 0x01;
    pub const USB_DESCRIPTOR_TYPE_CONFIGURATION: u8 = 0x02;
    pub const USB_DESCRIPTOR_TYPE_STRING: u8 = 0x03;
    pub const USB_DESCRIPTOR_TYPE_INTERFACE: u8 = 0x04;
    pub const USB_DESCRIPTOR_TYPE_ENDPOINT: u8 = 0x05;

    // MSC Sınıfına Özel İstekler (CBW/CSW için komut blokları)
    pub const USB_MSC_REQUEST_RESET: u8 = 0xFF;
    pub const USB_MSC_REQUEST_GET_MAX_LUN: u8 = 0xFE;

    // MSC Komut Kodları (SCSI komutları)
    pub const MSC_CMD_TEST_UNIT_READY: u8 = 0x00;
    pub const MSC_CMD_REQUEST_SENSE: u8 = 0x03;
    pub const MSC_CMD_INQUIRY: u8 = 0x12;
    pub const MSC_CMD_MODE_SENSE_6: u8 = 0x1A;
    pub const MSC_CMD_READ_FORMAT_CAPACITIES: u8 = 0x23;
    pub const MSC_CMD_READ_CAPACITY_10: u8 = 0x25;
    pub const MSC_CMD_READ_10: u8 = 0x28;
    pub const MSC_CMD_WRITE_10: u8 = 0x2A;
    pub const MSC_CMD_WRITE_VERIFY_10: u8 = 0x2E;
    pub const MSC_CMD_VERIFY_10: u8 = 0x2F;
    pub const MSC_CMD_MODE_SELECT_6: u8 = 0x15;
    pub const MSC_CMD_START_STOP_UNIT: u8 = 0x1B;

    // MSC Durum Kodları
    pub const MSC_STATUS_GOOD: u8 = 0x00;
    pub const MSC_STATUS_CHECK_CONDITION: u8 = 0x01;
    pub const MSC_STATUS_COMMAND_FAILED: u8 = 0x02;

    // CBW/CSW İşaretleri (Flags)
    pub const CBW_DIRECTION_OUT: u8 = 0x00; // Hosttan cihaza veri gönderme (OUT)
    pub const CBW_DIRECTION_IN: u8 = 0x80;  // Cihazdan hosta veri alma (IN)
}

use usb_protocol_constants::*;

// Command Block Wrapper (CBW) yapısı
#[repr(C, packed)]
struct CommandBlockWrapper {
    signature: u32,        // CBW İşareti (0x43425355 - "USBCBW")
    tag: u32,              // Etiket (Host tarafından oluşturulan, CSW ile eşleşir)
    data_transfer_length: u32, // Veri transfer uzunluğu (bayt cinsinden)
    flags: u8,             // İşaretler (yön bilgisi - IN/OUT)
    lun: u8,               // Logical Unit Number (LUN) - Genellikle 0
    command_block_length: u8, // Komut bloğu uzunluğu (1 ila 16 bayt)
    command_block: [u8; 16],  // Komut bloğu (SCSI komutu ve parametreleri)
}

impl CommandBlockWrapper {
    pub fn new() -> Self {
        CommandBlockWrapper {
            signature: 0x43425355, // "USBCBW"
            tag: 0x12345678,       // Örnek etiket (her CBW için farklı olabilir)
            data_transfer_length: 0,
            flags: 0,
            lun: 0,
            command_block_length: 0,
            command_block: [0u8; 16],
        }
    }
}


// Command Status Wrapper (CSW) yapısı
#[repr(C, packed)]
struct CommandStatusWrapper {
    signature: u32,        // CSW İşareti (0x53425355 - "USBCSW")
    tag: u32,              // Etiket (CBW'deki etiketle aynı olmalı)
    data_residue: u32,     // Kalan veri uzunluğu (beklenen - aktarılan)
    status: u8,            // Komut durumu (MSC_STATUS_GOOD, vb.)
}

impl CommandStatusWrapper {
    pub fn new() -> Self {
        CommandStatusWrapper {
            signature: 0x53425355, // "USBCSW"
            tag: 0,
            data_residue: 0,
            status: 0,
        }
    }
}


// ************************************************************************
// ÇEKİRDEK SEVİYESİ USB SÜRÜCÜ KODU (srcio_arm.rs'nin içeriği olabilir)
// ************************************************************************

// Okuma/yazma işlemleri için kullanılacak temel arabellek (kernel alanında)
static mut IO_BUFFER: [u8; 512] = [0u8; 512]; // Örnek 512 baytlık arabellek (sektör boyutu)

// Registerlara güvenli erişim için yardımcı fonksiyon (volatile okuma/yazma)
unsafe fn read_register(address: u32) -> u32 {
    (address as *mut u32).read_volatile()
}

unsafe fn write_register(address: u32, value: u32) {
    (address as *mut u32).write_volatile(value);
}


// USB kontrolcüsünü başlatma fonksiyonu (DONANIMA ÖZEL!)
fn usb_controller_init() {
    // TODO: DONANIMINIZA ÖZEL USB KONTROLCÜ BAŞLATMA KODUNU BURAYA YAZIN!
    // Örnek: USB kontrolcüyü etkinleştirme, endpointleri yapılandırma, vb.
    unsafe {
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
        write_register(usb_registers::USB_CONTROL_REG, 0x01); // USB kontrolcüyü etkinleştir
        // ... diğer başlatma adımları ...
    }
    kprintln!("USB kontrolcüsü başlatıldı.");
}


// USB aygıtını algılama fonksiyonu (BASİT ALGILAMA, GELİŞTİRİLEBİLİR)
fn usb_device_detect() -> bool {
    // TODO: USB aygıtı algılama mantığını buraya ekleyin.
    // Örnek: USB durum registerlarını kontrol etme, aygıt bağlantı durumu, vb.
    unsafe {
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
        let status = read_register(usb_registers::USB_STATUS_REG);
        if (status & 0x01) != 0 { // Örnek: Aygıt bağlı bitini kontrol et (0x01 biti)
            kprintln!("USB aygıtı algılandı.");
            return true;
        } else {
            kprintln!("USB aygıtı algılanamadı.");
            return false;
        }
    }
}


// Kontrol endpoint üzerinden USB kontrol isteği gönderme (Örn: Descriptor alma, adres ayarlama)
fn usb_control_request(request_type: u8, request: u8, value: u16, index: u16, length: u16, data_buffer: *mut u8) -> bool {
    // TODO: Kontrol endpoint üzerinden USB kontrol isteği gönderme mantığını buraya ekleyin.
    // Bu fonksiyon, SETUP paketini oluşturup kontrol endpoint'e göndermeli, veri transferini yönetmeli ve STATUS aşamasını tamamlamalıdır.
    kprintln!("USB kontrol isteği gönderiliyor (request: {:x})", request);

    // **BASİTLEŞTİRİLMİŞ ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**
    // Gerçek bir uygulamada, SETUP paketini doğru formatta oluşturmanız,
    // kontrol endpoint'e göndermeniz (donanım registerlarını kullanarak),
    // veri transferini yönetmeniz (IN/OUT), ve STATUS aşamasını işlemeniz gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    if request == USB_REQUEST_GET_DESCRIPTOR {
        // Descriptor alma örneği (BASİTLEŞTİRİLMİŞ!)
        if length > 0 && !data_buffer.is_null() {
            unsafe {
                // ÖRNEK DESCRIPTOR VERİSİ (GERÇEK DESCRIPTOR CİHAZDAN ALINMALIDIR!)
                let descriptor_data: [u8; 18] = [ // Örnek Cihaz Descriptor'ı (18 bayt)
                    0x12,       // bLength
                    USB_DESCRIPTOR_TYPE_DEVICE, // bDescriptorType (Device)
                    0x00, 0x02, // bcdUSB (USB 2.0)
                    0x00,       // bDeviceClass (Aygıt sınıfı yok, arayüzler sınıfı belirtir)
                    0x00,       // bDeviceSubClass
                    0x00,       // bDeviceProtocol
                    0x40,       // bMaxPacketSize0 (Endpoint 0 için maksimum paket boyutu - 64 bayt)
                    0x00, 0x12, // idVendor (Örnek Vendor ID)
                    0x34, 0x56, // idProduct (Örnek Product ID)
                    0x00, 0x01, // bcdDevice (Aygıt sürümü 1.0)
                    0x01,       // iManufacturer (Üretici string indexi)
                    0x02,       // iProduct (Ürün string indexi)
                    0x03,       // iSerialNumber (Seri numarası string indexi)
                    0x01        // bNumConfigurations (Yapılandırma sayısı - 1)
                ];
                // Örnek olarak descriptor verisini arabelleğe kopyala (GERÇEKTE USB'DEN OKUNMALIDIR!)
                for i in 0..length.min(descriptor_data.len() as u16) {
                    *data_buffer.add(i as usize) = descriptor_data[i as usize];
                }
            }
            kprintln!("Descriptor alındı (Örnek veri kullanıldı!)");
            return true; // Başarılı (ÖRNEK!)
        }
    } else if request == USB_REQUEST_SET_ADDRESS {
        // Adres ayarlama örneği (BASİTLEŞTİRİLMİŞ!)
        kprintln!("USB Adresi ayarlandı: {}", value);
        // TODO: USB kontrolcü registerlarına aygıt adresini yazma (DONANIMA ÖZEL!)
        return true; // Başarılı (ÖRNEK!)
    } else if request == USB_REQUEST_SET_CONFIGURATION {
        // Yapılandırma ayarlama örneği (BASİTLEŞTİRİLMİŞ!)
        kprintln!("USB Yapılandırması ayarlandı: {}", value);
        // TODO: Yapılandırma ayarlandıktan sonra yapılması gerekenler (endpointleri etkinleştirme, vb.) (DONANIMA ÖZEL!)
        return true; // Başarılı (ÖRNEK!)
    }

    kprintln!("USB Kontrol İsteği BAŞARISIZ (request: {:x})", request);
    false // Başarısız (ÖRNEK!)
}


// USB aygıtını yapılandırma (temel yapılandırma adımları)
fn usb_configure_device() -> bool {
    kprintln!("USB aygıtı yapılandırılıyor...");

    let mut descriptor_buffer: [u8; 256] = [0u8; 256]; // Descriptor verisi için arabellek

    // 1. Device Descriptor'ı al
    if !usb_control_request(0x80, USB_REQUEST_GET_DESCRIPTOR, (USB_DESCRIPTOR_TYPE_DEVICE as u16) << 8, 0, 18, descriptor_buffer.as_mut_ptr()) {
        kprintln!("Device Descriptor alınamadı!");
        return false;
    }
    kprintln!("Device Descriptor alındı.");
    // TODO: Device Descriptor'ı ayrıştırıp gerekli bilgileri kullanabilirsiniz.

    // 2. Adresi ayarla (genellikle 7 adresi kullanılır - keyfi seçim)
    if !usb_control_request(0x00, USB_REQUEST_SET_ADDRESS, 7, 0, 0, core::ptr::null_mut()) {
        kprintln!("Adres ayarlanamadı!");
        return false;
    }
    kprintln!("Adres ayarlandı.");
    // Aygıt adresi ayarlandıktan sonra, sonraki kontrol istekleri için adresli endpoint (endpoint 0 hala adres 0'da kalır) kullanılmalıdır.
    // Ancak bu örnekte basitlik için adres kullanımı atlanmıştır.

    // 3. Configuration Descriptor'ı al (ve tüm ilgili interface ve endpoint descriptor'larını)
    if !usb_control_request(0x80, USB_REQUEST_GET_DESCRIPTOR, (USB_DESCRIPTOR_TYPE_CONFIGURATION as u16) << 8, 0, 9, descriptor_buffer.as_mut_ptr()) { // İlk 9 baytı al (Configuration Descriptor başlığı)
        kprintln!("Configuration Descriptor başlığı alınamadı!");
        return false;
    }
    let config_descriptor_length = descriptor_buffer[2] as u16 | ((descriptor_buffer[3] as u16) << 8); // Toplam uzunluğu descriptor'dan oku
    if !usb_control_request(0x80, USB_REQUEST_GET_DESCRIPTOR, (USB_DESCRIPTOR_TYPE_CONFIGURATION as u16) << 8, 0, config_descriptor_length, descriptor_buffer.as_mut_ptr()) { // Tam Configuration Descriptor'ı al
        kprintln!("Configuration Descriptor'ın tamamı alınamadı!");
        return false;
    }
    kprintln!("Configuration Descriptor alındı (Tamamı). Uzunluk: {}", config_descriptor_length);
    // TODO: Configuration Descriptor'ı ve alt descriptor'ları ayrıştırıp endpoint bilgilerini çıkarın ve yapılandırın.
    // Özellikle BULK IN ve BULK OUT endpoint numaralarını ve adreslerini belirleyin.

    // 4. Yapılandırmayı ayarla (Genellikle 1. yapılandırma kullanılır)
    if !usb_control_request(0x00, USB_REQUEST_SET_CONFIGURATION, 1, 0, 0, core::ptr::null_mut()) {
        kprintln!("Yapılandırma ayarlanamadı!");
        return false;
    }
    kprintln!("Yapılandırma ayarlandı.");

    kprintln!("USB aygıtı yapılandırması tamamlandı.");
    true
}


// MSC Reset isteği gönderme
fn msc_reset() -> bool {
    kprintln!("MSC Reset isteği gönderiliyor...");
    // MSC Reset isteği kontrol endpoint üzerinden sınıf-özel bir istektir.
    if usb_control_request(0x21, USB_MSC_REQUEST_RESET, 0, 0, 0, core::ptr::null_mut()) { // Interface isteği (0x21), Alıcı Arayüzü (Interface)
        kprintln!("MSC Reset isteği gönderildi.");
        return true;
    } else {
        kprintln!("MSC Reset isteği BAŞARISIZ!");
        return false;
    }
}

// Maksimum LUN sayısını alma isteği
fn msc_get_max_lun() -> u8 {
    kprintln!("Maksimum LUN sayısı alınıyor...");
    let mut lun_buffer: [u8; 1] = [0];
    if usb_control_request(0xA1, USB_MSC_REQUEST_GET_MAX_LUN, 0, 0, 1, lun_buffer.as_mut_ptr()) { // Aygıttan veri alımı (0xA1), Alıcı Arayüzü (Interface)
        let max_lun = lun_buffer[0];
        kprintln!("Maksimum LUN sayısı alındı: {}", max_lun);
        return max_lun;
    } else {
        kprintln!("Maksimum LUN sayısı alınamadı! Varsayılan 0 kullanılıyor.");
        return 0; // Varsayılan olarak 0 LUN kabul et
    }
}


// CBW gönderme fonksiyonu (Bulk-OUT endpoint'e)
fn msc_send_cbw(cbw: &CommandBlockWrapper) -> bool {
    // TODO: Bulk-OUT endpoint'e CBW gönderme mantığını buraya ekleyin.
    // Bu fonksiyon, CBW yapısını bellekte oluşturmalı ve Bulk-OUT endpoint'e donanım registerları aracılığıyla göndermelidir.
    kprintln!("CBW gönderiliyor. Komut: {:x}", cbw.command_block[0]);

    // **BASİTLEŞTİRİLMİŞ ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**
    // Gerçek bir uygulamada, CBW yapısını bellekte doğru şekilde oluşturmanız,
    // Bulk-OUT endpoint'e göndermeniz (donanım registerlarını kullanarak),
    // veri transferini yönetmeniz ve gerekli durum kontrollerini yapmanız gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    unsafe {
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
        // Varsayım: Bulk-OUT endpoint'e veri yazma registerı (DONANIMA GÖRE DEĞİŞİR!)
        let bulk_out_data_reg = usb_registers::USB_CONTROLLER_BASE + 0x10; // Örnek adres
        let cbw_ptr = cbw as *const CommandBlockWrapper as *const u8; // CBW yapısının byte pointerı

        for i in 0..31 { // CBW boyutu (31 bayt)
            write_register(bulk_out_data_reg, *cbw_ptr.add(i) as u32); // Her baytı registera yaz
        }
    }
    kprintln!("CBW gönderildi (Örnek veri gönderme kullanıldı!)");
    true // Başarılı (ÖRNEK!)
}


// CSW alma fonksiyonu (Bulk-IN endpoint'ten)
fn msc_receive_csw(csw: &mut CommandStatusWrapper) -> bool {
    // TODO: Bulk-IN endpoint'ten CSW alma mantığını buraya ekleyin.
    // Bu fonksiyon, Bulk-IN endpoint'ten veri okumalı ve CSW yapısını doldurmalıdır.
    kprintln!("CSW bekleniyor...");

    // **BASİTLEŞTİRİLMİŞ ÖRNEK (GERÇEK KOD ÇOK DAHA KARMAŞIK)**
    // Gerçek bir uygulamada, Bulk-IN endpoint'ten veri okumanız (donanım registerlarını kullanarak),
    // CSW yapısını gelen verilere göre doldurmanız ve gerekli durum kontrollerini yapmanız gerekir.
    // Hata yönetimi ve zaman aşımları da dikkate alınmalıdır.

    unsafe {
        // ÖRNEK KOD (DOĞRU DEĞİL! DONANIMINIZA GÖRE DEĞİŞTİRİN!)
        // Varsayım: Bulk-IN endpoint'ten veri okuma registerı (DONANIMA GÖRE DEĞİŞİR!)
        let bulk_in_data_reg = usb_registers::USB_CONTROLLER_BASE + 0x20; // Örnek adres
        let csw_ptr = csw as *mut CommandStatusWrapper as *mut u8; // CSW yapısının byte pointerı

        for i in 0..13 { // CSW boyutu (13 bayt)
            *csw_ptr.add(i) = read_register(bulk_in_data_reg) as u8; // Her baytı registerdan oku
        }
    }
    kprintln!("CSW alındı (Örnek veri alma kullanıldı!). Durum: {:x}", csw.status);
    true // Başarılı (ÖRNEK!)
}


// MSC komutu gönderme ve CSW alma (Temel MSC işlem akışı)
fn msc_send_command(cbw: &mut CommandBlockWrapper, csw: &mut CommandStatusWrapper) -> bool {
    if !msc_send_cbw(cbw) {
        kprintln!("CBW gönderme başarısız!");
        return false;
    }
    if !msc_receive_csw(csw) {
        kprintln!("CSW alma başarısız!");
        return false;
    }

    if csw.status == MSC_STATUS_GOOD {
        kprintln!("MSC Komutu başarılı. Durum: GOOD");
        return true;
    } else if csw.status == MSC_STATUS_CHECK_CONDITION {
        kprintln!("MSC Komutu durumu: CHECK CONDITION. Sense isteği gönderilmeli.");
        // TODO: Sense isteği göndererek detaylı hata bilgisini alabilirsiniz (örnek dışı bırakıldı).
        return false; // CHECK CONDITION durumunda genellikle hata kabul edilir.
    } else {
        kprintln!("MSC Komutu BAŞARISIZ. Durum: {:x}", csw.status);
        return false;
    }
}


// MSC Test Unit Ready komutu (Aygıt hazır mı kontrolü)
fn msc_test_unit_ready() -> bool {
    kprintln!("Test Unit Ready komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new();
    cbw.flags = CBW_DIRECTION_IN; // Veri yok, durum bilgisini alacağız (IN durumu)
    cbw.command_block_length = 6;
    cbw.command_block[0] = MSC_CMD_TEST_UNIT_READY; // Test Unit Ready komutu

    let mut csw = CommandStatusWrapper::new();
    msc_send_command(&mut cbw, &mut csw)
}

// MSC Inquiry komutu (Aygıt bilgilerini alma)
fn msc_inquiry() -> bool {
    kprintln!("Inquiry komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new();
    cbw.flags = CBW_DIRECTION_IN; // Veri alacağız (IN)
    cbw.data_transfer_length = 36; // Inquiry verisi için beklenen uzunluk (standart)
    cbw.command_block_length = 6;
    cbw.command_block[0] = MSC_CMD_INQUIRY; // Inquiry komutu
    cbw.command_block[4] = 36; // Allocation Length (maksimum veri uzunluğu)

    let mut csw = CommandStatusWrapper::new();
    if msc_send_command(&mut cbw, &mut csw) {
        if csw.data_residue == 0 {
            kprintln!("Inquiry verisi alındı (örnek).");
            unsafe {
                // TODO: Bulk-IN endpoint'ten Inquiry verisini okuma (örnek dışı bırakıldı)
                // Veriyi IO_BUFFER'e okuyabilirsiniz.
                // Daha sonra IO_BUFFER'dan Inquiry verisini ayrıştırıp kullanabilirsiniz.
                kprintln!("Inquiry verisi (ilk 8 bayt - ÖRNEK):");
                for i in 0..8 {
                    kprint!("{:02x} ", IO_BUFFER[i]);
                }
                kprintln!("");
            }
            return true;
        } else {
            kprintln!("Inquiry verisi beklenenden farklı uzunlukta!");
            return false;
        }
    } else {
        kprintln!("Inquiry komutu başarısız!");
        return false;
    }
}


// MSC Read Capacity (10) komutu (Kapasite bilgisini alma)
fn msc_read_capacity_10() -> bool {
    kprintln!("Read Capacity (10) komutu gönderiliyor...");
    let mut cbw = CommandBlockWrapper::new();
    cbw.flags = CBW_DIRECTION_IN; // Veri alacağız (IN)
    cbw.data_transfer_length = 8;  // Read Capacity (10) verisi 8 bayt uzunluğunda
    cbw.command_block_length = 10;
    cbw.command_block[0] = MSC_CMD_READ_CAPACITY_10; // Read Capacity (10) komutu

    let mut csw = CommandStatusWrapper::new();
    if msc_send_command(&mut cbw, &mut csw) {
        if csw.data_residue == 0 {
            kprintln!("Read Capacity (10) verisi alındı (örnek).");
            unsafe {
                // TODO: Bulk-IN endpoint'ten Read Capacity verisini okuma (örnek dışı bırakıldı)
                // Veriyi IO_BUFFER'e okuyabilirsiniz.
                // Daha sonra IO_BUFFER'dan kapasite bilgisini ayrıştırıp kullanabilirsiniz.
                kprintln!("Read Capacity (10) verisi (baytlar 0-7 - ÖRNEK):");
                for i in 0..8 {
                    kprint!("{:02x} ", IO_BUFFER[i]);
                }
                kprintln!("");
            }
            return true;
        } else {
            kprintln!("Read Capacity (10) verisi beklenenden farklı uzunlukta!");
            return false;
        }
    } else {
        kprintln!("Read Capacity (10) komutu başarısız!");
        return false;
    }
}

// MSC Read (10) komutu (Sektör okuma)
fn msc_read_10(lba: u32, block_size: u16, block_count: u16) -> bool {
    kprintln!("Read (10) komutu gönderiliyor. LBA: {}, Blok Sayısı: {}, Blok Boyutu: {}", lba, block_count, block_size);
    let transfer_length = block_size as u32 * block_count as u32; // Toplam transfer uzunluğu
    let mut cbw = CommandBlockWrapper::new();
    cbw.flags = CBW_DIRECTION_IN; // Veri alacağız (IN)
    cbw.data_transfer_length = transfer_length; // Okunacak toplam veri uzunluğu
    cbw.command_block_length = 10;
    cbw.command_block[0] = MSC_CMD_READ_10; // Read (10) komutu
    // LBA (Logical Block Address) ve block_count'u command block'a yerleştirme (Büyük-endian)
    cbw.command_block[2] = (lba >> 24) as u8;
    cbw.command_block[3] = (lba >> 16) as u8;
    cbw.command_block[4] = (lba >> 8) as u8;
    cbw.command_block[5] = lba as u8;
    cbw.command_block[7] = (block_count >> 8) as u8;
    cbw.command_block[8] = block_count as u8;

    let mut csw = CommandStatusWrapper::new();
    if msc_send_command(&mut cbw, &mut csw) {
        if csw.data_residue == 0 {
            kprintln!("Read (10) komutu başarılı. Veri alımı bekleniyor ({} bayt).", transfer_length);
            unsafe {
                // TODO: Bulk-IN endpoint'ten veri okuma (örnek dışı bırakıldı)
                // Transfer_length kadar veriyi Bulk-IN endpoint'ten okuyup IO_BUFFER'e veya başka bir arabelleğe yazmanız gerekir.
                kprintln!("Okunan veri (ilk 16 bayt - ÖRNEK):"); // Sadece ilk 16 baytı örnek olarak gösteriyoruz
                for i in 0..16.min(transfer_length as usize) {
                    kprint!("{:02x} ", IO_BUFFER[i]);
                }
                kprintln!(" ...");
            }
            return true;
        } else {
            kprintln!("Read (10) verisi beklenenden farklı uzunlukta!");
            return false;
        }
    } else {
        kprintln!("Read (10) komutu başarısız!");
        return false;
    }
}


// ************************************************************************
// ÇEKİRDEK GİRİŞ NOKTASI ve TEST KODU (Örnek amaçlı)
// ************************************************************************

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    kprintln!("srcio_arm.rs çekirdek örneği başladı!");

    usb_controller_init(); // USB kontrolcüsünü başlat
    if usb_device_detect() { // USB aygıtı algılandı mı?
        if usb_configure_device() { // USB aygıtını yapılandır
            kprintln!("USB aygıtı başarıyla yapılandırıldı.");

            if msc_reset() { // MSC Reset gönder
                kprintln!("MSC Reset gönderildi.");
                if msc_get_max_lun() >= 0 { // Maksimum LUN sayısını al (veya 0 kabul et)
                    if msc_test_unit_ready() { // Test Unit Ready komutu gönder
                        kprintln!("MSC Aygıt Hazır.");
                        if msc_inquiry() { // Inquiry komutu gönder (aygıt bilgisi al)
                            if msc_read_capacity_10() { // Read Capacity (10) komutu gönder (kapasite al)
                                // Örnek olarak ilk sektörü okuma (LBA 0, 1 sektör, 512 bayt sektör boyutu varsayımı)
                                if msc_read_10(0, 512, 1) {
                                    kprintln!("Sektör okuma başarılı! (LBA 0)");
                                    // TODO: Okunan sektörü (IO_BUFFER) işleyebilirsiniz.
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
                        kprintln!("MSC Aygıt HAZIR DEĞİL!");
                    }
                }
            } else {
                kprintln!("MSC Reset BAŞARISIZ!");
            }
        } else {
            kprintln!("USB aygıtı yapılandırma BAŞARISIZ!");
        }
    }

    kprintln!("srcio_arm.rs çekirdek örneği sona erdi.");
    loop {} // Sonsuz döngü (çekirdek sonlanmamalı)
}


// Çekirdek println! makrosu (çok basit, çekirdek seviyesi için)
macro_rules! kprintln {
    () => (kprint!("\r\n"));
    ($($arg:tt)*) => (kprint!("{}\r\n", format_args!($($arg)*)));
}

// Çekirdek print! makrosu (çok basit, çekirdek seviyesi için - UART/Konsol varsayımı)
macro_rules! kprint {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = KernelWriter;
        writer.write_fmt(format_args!($($arg)*)).unwrap();
    });
}

// Çekirdek Yazıcı yapısı (UART/Konsola yazmak için basit bir yapı)
struct KernelWriter;

impl core::fmt::Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // TODO: Gerçek donanım UART/Konsol yazma fonksiyonunuzu buraya çağırın!
        // Bu kısım donanıma özel olmalı. Örnek olarak basit bir karakter döngüsü:
        for byte in s.bytes() {
            unsafe {
                // ÖRNEK UART VERI REGISTER ADRESI (DONANIMINIZA GÖRE DEĞİŞİR!)
                let uart_data_register = 0xFFF00000 as *mut u8; // Örnek adres
                uart_data_register.write_volatile(byte); // Karakteri UART'a yaz
            }
        }
        Ok(())
    }
}


// Panik işleyici (çekirdek panik durumunda çağrılır)
use core::panic::PanicInfo;
#[panic]
fn panic(_info: &PanicInfo) -> ! {
    kprintln!("KERNEL PANIC!");
    kprintln!("{}", _info);
    loop {}
}


// main fonksiyonunu kapat (no_main kullandığımız için gerekli)
#[no_mangle]
pub extern "C" fn main() {
    // kernel_main fonksiyonunu çağır (çekirdek giriş noktası)
    kernel_main();
}