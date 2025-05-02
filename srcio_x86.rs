#![no_std] // Standart kütüphaneye ihtiyacımız yok
#![no_main] // Rust'ın varsayılan giriş noktasını (main) kullanmıyoruz
#![allow(dead_code)] // Geliştirme sırasında kullanılmayan kodlara izin ver

// Core kütüphanesinden gerekli öğeler
use core::panic::PanicInfo; // Panik işleyicisi için
use core::fmt::Write; // Yazma trait'i için (debug çıktısı için)
use core::hint::spin_loop; // Basit bekleme döngüsü için
use core::ptr; // Pointer manipülasyonları için
use core::mem; // memory işlemleri için (size_of vb.)


// Donanım registerlarına erişim için `volatile` crate'i ve bit alanları için `bitflags` crate'i kullanılır.
// Bu crate'ler, derleyicinin donanım erişimlerini optimize etmesini engeller ve register bitlerini yönetmeyi kolaylaştırır.
use volatile::Volatile;
use bitflags::bitflags;


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


// Panik durumunda ne yapılacağını tanımlayan fonksiyon.
// Çekirdek panik olduğunda bu fonksiyon çağrılır.
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


// --- Donanım Tanımları (EHCI Host Controller Örneği) ---

// EHCI (Enhanced Host Controller Interface) Base Adresi (X86 için ÖRNEK - GERÇEK ADRES DEĞİL!)
// Gerçek bir sürücü, bu adresi genellikle PCI konfigürasyon alanından veya
// ACPI tablolarından bulmalıdır.
const EHCI_BASE_ADDRESS: usize = 0xFEDC_0000; // ÖRNEK ADRES! GERÇEK SİSTEMİNİZE GÖRE DEĞİŞİR!


// EHCI Capability Register Offsetleri (Base Address'e göre)
const EHCI_CAP_LENGTH_OFFSET: usize = 0x00; // Capability Register Length (CAPLENGTH) - 8 bit
const EHCI_VERSION_OFFSET: usize = 0x02; // Host Controller Interface Version (HCIVERSION) - 16 bit
const EHCI_HCSPARAMS_OFFSET: usize = 0x04; // Host Controller Structural Parameters (HCSPARAMS) - 32 bit
const EHCI_HCCPARAMS_OFFSET: usize = 0x08; // Host Controller Capability Parameters (HCCPARAMS) - 32 bit
// ... diğer Capability register offsetleri ...

// EHCI Operational Register Offsetleri (Base Address + CAPLENGTH'e göre)
const EHCI_USBCMD_OFFSET: usize = 0x00;  // USB Command Register (USBCMD) - 32 bit
const EHCI_USBSTS_OFFSET: usize = 0x04;  // USB Status Register (USBSTS) - 32 bit
const EHCI_USBINTREN_OFFSET: usize = 0x08; // USB Interrupt Enable Register (USBINTREN) - 32 bit
const EHCI_FRINDEX_OFFSET: usize = 0x0C; // Frame Index Register (FRINDEX) - 32 bit
const EHCI_CONFIGFLAG_OFFSET: usize = 0x40; // Configure Flag Register (CONFIGFLAG) - 32 bit
const EHCI_PORTSC_BASE_OFFSET: usize = 0x44; // Port Status and Control Registers (PORTSC) - 32 bit her port için, 1'den başlar


// --- Register Bit Alanları ve Bayraklar (`bitflags` kullanarak) ---
// EHCI Spesifikasyonu Revision 1.0'a göre bit tanımları

bitflags! {
    #[repr(transparent)] // Underlying type'ın (u32) bellek düzenini korumak için
    pub struct UsbCommand: u32 {
        const RUN_STOP                   = 1 << 0;   // RS
        const HOST_CONTROLLER_RESET      = 1 << 1;   // HCR
        const FRAME_LIST_SIZE_MASK       = 0b11 << 2;   // FLS (00b: 1024, 01b: 512, 10b: 256, 11b: Reserved)
        const PERIODIC_SCHEDULE_ENABLE   = 1 << 4;   // PSE
        const ASYNCHRONOUS_SCHEDULE_ENABLE = 1 << 5;   // ASE
        const INT_ON_ASYNC_ADVANCE_DOORBELL = 1 << 6;   // IAAD
        const LIGHT_HOST_CONTROLLER_RESET = 1 << 7;   // LHCR - Optional
        const ASYNC_ADVANCE_ARM          = 1 << 8; // ASAA - Optional
        // bit 9, 10, 11 Reserved
        // bit 15:12 Reserved
    }
}

bitflags! {
    #[repr(transparent)]
    pub struct UsbStatus: u32 {
        const USB_INTERRUPT              = 1 << 0;   // USBINT - Transaction Completed Interrupt (PCI)
        const ERROR_INTERRUPT            = 1 << 1;   // EINT - USB Error Interrupt (UEI)
        const PORT_CHANGE_DETECT         = 1 << 2;   // PCI - Port Change Interrupt
        const FATAL_ERROR              = 1 << 3;   // FLR - Fatal Error Interrupt
        const HOST_SYSTEM_ERROR          = 1 << 4;   // HSE - Host System Error
        const ASYNC_SCHEDULE_STATUS      = 1 << 5;   // ASS - Asynchronous Schedule Status
        const PERIODIC_SCHEDULE_STATUS   = 1 << 6;   // PSS - Periodic Schedule Status
        const COMMAND_STATUS             = 1 << 12;  // HCH - Host Controller Halted
        const INTERRUPT_ON_ASYNC_ADVANCE_STATUS = 1 << 13; // IAA - Interrupt on Async Advance Status
        // bit 14 Reserved
        // bit 15 IOFLT - I/O Flow Control (Optional)
    }
}

 // USB Interrupt Enable Register (USBINTREN) bitleri genellikle USBSTS ile aynı anlamlara sahiptir,
 // karşılık gelen kesmeyi etkinleştirmek için kullanılır.
bitflags! {
    #[repr(transparent)]
    pub struct UsbInterruptEnable: u32 { // Register adı UsbInterrupt yerine UsbInterruptEnable daha açık
        const USB_INTERRUPT_ENABLE       = 1 << 0;   // USBIE
        const ERROR_INTERRUPT_ENABLE     = 1 << 1;   // EIE
        const PORT_CHANGE_DETECT_ENABLE  = 1 << 2;   // PCIE
        const FATAL_ERROR_ENABLE         = 1 << 3;   // FLE - Should be FLE, not FAE
        const SYSTEM_ERROR_ENABLE        = 1 << 4;   // SYSE
        const ASYNC_ADVANCE_ENABLE       = 1 << 6;   // IAADE - Interrupt on Async Advance Doorbell Enable
         // bit 5 Reserved
         // bit 7:1 Reserved (depends on spec version)
         // bit 15:8 Reserved
    }
}


 // Host Controller Structural Parameters (HCSPARAMS)
 bitflags! {
     #[repr(transparent)]
     pub struct HcsParams: u32 {
         const NUMBER_OF_PORTS_MASK    = 0b1111 << 0; // Number of Ports (N_PORTS) - bits 3:0
         const PCC_INDICATOR           = 1 << 4;   // PCC - Port Power Control
         const PCC_PORT_POWER_CONTROL  = 0b1111 << 4; // Port Power Control scheme - bits 7:4
         const PORT_ROUTING_RULES      = 1 << 8;   // PRR - Port Routing Rules
         const PORT_PER_COMPANION_HC_MASK = 0b1111 << 12; // N_PCC - Number of Ports Per Companion Controller - bits 15:12
         const NUMBER_OF_COMPANION_HC_MASK = 0b1111 << 16; // N_CC - Number of Companion Controllers - bits 19:16
         const CS_INDICATOR            = 1 << 20;  // CS - Companion HC Indicator
         const P_INDICATOR_SUPPORT     = 1 << 21;  // P_INDICATOR - Port Indicator Support
         const DEBUG_PORT_NUMBER_MASK  = 0b1111 << 24; // N_P_DBG - Debug Port Number - bits 27:24
     }
 }

 impl HcsParams {
     pub fn port_count(&self) -> u8 {
         ((self.bits() & HcsParams::NUMBER_OF_PORTS_MASK.bits()) >> 0) as u8
     }
     pub fn companion_controller_count(&self) -> u8 {
         ((self.bits() & HcsParams::NUMBER_OF_COMPANION_HC_MASK.bits()) >> 16) as u8
     }
     pub fn ports_per_companion_controller(&self) -> u8 {
         ((self.bits() & HcsParams::PORT_PER_COMPANION_HC_MASK.bits()) >> 12) as u8
     }
 }


 // Host Controller Capability Parameters (HCCPARAMS)
 bitflags! {
     #[repr(transparent)]
     pub struct HccParams: u32 {
         const RTC_SUPPORT           = 1 << 0; // RTD - Routing Time Delay
         const PPC_SUPPORT           = 1 << 1; // PPC - Port Power Control
         const ISOCHRONOUS_SCHED_THRESH_MASK = 0b1111 << 4; // IST - Isochronous Scheduling Threshold - bits 7:4
         const EHCI_PLUS_PLUS_SUPPORT = 1 << 8; // ESP - EHCI++ Support
         const PROGRAMMABLE_FRAME_LIST_FLAG = 1 << 9; // PFLF - Programmable Frame List Flag
         const ASYNC_PARK_CAPABLE    = 1 << 10; // ASPC - Asynchronous Schedule Park Capable
         const ASYNC_PARK_MODE_SELECT_MASK = 0b11 << 11; // AST - Asynchronous Schedule Park Mode Select - bits 12:11
         const ASYNC_PARK_MODE_ENABLE = 1 << 13; // ASPE - Asynchronous Schedule Park Mode Enable
         const ISOCHRONOUS_CACHE_DISABLE = 1 << 14; // ISOC_CACHE - Isochronous Cache Disable (Optional)
         const EHCI_EXT_CAP_POINTER_MASK = 0xFF << 16; // ECCP - EHCI Extended Capabilities Pointer - bits 23:16
     }
 }

 impl HccParams {
     pub fn ext_cap_pointer(&self) -> u8 {
         ((self.bits() & HccParams::EHCI_EXT_CAP_POINTER_MASK.bits()) >> 16) as u8
     }
 }

bitflags! {
    #[repr(transparent)]
    pub struct PortStatusControlFlags: u32 {
        const CURRENT_CONNECT_STATUS     = 1 << 0;   // CCS - Current Connect Status (R)
        const PORT_ENABLED_DISABLED      = 1 << 1;   // PES - Port Enabled/Disabled Status (R/W)
        const PORT_SUSPEND               = 1 << 2;   // SUSP - Suspend (R/W)
        const PORT_OVER_CURRENT_ACTIVE   = 1 << 3;   // OCA - Over-current Active (R)
        const PORT_RESET                 = 1 << 4;   // PRS - Port Reset (R/W)
        const PORT_POWER                 = 1 << 8;   // PP - Port Power (R/W)
        const LINE_STATUS_MASK           = 0b11 << 10; // LS - Line Status (R) (00=SE0, 01=K-state, 10=J-state, 11=Undefined)
        const PORT_OWNER                 = 1 << 13;  // PO - Port Owner (R/W)
        const PORT_INDICATOR_CONTROL_MASK = 0b11 << 14; // PIC - Port Indicator Control (R/W)
        // Change Status Bits (Write 1 to Clear - W1C)
        const CONNECT_STATUS_CHANGE      = 1 << 16;  // CSC - Connect Status Change
        const PORT_ENABLED_CHANGE        = 1 << 17;  // PESC- Port Enable Status Change
        const PORT_SUSPEND_CHANGE        = 1 << 18;  // PSSC - Port Suspend Change
        const OVER_CURRENT_CHANGE        = 1 << 19;  // OCC - Over-current Change
        const PORT_RESET_CHANGE          = 1 << 20;  // PRSC - Port Reset Change

        // Port Speed Indication (R) - Bits 27:26
         00b: Full-speed
         01b: Low-speed
         10b: High-speed
         11b: Undefined
        const PORT_SPEED_MASK          = 0b11 << 26;
    }
}

impl PortStatusControlFlags {
     pub fn speed(&self) -> PortSpeed {
         match (self.bits() & PortStatusControlFlags::PORT_SPEED_MASK.bits()) >> 26 {
             0b00 => PortSpeed::FullSpeed,
             0b01 => PortSpeed::LowSpeed,
             0b10 => PortSpeed::HighSpeed,
             _    => PortSpeed::Unknown,
         }
     }
      pub fn line_status(&self) -> LineStatus {
          match (self.bits() & PortStatusControlFlags::LINE_STATUS_MASK.bits()) >> 10 {
              0b00 => LineStatus::SE0, // Single Ended Zero (Disconnected)
              0b01 => LineStatus::KState,
              0b10 => LineStatus::JState,
              _    => LineStatus::Undefined,
          }
      }
}

 #[derive(Debug, PartialEq)] // PartialEq eklendi karşılaştırma için
 enum LineStatus {
     SE0, // Single Ended Zero (Disconnected)
     KState,
     JState,
     Undefined,
 }


#[derive(Debug, PartialEq)] // PartialEq eklendi karşılaştırma için
enum PortSpeed {
    LowSpeed,
    FullSpeed,
    HighSpeed,
    Unknown,
}


// --- Descriptor Tipleri ---
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    DeviceQualifier = 6,
    OtherSpeedConfiguration = 7,
    InterfacePower = 8,
    Otg = 9, // Should be OTG, not Ota
    Debug = 10,
    BOS = 0x0F, // Binary Device Object Store
    Report = 0x22, // HID Report Descriptor
    PhysicalDescriptor = 0x23, // HID Physical Descriptor
}


// --- USB Host Controller Yapısı (`Volatile` kullanarak) ---

// EHCI Kayıt Yapısı
// EHCI spesifikasyonu 2.2 bölümüne göre register adresleri ve boyutları.
// Operational register'lar, Capability register bloğunun bitiminden sonra başlar.
// Capability register bloğunun boyutu CAPLENGTH register'ından okunur.
#[repr(C)] // C yapısı düzenini garanti etmek için
struct EhciRegisters {
     // Capability Registers (offsetler base_address'e göre)
     cap_length: Volatile<u8>, // 0x00
     _reserved1: [u8; 1], // 0x01 (padding)
     hci_version: Volatile<u16>, // 0x02
     hcs_params: Volatile<HcsParams>, // 0x04
     hcc_params: Volatile<HccParams>, // 0x08
     // ... diğer Capability registerları ...

     // Operational Registers Base Adresi (base_address + cap_length)
     // Operational register'lara erişmek için bu base adres kullanılır.
     operational_base: usize,

      Operational Registers (erişim operational_base'e göre offsetlenir)
      Örnek olarak bazıları burada gösterilmiştir. Tamamı eklenmelidir.
      usbcmd: Volatile<UsbCommand>, // USB Command Register (0x00 from Operational Base)
      usbsts: Volatile<UsbStatus>, // USB Status Register (0x04 from Operational Base)
      usbintren: Volatile<UsbInterruptEnable>, // USB Interrupt Enable (0x08 from Operational Base)
      frindex: Volatile<u32>, // Frame Index (0x0C from Operational Base)
      configflag: Volatile<ConfigFlag>, // Configure Flag Register (0x40 from Operational Base)
      portsc: [Volatile<PortStatusControlFlags>; 15], // PORTSC1..15 (0x44 + (n-1)*4 from Operational Base)

     // `Volatile` alanları doğrudan struct içinde tanımlamak yerine,
     // operational_base adresini kullanarak manuel olarak erişmek daha esnektir
     // ve tüm PORTSC registerlarını dizi olarak tanımlama zorunluluğunu ortadan kaldırır.
     // `Volatile`'ın read/write/modify metodları ham pointer ile çalıştığı için bu mümkündür.


     base_address: usize, // Capability register bloğunun temel adresini sakla
}

impl EhciRegisters {
     /// Belirtilen base adresten EHCI Host Controller kayıt yapılarını oluşturur.
     /// # Güvenlik
     /// Sağlanan `base_address` geçerli bir EHCI denetleyicisinin başlangıcı olmalıdır.
     /// `cap_length` registerını okuyarak Operational Registerların base adresini hesaplar.
     unsafe fn new(base_address: usize) -> Self {
         // CAPLENGTH registerını oku (base adresin hemen başında bulunur)
         let cap_length_ptr: *const u8 = base_address as *const u8;
         let cap_length = Volatile::new(cap_length_ptr as *mut u8).read(); // unsafe

         // Operational Registerların base adresini hesapla
         let operational_base = base_address.wrapping_add(cap_length as usize);

         kprintln!("EHCI Base: {:p}, CAPLENGTH: {}, Op Base: {:p}", base_address as *const (), cap_length, operational_base as *const ());


         EhciRegisters {
             // Capability Register Volatile alanlarını oluştur (doğrudan pointer ile)
             cap_length: Volatile::new(base_address.wrapping_add(EHCI_CAP_LENGTH_OFFSET) as *mut u8),
             _reserved1: [0; 1], // Padding
             hci_version: Volatile::new(base_address.wrapping_add(EHCI_VERSION_OFFSET) as *mut u16),
             hcs_params: Volatile::new(base_address.wrapping_add(EHCI_HCSPARAMS_OFFSET) as *mut HcsParams),
             hcc_params: Volatile::new(base_address.wrapping_add(EHCI_HCCPARAMS_OFFSET) as *mut HccParams),
             // ... diğer Capability registerları ...

             operational_base, // Hesaplanan operational base adresini sakla
             base_address, // Base adresini sakla
         }
     }

     // Operational Registerlara erişim için helper fonksiyonlar (operational_base'i kullanarak)
     /// USB Command Register (USBCMD) erişimi
     pub unsafe fn usbcmd(&self) -> Volatile<UsbCommand> {
         Volatile::new(self.operational_base.wrapping_add(EHCI_USBCMD_OFFSET) as *mut UsbCommand)
     }

     /// USB Status Register (USBSTS) erişimi
     pub unsafe fn usbsts(&self) -> Volatile<UsbStatus> {
         Volatile::new(self.operational_base.wrapping_add(EHCI_USBSTS_OFFSET) as *mut UsbStatus)
     }

     /// USB Interrupt Enable Register (USBINTREN) erişimi
     pub unsafe fn usbintren(&self) -> Volatile<UsbInterruptEnable> {
         Volatile::new(self.operational_base.wrapping_add(EHCI_USBINTREN_OFFSET) as *mut UsbInterruptEnable)
     }

     /// Port Status and Control Register (PORTSC) erişimi
     /// # Güvenlik
     /// `port_num` 1'den HCSPARAMS'ta belirtilen port sayısına kadar geçerli olmalıdır.
     pub unsafe fn portsc(&self, port_num: u8) -> Volatile<PortStatusControlFlags> {
         let port_offset = EHCI_PORTSC_BASE_OFFSET.wrapping_add(((port_num - 1) as usize) * mem::size_of::<u32>());
         Volatile::new(self.operational_base.wrapping_add(port_offset) as *mut PortStatusControlFlags)
     }

     /// Configure Flag Register (CONFIGFLAG) erişimi
      pub unsafe fn configflag(&self) -> Volatile<ConfigFlag> {
          Volatile::new(self.operational_base.wrapping_add(EHCI_CONFIGFLAG_OFFSET) as *mut ConfigFlag)
      }

      // TODO: Diğer Operational Registerlar için helper fonksiyonlar ekleyin (FRINDEX, CTRLDSSEGMENT, vb.)
}

// PortStatusControl struct'ına artık gerek yok, PORTSC registerına doğrudan
 EhciRegisters::portsc helper fonksiyonu ile Volatile<PortStatusControlFlags> olarak erişilir.
 impl PortStatusControl { ... } // Kaldırıldı
 #[repr(C)] struct PortStatusControl { ... } // Kaldırıldı


// --- Yardımcı Fonksiyon: Debug Çıktısı ---
 print_string fonksiyonu yerine Sahne64 console makroları kullanılır.
 fn print_string(s: &str) { ... } // Kaldırıldı


// Çekirdek giriş noktası. `_start` fonksiyonu genellikle çekirdeklerin başlangıç noktasıdır.
// Linker script tarafından çağrılır.
#[no_mangle]
pub extern "C" fn _start() -> ! {

    // Sahne64 konsol makrolarının std dışı ortamda çalışması için gerekli
    // ilk ayarlar burada veya platform başlangıcında yapılmalıdır.
    // Örnekte kprintln! Sahne64 makrolarını kullanıyor (varsayım).
    kprintln!("srcio_x86.rs çekirdek örneği başladı! (x86 EHCI)");

    // EHCI Host Controller Base Adresinin geçerli olduğu unsafe block içinde çalış.
    unsafe {
         // EHCI Host Controller kayıt yapılarını oluştur (base adresini ve caplength'i kullanarak)
         // EHCI Base adresinin doğru ve MMIO için erişilebilir olduğu varsayılır.
         let ehci_registers = EhciRegisters::new(EHCI_BASE_ADDRESS); // unsafe çağrı

         kprintln!("EHCI HCI Version: {:04x}", ehci_registers.hci_version.read()); // unsafe çağrı
         let hcs_params = ehci_registers.hcs_params.read(); // unsafe çağrı
         let hcc_params = ehci_registers.hcc_params.read(); // unsafe çağrı
         kprintln!("HCSPARAMS: {:08x} (Ports: {}), HCCPARAMS: {:08x}",
                   hcs_params.bits(), hcs_params.port_count(),
                   hcc_params.bits());


         // 1. Host Controller'ı Resetleme
         kprintln!("Host Controller Resetleniyor...");
         // USBCMD registerındaki HCR bitini set et (Write-1-to-set)
         ehci_registers.usbcmd().write(UsbCommand::HOST_CONTROLLER_RESET); // unsafe çağrı
         // HCR bitinin temizlenmesini bekle (Donanım reset bitince bit otomatik temizlenir)
         // veya USBSTS registerındaki HCH (Host Controller Halted) bitinin set olmasını bekle
         // Genellikle HCR bitinin temizlenmesini beklemek daha güvenlidir.
         while ehci_registers.usbcmd().read().contains(UsbCommand::HOST_CONTROLLER_RESET) { // unsafe çağrı
             spin_loop(); // Reset tamamlanana kadar bekle
             // TODO: Zaman aşımı ekle
         }
         kprintln!("Host Controller Resetlendi.");

         // 2. Host Controller'ı Durdur (varsa zaten çalışıyorsa) ve yapılandır
         // CONFIGFLAG registerını set et (Genellikle 1) - Root Hub'ın EHCI'a ait olduğunu belirtir.
          ehci_registers.configflag().write(ConfigFlag::CF); // unsafe çağrı
          kprintln!("CONFIGFLAG ayarlandı.");

         // 3. Host Controller'ı Çalıştırma (Run) moduna alma
         kprintln!("Host Controller Çalıştırma Moduna Alınıyor...");
         // USBCMD registerındaki Run/Stop (RS) bitini set et (Write-1-to-set)
         // Asenkron (ASE) ve Periyodik (PSE) çizelgeler henüz etkin değilse, HC Halted durumda kalabilir.
         // Bu örnekte çizelge yapıları kurulmadığı için sadece RS bitini set ediyoruz.
         ehci_registers.usbcmd().modify(|usbcmd| usbcmd.insert(UsbCommand::RUN_STOP)); // unsafe çağrı

         // USBSTS registerındaki HCH (Host Controller Halted) bitinin temizlenmesini bekle
         while ehci_registers.usbsts().read().contains(UsbStatus::COMMAND_STATUS) { // unsafe çağrı (HCH = Command Status)
             spin_loop(); // HC çalışmaya başlayana kadar bekle
             // TODO: Zaman aşımı ekle
         }
         kprintln!("Host Controller Çalışıyor.");

         // TODO: Bellek yönetimi için gerekli adımları burada uygulayın.
         // EHCI, Transfer Descriptor (TD), Queue Head (QH), Frame List gibi veri yapıları için DMA (Doğrudan Bellek Erişimi) kullanır.
         // Çekirdeğinizde DMA için uygun (coherent, hizalı) bellek alanları ayırmanız ve bu alanların fiziksel adreslerini
         // EHCI'nın ilgili registerlarına (örn. ASYNCLISTADDR, PERIODICLISTBASE) yazmanız GEREKİR.
         // Bu adımlar, bu basitleştirilmiş örneğin kapsamı dışındadır ve karmaşıktır.


         // 4. USB Aygıt Bağlantı Noktalarını (Portları) Kontrol Etme ve İşleme

         let num_ports = ehci_registers.hcs_params.read().port_count(); // unsafe çağrı
         kprintln!("Algılanan Port Sayısı: {}", num_ports);

         // Tüm portları tara
         for port_num in 1..=num_ports {
             // Port Status & Control Register'ını oku
             let portsc_register = ehci_registers.portsc(port_num); // unsafe çağrı
             let mut port_status = portsc_register.read(); // unsafe çağrı

             kprintln!("Port {} Durumu: {:08x}", port_num, port_status.bits());


             // Bağlantı Durumu Değişikliği (CSC) var mı?
             if port_status.contains(PortStatusControlFlags::CONNECT_STATUS_CHANGE) {
                  kprintln!("Port {} Bağlantı Durumu Değişikliği Algılandı.", port_num);

                 // Bağlantı durumu değişim bitini temizle (Write-1-to-Clear)
                  portsc_register.write(PortStatusControlFlags::CONNECT_STATUS_CHANGE); // unsafe çağrı
                  spin_loop(); // Yazmanın tamamlanması için kısa bekleme
                  kprintln!("Port {} CSC temizlendi.", port_num);

                  // Gerçek Bağlantı Durumunu (CCS) kontrol et
                  port_status = portsc_register.read(); // Durum tekrar okunmalı // unsafe çağrı
                  if port_status.contains(PortStatusControlFlags::CURRENT_CONNECT_STATUS) {
                      // Yeni aygıt bağlandı!
                      kprintln!("Yeni USB aygıt Port {}'e bağlandı!", port_num);
                      // Aygıtı başlatma ve yapılandırma adımları buraya gelecek.
                      handle_new_device(port_num, &ehci_registers); // unsafe çağrı
                  } else {
                      // Aygıt çıkarıldı.
                      kprintln!("USB aygıt Port {}'den çıkarıldı.", port_num);
                      // Aygıt ile ilgili kaynakları temizleme adımları buraya gelecek.
                      handle_device_removal(port_num);
                  }
             } else {
                 // Bağlantı durumu değişmedi, ama mevcut bağlı aygıtları da işleyebiliriz.
                  if port_status.contains(PortStatusControlFlags::CURRENT_CONNECT_STATUS) {
                      // Aygıt zaten bağlı ve daha önce işlenmemiş olabilir (veya periyodik kontrol yapılıyorsa).
                       kprintln!("Port {} üzerinde zaten bağlı bir aygıt var.", port_num);
                       // TODO: Gerekirse bağlı aygıtı işleme/yeniden numaralandırma mantığı ekleyin.
                  } else {
                       // Port boş.
                       kprintln!("Port {} Boş.", port_num);
                  }
             }

             // TODO: Diğer port değişikliklerini (Enable/Disable, Suspend, Over-Current, Reset) ve hataları (PEC, PSSC, OCC, PRSC) kontrol et ve işle.
              port_status.contains(PortStatusControlFlags::PORT_ENABLE_CHANGE)
              port_status.contains(PortStatusControlFlags::PORT_RESET_CHANGE) // etc.
              let change_bits = PortStatusControlFlags::PORT_ENABLE_CHANGE |
                                PortStatusControlFlags::PORT_SUSPEND_CHANGE |
                                PortStatusControlFlags::OVER_CURRENT_CHANGE |
                                PortStatusControlFlags::PORT_RESET_CHANGE;
             if port_status.intersects(change_bits) {
                 kprintln!("Port {} Üzerinde Başka Değişiklikler: {:08x}", port_num, port_status.intersects(change_bits).bits());
                 // İlgili değişiklik bitlerini temizle (W1C)
                 portsc_register.write(port_status.intersects(change_bits)); // unsafe çağrı
                 spin_loop(); // Kısa bekleme
             }
         } // Port döngüsü sonu

         // TODO: Kesme tabanlı bir sürücü için, USBINTREN registerını yapılandırın
         // ve bir kesme işleyicisi (IRQ handler) uygulayın. IRQ geldiğinde
         // USBSTS registerını okuyup hangi olayın (Transfer Tamamlanma, Port Değişikliği vb.)
         // gerçekleştiğini belirlemeli ve ilgili işlemleri (QH/TD işleme, port işleme vb.) yapmalıdır.
         // Polleme tabanlı bir sürücü ise, yukarıdaki port tarama döngüsü periyodik olarak çalıştırılmalıdır.


         // 5. USB Aygıt Yapılandırma ve Veri Transferi (Çok Basitleştirilmiş)
         // Yeni bir aygıt bağlandığında `handle_new_device` fonksiyonu çağrılacak.
         // Bu fonksiyon içinde aygıtın tanımlayıcılarını (descriptor) okuma,
         // adres atama, uç noktaları (endpoint) yapılandırma ve veri transferi işlemleri yapılacak.
         // Bu işlemlerin iskeleti `handle_new_device` içinde gösterilmiştir ancak
         // gerçek Host Controller programlama modelini yansıtmamaktadır.


    } // unsafe block sonu (Tüm HC erişimleri)


    kprintln!("srcio_x86.rs çekirdek örneği tamamlandı. Sonsuz döngüye giriliyor.");
    // Çekirdek sonsuz döngüde çalışmaya devam edecek.
    // Gerçek bir kernelde burası task scheduler veya event loop olurdu.
    loop {
         spin_loop(); // CPU'yu meşgul etmemek için
        // TODO: Kesme işleme (eğer kesme tabanlıysa) veya periyodik polleme (eğer polleme tabanlıysa).
        // TODO: Diğer kernel görevlerini çalıştır.
    }
}


// --- Yüksek Seviye İşleyiciler (Sadece İskeletler) ---

/// Yeni bağlanan USB aygıtını işleme (Örnek İskelet).
/// Numaralandırma (Enumeration) sürecini başlatır.
/// # Güvenlik
/// Donanım erişimi ve alt seviye transfer fonksiyonları içerdiği için 'unsafe'dır.
unsafe fn handle_new_device(port_num: u8, ehci_registers: &EhciRegisters) { // unsafe eklendi
    kprintln!("Port {} üzerindeki yeni aygıt işleniyor...", port_num);

    // Port Status Control Register'ını al
    let portsc_register = ehci_registers.portsc(port_num); // unsafe

    // 1. Portu Resetle (Port Reset - PR)
    // Cihaz bağlandığında HC tarafından otomatik resetlenmiş olabilir, veya yazılımla resetlememiz gerekebilir.
    // Port reset bitini set et (W1S). Donanım reset bitince biti otomatik temizler.
    kprintln!("Port {} Resetleniyor...", port_num);
    portsc_register.write(portsc_register.read().union(PortStatusControlFlags::PORT_RESET)); // Read-Modify-Write // unsafe
    // PR bitinin temizlenmesini bekle
    while portsc_register.read().contains(PortStatusControlFlags::PORT_RESET) { // unsafe
         spin_loop();
         // TODO: Zaman aşımı ekle (USB spec'e göre reset süresi)
    }
    kprintln!("Port {} Reset Tamamlandı.", port_num);

     // Reset sonrası Port Enable (PE) biti set olmalı ve hız belirlenmeli.
     let mut port_status_after_reset = portsc_register.read(); // unsafe
     kprintln!("Port {} Reset Sonrası Durum: {:08x}", port_num, port_status_after_reset.bits());

     // Hata/Değişiklik bayraklarını temizle (W1C) - Reset sonrası set olabilen CSC, PEC, OCC gibi
     let change_status_bits = PortStatusControlFlags::CONNECT_STATUS_CHANGE |
                              PortStatusControlFlags::PORT_ENABLED_CHANGE |
                              PortStatusControlFlags::OVER_CURRENT_CHANGE |
                              PortStatusControlFlags::PORT_RESET_CHANGE;
     portsc_register.write(change_status_bits); // unsafe (Sadece W1C bitlerini yazar)
     spin_loop(); // Kısa bekleme

     // Port Enable (PE) bitinin set olduğunu kontrol et (reset başarılı olduysa set olmalı)
     port_status_after_reset = portsc_register.read(); // Tekrar oku // unsafe
     if !port_status_after_reset.contains(PortStatusControlFlags::PORT_ENABLED_DISABLED) { // PES bitini kontrol et (0 = Enabled)
         kprintln!("HATA: Port {} Reset Sonrası Etkinleşmedi!", port_num);
         // TODO: Hata durumunu işle, panik veya kurtarma.
         return; // İşlemeye devam etme
     }
     kprintln!("Port {} Etkinleşti.", port_num);


    // 2. Aygıt hızını belirleme (Port Status Control Register'dan)
     let port_speed = portsc_register.read().speed(); // unsafe çağrı
     kprintln!("Port {} Hızı: {:?}", port_num, port_speed);

     // EHCI genellikle sadece High-Speed cihazları doğrudan işler. Low-Speed/Full-Speed cihazlar
     // companion (OHCI/UHCI) Host Controller'a devredilir. Bu, PORTSC'deki Port Owner (PO) biti ile kontrol edilir.
     if port_status_after_reset.contains(PortStatusControlFlags::PORT_OWNER) {
         kprintln!("Port {} Companion HC'ye devredildi. EHCI tarafından işlenmeyecek.", port_num);
         // TODO: Companion HC sürücüsüne bu portu işlemesini söyle.
         return; // EHCI sürücüsü burada işlemeye devam etmez.
     }

     // EHCI'ya ait bir cihaz (High-Speed veya zorla Full-Speed)
     kprintln!("Port {} EHCI'a ait. Numaralandırma başlatılıyor...", port_num);


    // 3. Aygıta geçici adres atama (Set Address komutu - Kontrol transferi ile)
    // Yeni resetlenmiş cihazlar adres 0'da iletişim kurar. İlk komut SET_ADDRESS olmalıdır.
    // Geçici adres (örneğin 1) atanır. Adres 0 kontrol transferleri için kullanılır.
    let temporary_device_address: u8 = 1; // Örnek adres
     // send_set_address_command(ehci_registers, port_num, temporary_device_address); // unsafe çağrı
     kprintln!("Port {} aygıtına geçici adres {} atanıyor... (Örnek - SET_ADDRESS transferi gerekli)", port_num, temporary_device_address);
     // TODO: SET_ADDRESS kontrol transferini gerçek kontrol transfer fonksiyonu ile yap.

     // SET_ADDRESS komutundan sonra aygıt yeni adresine geçer. Daha sonraki transferler bu yeni adresi kullanır.
     let device_address = temporary_device_address; // Artık bu adresi kullanacağız.


    // 4. Aygıt tanımlayıcılarını (descriptor) okuma (Get Descriptor komutu - Kontrol transferi ile)
    // Endpoint 0 üzerinden (Kontrol Endpoint) GET_DESCRIPTOR kontrol transferleri yapılır.
    // Önce Device Descriptor'ın ilk 8 baytı okunur (maks paket boyutu ve tam descriptor uzunluğu için).
    // Sonra tüm Device Descriptor (18 bayt) okunur.
    // Ardından Configuration Descriptor(lar) okunur.
    // ... ve diğer descriptor'lar.

    // TODO: Descriptor okuma transferlerini gerçek kontrol transfer fonksiyonu ile yap.
    // Aygıt numaralandırma süreci çok detaylıdır ve bu örnek iskelette tam olarak gösterilemez.
     get_usb_device_descriptor(device_address); // Varsayımsal fonksiyon çağrısı


    // 5. Yapılandırma seçme (Set Configuration komutu - Kontrol transferi ile)
    // Cihazın desteklediği yapılandırmalardan biri seçilir ve SET_CONFIGURATION komutu gönderilir.
    // TODO: SET_CONFIGURATION kontrol transferini yap.


    // 6. Aygıt sınıfını belirleme ve uygun sürücüyü yükleme
    // Okunan descriptor'lardan (Device, Interface) cihaz sınıfı (HID, MSC, CDC vb.) belirlenir.
    // İlgili sınıf sürücüsü başlatılır ve bu cihaza bağlanır.
    // TODO: Sınıf belirleme ve sürücü yükleme mantığı ekleyin.


    // 7. Veri transferi (Bulk veya Interrupt transferleri ile - Uç noktalara göre)
    // Cihaz sınıfına göre gerekli veri transferleri (Bulk IN/OUT, Interrupt IN/OUT)
    // Host Controller'ın DMA ve çizelge yapıları (Asenkron/Periyodik listeler) kullanılarak yönetilir.
    // TODO: Veri transfer mekanizmalarını (TD/QH oluşturma, listelere ekleme, durum takibi) uygulayın.


    kprintln!("Port {} üzerindeki aygıt numaralandırma iskeleti tamamlandı. (Adres: {})\n", port_num, device_address);
}

/// Aygıt çıkarıldığında yapılacak işlemler (Örnek İskelet).
fn handle_device_removal(port_num: u8) {
    kprintln!("Port {} üzerindeki aygıt çıkarıldı. (Örnek Temizlik)", port_num);
    // TODO: Bu portla ilişkili tüm kaynakları serbest bırakın (DMA bellekleri, TD/QH yapıları, sürücü instance'ları vb.).
    // İlgili uç noktaları ve çizelge girdilerini HC çizelgelerinden kaldırın.
}


// --- USB Kontrol Transfer Fonksiyonları (Çok Basitleştirilmiş Örnekler) ---

#[derive(Debug)]
enum TransferError {
    Timeout,
    Stall,
    Nak, // NAK yanıtı alındı
    DataToggleError,
    Babble, // Çok fazla veri gönderildi/alındı
    TransactionError, // Diğer USB transaction hataları
    // ... diğer EHCI spesifik hatalar ...
}

/// USB Kontrol Transferi (IN veya OUT) için basitleştirilmiş iskelet.
/// **DİKKAT**: Bu fonksiyon ÇOK BASİT bir örnek iskelettir ve gerçek bir EHCI sürücüsü için
/// YETERSİZDİR. Gerçek Host Controller programlama modelini yansıtmaz.
/// EHCI için Transfer Descriptor (TD) ve Queue Head (QH) yapıları hazırlanır,
/// DMA kullanılır ve HC'nin Asenkron listesine eklenir.
/// # Güvenlik
/// Alt seviye Host Controller erişimi, DMA arabellekleri (data_buffer) ve karmaşık zamanlama/hata yönetimi içerdiği için 'unsafe'dır.
/// data_buffer pointer'ı geçerli olmalı ve DMA için uygun (coherent) bellek alanını işaret etmelidir.
 unsafe fn control_transfer( // Fonksiyon adı control_transfer olarak düzeltildi
    ehci_registers: &EhciRegisters,
    device_address: u8, // Aygıtın atanmış adresi (adres 0 hariç)
    request_type: u8,   // bmRequestType
    request: u8,        // bRequest
    value: u16,         // wValue
    index: u16,         // wIndex
    length: u16,        // wLength (veri aşamasında beklenen/gönderilecek veri uzunluğu)
    data_buffer: *mut u8, // Veri aşaması için arabellek (NULL olabilir)
    buffer_size: usize, // Sağlanan tamponun gerçek boyutu (veri_buffer NULL değilse)
) -> Result<usize, TransferError> { // Kaç bayt transfer edildiğini de dönebiliriz.

    let is_in_transfer = (request_type & 0x80) != 0; // bmRequestType'ın D7 biti: 1=IN, 0=OUT
    kprintln!("Kontrol Transferi Başlatılıyor (DevAddr: {}, {} {} Req: {:02x}, wValue: {:04x}, wIndex: {:04x}, wLength: {})",
        device_address,
        if is_in_transfer { "IN" } else { "OUT" },
        match request_type & 0x60 { // Mask bits 6:5 for Type
            0x00 => "Standard",
            0x20 => "Class",
            0x40 => "Vendor",
            _   => "Reserved",
        },
        request, value, index, length);


    // **DİKKAT: AŞAĞIDAKİ KOD GERÇEK BİR EHCI/USB KONTROLCÜ SÜRÜCÜSÜ DEĞİLDİR.**
    // **BU SADECE KAVRAMLARI TEMSİL ETMEK İÇİN YAZILMIŞ HAYALİ KODDUR.**
    // **GERÇEK DONANIMINIZIN DATASHEET'İNE VE EHCI SPESİFİKASYONUNA GÖRE TAMAMEN YENİDEN YAZILMASI GEREKİR.**
    // **EHCI programlama modeli QH (Queue Head), TD (Transfer Descriptor) yapıları ve DMA üzerine kuruludur.**

    // Kontrol transferi 3 aşamadan oluşur: SETUP (Host -> Device), DATA (IN veya OUT), STATUS (karşı yön).
    // Her aşama için bir TD (Transfer Descriptor) veya zincirlenmiş TD'ler kullanılır.
    // Tüm transfer, bir QH (Queue Head) yapısı altında yönetilir.
    // QH'ler ve TD'ler DMA için uygun (coherent, hizalı) bellek alanlarında bulunur.

    // ÖRNEK: QH/TD yapılarını bellekte hazırla (HAYALİ DMA BELLEĞİ)
     let setup_td = allocate_dma_safe_memory(size_of::<EhciTransferDescriptor>()); // Varsayımsal bellek tahsisi
     let data_tds = allocate_dma_safe_memory(size_of::<EhciTransferDescriptor>() * num_data_packets); // length'e göre hesaplanır
     let status_td = allocate_dma_safe_memory(size_of::<EhciTransferDescriptor>());
     let dma_data_buffer = allocate_dma_safe_memory(length as usize); // Veri için DMA tamponu

    // TODO: SETUP TD'sini oluştur (setup_packet içeriği, uzunluğu=8, PID=SETUP, Data Toggle=0)
    // TODO: DATA TD(ler)ini oluştur (dma_data_buffer adresi, uzunluğu=length, PID=IN/OUT, Data Toggle=1 (toggle edilecek))
    // TODO: STATUS TD'sini oluştur (uzunluk=0, PID=IN/OUT (veri aşamasının tersi), Data Toggle=1 (toggle edilecek))
    // TODO: Bu TD'leri birbirine zincirle (next_td pointerları, son TD'nin next_td'si NULL veya Terminate)
    // TODO: Endpoint 0 (Kontrol Endpoint) için bir Queue Head (QH) yapısı oluştur (Device Address, Endpoint Num=0, Hız, Max Packet Size 0).
    // TODO: QH'ye ilk TD'nin (SETUP TD) fiziksel adresini yaz.
    // TODO: QH'nin fiziksel adresini HC'nin Asenkron Listesine ekle (ASYNCLISTADDR registerı ve listeyi yöneten yapılar aracılığıyla). QH'ler de birbirine zincirlenir.
    // TODO: USBCMD registerındaki Asenkron Liste Etkinleştirme (ASE) bitini set et.
    // TODO: IAAD (Interrupt on Async Advance Doorbell) biti set ederek transferi tetikle (isteğe bağlı, ASE tek başına da tetikleyebilir).


    // 1. SETUP Paketi Hazırlama (8 bayt)
    let setup_packet: [u8; 8] = [ // USB Kontrol Transfer Setup Paketi (8 bayt, Little-endian alanlar)
        request_type,
        request,
        value as u8,        // wValue (low byte)
        (value >> 8) as u8, // wValue (high byte)
        index as u8,        // wIndex (low byte)
        (index >> 8) as u8, // wIndex (high byte)
        length as u8,       // wLength (low byte)
        (length >> 8) as u8, // wLength (high byte)
    ];

     // Örnek: Setup paketini DMA tamponuna kopyala (Eğer setup_packet ayrı bir tampon gerektiriyorsa)
      ptr::copy_nonoverlapping(setup_packet.as_ptr(), setup_dma_buffer_ptr, 8); // Varsayımsal


    // 2. Veri Aşaması (Data Stage) Hazırlama
    // Eğer length > 0 ise, Veri aşaması (IN veya OUT) için TD(ler) hazırlanır.
    // Eğer OUT transferi ise, data_buffer'daki veri önceden dma_data_buffer'a kopyalanmalıdır.
    if length > 0 && !data_buffer.is_null() && !is_in_transfer {
         // TODO: OUT transferi için, data_buffer'dan dma_data_buffer'a veriyi kopyala (ve cache/DMA senkronizasyonunu yap).
          ptr::copy_nonoverlapping(data_buffer, dma_data_buffer_ptr, length as usize); // Varsayımsal
    }

    // 3. Transferin Başlatılması ve Tamamlanması için Bekleme (Polling Örneği)
    // USBCMD registerındaki çizelge etkinleştirme bitlerini set et (PSE ve/veya ASE)
    // İlgili kesme durum bitlerinin (USBINT, EINT, IAA) veya HC Halted bitinin (HCH) değişmesini bekle.
    // Veya TD/QH yapılarındaki durum bitlerini (örn. Active bit, Error bits) polleyerek transferin tamamlanmasını kontrol et.
    unsafe { // unsafe block for EHCI register access
         let usbcmd_reg = ehci_registers.usbcmd();
         let usbsts_reg = ehci_registers.usbsts();

         // Asenkron listeyi etkinleştir (Kontrol transferleri Asenkron listede işlenir)
         usbcmd_reg.modify(|cmd| cmd.insert(UsbCommand::ASYNCHRONOUS_SCHEDULE_ENABLE)); // unsafe
          kprintln!("Asenkron Çizelge Etkinleştirildi.");


        // TODO: Transfer tamamlanması için bekleme döngüsü (TD durumunu polleyerek veya USBSTS'yi polleyerek/kesmeyle)
        let mut timeout = 10_000_000; // Örnek zaman aşımı sayacı
        let completion_status_bits = UsbStatus::USB_INTERRUPT | UsbStatus::ERROR_INTERRUPT | UsbStatus::HOST_SYSTEM_ERROR; // Tamamlanma/Hata bitleri

        let mut status = usbsts_reg.read(); // unsafe
         // Transfer tamamlanma veya hata bitlerinden biri set olana kadar bekle
         while status.intersects(completion_status_bits) == false && timeout > 0 {
             spin_loop(); // Basit polleme
             status = usbsts_reg.read(); // unsafe
             timeout -= 1;
         }

        if timeout == 0 {
             kprintln!("HATA: Kontrol Transferi Zaman Aşımı!");
             // TODO: Zaman aşımı durumunda temizlik ve kurtarma (TD/QH serbest bırakma, HC durdurma vb.)
             return Err(TransferError::Timeout);
        }

         kprintln!("Transfer Tamamlanma Durumu (USBSTS): {:08x}", status.bits());

        // EHCI'da USBSTS'deki kesme bitlerini temizle (W1C)
         usbsts_reg.write(status.intersects(completion_status_bits)); // unsafe
         kprintln!("USBSTS temizlendi.");

        // TODO: TD/QH yapılarından transferin gerçek durumunu ve hata kodlarını oku.
        // Bu, transferin başarılı olup olmadığını, kaç baytın transfer edildiğini ve
        // hangi hata (STALL, NAK vb.) oluştuğunu kesin olarak belirleyen yerdir.
        // Transfer Result (TR) alanı TD'lerde kontrol edilmelidir.


        // **ÖRNEK: Başarılı veya Hata Simülasyonu (TD/QH okumadan)**
        // Gerçek uygulamada, aşağıdaki başarı/hata kontrolü TD durumuna göre yapılmalıdır.
         if status.contains(UsbStatus::ERROR_INTERRUPT) || status.contains(UsbStatus::HOST_SYSTEM_ERROR) /* || TD'de hata bayrakları set */ {
             kprintln!("HATA: Kontrol Transferinde Donanım Hatası!");
             // TODO: TD'den spesifik hata kodunu oku (örn. STALL, NAK)
              Err(TransferError::Stall) // Örnek: STALL hatası
             Err(TransferError::TransactionError) // Genel işlem hatası
         } else {
             // Transferin başarılı olduğunu varsay (TD durumuna göre kontrol edilmeli)
              let transferred_byte_count = length as usize; // Örnek: İstendiği kadar bayt transfer edildiğini varsay
              kprintln!("Kontrol Transferi Başarılı. {} bayt transfer edildi.", transferred_byte_count);

             // 4. Veri Aşaması (Data Stage) Veri Kopyalama (IN Transferi)
             // Eğer IN transferi ise ve veri alındıysa, veri dma_data_buffer'dan
             // sağlanan data_buffer'a kopyalanmalıdır. Cache/DMA senkronizasyonu gerekebilir.
             // Bazı durumlarda, sağlanan data_buffer doğrudan DMA için kullanılabilir.
             if is_in_transfer && !data_buffer.is_null() && transferred_byte_count > 0 {
                  if transferred_byte_count <= buffer_size {
                       // Veri zaten DMA ile data_buffer'a yazıldıysa bu adım gereksiz olabilir.
                       // Eğer DMA farklı bir tampona yazdıysa, o tampondan data_buffer'a kopyalama burada yapılmalıdır.
                        ptr::copy_nonoverlapping(dma_source_ptr, data_buffer, transferred_byte_count); // Varsayımsal kopyalama
                       // TODO: Cache invalidasyonu (eğer DMA tamponu cache edilebilir bellekteyse)
                       kprintln!("Alınan veri ({} bayt) data_buffer'a kopyalandı (varsayımsal).", transferred_byte_count);
                  } else {
                       kprintln!("HATA: Kontrol Transferi Alınan Boyut Tampondan Büyük! (Okunan: {}, Tampon: {})", transferred_byte_count, buffer_size);
                       // Bu bir faz hatası (Phase Error) veya yazılım hatası olabilir.
                       // TODO: Kurtarma işlemi (QH/TD temizliği vb.)
                       return Err(TransferError::TransactionError); // Yetersiz Tampon Boyutu Hatası gibi spesifik bir hata dönebilir.
                  }
             }

             // 5. Kaynakları Temizleme
             // TODO: Kullanılan TD/QH yapılarını serbest bırak (DMA bellek havuzuna geri döndür).
             // HC çizelgesinden QH'yi kaldır (eğer tek kullanımlıksa veya hata olduysa).

             Ok(transferred_byte_count) // Başarılı dönüş ve transfer edilen bayt sayısı
         }
    } // unsafe block sonu (Transfer bekleme ve durum kontrolü)


    // **DİKKAT**: Bu fonksiyonun hata yönetimi, kesme işleme, zaman aşımları,
    //       DMA yönetimi, PID senkronizasyonu, paket bölme gibi
    //       kritik kısımları ÇOK BASİTTİR ve GERÇEK BİR UYGULAMA İÇİN YETERSİZDİR.
    //       Gerçek bir sürücüde bu bölümler çok daha detaylı ve sağlam olmalıdır.
}

 // control_transfer_in fonksiyonu kaldırıldı, control_transfer genel fonksiyonu kullanıldı.
  fn control_transfer_in(...) { ... } // Kaldırıldı


// SET_ADDRESS komutunu gönderme fonksiyonu (Kontrol Transferi kullanan basitleştirilmiş iskelet)
/// Belirtilen aygıta adres atar.
/// # Güvenlik
/// Alt seviye kontrol transfer fonksiyonu çağırdığı için 'unsafe'dır.
 unsafe fn send_set_address_command(ehci_registers: &EhciRegisters, port_num: u8, device_address: u8) -> Result<(), TransferError> { // unsafe eklendi
    kprintln!("Port {} aygıtına adres {} atanıyor... (SET_ADDRESS transferi)", port_num, device_address);

    // SET_ADDRESS standart cihaz isteğidir.
     bmRequestType: 0000 0000b (Standard | Device | Host-to-Device)
     bRequest: 0x05 (SET_ADDRESS)
     wValue: Yeni aygıt adresi (high byte 0 olmalı)
     wIndex: 0x0000
     wLength: 0x0000 (Veri aşaması yok)

    // Aygıt, SET_ADDRESS komutunu Status aşaması tamamlandıktan sonra kabul eder.
    // Bu aşama tamamlanana kadar önceki adresi kullanır.
    // Status aşaması tamamlandıktan sonra aygıt yeni adrese geçer.

     control_transfer(
        ehci_registers,
        0, // Yeni resetlenmiş cihazlar adres 0'da başlar
        USB_REQ_TYPE_STANDARD_DEVICE_OUT, // 0x00
        USB_REQ_SET_ADDRESS, // 0x05
        device_address as u16, // Yeni adres wValue'da
        0, // wIndex = 0
        0, // wLength = 0 (Veri aşaması yok)
        ptr::null_mut(), // Veri tamponu NULL
        0 // Tampon boyutu 0
     ); // unsafe çağrı

    // TODO: Gerçek control_transfer fonksiyonu ile SET_ADDRESS komutunu gönderin.
    // control_transfer başarıyla tamamlandıysa, aygıt yeni adresi kullanmaya başlayacaktır.

    kprintln!("Port {} aygıtına adres {} atama transferi tamamlandı (SIMULE EDILDI!).", port_num, device_address);
    // Başarılı olduğunu varsayıyoruz (SIMULE EDILDI!)
    Ok(()) // Başarılı olduğunu varsayarak dön
 }


// --- Placeholder Register Structları (bitflags dışında kalanlar) ---

// EHCI Capabilities Registerları içinde yer alabilecek diğer structlar (eğer gerekirse)
 struct HciVersion(u16); // Zaten Volatile<u16> ile erişiliyor
 struct HcsParams(u32); // Zaten bitflags olarak tanımlı
 struct HccParams(u32); // Zaten bitflags olarak tanımlı

// EHCI Operational Registerları içinde yer alabilecek diğer structlar (eğer gerekirse)
 struct FrIndex(u32); // Zaten Volatile<u32> ile erişiliyor
 struct CtrlDsSegment(u32); // Zaten Volatile<u32> ile erişiliyor
 struct ConfigFlag(u32); // Zaten bitflags olarak tanımlı
