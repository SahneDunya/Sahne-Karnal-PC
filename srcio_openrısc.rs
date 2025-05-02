#![no_std] // Standart kütüphaneye ihtiyacımız yok
#![crate_type = "staticlib"] // Bu dosya bir statik kütüphane olarak derlenecek
#![allow(dead_code)] // Geliştirme sırasında kullanılmayan kodlara izin ver

// Core kütüphanesinden gerekli öğeler
use core::ptr::{read_volatile, write_volatile}; // Volatile okuma/yazma için
use core::fmt::Write; // Yazma trait'i için (debug çıktısı için)
use core::slice; // Slice işlemleri için
use core::mem; // Bellek işlemleri için (örneğin size_of)


// 'volatile' krateri, bellek eşlemeli (memory-mapped) I/O için yapılandırılmış erişim sağlar.
// Doğrudan ham pointer kullanmak yerine, kayıtları struct olarak tanımlamak için tercih edilebilir.
use volatile::Volatile; // <-- Imported volatile crate


// Panik durumunda ne yapılacağını tanımlayın (kernelde standart kütüphane yok).
use core::panic::PanicInfo;


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


// --- Sabitler ve Yapılandırmalar ---

// USB Ana Makine Denetleyici (Host Controller) Temel Adresi
// Bu adres, hedef donanımınıza (kendi çekirdeğinizin çalıştığı OpenRISC platformu) göre AYARLANMALIDIR.
// OpenRISC'te I/O genellikle belirli bellek aralıklarına maplenir.
// Örnek olarak, yaygın bir EHCI (Enhanced Host Controller Interface) denetleyicisi için varsayımsal bir adres:
const USB_CONTROLLER_BASE_ADDRESS: usize = 0xFEDC_0000; // ÖRNEK DEĞER! OpenRISC Donanımınıza göre DEĞİŞTİRİLMELİ!

// USB Denetleyici Kayıt偏移leri (Offset) - Bunlar kullanılan Host Controller tipine (EHCI, OHCI, xHCI vb.) özgüdür ve
// denetleyicinin veri sayfalarından (datasheet) alınmalıdır. AŞAĞIDAKİLER EHCI ÖRNEĞİDİR.
const USB_HC_CAPLENGTH_OFFSET: usize = 0x000; // Capability Register Length (offset from CAPLENGTH)
const USB_HC_VERSION_OFFSET: usize = 0x002; // Host Controller Interface Version (offset from CAPLENGTH)

const USB_HC_STRUCTURAL_PARAMETERS_OFFSET: usize = 0x004; // HCSPARAMS
const USB_HC_CAPABILITY_PARAMETERS_OFFSET: usize = 0x008; // HCCPARAMS

// Operational Registers (offsets relative to Base Address + CAPLENGTH)
const USB_HC_COMMAND_OFFSET: usize = 0x040;  // USBCMD
const USB_HC_STATUS_OFFSET: usize = 0x044;  // USBSTS
const USB_HC_INTERRUPT_ENABLE_OFFSET: usize = 0x048; // USBINTREN
const USB_HC_PORT_STATUS_AND_CONTROL_OFFSET: usize = 0x04C; // PORTSC1 (for first port) - Subsequent ports are at +0x4 offset
const USB_HC_CONFIG_FLAG_OFFSET: usize = 0x050; // CONFIGFLAG
const USB_HC_PERIODIC_FRAME_LIST_BASE_OFFSET: usize = 0x054; // PRRDICLISTBASE
const USB_HC_ASYNC_LIST_BASE_OFFSET: usize = 0x058; // ASYNCLISTADDR

// EHCI USBCMD Register Bitleri (Örnek)
mod usb_cmd_bits {
    pub const RUN_STOP: u32 = 1 << 0; // 1: HC Çalışıyor, 0: HC Durdu
    pub const HC_RESET: u32 = 1 << 1; // 1: HC Resetle
    pub const FRAME_LIST_SIZE: u32 = 3 << 2; // Frame List Size Mask (00=1024, 01=512, 10=256)
    pub const PERIODIC_ENABLE: u32 = 1 << 4; // Periyodik Liste Etkinleştir
    pub const ASYNC_ENABLE: u32 = 1 << 5;  // Asenkron Liste Etkinleştir
    // ... diğer bitler
}
use usb_cmd_bits as uhci_cmd; // Kısa isim

// EHCI USBSTS Register Bitleri (Örnek)
mod usb_sts_bits {
    pub const HC_HALTED: u32 = 1 << 12; // 1: HC Durdu
    pub const PORT_CHANGE_DETECT: u32 = 1 << 2; // Port Durum Değişikliği Algılandı
    pub const USB_INTERRUPT: u32 = 1 << 0; // USB Tamamlama Kesmesi (Transaction Completed Interrupt - PCI)
    pub const ERROR_INTERRUPT: u32 = 1 << 1; // USB Hata Kesmesi (USB Error Interrupt - PCI)
    // ... diğer bitler
}
use usb_sts_bits as uhci_sts; // Kısa isim


// USB Cihaz Tanımlayıcı (Device Descriptor) Uzunluğu (tipik olarak 18 bayt)
const USB_DEVICE_DESCRIPTOR_SIZE: usize = 18;

// USB İsteği Türleri (bmRequestType baytının bit alanları)
 b7: Data Transfer Direction (0=Host to Device, 1=Device to Host)
 b6..5: Type (0=Standard, 1=Class, 2=Vendor, 3=Reserved)
 b4..0: Recipient (0=Device, 1=Interface, 2=Endpoint, 3=Other)
const USB_REQ_TYPE_STANDARD_DEVICE_IN: u8 = 0x80; // 1000 0000
const USB_REQ_TYPE_STANDARD_DEVICE_OUT: u8 = 0x00; // 0000 0000
const USB_REQ_TYPE_STANDARD_INTERFACE_IN: u8 = 0x81; // 1000 0001
const USB_REQ_TYPE_STANDARD_INTERFACE_OUT: u8 = 0x01; // 0000 0001
const USB_REQ_TYPE_STANDARD_ENDPOINT_IN: u8 = 0x82; // 1000 0010
const USB_REQ_TYPE_STANDARD_ENDPOINT_OUT: u8 = 0x02; // 0000 0010
// ... diğer tipler ...


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

// USB Tanımlayıcı Tipleri (wValue'nun yüksek baytı, GET_DESCRIPTOR isteği için)
const USB_DESC_TYPE_DEVICE_VAL: u16 = 0x0100;
const USB_DESC_TYPE_CONFIGURATION_VAL: u16 = 0x0200;
const USB_DESC_TYPE_STRING_VAL: u16 = 0x0300;
const USB_DESC_TYPE_INTERFACE_VAL: u16 = 0x0400;
const USB_DESC_TYPE_ENDPOINT_VAL: u16 = 0x0500;
// ... diğer tanımlayıcı tipleri ...


// --- Yapılar ---

// USB Cihaz Tanımlayıcı Yapısı (Device Descriptor) - Örnek olarak temel alanlar
#[repr(C, packed)] // C uyumlu düzen ve paketlenmiş yapı
#[derive(Debug)] // Debug özelliği eklendi (derleme sırasında --cfg 'feature="debug"')
pub struct UsbDeviceDescriptor {
    bLength: u8,         // Tanımlayıcının boyutu (her zaman 18)
    bDescriptorType: u8,   // Tanımlayıcı tipi (Cihaz Tanımlayıcı için 0x01)
    bcdUSB: u16,          // USB spesifikasyonunun desteklenen sürümü (BCD formatında) (Little-endian)
    bDeviceClass: u8,      // Cihaz sınıfı (0x00: arabirim tarafından tanımlanır, diğer sınıflar USB-IF tarafından tanımlanır)
    bDeviceSubClass: u8,   // Cihaz alt sınıfı
    bDeviceProtocol: u8,   // Cihaz protokolü
    bMaxPacketSize0: u8,   // 0 numaralı uç noktanın maksimum paket boyutu
    idVendor: u16,         // Üretici kimliği (VID) (Little-endian)
    idProduct: u16,        // Ürün kimliği (PID) (Little-endian)
    bcdDevice: u16,         // Cihaz sürüm numarası (BCD formatında) (Little-endian)
    iManufacturer: u8,     // Üretici dizesi tanımlayıcı indeksi
    iProduct: u8,          // Ürün dizesi tanımlayıcı indeksi
    iSerialNumber: u8,     // Seri numarası dizesi tanımlayıcı indeksi
    bNumConfigurations: u8, // Olası konfigürasyon sayısı
}
// statik olarak boyut kontrolü (isteğe bağlı, derleme zamanında kontrol sağlar)
const _: () = assert!(core::mem::size_of::<UsbDeviceDescriptor>() == USB_DEVICE_DESCRIPTOR_SIZE);


// --- MMIO Okuma/Yazma Yardımcı Fonksiyonları ---

// Belleğe eşlenmiş G/Ç (MMIO) okuma fonksiyonu
// `address`: Okunacak bellek adresi
// `T`: Okunacak veri tipi (örn. u32, u16, u8). T volatile::Volatile<U> olmalıdır.
/// # Güvenlik
/// Ham bellek adresinden okuma yaptığı için 'unsafe'dır. Adresin geçerli olması çağırana bağlıdır.
#[inline(always)] // Genellikle MMIO helper'ları inline yapmak performansı artırır.
unsafe fn mmio_read<T>(address: usize) -> T {
     // volatile::Volatile<U> tipindeki bir nesneye dönüşüm ve okuma
    (address as *const T).read_volatile() // T = Volatile<U> beklenir, read_volatile Volatile'ın içindeki U'yu döndürür.
}

// Belleğe eşlenmiş G/Ç (MMIO) yazma fonksiyonu
// `address`: Yazılacak bellek adresi
// `value`: Yazılacak değer (U tipinde)
// `T`: Yazma yapılan Volatile tipi (volatile::Volatile<U>)
/// # Güvenlik
/// Ham bellek adresine yazma yaptığı için 'unsafe'dır. Adresin geçerli olması çağırana bağlıdır.
#[inline(always)] // Genellikle MMIO helper'ları inline yapmak performansı artırır.
unsafe fn mmio_write<T>(address: usize, value: T) { // value T yerine U olmalıydı, T = Volatile<U> değil, U tipinde olmalı.
     // volatile::Volatile<U> tipindeki bir nesneye dönüşüm ve yazma
     // Bu kullanım, T'nin Volatile<U> olduğu durumda T'yi doğrudan yazmaya çalışır.
     // Doğrusu: value U tipinde olmalı ve write_volatile'a U *mut pointer verilmelidir.
     (address as *mut T).write_volatile(value); // T = U beklenir, write_volatile U'yu yazar.
}

// Düzeltilmiş MMIO helper'ları (Volatile crate'i ile uyumlu hale getirildi - Read/Write metodunu kullanmak için Volatile sarmalayıcısı adres pointer'ına uygulanır)
// veya ham pointerlarla (mmio_read/write) Volatile kullanmadan devam edilebilir.
// Mevcut kod ham pointer + read_volatile/write_volatile kullanıyor, bu da doğru.
// mmio_read<T>/mmio_write<T> imzaları biraz yanıltıcı, T read_volatile/write_volatile'ın beklediği tip olmalı (örn. u32, u8).
// Mevcut kullanımlara uyum sağlamak için imzaları U olarak tutalım ve T'yi Unsafe fn içinde cast edelim.

 #[inline(always)]
 unsafe fn mmio_read_u32(address: usize) -> u32 {
     (address as *const u32).read_volatile()
 }

 #[inline(always)]
 unsafe fn mmio_write_u32(address: usize, value: u32) {
     (address as *mut u32).write_volatile(value);
 }

// Diğer boyutlar için de eklenebilir:
 unsafe fn mmio_read_u8(address: usize) -> u8 { (address as *const u8).read_volatile() }
 unsafe fn mmio_write_u8(address: usize, value: u8) { (address as *mut u8).write_volatile(value) }
 unsafe fn mmio_read_u16(address: usize) -> u16 { (address as *const u16).read_volatile() }
 unsafe fn mmio_write_u16(address: usize, value: u16) { (address as *mut u16).write_volatile(value) }


// USB Denetleyici Kayıt Erişim Fonksiyonları (Okuma)
// Bu fonksiyonlar, belirli EHCI kayıtlarını okumak için MMIO yardımcılarını kullanır.
// # Güvenlik
// Alt seviye MMIO helper'ları 'unsafe' olduğu için bu fonksiyonlar da 'unsafe' olmalıdır.
pub unsafe fn read_hc_version() -> u16 { // Version reg 16 bit (bcd formatında)
     mmio_read_u32(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CAPLENGTH_OFFSET + USB_HC_VERSION_OFFSET - 2) as u16 // Version EHCI CAPLENGTH + 2 de başlar
      mmio_read_u16(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CAPLENGTH_OFFSET + 0x02) // Eğer u16 okuyucu varsa
}

pub unsafe fn read_hc_caplength() -> u8 { // Caplength 8 bit
     mmio_read_u32(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CAPLENGTH_OFFSET) as u8 // İlk bayt
     mmio_read_u8(USB_CONTROLLER_BASE_ADDRESS + USB_HC_CAPLENGTH_OFFSET) // Eğer u8 okuyucu varsa
}

// Hesaplanan Operational Register Base Adresi (Base + Caplength)
unsafe fn get_operational_registers_base() -> usize {
    let caplength = read_hc_caplength() as usize; // Okunan Capability Register Length
    USB_CONTROLLER_BASE_ADDRESS.wrapping_add(caplength) // Base + Caplength = Operational Registers Base
}


// Operational Register Erişim Fonksiyonları (Okuma)
// # Güvenlik
// Alt seviye MMIO helper'ları ve Operational Register Base hesaplama 'unsafe' olduğu için bu fonksiyonlar da 'unsafe' olmalıdır.
pub unsafe fn read_hc_command() -> u32 {
     let op_reg_base = get_operational_registers_base();
     mmio_read_u32(op_reg_base + USB_HC_COMMAND_OFFSET)
}

pub unsafe fn read_hc_status() -> u32 {
     let op_reg_base = get_operational_registers_base();
     mmio_read_u32(op_reg_base + USB_HC_STATUS_OFFSET)
}

pub unsafe fn read_hc_interrupt_enable() -> u32 {
     let op_reg_base = get_operational_registers_base();
     mmio_read_u32(op_reg_base + USB_HC_INTERRUPT_ENABLE_OFFSET)
}

// Port 1 Durum ve Kontrol Kaydı (EHCI'da PORTSC1)
pub unsafe fn read_hc_port_status_and_control(port_num: u8) -> u32 {
     let op_reg_base = get_operational_registers_base();
     // PORTSC registerları ardışık memory adreslerindedir. Port N için offset = PORTSC1_OFFSET + (N-1) * 4 (32-bit register)
     let port_offset = USB_HC_PORT_STATUS_AND_CONTROL_OFFSET.wrapping_add(((port_num - 1) as usize) * mem::size_of::<u32>());
     mmio_read_u32(op_reg_base + port_offset)
}


// USB Denetleyici Kayıt Erişim Fonksiyonları (Yazma)
// # Güvenlik
// Alt seviye MMIO helper'ları ve Operational Register Base hesaplama 'unsafe' olduğu için bu fonksiyonlar da 'unsafe' olmalıdır.
pub unsafe fn write_hc_command(value: u32) {
     let op_reg_base = get_operational_registers_base();
     mmio_write_u32(op_reg_base + USB_HC_COMMAND_OFFSET, value);
}

pub unsafe fn write_hc_status(value: u32) { // STS kaydı yazarak bayrakları temizler (W1C)
     let op_reg_base = get_operational_registers_base();
     mmio_write_u32(op_reg_base + USB_HC_STATUS_OFFSET, value);
}

pub unsafe fn write_hc_interrupt_enable(value: u32) {
     let op_reg_base = get_operational_registers_base();
     mmio_write_u32(op_reg_base + USB_HC_INTERRUPT_ENABLE_OFFSET, value);
}

pub unsafe fn write_hc_interrupt_disable(value: u32) {
     // EHCI'da INTREN'e yazmak EN, INTRDISE kaydına yazmak DIS'tir.
     // Bu fonksiyon INTREN'e yazarak kesme etkinleştirir, disable için başka fonksiyon gerekir.
     // Fonksiyon adı yanlış, enable olmalıydı veya disable kaydı farklı olmalıydı.
     // EHCI'da ayrı bir Disable register'ı yoktur, sadece Enable register'ına 0 yazarak disable edilir.
     // Bu fonksiyon adı tutarsız, adını değiştirelim veya kaldıralım.
      write_hc_interrupt_enable(read_hc_interrupt_enable() & !value); // Disable için bu mantık kullanılabilir
     kprintln!("UYARI: write_hc_interrupt_disable fonksiyonu EHCI spesifikasyonuyla uyumlu değil.");
     // EHCI'da interrupt_enable registerına 0 yazarak disable edilir.
     // Bu fonksiyonu kullanmak yerine write_hc_interrupt_enable(0) veya maskeleme kullanılmalı.
     // Şimdilik kaldırıyoruz, yerine write_hc_interrupt_enable kullanılsın.
      let op_reg_base = get_operational_registers_base();
      mmio_write_u32(op_reg_base + USB_HC_INTERRUPT_DISABLE_OFFSET, value); // Bu offset EHCI'da yok!
}

// Port Durum ve Kontrol Kaydına Yazma (EHCI'da PORTSC1)
pub unsafe fn write_hc_port_status_and_control(port_num: u8, value: u32) {
     let op_reg_base = get_operational_registers_base();
     let port_offset = USB_HC_PORT_STATUS_AND_CONTROL_OFFSET.wrapping_add(((port_num - 1) as usize) * mem::size_of::<u32>());
     mmio_write_u32(op_reg_base + port_offset, value);
}

// USB Denetleyici Yapılandırma Kaydına Yazma (EHCI CONFIGFLAG)
pub unsafe fn write_hc_config_flag(value: u32) { // İsim düzeltildi: CONFIGFLAG
     let op_reg_base = get_operational_registers_base();
     mmio_write_u32(op_reg_base + USB_HC_CONFIG_OFFSET, value);
}


// --- Yüksek Seviye Fonksiyonlar (Örnek olarak Cihaz Tanımlayıcı Okuma) ---

// USB Cihaz Tanımlayıcısını Okuma (Örnek Fonksiyon)
// Endpoint 0 (kontrol endpoint) üzerinden aygıt tanımlayıcısını okumayı dener.
/// # Güvenlik
/// Alt seviye kontrol transfer fonksiyonu 'unsafe' olduğu için bu fonksiyon da 'unsafe' olmalıdır.
pub unsafe fn get_usb_device_descriptor() -> Option<UsbDeviceDescriptor> { // endpoint_address parametresi kaldırıldı, EP0 kullanılır
    kprintln!("USB Cihaz Tanımlayıcısı Okunuyor...");
    // descriptor için static veya heap'ten tahsis edilmiş bir arabellek gerekli.
    // Örnekte, control_transfer_in doğrudan descriptor struct'ına yazıyor (eğer buffer_size yeterliyse).
    // Bu yaklaşım, descriptor boyutunun sabit olması durumunda pratiktir.

    let mut descriptor_buffer: [u8; USB_DEVICE_DESCRIPTOR_SIZE] = [0u8; USB_DEVICE_DESCRIPTOR_SIZE]; // Stack'te arabellek
    let descriptor_ptr = descriptor_buffer.as_mut_ptr();

    let transfer_result = control_transfer_in(
         0, // Uç nokta adresi 0 (kontrol uç noktası)
         USB_REQ_TYPE_STANDARD_DEVICE_IN, // bmRequestType: Standart cihaz isteği, IN yönü
         USB_REQ_GET_DESCRIPTOR, // bRequest: GET_DESCRIPTOR isteği (0x06)
         USB_DESC_TYPE_DEVICE_VAL,   // wValue: Tanımlayıcı tipi (Device Descriptor 0x0100)
         0,                      // wIndex: Genellikle 0 (Language ID veya Interface/Endpoint Index olabilir)
         USB_DEVICE_DESCRIPTOR_SIZE as u16, // wLength: Okunacak boyut (18 bayt)
         descriptor_ptr, // Veri tamponu pointer'ı
         USB_DEVICE_DESCRIPTOR_SIZE, // Tampon boyutu
    );

    if transfer_result.is_ok() {
         // Başarılı transfer sonrası, arabellekteki baytları struct'a kopyalayabilir veya arabelleği doğrudan struct pointer olarak kullanabiliriz.
         // #[repr(C, packed)] sayesinde direct cast genellikle çalışır, ancak memcpy daha güvenlidir.
         let mut descriptor = UsbDeviceDescriptor { // Sıfır değerleri ile başlatma
             bLength: 0, bDescriptorType: 0, bcdUSB: 0, bDeviceClass: 0,
             bDeviceSubClass: 0, bDeviceProtocol: 0, bMaxPacketSize0: 0,
             idVendor: 0, idProduct: 0, bcdDevice: 0, iManufacturer: 0,
             iProduct: 0, iSerialNumber: 0, bNumConfigurations: 0,
         };
         // Descriptor bytes'ı struct'a kopyala
         ptr::copy_nonoverlapping(descriptor_ptr, &mut descriptor as *mut UsbDeviceDescriptor as *mut u8, USB_DEVICE_DESCRIPTOR_SIZE);

         // Descriptor'ın ilk iki baytını kontrol et (uzunluk ve tip)
         if descriptor.bLength == USB_DEVICE_DESCRIPTOR_SIZE as u8 && descriptor.bDescriptorType == 0x01 {
             kprintln!("USB Cihaz Tanımlayıcısı Başarıyla Okundu.");
             #[cfg(feature = "debug")]
             kprintln!("Descriptor Detayları: {:?}", descriptor);
             Some(descriptor)
         } else {
             kprintln!("HATA: USB Cihaz Tanımlayıcısı Boyut/Tip Uyuşmuyor (Uzunluk: {}, Tip: {:02x})", descriptor.bLength, descriptor.bDescriptorType);
             None // Boyut veya tip yanlışsa None
         }
    } else {
        kprintln!("USB Cihaz Tanımlayıcı Okuma Transfer Hatası!");
        None // Hata durumunda None döndür
    }
}


// --- Alt Seviye USB Kontrol Transfer Fonksiyonu (ÖRNEK - Gerçek Donanıma Göre Uyum Sağlanmalı) ---

// USB Kontrol 'IN' Transferi (Cihazdan veri okuma)
// **DİKKAT**: Bu fonksiyon ÇOK BASİT bir örnek iskelettir.
// Gerçek bir donanım sürücüsünde, USB ana makine denetleyicinizin
// (örn. EHCI, OHCI, xHCI) spesifikasyonlarına GÖRE UYARLANMALIDIR.
// Hata yönetimi, zaman aşımları, kesmeler vb. gibi birçok detay eksiktir.
// EHCI için Queue Head (QH) ve Transfer Descriptor (TD) veri yapıları hazırlanır,
// DMA kullanılır ve Denetleyici Asenkron Listesi (Async List) veya Periyodik Listeye (Periodic List) eklenir.
/// # Güvenlik
/// Donanım registerlarına doğrudan erişim, ham pointer (data_buffer) kullanımı ve
/// donanıma özgü bekleme/durum kontrolü içerdiğinden 'unsafe'dır. data_buffer geçerli olmalıdır.
unsafe fn control_transfer_in(
    endpoint_address: u8,
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    data_buffer: *mut u8,
    buffer_size: usize, // Sağlanan tamponun gerçek boyutu
) -> Result<(), &'static str> {

    kprintln!("Kontrol Transferi (IN) Başlatılıyor (EP: {}, ReqType: {:02x}, Req: {:02x}, Len: {})",
        endpoint_address, request_type, request, length);

    // **DİKKAT: AŞAĞIDAKİ KOD GERÇEK BİR EHCI/USB KONTROLCÜ SÜRÜCÜSÜ DEĞİLDİR.**
    // **BU SADECE KAVRAMLARI TEMSİL ETMEK İÇİN YAZILMIŞ HAYALİ KODDUR.**
    // **GERÇEK DONANIMINIZIN DATASHEET'İNE GÖRE TAMAMEN YENİDEN YAZILMASI GEREKİR.**

    // 1. Komut Hazırlama (USB isteği kurulumu - SETUP Paketi)
    //    Bu bölüm, USB isteğini (setup paketini) oluşturmayı ve
    //    ana makine denetleyicinin komut kuyruğuna (command queue) eklemeyi içerir.
    //    **DENETLEYİCİYE ÖZGÜ KOMUT FORMATI VE VERİ YAPILARI (QH, TD) KULLANILMALIDIR.**

    let setup_packet: [u8; 8] = [ // USB Kontrol Transfer Setup Paketi (8 bayt)
        request_type,
        request,
        value as u8,        // wValue (low byte)
        (value >> 8) as u8, // wValue (high byte)
        index as u8,        // wIndex (low byte)
        (index >> 8) as u8, // wIndex (high byte)
        length as u8,       // wLength (low byte)
        (length >> 8) as u8, // wLength (high byte)
    ];

    // **HAYALİ EHCI İŞLEM AKIŞI TEMSİLİ:**
    // - Bellekte bir Queue Head (QH) ve bir veya daha fazla Transfer Descriptor (TD) yapısı hazırla.
    // - SETUP TD'sini oluştur (setup_packet adresini ve uzunluğunu içerir).
    // - DATA TD(ler)ini oluştur (data_buffer adresini, uzunluğunu ve yönünü içerir).
    // - STATUS TD'sini oluştur (0 uzunluklu, karşı yönü gösterir).
    // - Bu TD'leri QH'ye bağla.
    // - QH'yi HC'nin Asenkron Listesine ekle (veya varsa Komut Kuyruğuna).
    // - HC'ye listeyi işlemesini bildir (USBCMD registerındaki ilgili biti set ederek).

    // **ÖRNEK: Kontrolcü Komut/Kontrol Kaydına yazarak transferi başlatma sinyali (ÇOK BASİT TEMSİLİ)**
    unsafe {
        let op_reg_base = get_operational_registers_base();
        let command_reg_addr = op_reg_base.wrapping_add(USB_HC_COMMAND_OFFSET);
        let status_reg_addr = op_reg_base.wrapping_add(USB_HC_STATUS_OFFSET);

        // **DİKKAT: AŞAĞIDAKİ MMIO YAZMA İŞLEMİ HAYALİDİR VE GERÇEK DENETLEYİCİ MEKANİZMASINI TEMSİL ETMEZ.**
        // Gerçekte, USBCMD registerındaki Async Schedule Enable (ASE) gibi bitler ve
        // ASYNCLISTADDR registerı kullanılır.
         mmio_write_u32(command_reg_addr, 0x12345678); // ÖRNEK DEĞER! Denetleyici Komut Kaydına Hayali Yazma

        // TODO: Transferin tamamlanmasını bekle (Polling veya Kesme)
        // EHCI'da bu, TD'deki durum bitlerini kontrol etmek veya USBCMD'deki 'Run/Stop' bitini kontrol etmek olabilir.
        // Veya bir Transfer Tamamlama Kesmesi (PCI) beklemek.
        let mut timeout = 100000; // Basit zaman aşımı sayacı (Örnek)
        let transfer_done_bit: u32 = 1 << 0; // Örnek Transfer Tamamlama Biti (PCI)
        let error_bit: u32 = 1 << 1; // Örnek Hata Biti (UEI)

        let mut status = mmio_read_u32(status_reg_addr);
        while (status & (transfer_done_bit | error_bit)) == 0 && timeout > 0 {
             core::hint::spin_loop(); // Basit polleme
             status = mmio_read_u32(status_reg_addr);
             timeout -= 1;
        }

        if timeout == 0 {
             kprintln!("HATA: Kontrol Transferi Zaman Aşımı!");
             // TODO: Zaman aşımı durumunda kurtarma (HC reset, port reset vb.)
             return Err("Zaman Aşımı");
        }

         // Kesme durumunu temizle (W1C)
         mmio_write_u32(status_reg_addr, status & (transfer_done_bit | error_bit)); // Sadece ilgili bitleri temizle

        // TODO: TD/QH yapılarından transfer durumunu ve hata kodlarını oku.
        // Başarı durumunu ve kaç bayt transfer edildiğini belirle.
        // Eğer hata oluştuysa, hangi hata olduğunu belirle (STALL, NAK, CRC hatası vb.).

        // **TEMSİLİ: Durum kontrolü**
        if (status & error_bit) != 0 {
             kprintln!("HATA: Kontrol Transferinde Hata Biti Set Oldu!");
             // TODO: Hata detaylarını oku ve uygun kurtarma yap (Endpoint STALL temizleme vb.)
             return Err("Donanım Hatası");
        }

        // 2. Veri Transferi (Simüle Edilmiş)
        //    Denetleyici, USB cihazından gelen veriyi belirli bir bellek bölgesine (DMA tamponu) yazar.
        //    Bu bölgeden veriyi sağlanan 'data_buffer'a kopyalamamız gerekir.
        //    **VERİ OKUMA MEKANİZMASI DENETLEYİCİYE GÖRE DEĞİŞİR.**
        //    EHCI'da DMA yaygın kullanılır.

        // **TEMSİLİ KOD BAŞLANGICI**
        //    Burada, verinin HAYALİ bir DMA tamponunda (buffer_address - HAYALİ) olduğunu varsayıyoruz.
        //    Gerçekte, bu adres önceden tahsis edilmiş bir DMA-safe arabelleğin adresi olur.
        let buffer_address: usize = 0xABCDEF00; // **TAMAMEN HAYALİ DMA TAMPON ADRESİ**
        let transferred_bytes = length as usize; // Örnek: Beklenen kadar transfer edildiğini varsayalım

        // **TEMSİLİ: Veriyi HAYALİ DMA tamponundan al ve sağlanan 'data_buffer'a kopyala**
        //    Gerçekte, denetleyici DMA ile veriyi 'data_buffer'a (eğer DMA-safe ise) veya
        //    ayrı bir DMA tamponuna yazmıştır.
        //    Aşağıdaki döngü TEMSİLİDİR ve gerçek DMA veya veri alma mekanizmasını YANSITMAZ.
        if !data_buffer.is_null() && transferred_bytes > 0 && transferred_bytes <= buffer_size {
             // HAYALİ DMA tamponundan oku ve hedef arabelleğe yaz
             let src_ptr = buffer_address as *const u8; // HAYALİ KAYNAK ADRES
             let dest_ptr = data_buffer; // Hedef arabellek pointer'ı

             // Bellekten belleğe kopyalama (örnek)
             for i in 0..transferred_bytes {
                  // Kaynak adresten volatile okuma ve hedef adrese volatile yazma (veya memcpy kullanma)
                  let byte = (src_ptr.add(i) as *const u8).read_volatile(); // HAYALİ KAYNAKTAN VOLATILE OKU
                  (dest_ptr.add(i) as *mut u8).write_volatile(byte); // HEDEFE VOLATILE YAZ
             }
        } else if transferred_bytes > buffer_size {
             kprintln!("HATA: Kontrol Transferi Okunan Boyut Tampondan Büyük! (Okunan: {}, Tampon: {})", transferred_bytes, buffer_size);
             // Bu bir faz hatası (Phase Error) veya yazılım hatası olabilir.
             return Err("Tampon Yetersiz");
        }
        // TODO: Eğer data_buffer NULL ise ve length > 0 ise (SETUP veya STATUS aşaması), veri transferi yapılmamalıdır.

        // 3. STATUS Aşaması (Otomatik veya Manuel - Kontrolcüye Bağlı)
        // EHCI genellikle STATUS aşamasını otomatik yönetir.

        // Kontrol Transferinin Başarılı Tamamlandığını Varsayalım (Yukarıdaki hata kontrolü başarılıysa)
        Ok(())

        // **DİKKAT**: Bu fonksiyonun hata yönetimi, kesme işleme, zaman aşımları,
        //       DMA yönetimi, PID senkronizasyonu, paket bölme gibi
        //       kritik kısımları ÇOK BASİTTİR ve GERÇEK BİR UYGULAMA İÇİN YETERSİZDİR.
        //       Gerçek bir sürücüde bu bölümler çok daha detaylı ve sağlam olmalıdır.
    }


// --- Çekirdek Giriş Noktası (Örnek - Kendi Çekirdeğinize Uygun Hale Getirin) ---

// Çekirdek modülü giriş fonksiyonu (OpenRISC çekirdeğinize göre düzenleyin)
// Bu fonksiyon, çekirdek yüklendiğinde veya başlatıldığında çağrılır.
#[no_mangle] // İsim bozmayı engelle (linker için)
pub extern "C" fn init_module() -> i32 {
     kprintln!("srcio_openrisc.rs: USB Sürücü Modülü Başlatılıyor (OpenRISC)...");

     // Sahne64 konsol makrolarının std dışı ortamda çalışması için gerekli
     // ilk ayarlar burada veya platform başlangıcında yapılmalıdır.
     // Örnekte kprintln! Sahne64 makrolarını kullanıyor (varsayım).


    // 1. USB Ana Makine Denetleyiciyi Başlatma
    // Denetleyici adresinin geçerli olduğu unsafe block içinde çalış.
    unsafe {
         // a. Denetleyiciyi Sıfırlama (varsa, denetleyiciye özgü sıfırlama prosedürü)
         //    Örneğin, EHCI için USBCMD registerındaki HC Reset bitini set et ve bitin temizlenmesini bekle.
          let op_reg_base = get_operational_registers_base();
          let command_reg_addr = op_reg_base.wrapping_add(USB_HC_COMMAND_OFFSET);
          let status_reg_addr = op_reg_base.wrapping_add(USB_HC_STATUS_OFFSET);
          let halted_bit = uhci_sts::HC_HALTED; // HC Halted biti

          kprintln!("HC Resetleniyor...");
          mmio_write_u32(command_reg_addr, uhci_cmd::HC_RESET); // Reset Bitini Set Et
          // Reset bitinin temizlenmesini ve HC'nin durmasını bekle
          while (mmio_read_u32(command_reg_addr) & uhci_cmd::HC_RESET) != 0 || (mmio_read_u32(status_reg_addr) & halted_bit) == 0 {
               core::hint::spin_loop();
          }
          kprintln!("HC Resetlendi.");

         // b. Denetleyiciyi Çalıştırma (Run) Moduna Alma
         //    USBCMD registerındaki Run/Stop bitini set et.
         kprintln!("HC Çalıştırma Moduna Alınıyor...");
         let mut command_reg = mmio_read_u32(command_reg_addr);
         command_reg |= uhci_cmd::RUN_STOP;
         mmio_write_u32(command_reg_addr, command_reg);
         // HC'nin çalışmaya başlamasını bekle (HCHalted bitinin temizlenmesini bekle)
          while (mmio_read_u32(status_reg_addr) & halted_bit) != 0 {
               core::hint::spin_loop();
          }
          kprintln!("HC Çalışıyor.");

         // c. Root Hub Yapılandırması (Portları Etkinleştirme, Resetleme)
         //    PORTSC registerları üzerinden yapılır.
         // TODO: Bağlı portları algıla ve resetle.
         kprintln!("Root Hub Portları Yapılandırılıyor (Örnek)...");
         let num_ports = (mmio_read_u32(USB_CONTROLLER_BASE_ADDRESS + USB_HC_STRUCTURAL_PARAMETERS_OFFSET) >> 0) & 0b1111; // EHCI HCSPARAMS bit 3:0
         kprintln!("Algılanan Port Sayısı: {}", num_ports);

         for i in 1..=num_ports {
              kprintln!("Port {} Resetleniyor...", i);
              let mut portsc = read_hc_port_status_and_control(i as u8);
              // Port Reset bitini set et (PR) ve bitin temizlenmesini bekle (otomatik temizlenir)
              portsc |= (1 << 4); // EHCI PORTSC PR bit (Port Reset)
              write_hc_port_status_and_control(i as u8, portsc);
              // Port Reset bitinin otomatik temizlenmesini bekle
              while (read_hc_port_status_and_control(i as u8) & (1 << 4)) != 0 { // PR biti hala set mi?
                   core::hint::spin_loop();
              }
              kprintln!("Port {} Resetlendi.", i);
              // Reset sonrası biraz bekleme (EHCI spec 20ms)
              // TODO: Gecikme ekle
               core::hint::spin_loop(); // Simülasyon gecikmesi

              // Portu Etkinleştir (PE) - Genellikle port reset sonrası otomatik olur veya gerekirse set edilir.
              // Hata bayraklarını temizle (CSC, PEC, OCC, WCC) - W1C (yazarak temizlenir)
               portsc = read_hc_port_status_and_control(i as u8);
               let status_bits_to_clear = (1 << 1) | (1 << 3) | (1 << 8) | (1 << 13); // CSC, PEC, OCC, WCC
               write_hc_port_status_and_control(i as u8, portsc | status_bits_to_clear);
         }
         kprintln!("Root Hub Portları Yapılandırması Tamamlandı (Örnek).");


         // d. Gerekli Kesmeleri Etkinleştirme (Host Controller Interface'in kesmelerini)
         //    USBCMD registerındaki interrupt_enable bitleri ve/veya INTREN registerı üzerinden yapılır.
         //    Periyodik ve Asenkron listeler için de kesme etkinleştirilebilir.
          let interrupts_to_enable = uhci_sts::PORT_CHANGE_DETECT | uhci_sts::USB_INTERRUPT | uhci_sts::ERROR_INTERRUPT; // Örnek: Port değişimi, transfer tamamlanma, hata
          write_hc_interrupt_enable(interrupts_to_enable); // USBCMD INTREN bit alanı veya ayrı register (Denetleyiciye bağlı)
          kprintln!("Host Controller Kesmeleri Etkinleştirildi (Örnek).");

         // TODO: DMA için bellek alanlarını ayarla (QH, TD listeleri, veri tamponları) ve bu adresleri denetleyiciye bildir (MMIO registerları).
         // Örneğin: ASYNCLISTADDR, PERIODICLISTBASE registerlarına DMA alanlarının fiziksel adresleri yazılmalıdır.
         // EHCI 32-bit fiziksel adresler kullanır.


    } // unsafe block sonu (Denetleyici init)


    // 2. USB Cihazlarını Tarama ve Numaralandırma (Asenkron veya Eşitleme ile)
    // Genellikle bir Port Bağlantı Değişikliği Kesmesi (Port Change Detect - PCD) beklenerek başlar.
    // Kesme geldiğinde, hangi portun durumunun değiştiği kontrol edilir ve yeni bağlanan cihaz varsa numaralandırma başlatılır.
    // Numaralandırma (Enumeration) = Bus reset, adres atama, descriptor okuma (Device, Configuration, Interface, Endpoint), sınıf belirleme, endpoint yapılandırma.
    // Bu işlemler kontrol transferleri (Endpoint 0 üzerinden) ve Host Controller API'si kullanılarak yapılır.

    // Örnek: Cihaz Tanımlayıcısını okuma (Sadece TEST AMAÇLI - Başarılı olması için cihazın Root Hub'a bağlı ve basitçe resetlenmiş olması gerekir)
    // get_usb_device_descriptor unsafe olduğu için unsafe block içinde çağrılmalı.
     unsafe {
         if let Some(descriptor) = get_usb_device_descriptor() { // 0 numaralı uç noktadan oku (kontrol)
             // Başarılı şekilde tanımlayıcı okundu
             kprintln!("USB Cihaz Tanımlayıcı Okundu: Vendor ID: {:04x}, Product ID: {:04x}",
                         descriptor.idVendor.swap_bytes(), descriptor.idProduct.swap_bytes()); // Descriptor alanları Little-endian, MIPS/OpenRISC big-endian olabilir, swap gerekebilir.
             #[cfg(feature = "debug")] // Debug derlemelerde detaylı çıktı
             kprintln!("Descriptor Detayları (Tamamı): {:?}", descriptor);
         } else {
             kprintln!("USB Cihaz Tanımlayıcı Okuma HATASI veya Aygıt Algılanamadı!");
         }
     }


    kprintln!("srcio_openrisc.rs: USB Sürücü Modülü Başlatma Tamamlandı.");
    0 // Başarılı dönüş kodu
}

// Çekirdek modülü çıkış fonksiyonu (isteğe bağlı, kendi çekirdeğinize göre düzenleyin)
// Modül kaldırıldığında veya sistem kapatıldığında çağrılır (eğer çekirdek destekliyorsa).
#[no_mangle]
pub extern "C" fn exit_module() {
    kprintln!("srcio_openrisc.rs: USB Sürücü Modülü Çıkartılıyor...");
    // USB denetleyiciyi durdurma, kesmeleri devre dışı bırakma, DMA alanlarını temizleme vb. (temizlik işlemleri)
    unsafe { // Donanım registerlarına erişim unsafe
         let op_reg_base = get_operational_registers_base();
         let command_reg_addr = op_reg_base.wrapping_add(USB_HC_COMMAND_OFFSET);

         // Denetleyiciyi Durdur (USBCMD Run/Stop bitini 0 yap)
         let mut command_reg = mmio_read_u32(command_reg_addr);
         command_reg &= !uhci_cmd::RUN_STOP;
         mmio_write_u32(command_reg_addr, command_reg);
         // HC'nin durmasını bekle (HCHalted bitinin set olmasını bekle)
          let status_reg_addr = op_reg_base.wrapping_add(USB_HC_STATUS_OFFSET);
          let halted_bit = uhci_sts::HC_HALTED;
          while (mmio_read_u32(status_reg_addr) & halted_bit) == 0 {
               core::hint::spin_loop();
          }
         kprintln!("Host Controller Durduruldu.");

         // Tüm Host Controller kesmelerini devre dışı bırak
         write_hc_interrupt_enable(0); // Tüm bitleri 0 yaparak disable et
         kprintln!("Host Controller Kesmeleri Devre Dışı Bırakıldı.");

         // TODO: DMA alanlarını serbest bırak (eğer tahsis edildiyse).
    } // unsafe block sonu (Temizlik)

    kprintln!("srcio_openrisc.rs: USB Sürücü Modülü Çıkartıldı.");
}


// --- Panik İşleyici (Zorunlu - no_std ortamda) ---
// Sistemde bir panic! olduğunda bu fonksiyon çağrılır.

 use core::panic::PanicInfo; // Zaten yukarıda import edildi
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Panik bilgisini Sahne64 konsol makrolarını kullanarak yazdır
    #[cfg(feature = "std")] std::eprintln!("KERNEL PANIC: {}", info);
    #[cfg(not(feature = "std"))] eprintln!("KERNEL PANIC: {}", info); // Varsayım: Sahne64 eprintln! makrosu

     // Eğer panik bilgisinde location ve message varsa onları da yazdır.
     if let Some(location) = info.location() {
         #[cfg(feature = "std")] std::eprintln!("at {}", location);
         #[cfg(not(feature = "std"))] eprintln!("at {}", location);
     }
     if let Some(message) = info.message() {
         #[cfg(feature = "std")] std::eprintln!(": {}", message);
         #[cfg(not(feature = "std"))] eprintln!(": {}", message);
     }
     #[cfg(feature = "std")] std::eprintln!("\n");
     #[cfg(not(feature = "std"))] eprintln!("\n");


    // **BURAYA PANİK ANINDA YAPILACAK DİĞER ÖNEMLİ İŞLEMLERİ EKLEYİN.**
    // Örneğin: Donanımı güvenli bir duruma getir, CPU'yu durdur, hata kodunu kaydet, watchdog timer'ı devre dışı bırak, yeniden başlatma vb.
    // Donanıma özgü durdurma işlemleri burada yapılabilir (MMIO yazma vb.).
    loop {} // Sonsuz döngüde kal
}
