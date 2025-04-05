#![no_std]
#![crate_type = "staticlib"]
#![allow(dead_code)] // Şimdilik kullanılmayan kodlara izin ver

// Bu dosya srcio_openrısc.rs

// --- Sabitler ve Yapılandırmalar ---

// USB Ana Makine Denetleyici (Host Controller) Temel Adresi
// Bu adres, hedef donanımınıza (kendi çekirdeğinizin çalıştığı platform) göre AYARLANMALIDIR.
// Örnek olarak, yaygın bir EHCI (Enhanced Host Controller Interface) denetleyicisi için bir adres aralığı:
const USB_CONTROLLER_BASE_ADDRESS: usize = 0xFEDC_0000; // ÖRNEK DEĞER!

// USB Denetleyici Kayıt偏移leri (Offset) - Bunlar denetleyiciye özgüdür ve
// veri sayfalarından (datasheet) alınmalıdır. ÖRNEK değerlerdir.
const USB_HC_VERSION_OFFSET: usize = 0x000;
const USB_HC_CAPLENGTH_OFFSET: usize = 0x004;
const USB_HC_CONTROL_OFFSET: usize = 0x040;
const USB_HC_STATUS_OFFSET: usize = 0x044;
const USB_HC_COMMAND_OFFSET: usize = 0x048;
const USB_HC_INTERRUPT_ENABLE_OFFSET: usize = 0x04C;
const USB_HC_INTERRUPT_DISABLE_OFFSET: usize = 0x050;
const USB_HC_CONFIG_OFFSET: usize = 0x054;
const USB_HC_CURRENT_FRAME_INDEX_OFFSET: usize = 0x058;

// USB Cihaz Tanımlayıcı (Device Descriptor) Uzunluğu (tipik olarak 18 bayt)
const USB_DEVICE_DESCRIPTOR_SIZE: usize = 18;

// USB İsteği Türleri (bControlType için)
const USB_REQ_TYPE_STANDARD: u8 = 0x00;
const USB_REQ_TYPE_CLASS: u8 = 0x20;
const USB_REQ_TYPE_VENDOR: u8 = 0x40;
const USB_REQ_TYPE_RESERVED: u8 = 0x60;

// USB İsteği Alıcıları (bRecipient için)
const USB_REQ_RECIP_DEVICE: u8 = 0x00;
const USB_REQ_RECIP_INTERFACE: u8 = 0x01;
const USB_REQ_RECIP_ENDPOINT: u8 = 0x02;
const USB_REQ_RECIP_OTHER: u8 = 0x03;

// USB Standart İstek Kodları (bRequest için)
const USB_REQ_GET_STATUS: u8 = 0x00;
const USB_REQ_CLEAR_FEATURE: u8 = 0x01;
// const USB_REQ_SET_FEATURE: u8 = 0x03; // Tanımlanmamış - yorum satırı yapıldı
const USB_REQ_SET_ADDRESS: u8 = 0x05;
const USB_REQ_GET_DESCRIPTOR: u8 = 0x06;
const USB_REQ_SET_DESCRIPTOR: u8 = 0x07;
const USB_REQ_GET_CONFIGURATION: u8 = 0x08;
const USB_REQ_SET_CONFIGURATION: u8 = 0x09;
const USB_REQ_GET_INTERFACE: u8 = 0x0A;
const USB_REQ_SET_INTERFACE: u8 = 0x0B;
const USB_REQ_SYNCH_FRAME: u8 = 0x0C;

// USB Tanımlayıcı Tipleri (wValue'nun yüksek baytı)
const USB_DESC_TYPE_DEVICE: u16 = 0x0100;
const USB_DESC_TYPE_CONFIGURATION: u16 = 0x0200;
const USB_DESC_TYPE_STRING: u16 = 0x0300;
const USB_DESC_TYPE_INTERFACE: u16 = 0x0400;
const USB_DESC_TYPE_ENDPOINT: u16 = 0x0500;
const USB_DESC_TYPE_DEVICE_QUALIFIER: u16 = 0x0600;
const USB_DESC_TYPE_OTHER_SPEED_CONFIGURATION: u16 = 0x0700;
const USB_DESC_TYPE_INTERFACE_POWER: u16 = 0x0800;
const USB_DESC_TYPE_OTG: u16 = 0x0900;
const USB_DESC_TYPE_DEBUG: u16 = 0x0A00;
const USB_DESC_TYPE_INTERFACE_ASSOCIATION: u16 = 0x0B00;
const USB_DESC_TYPE_BOS: u16 = 0x0F00;
const USB_DESC_TYPE_DEVICE_CAPABILITY: u16 = 0x1000;
const USB_DESC_TYPE_SUPERSPEED_ENDPOINT_COMPANION: u16 = 0x3000;
const USB_DESC_TYPE_SUPERSPEED_CONFIGURATION: u16 = 0x0900; // Dikkat: Datasheet'te 0x0900 ve 0x3100 çakışması var! Kontrol et!
const USB_DESC_TYPE_SS_DEVICE_CAPABILITY: u16 = 0x3100;
const USB_DESC_TYPE_CONTAINER_ID: u16 = 0x4000;
const USB_DESC_TYPE_WIRELESS_ENDPOINT_COMPANION: u16 = 0x06; // Datasheet'te 0x06 ve 0x3200 çakışması var! Kontrol et!
const USB_DESC_TYPE_WIRELESS_INTERFACE_COMPANION: u16 = 0x3200;

// --- Yapılar ---

// USB Cihaz Tanımlayıcı Yapısı (Device Descriptor) - Örnek olarak temel alanlar
#[repr(C, packed)] // C uyumlu düzen ve paketlenmiş yapı
#[derive(Debug)] // Debug özelliği eklendi (derleme sırasında --cfg 'feature="debug"')
pub struct UsbDeviceDescriptor {
    bLength: u8,         // Tanımlayıcının boyutu (her zaman 18)
    bDescriptorType: u8,   // Tanımlayıcı tipi (Cihaz Tanımlayıcı için 0x01)
    bcdUSB: u16,          // USB spesifikasyonunun desteklenen sürümü (BCD formatında)
    bDeviceClass: u8,      // Cihaz sınıfı (0x00: arabirim tarafından tanımlanır, diğer sınıflar USB-IF tarafından tanımlanır)
    bDeviceSubClass: u8,   // Cihaz alt sınıfı
    bDeviceProtocol: u8,   // Cihaz protokolü
    bMaxPacketSize0: u8,   // 0 numaralı uç noktanın maksimum paket boyutu
    idVendor: u16,         // Üretici kimliği (VID)
    idProduct: u16,        // Ürün kimliği (PID)
    bcdDevice: u16,         // Cihaz sürüm numarası (BCD formatında)
    iManufacturer: u8,     // Üretici dizesi tanımlayıcı indeksi
    iProduct: u8,          // Ürün dizesi tanımlayıcı indeksi
    iSerialNumber: u8,     // Seri numarası dizesi tanımlayıcı indeksi
    bNumConfigurations: u8, // Olası konfigürasyon sayısı
}
// statik olarak boyut kontrolü (isteğe bağlı, derleme zamanında kontrol sağlar)
const _: () = assert!(core::mem::size_of::<UsbDeviceDescriptor>() == USB_DEVICE_DESCRIPTOR_SIZE);


// --- Fonksiyonlar ---

// Belleğe eşlenmiş G/Ç (MMIO) okuma fonksiyonu
// `address`: Okunacak bellek adresi
// `T`: Okunacak veri tipi (örn. u32, u16, u8)
unsafe fn mmio_read<T>(address: usize) -> T {
    (address as *mut T).read_volatile()
}

// Belleğe eşlenmiş G/Ç (MMIO) yazma fonksiyonu
// `address`: Yazılacak bellek adresi
// `value`: Yazılacak değer
unsafe fn mmio_write<T>(address: usize, value: T) {
    (address as *mut T).write_volatile(value);
}

// USB Denetleyici Sürümünü Okuma
pub fn read_hc_version() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_VERSION_OFFSET)
    }
}

// USB Denetleyici Yetenek Uzunluğunu Okuma
pub fn read_hc_caplength() -> u8 {
    unsafe {
        mmio_read::<u8>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CAPLENGTH_OFFSET) as u8 // İlk bayt yeterli
    }
}

// USB Denetleyici Kontrol Kaydını Okuma
pub fn read_hc_control() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CONTROL_OFFSET)
    }
}

// USB Denetleyici Durum Kaydını Okuma
pub fn read_hc_status() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_STATUS_OFFSET)
    }
}

// USB Denetleyici Komut Kaydını Okuma
pub fn read_hc_command() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_COMMAND_OFFSET)
    }
}

// USB Denetleyici Kesme Etkinleştirme Kaydını Okuma
pub fn read_hc_interrupt_enable() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_INTERRUPT_ENABLE_OFFSET)
    }
}

// USB Denetleyici Kesme Devre Dışı Bırakma Kaydını Okuma
pub fn read_hc_interrupt_disable() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_INTERRUPT_DISABLE_OFFSET)
    }
}

// USB Denetleyici Yapılandırma Kaydını Okuma
pub fn read_hc_config() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CONFIG_OFFSET)
    }
}

// USB Denetleyici Geçerli Çerçeve İndeksi Kaydını Okuma
pub fn read_hc_current_frame_index() -> u32 {
    unsafe {
        mmio_read::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CURRENT_FRAME_INDEX_OFFSET)
    }
}

// USB Denetleyici Kontrol Kaydına Yazma
pub fn write_hc_control(value: u32) {
    unsafe {
        mmio_write::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CONTROL_OFFSET, value);
    }
}

// USB Denetleyici Komut Kaydına Yazma
pub fn write_hc_command(value: u32) {
    unsafe {
        mmio_write::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_COMMAND_OFFSET, value);
    }
}

// USB Denetleyici Kesme Etkinleştirme Kaydına Yazma
pub fn write_hc_interrupt_enable(value: u32) {
    unsafe {
        mmio_write::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_INTERRUPT_ENABLE_OFFSET, value);
    }
}

// USB Denetleyici Kesme Devre Dışı Bırakma Kaydına Yazma
pub fn write_hc_interrupt_disable(value: u32) {
    unsafe {
        mmio_write::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_INTERRUPT_DISABLE_OFFSET, value);
    }
}

// USB Denetleyici Yapılandırma Kaydına Yazma
pub fn write_hc_config(value: u32) {
    unsafe {
        mmio_write::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CONFIG_OFFSET, value);
    }
}


// --- Yüksek Seviye Fonksiyonlar (Örnek olarak Cihaz Tanımlayıcı Okuma) ---

// USB Cihaz Tanımlayıcısını Okuma (Örnek Fonksiyon)
pub fn get_usb_device_descriptor(endpoint_address: u8) -> Option<UsbDeviceDescriptor> {
    let mut descriptor = UsbDeviceDescriptor { // Sıfır değerleri ile başlatma
        bLength: 0,
        bDescriptorType: 0,
        bcdUSB: 0,
        bDeviceClass: 0,
        bDeviceSubClass: 0,
        bDeviceProtocol: 0,
        bMaxPacketSize0: 0,
        idVendor: 0,
        idProduct: 0,
        bcdDevice: 0,
        iManufacturer: 0,
        iProduct: 0,
        iSerialNumber: 0,
        bNumConfigurations: 0,
    };

    let transfer_result = control_transfer_in(
        endpoint_address, // Uç nokta adresi (genellikle 0, kontrol uç noktası)
        USB_REQ_TYPE_STANDARD | USB_REQ_RECIP_DEVICE, // bRequestType: Standart cihaz isteği
        USB_REQ_GET_DESCRIPTOR, // bRequest: GET_DESCRIPTOR isteği
        USB_DESC_TYPE_DEVICE,   // wValue: Tanımlayıcı tipi (Cihaz Tanımlayıcı)
        0,                      // wIndex: Genellikle 0
        USB_DEVICE_DESCRIPTOR_SIZE as u16, // wLength: Okunacak boyut
        &mut descriptor as *mut UsbDeviceDescriptor as *mut u8, // Veri tamponu ve boyutu
        USB_DEVICE_DESCRIPTOR_SIZE,
    );

    if transfer_result.is_ok() {
        Some(descriptor)
    } else {
        None // Hata durumunda None döndür
    }
}


// --- Alt Seviye USB Kontrol Transfer Fonksiyonu (ÖRNEK - Gerçek Donanıma Göre Uyum Sağlanmalı) ---

// USB Kontrol 'IN' Transferi (Cihazdan veri okuma)
// **DİKKAT**: Bu fonksiyon ÇOK BASİT bir örnek iskelettir.
// Gerçek bir donanım sürücüsünde, USB ana makine denetleyicinizin
// (örn. EHCI, OHCI, xHCI) spesifikasyonlarına GÖRE UYARLANMALIDIR.
// Hata yönetimi, zaman aşımları, kesmeler vb. gibi birçok detay eksiktir.
fn control_transfer_in(
    endpoint_address: u8,
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    data_buffer: *mut u8,
    buffer_size: usize,
) -> Result<(), &'static str> {
    // 1. Komut Hazırlama (USB isteği kurulumu)
    //    Bu bölüm, USB isteğini (setup paketini) oluşturmayı ve
    //    ana makine denetleyicinin komut kuyruğuna (command queue) eklemeyi içerir.
    //    **DENETLEYİCİYE ÖZGÜ KOMUT FORMATI KULLANILMALIDIR.**
    //    Aşağıdaki kod TAMAMEN TEMSİLİDİR ve gerçek bir denetleyici ile ÇALIŞMAYACAKTIR.

    // **ÖRNEK BASİT YAKLAŞIM - GERÇEK KOD ÇOK DAHA KARMAŞIK OLUR.**
    //    Gerçekte, Descriptor Request Block (DRB) veya Transfer Descriptor (TD) gibi
    //    veri yapıları hazırlanır ve denetleyiciye MMIO üzerinden bildirilir.

    // **TEMSİLİ KOD BAŞLANGICI**
    let setup_packet: [u8; 8] = [ // USB Kontrol Transfer Setup Paketi (8 bayt)
        request_type,
        request,
        value as u8,        // wValue (low byte)
        (value >> 8) as u8, // wValue (high byte)
        index as u8,        // wIndex (low byte)
        (index >> 8) as u8, // wIndex (high byte)
        length as u8,       // wLength (low byte)
        (length >> 8) as u8, // wLength (high byte)
    ];

    // **TEMSİLİ: Denetleyici Komut Kaydına (ÖRNEK OFFSET) yazarak transferi başlat**
    unsafe {
        // **DİKKAT: BU TAMAMEN HAYALİ BİR YAZMA İŞLEMİDİR.**
        // Gerçekte, komut denetleyicinin bellek yapılarında (örn. TD listeleri) hazırlanır
        // ve denetleyiciye başlangıç adresi veya tetikleme sinyali verilir.
        // Örnek: write_hc_command(COMMAND_TRANSFER_START | /* ... diğer parametreler ... */);
        // Bu satır, GERÇEK DONANIM İÇİN UYGUN DEĞİLDİR ve DEĞİŞTİRİLMELİDİR.
        mmio_write::<u32>(USB_CONTROLLER_BASE_ADDRESS + USB_HC_COMMAND_OFFSET, 0x12345678); // ÖRNEK DEĞER!
    }

    // 2. Veri Transferi (Denetleyiciden veri okuma)
    //    Denetleyici, USB cihazından gelen veriyi belirli bir bellek bölgesine yazar.
    //    Bu bölgeden veriyi okumamız gerekir.
    //    **VERİ OKUMA MEKANİZMASI DENETLEYİCİYE GÖRE DEĞİŞİR.**
    //    Örneğin, DMA (Doğrudan Bellek Erişimi) kullanılabilir.

    // **TEMSİLİ KOD BAŞLANGICI**
    //    Burada, verinin BELİRLİ BİR ADRESTE (buffer_address - HAYALİ) olduğunu varsayıyoruz.
    let buffer_address: usize = 0xABCDEF00; // **TAMAMEN HAYALİ ADRES**

    unsafe {
        // **TEMSİLİ: Veriyi HAYALİ bellek adresinden al ve sağlanan tampona kopyala**
        //    Gerçekte, denetleyici DMA ile veriyi bir tampona yazmış olabilir.
        //    Doğrudan okuma veya DMA tamponundan kopyalama yapılabilir.
        //    Aşağıdaki döngü TEMSİLİDİR ve gerçek DMA veya veri alma mekanizmasını YANSITMAZ.
        for i in 0..buffer_size {
            let data_byte = mmio_read::<u8>(buffer_address + i); // **HAYALİ OKUMA**
            *data_buffer.add(i) = data_byte; // Veri tamponuna yaz
        }
    }

    // 3. Transfer Durumu Kontrolü ve Hata Yönetimi
    //    Transferin başarılı olup olmadığını, hata oluşup oluşmadığını kontrol et.
    //    **DURUM BİLGİSİ DENETLEYİCİ KAYITLARINDAN OKUNUR.**
    //    Zaman aşımları (timeouts) ve diğer hata durumları da işlenmelidir.

    // **TEMSİLİ KOD BAŞLANGICI**
    let status = read_hc_status(); // **TEMSİLİ DURUM OKUMA**
    if (status & 0x00000001) != 0 { // **TAMAMEN HAYALİ DURUM BİTİ KONTROLÜ**
        // Transfer başarılı oldu (ÖRNEK KONTROL)
        Ok(())
    } else {
        // Transfer başarısız oldu (ÖRNEK HATA)
        Err("USB Kontrol Transfer Hatası")
    }

    // **DİKKAT**: Bu fonksiyonun hata yönetimi, kesme işleme, zaman aşımları gibi
    // kritik kısımları ÇOK BASİTTİR ve GERÇEK BİR UYGULAMA İÇİN YETERSİZDİR.
    // Gerçek bir sürücüde bu bölümler çok daha detaylı ve sağlam olmalıdır.
}


// --- Çekirdek Giriş Noktası (Örnek - Kendi Çekirdeğinize Uygun Hale Getirin) ---

// Çekirdek modülü giriş fonksiyonu (kendi çekirdeğinize göre düzenleyin)
#[no_mangle] // İsim bozmayı engelle (linker için)
pub extern "C" fn init_module() -> i32 {
    // 1. USB Ana Makine Denetleyiciyi Başlatma

    // a. Denetleyiciyi Sıfırlama (varsa, denetleyiciye özgü sıfırlama prosedürü)
    //    Örneğin, EHCI için "HCHalted" biti temizlenmeli ve "Run/Stop" biti ayarlanmalıdır.
    //    write_hc_command(COMMAND_RESET); // **ÖRNEK - DENETLEYİCİYE ÖZEL SIFIRLAMA**

    // b. Denetleyiciyi Çalıştırma (Run) Moduna Alma
    //    write_hc_control(CONTROL_RUN_STOP); // **ÖRNEK - DENETLEYİCİ ÇALIŞTIRMA**

    // c. Gerekli Kesmeleri Etkinleştirme (isteğe bağlı, anketleme (polling) de kullanılabilir)
    //    write_hc_interrupt_enable(INTERRUPT_DEVICE_CONNECTION | INTERRUPT_TRANSFER_COMPLETION); // **ÖRNEK KESME ETKİNLEŞTİRME**


    // 2. USB Cihazlarını Tarama (Polling veya Kesmeler ile)

    // Örnek: Sürekli olarak USB cihaz bağlantı durumunu kontrol etme (polling)
    // loop {
    //     if cihaz_bagli_mi() { // **HAYALİ FONKSİYON - GERÇEKTE DONANIM KAYITLARINDAN OKUMA YAPILMALI**
    //         // Yeni bir USB cihazı bağlandı!
    //         handle_usb_device_connection(); // Cihazı işle
    //     }
    //     // Çekirdek zamanlayıcısı veya başka bir mekanizma ile uygun aralıklarla bekle
    // }


    // 3. Örnek USB İşlemi: Cihaz Tanımlayıcı Okuma (Sadece TEST AMAÇLI)
    if let Some(descriptor) = get_usb_device_descriptor(0) { // 0 numaralı uç noktadan oku (kontrol)
        // Başarılı şekilde tanımlayıcı okundu
        printk!("USB Cihaz Tanımlayıcı Okundu: {:?}\n", descriptor); // `printk!` - Kendi çekirdeğinizin çıktı fonksiyonu
    } else {
        printk!("USB Cihaz Tanımlayıcı Okuma HATASI!\n");
    }


    printk!("srcio_openrısc.rs: USB Sürücü Modülü Başlatıldı.\n"); // `printk!` - Kendi çekirdeğinizin çıktı fonksiyonu
    0 // Başarılı dönüş kodu
}

// Çekirdek modülü çıkış fonksiyonu (isteğe bağlı, kendi çekirdeğinize göre düzenleyin)
#[no_mangle]
pub extern "C" fn exit_module() {
    // USB denetleyiciyi durdurma, kesmeleri devre dışı bırakma, vb. (isteğe bağlı temizleme işlemleri)
    // write_hc_control(CONTROL_HALT); // **ÖRNEK - DENETLEYİCİ DURDURMA**

    printk!("srcio_openrısc.rs: USB Sürücü Modülü Çıkartıldı.\n"); // `printk!` - Kendi çekirdeğinizin çıktı fonksiyonu
}


// --- Yardımcı Fonksiyonlar (Çekirdeğinize Özel printk! Örneği) ---
// Bu bölümü kendi çekirdeğinizin çıktı mekanizmasına göre UYARLAYIN.

// Basit bir çekirdek çıktı fonksiyonu örneği (kendi çekirdeğinizde varsa onu kullanın)
#[cfg(feature = "debug")] // Sadece "debug" özelliği etkinse derlensin
macro_rules! printk {
    ($($arg:tt)*) => ({
        // **DİKKAT**: Bu ÇOK BASİT bir örnektir ve thread-safe (iş parçacığı güvenli) değildir.
        // Gerçek bir çekirdekte, çıktı mekanizması daha karmaşık ve güvenli olmalıdır.
        use core::fmt::Write;
        let mut serial_port = DummySerialPort; // **Kendi seri port yapınızı kullanın**
        core::fmt::write(&mut serial_port, format_args!($($arg)*)).unwrap();
    });
}

#[cfg(not(feature = "debug"))] // "debug" özelliği yoksa boş makro
macro_rules! printk {
    ($($arg:tt)*) => {{}}
}


// Örnek Dummy Seri Port Yapısı (Gerçekte kendi donanım çıktı mekanizmanızı kullanın)
struct DummySerialPort;

impl core::fmt::Write for DummySerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // **DİKKAT**: Bu sadece örnek bir çıktıdır. Gerçekte, donanım seri portuna veya
        // kendi çekirdeğinizin çıktı mekanizmasına yönlendirmeniz gerekir.
        // Örneğin, qemu debug portu, VGA ekran, veya loglama sistemi.
        for byte in s.bytes() {
            // **BURAYA DONANIM ÇIKTI KODUNUZU YAZIN**.
            // ÖRNEK: unsafe { volatile_write_byte(SERIAL_PORT_ADDRESS, byte); }
            // Şimdilik basitçe karakterleri kaybet
            let _ = byte; // Kullanılmayan değişken uyarısını engelle
        }
        Ok(())
    }
}


// --- Panik İşleyici (Zorunlu - no_std ortamda) ---
use core::panic::PanicInfo;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printk!("ÇEKİRDEK PANİK!\n");
    printk!("Konum: {}\n", info.location().unwrap());
    printk!("Mesaj: {:?}\n", info.message().unwrap());

    // **BURAYA PANİK ANINDA YAPILACAK İŞLEMLERİ EKLEYİN.**
    // Örneğin: Donanımı durdur, hata kodunu kaydet, yeniden başlatma, vb.
    loop {} // Sonsuz döngüde kal
}