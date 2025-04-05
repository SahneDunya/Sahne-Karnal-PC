#![no_std]
#![no_main]

// Çekirdek geliştirme ortamı için gerekli özellikler ve kütüphaneler buraya eklenecek.
// Örneğin, belirli bir hedef mimari (x86 için 'i686-pc-windows-gnu' veya 'x86_64-unknown-none')
// ve bazı çekirdek seviyesi yardımcı kütüphaneler.

// Örneğin, `volatile` kütüphanesi donanım registerlerine erişim için kullanılabilir.
use volatile::Volatile;

// Panik durumunda ne yapılacağını tanımlayan fonksiyon.
// Çekirdek panik olduğunda bu fonksiyon çağrılır.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Çekirdek giriş noktası. `_start` fonksiyonu genellikle çekirdeklerin başlangıç noktasıdır.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. USB Host Controller'ı (Örn: EHCI, xHCI) Başlatma ve Tanımlama

    // USB host controller genellikle MMIO (Memory-Mapped I/O) veya Port I/O aracılığıyla
    // erişilebilen registerlara sahiptir.
    // x86 mimarisinde, bu registerların fiziksel adresleri genellikle belirli bellek aralıklarında
    // bulunur ve anakart üreticisi tarafından belirlenir.
    // Bu adresleri anakart dökümantasyonundan veya yonga seti özelliklerinden bulmanız gerekir.

    // ÖRNEK: EHCI (Enhanced Host Controller Interface) base adresini varsayalım (BU TAMAMEN ÖRNEK, GERÇEK ADRES DEĞİL!)
    const EHCI_BASE_ADDRESS: usize = 0xFEDC_0000; // ÖRNEK ADRES! GERÇEK SİSTEMİNİZE GÖRE DEĞİŞİR!

    // Base adresi kullanarak EHCI registerlarına erişmek için `Volatile` kullanabiliriz.
    // `Volatile` Rust'ın optimizasyonlarının donanım registerlerine erişimi atlamasını engeller.
    let ehci_registers = EhciRegisters::new(EHCI_BASE_ADDRESS);

    // EHCI Controller'ı başlatma adımları (çok basitleştirilmiş örnek):
    // Gerçek bir başlatma süreci USB standardına ve host controller spesifikasyonuna göre çok daha karmaşıktır.

    // 1. Host Controller'ı Resetleme
    unsafe { ehci_registers.usbcmd.write(UsbCommand::HOST_CONTROLLER_RESET); }
    while unsafe { ehci_registers.usbcmd.read() }.contains(UsbCommand::HOST_CONTROLLER_RESET) {
        // Reset bit'i temizlenene kadar bekle
    }

    // 2. Host Controller'ı Durdurma (varsa zaten çalışıyorsa)
    unsafe { ehci_registers.usbcmd.modify(|usbcmd| usbcmd.remove(UsbCommand::RUN_STOP)); }
    while unsafe { ehci_registers.usbsts.read() }.contains(UsbStatus::HOST_CONTROLLER_HALTED) == false {
        // Host Controller'ın durmasını bekle
    }

    // 3. Frame List Base Address ve diğer gerekli registerları ayarlama (çok basitleştirilmiş)
    // ... (Gerçekte, bu adımlar çok daha karmaşık ve host controller tipine özeldir)

    // 4. Host Controller'ı Çalıştırma
    unsafe { ehci_registers.usbcmd.modify(|usbcmd| usbcmd.insert(UsbCommand::RUN_STOP)); }


    // 2. USB Aygıt Bağlantı Noktalarını (Portları) Kontrol Etme

    // USB port durum registerlarını okuyarak aygıt bağlantılarını kontrol edebiliriz.
    // Örneğin, Port Status & Control Register'ı (PortSC) kullanarak.

    for port_num in 1..=ehci_registers.hccparams.read().port_count() { // Varsayılan port sayısı, gerçekte dinamik olarak alınmalı.
        let portsc_offset = 0x44 + (port_num - 1) * 4; // Port Status Control Register offset'i (EHCI'de)
        let portsc_address = EHCI_BASE_ADDRESS + portsc_offset;
        let portsc_register = PortStatusControl::new(portsc_address);

        let port_status = unsafe { portsc_register.read() };

        if port_status.contains(PortStatusControlFlags::CONNECT_STATUS_CHANGE) {
            // Bağlantı durumu değişti, yeni bir aygıt bağlanmış veya çıkarılmış olabilir.
            if port_status.contains(PortStatusControlFlags::CONNECT_STATUS) {
                // Yeni aygıt bağlandı!
                // [Image of Yeni USB aygıt bağlandı uyarısı]
                print_string("Yeni USB aygıt bağlandı!\n");
                // Aygıtı başlatma ve yapılandırma adımları buraya gelecek.
                handle_new_device(port_num, &ehci_registers);
            } else {
                // Aygıt çıkarıldı.
                // [Image of USB aygıt çıkarıldı uyarısı]
                print_string("USB aygıt çıkarıldı!\n");
                // Aygıt ile ilgili kaynakları temizleme adımları buraya gelecek.
                handle_device_removal(port_num);
            }
            // Bağlantı durumu değişim bitini temizle (Write-1-to-Clear özelliği olabilir)
            unsafe { portsc_register.write(PortStatusControlFlags::CONNECT_STATUS_CHANGE); }
        }
    }


    // 3. USB Aygıt Yapılandırma ve Veri Transferi (Çok Basitleştirilmiş)

    // Yeni bir aygıt bağlandığında `handle_new_device` fonksiyonu çağrılacak.
    // Bu fonksiyon içinde aygıtın tanımlayıcılarını (descriptor) okuma,
    // adres atama, uç noktaları (endpoint) yapılandırma ve veri transferi işlemleri yapılacak.

    // ... (Aygıt yapılandırma ve veri transferi işlemleri çok daha karmaşık ve USB protokolüne özeldir)


    // Örnek olarak, basit bir metin yazdırma fonksiyonu (VGA metin modu veya seri port üzerinden)
    loop {} // Çekirdek sonsuz döngüde çalışmaya devam edecek.
}


fn handle_new_device(port_num: u8, ehci_registers: &EhciRegisters) {
    // Yeni bağlanan aygıtı işleme adımları:
    // 1. Port'u etkinleştirme (Port Status Control Register)
    let portsc_offset = 0x44 + (port_num - 1) * 4;
    let portsc_address = ehci_registers.base_address + portsc_offset;
    let portsc_register = PortStatusControl::new(portsc_address);
    unsafe { portsc_register.modify(|portsc| portsc.insert(PortStatusControlFlags::PORT_ENABLE)); }

    // 2. Aygıt hızını belirleme (Port Status Control Register'dan)
    let port_speed = unsafe { portsc_register.read() }.speed();
    print_string(&format!("Port {} Hızı: {:?}\n", port_num, port_speed));

    // 3. Aygıta adres atama (Set Address komutu - Kontrol transferi ile)
    let device_address: u8 = 10; // Örnek adres
    send_set_address_command(ehci_registers, port_num, device_address);


    // 4. Aygıt tanımlayıcılarını (descriptor) okuma (Get Descriptor komutu - Kontrol transferi ile)
    //    - Aygıt tanımlayıcısı (Device Descriptor)
    //    - Yapılandırma tanımlayıcıları (Configuration Descriptor)
    //    - Arayüz tanımlayıcıları (Interface Descriptor)
    //    - Uç nokta tanımlayıcıları (Endpoint Descriptor)
    //    ...

    // Örnek: Aygıt tanımlayıcısını (Device Descriptor) okuma (ÇOK BASITLEŞTIRILMIŞ ÖRNEK)
    let device_descriptor_buffer: [u8; 8] = [0; 8]; // En azından ilk 8 byte için buffer.
    let descriptor_type = DescriptorType::Device;
    let descriptor_index = 0;
    let transfer_result = control_transfer_in(
        ehci_registers,
        device_address,
        descriptor_type,
        descriptor_index,
        device_descriptor_buffer.as_mut_ptr(),
        device_descriptor_buffer.len() as u16,
    );

    match transfer_result {
        Ok(_) => {
            print_string("Aygıt Tanımlayıcısı Okundu!\n");
            // Descriptor verilerini işle
            // ...
        }
        Err(error) => {
            print_string(&format!("Aygıt Tanımlayıcısı Okuma Hatası: {:?}\n", error));
        }
    }


    // 5. Yapılandırma seçme (Set Configuration komutu - Kontrol transferi ile)
    // ...

    // 6. Aygıt ile veri transferi (Bulk veya Interrupt transferleri ile - Uç noktalara göre)
    // ...


    print_string(&format!("Port {} üzerindeki aygıt işleniyor. (Adres: {})\n", port_num, device_address));
}

fn handle_device_removal(port_num: u8) {
    // Aygıt çıkarıldığında yapılacak işlemler (kaynakları serbest bırakma, vb.)
    print_string(&format!("Port {} üzerindeki aygıt çıkarıldı.\n", port_num));
}


// --- USB Kontrol Transfer Fonksiyonları (Çok Basitleştirilmiş Örnekler) ---

#[derive(Debug)]
enum TransferError {
    Timeout,
    Stall,
     інші_помилки, // Diğer hatalar
}

fn control_transfer_in(
    ehci_registers: &EhciRegisters,
    device_address: u8,
    descriptor_type: DescriptorType,
    descriptor_index: u16,
    buffer_ptr: *mut u8,
    buffer_len: u16,
) -> Result<(), TransferError> {
    // ... (Gerçek kontrol transferi implementasyonu çok daha karmaşık ve EHCI/xHCI spesifikasyonlarına bağlıdır)

    // BU SADECE ÇOK BASITLEŞTIRILMIŞ BIR ÖRNEK KOD PARÇASI!
    // GERÇEK BIR IMPLEMENTASYON ÇOK DAHA FAZLA DETAY VE HATA YÖNETIMI GEREKTIRIR.

    print_string("Kontrol Transfer Giriş (IN) işlemi başlatıldı...\n");

    // 1. Transfer Descriptor (TD) oluşturma ve ayarlama
    //    - Data Buffer Pointer
    //    - Transfer Boyutu
    //    - Kontrol Bitleri (PID, Data Toggle, vb.)
    //    ...

    // 2. Queue Head (QH) oluşturma ve ayarlama
    //    - Endpoint Özellikleri (Hız, Maksimum Paket Boyutu, Adres, Endpoint Numarası, Tür)
    //    - Current TD Pointer
    //    - Overlay Area (Transfer durumu, vb. için)
    //    ...

    // 3. QH ve TD'leri bellek üzerinde uygun konumlara yerleştirme ve birbirlerine bağlama (linked list gibi)
    // ...

    // 4. Endpoint'i çalıştırma (Endpoint List Head Register veya benzeri aracılığıyla)
    // ...

    // 5. Transferin tamamlanmasını bekleme (Interrupt veya polling ile Status Register'ları kontrol etme)
    //    - Timeout mekanizması eklemek önemlidir.

    // 6. Transfer durumunu kontrol etme (TD Overlay Area'dan veya Status Register'lardan)
    //    - Başarılı mı, hata oluştu mu (Stall, Timeout, vb.)?

    // 7. Veriyi buffer'a kopyalama (başarılıysa)
    //    ...

    // 8. Kaynakları temizleme (TD, QH, vb.)

    // ÖRNEK: Başarılı transfer simülasyonu (GERÇEK IMPLEMENTASYON DEĞIL!)
    print_string("Kontrol Transfer Giriş (IN) işlemi BAŞARILI (SIMULE EDILDI!).\n");
    Ok(())

    // ÖRNEK: Hata simülasyonu (GERÇEK IMPLEMENTASYON DEĞIL!)
    // Err(TransferError::Timeout)
}


fn send_set_address_command(ehci_registers: &EhciRegisters, port_num: u8, device_address: u8) {
    // ... (Set Address komutunu gönderme implementasyonu - Kontrol transferi kullanarak)
    // BU DA SADECE BIR YER TUTUCU! GERÇEK IMPLEMENTASYON ÇOK DAHA DETAYLI OLACAK.
    print_string(&format!("Port {} aygıtına adres {} atanıyor (SIMULE EDILDI!).\n", port_num, device_address));
    // Gerçek implementasyonda:
    // - Setup paketini oluştur (SET_ADDRESS komutu için)
    // - Data paketini (adres bilgisi) oluştur
    // - Status aşamasını yönet
    // - Kontrol transferini gerçekleştir (control_transfer_out veya benzeri bir fonksiyon kullanarak)
}


// --- Yardımcı Fonksiyonlar ve Yapılar ---

#[repr(C)] // C yapısı düzenini garanti etmek için
struct EhciRegisters {
    usbcmd: Volatile<UsbCommand>,         // 0x00
    usbsts: Volatile<UsbStatus>,         // 0x04
    usbintr: Volatile<UsbInterrupt>,        // 0x08
    frindex: Volatile<FrameIndex>,         // 0x0C
    ctrl_ds_segment_base_addr: Volatile<u32>, // 0x10
    configflag: Volatile<ConfigFlag>,        // 0x14
    portsc1: PortStatusControl,          // 0x44 (Port 1 için, diğer portlar için +4 offset)
    // ... diğer registerlar ...
    hccparams: Volatile<HccParams>,       // Host Controller Capabilities Parameter Register
    // ... diğer registerlar ...
    base_address: usize, // Base adres bilgisi (fonksiyonlarda kullanmak için)
}

impl EhciRegisters {
    fn new(base_address: usize) -> Self {
        EhciRegisters {
            usbcmd: Volatile::new(base_address as *mut UsbCommand),
            usbsts: Volatile::new((base_address + 0x04) as *mut UsbStatus),
            usbintr: Volatile::new((base_address + 0x08) as *mut UsbInterrupt),
            frindex: Volatile::new((base_address + 0x0C) as *mut FrameIndex),
            ctrl_ds_segment_base_addr: Volatile::new((base_address + 0x10) as *mut u32),
            configflag: Volatile::new((base_address + 0x14) as *mut ConfigFlag),
            portsc1: PortStatusControl::new(base_address + 0x44), // Port 1 için, diğer portlar için ayarlanmalı
            hccparams: Volatile::new((base_address + 0x040) as *mut HccParams), // HCCPARAMS offset'i varsayıldı
            base_address, // Base adres bilgisini sakla
            // ... diğer registerlar için Volatile yapıları oluşturulacak ...
        }
    }
}


#[repr(C)]
struct PortStatusControl {
    address: usize,
}

impl PortStatusControl {
    fn new(address: usize) -> Self {
        PortStatusControl { address }
    }

    unsafe fn read(&self) -> PortStatusControlFlags {
        Volatile::<PortStatusControlFlags>::new(self.address as *mut PortStatusControlFlags).read()
    }

    unsafe fn write(&self, value: PortStatusControlFlags) {
        Volatile::<PortStatusControlFlags>::new(self.address as *mut PortStatusControlFlags).write(value);
    }
    unsafe fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut PortStatusControlFlags),
    {
        Volatile::<PortStatusControlFlags>::new(self.address as *mut PortStatusControlFlags).modify(f);
    }

    fn speed(&self) -> PortSpeed {
        let flags = unsafe { self.read() };
        if flags.contains(PortStatusControlFlags::HIGH_SPEED) {
            PortSpeed::HighSpeed
        } else if flags.contains(PortStatusControlFlags::FULL_SPEED) {
            PortSpeed::FullSpeed
        } else if flags.contains(PortStatusControlFlags::LOW_SPEED) {
            PortSpeed::LowSpeed
        } else {
            PortSpeed::Unknown
        }
    }
}

#[derive(Debug)]
enum PortSpeed {
    LowSpeed,
    FullSpeed,
    HighSpeed,
    Unknown,
}


// --- Register Bit Alanları ve Bayraklar ---

bitflags! {
    #[repr(transparent)] // Register yapısının bellek düzenini kontrol etmek için
    pub struct UsbCommand: u32 {
        const RUN_STOP                   = 1 << 0;   // RS
        const HOST_CONTROLLER_RESET      = 1 << 1;   // HCR
        const FRAME_LIST_SIZE_MASK       = 3 << 2;   // FLS (00b: 1K, 01b: 2K, 10b: 4K, 11b: Reserved)
        const PCI_CONFIG_RETRY_ENABLE    = 1 << 4;   // PRE
        const IOCE                       = 1 << 5;   // Interrupt On Complete Enabled
        const ISOCHRONOUS_ENABLE         = 1 << 6;   // IE
        const DOORBELL_ENABLE            = 1 << 7;   // DBE
        const LIGHT_RETRY_ENABLE         = 1 << 8;   // LRT
        const PARK_MODE_ENABLE_MASK      = 3 << 9;   // PME (Park mode values)
        const ENABLE_64BIT_ADDR_CAP      = 1 << 11;  // E64B
    }
}

bitflags! {
    #[repr(transparent)]
    pub struct UsbStatus: u32 {
        const USB_INTERRUPT              = 1 << 0;   // USBINT
        const ERROR_INTERRUPT            = 1 << 1;   // EINT
        const PORT_CHANGE_DETECT         = 1 << 2;   // PCI
        const FATAL_ERROR              = 1 << 3;   // FATAL
        const SYSTEM_ERROR               = 1 << 4;   // SYSERR
        const HOST_CONTROLLER_HALTED   = 1 << 12;  // HCH
        const RESERVED                   = 1 << 13;
        const INTERRUPT_ON_COMPLETION    = 1 << 14;  // IOC
        const ISOCHRONOUS_SCHED_STATUS = 1 << 15;  // ISS
        const DOORBELL_STATUS          = 1 << 16;  // DS
    }
}

bitflags! {
    #[repr(transparent)]
    pub struct UsbInterrupt: u32 {
        const USB_INTERRUPT_ENABLE       = 1 << 0;   // USBIE
        const ERROR_INTERRUPT_ENABLE     = 1 << 1;   // EIE
        const PORT_CHANGE_DETECT_ENABLE  = 1 << 2;   // PCIE
        const FATAL_ERROR_ENABLE         = 1 << 3;   // FAE
        const SYSTEM_ERROR_ENABLE        = 1 << 4;   // SYSE
        const INTERRUPT_ON_COMPLETION_ENABLE = 1 << 14; // IOCE
        const ISOCHRONOUS_SCHED_STATUS_ENABLE = 1 << 15; // ISSE
        const DOORBELL_STATUS_ENABLE     = 1 << 16; // DSE
    }
}

bitflags! {
    #[repr(transparent)]
    pub struct FrameIndex: u32 {
        const FRAME_INDEX_MASK           = 0x3FFF; // 14-bit frame index mask
    }
}


bitflags! {
    #[repr(transparent)]
    pub struct ConfigFlag: u32 {
        const CF                         = 1 << 0;   // Configuration Flag
    }
}


bitflags! {
    #[repr(transparent)]
    pub struct PortStatusControlFlags: u32 {
        const CONNECT_STATUS             = 1 << 0;   // CCS - Current Connect Status
        const PORT_ENABLE                = 1 << 1;   // PES - Port Enabled/Disabled Status
        const PORT_SUSPEND               = 1 << 2;   // SUSP - Suspend
        const OVER_CURRENT_ACTIVE        = 1 << 3;   // OCA - Over-current Active
        const PORT_RESET                 = 1 << 4;   // PRS - Port Reset
        const PORT_POWER                 = 1 << 8;   // PP - Port Power
        const LINE_STATUS_MASK           = 3 << 9;   // Line Status (LS)
        const PORT_TEST_CONTROL_MASK     = 0xF << 11; // Test Mode (TM)
        const PORT_INDICATOR_CONTROL_MASK = 3 << 15; // Port Indicator Control (PIC)
        const CONNECT_STATUS_CHANGE      = 1 << 16;  // CSC - Connect Status Change
        const PORT_ENABLE_CHANGE         = 1 << 17;  // PESC- Port Enable Status Change
        const PORT_SUSPEND_CHANGE        = 1 << 18;  // PSSC - Port Suspend Change
        const OVER_CURRENT_CHANGE        = 1 << 19;  // OCC - Over-current Change
        const PORT_RESET_CHANGE          = 1 << 20;  // PRSC - Port Reset Change
        const PORT_POWER_CONTROL_MASK    = 1 << 21;  // Port Power Control (PPC) - RW - EHCI 1.0 Feature
        const HIGH_SPEED               = 1 << 26;  // Port Speed: High-speed
        const FULL_SPEED               = 0 << 26;  // Port Speed: Full-speed (değer 0 olmalı, kontrol için bit maskesi yok)
        const LOW_SPEED                = 0 << 27;  // Port Speed: Low-speed (değer 0 olmalı, kontrol için bit maskesi yok)

    }
}

bitflags! {
    #[repr(transparent)]
    pub struct HccParams: u32 {
        const PORT_COUNT_MASK = 0xFF; // Port Count (Port Sayısı) - İlk 8 bit
    }
}

impl HccParams {
    pub fn port_count(&self) -> u8 {
        (self.bits() & HccParams::PORT_COUNT_MASK.bits()) as u8
    }
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
    Ota = 9,
    Debug = 10,
    BOS = 0x0F, // Binary Device Object Store
    Report = 0x22,
    PhysicalDevice = 0x23,
}


// --- Yardımcı Fonksiyon: String Yazdırma ---
// Bu fonksiyon gerçek bir çekirdek ortamında VGA metin modu, seri port veya benzeri bir mekanizma
// kullanarak çıktı vermeyi sağlamalıdır. Şu anda sadece bir yer tutucu.
fn print_string(s: &str) {
    // Gerçek çekirdek çıktısı implementasyonu buraya gelecek.
    // Örneğin, VGA metin moduna karakter yazdırma veya seri porttan gönderme.
    // Şu an için sadece simülasyon çıktısı:
    static mut OUTPUT_BUFFER: [u8; 1024] = [0; 1024];
    static mut OUTPUT_INDEX: usize = 0;

    for byte in s.bytes() {
        unsafe {
            if OUTPUT_INDEX < OUTPUT_BUFFER.len() {
                OUTPUT_BUFFER[OUTPUT_INDEX] = byte;
                OUTPUT_INDEX += 1;
            }
        }
    }
    let output_str = unsafe { core::str::from_utf8_unchecked(&OUTPUT_BUFFER[0..OUTPUT_INDEX]) };
    // Şu anlık sadece konsola yazdır (çekirdek ortamında bu çalışmaz)
    // Web ortamında veya simülatörde çıktıyı görmek için
    println!("{}", output_str);


}