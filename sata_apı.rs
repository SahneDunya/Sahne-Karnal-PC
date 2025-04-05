#![no_std] // Standart kütüphaneye bağımlılığı kaldırır (çekirdek seviyesi için yaygın)

// SATA kontrolcüsünün temel adresleri (gerçek değerler donanıma özeldir)
const AHCI_BASE: usize = 0xF0000000; // Örnek temel adres
const AHCI_PORTS_OFFSET: usize = 0x100; // Port yapıları için ofset

// Port yapısı (AHCI spesifikasyonuna göre basitleştirilmiştir)
#[repr(C)]
struct Port {
    clb: u32,    // Command List Base Address (alt 32 bit)
    clbu: u32,   // Command List Base Address Upper 32 bits
    fb: u32,     // FIS Base Address (alt 32 bit)
    fbu: u32,    // FIS Base Address Upper 32 bits
    is: u32,     // Interrupt Status
    ie: u32,     // Interrupt Enable
    cmd: u32,    // Command and Status
    rsv0: u32,
    tfd: u32,    // Task File Data
    sig: u32,    // Signature
    ssts: u32,   // SATA Status (speed, etc.)
    sctl: u32,   // SATA Control
    serr: u32,   // SATA Error
    sact: u32,   // SATA Active
    ci: u32,     // Command Issued
    sntf: u32,   // SATA Notification
    fbs: u32,    // FIS Based Switching Control
    rsv1: [u32; 9],
    vendor: [u8; 96], // Vendor specific
}

// AHCI kontrolcüsü yapısı (basitleştirilmiştir)
#[repr(C)]
struct AhciController {
    cap: u32,    // Host Capabilities
    ghc: u32,    // Global Host Control
    is: u32,     // Interrupt Status
    pi: u32,     // Ports Implemented
    vs: u32,     // Version
    ccc_ctl: u32,
    ccc_ports: u32,
    em_loc: u32,
    em_ctl: u32,
    cap2: u32,
    bohc: u32,
    rsv: [u8; 84],
    ports: [Port; 32], // Maksimum 32 port varsayımı
}

// Komut listesi yapısı (port başına)
#[repr(C, align(128))] // 128 byte hizalama zorunlu
struct CommandList {
    rsv0: [u64; 4], // Ayrılmış alan
    cfis: [u8; 64], // Command FIS (Frame Information Structure)
    acmd: [CommandTableEntry; 32], // Command Table Entries
    rsv1: [u8; 96], // Ayrılmış alan
}

// Komut tablosu girdisi yapısı
#[repr(C)]
struct CommandTableEntry {
    prdtl: u16, // Physical Region Descriptor Table Length (girdiler halinde)
    rsv0: u16,
    flags: u32, // Komut bayrakları (örneğin, Write FUA)
    rsv1: [u32; 4],
    prdt: [PhysicalRegionDescriptor; 1], // Fiziksel Bölge Tanımlayıcı Tablosu (örnekte sadece bir girdi)
}

// Fiziksel bölge tanımlayıcı yapısı
#[repr(C)]
struct PhysicalRegionDescriptor {
    dba: u64, // Data Base Address
    dbc: u32, // Data Byte Count
    rsv0: u32,
}

// FIS yapısı (Frame Information Structure)
#[repr(C)]
union Fis {
    d2h: D2HFis, // Device to Host FIS
    h2d: H2DFis, // Host to Device FIS
    pio: PioFis, // PIO Setup FIS
    sdb: SdbFis, // Set Device Bits FIS
    rsv: [u8; 64],
}

// Device to Host FIS yapısı
#[repr(C)]
struct D2HFis {
    fis_type: u8, // 0x34
    pm_port: u8,
    rsv0: u8,
    interrupt: u8,
    status: u8,
    rsv1: [u8; 7],
    command: u8,
    feature: u8,
    lba_low: u8,
    lba_mid: u8,
    lba_high: u8,
    device: u8,
    lba_low_exp: u8,
    lba_mid_exp: u8,
    lba_high_exp: u8,
    feature_exp: u8,
    count: u8,
    rsv2: u8,
    count_exp: u8,
    rsv3: u8,
    error: u8,
    rsv4: [u8; 20],
}

// Host to Device FIS yapısı
#[repr(C)]
struct H2DFis {
    fis_type: u8, // 0x27
    pm_port: u8,
    rsv0: u8,
    control: u8,
    command: u8,
    feature_low: u8,
    feature_high: u8,
    lba0: u8,
    lba1: u8,
    lba2: u8,
    device: u8,
    lba3: u8,
    lba4: u8,
    lba5: u8,
    feature_low_exp: u8,
    feature_high_exp: u8,
    rsv1: u8,
    rsv2: u8,
    count: u8,
    rsv3: u8,
    count_exp: u8,
    rsv4: u8,
    icc: u8,
    rsv5: [u8; 20],
}

// PIO Setup FIS yapısı
#[repr(C)]
struct PioFis {
    fis_type: u8, // 0x5F
    pm_port: u8,
    rsv0: u8,
    interrupt: u8,
    status: u8,
    rsv1: [u8; 3],
    command: u8,
    feature: u8,
    lba_low: u8,
    lba_mid: u8,
    lba_high: u8,
    device: u8,
    lba_low_exp: u8,
    lba_mid_exp: u8,
    lba_high_exp: u8,
    feature_exp: u8,
    count: u8,
    rsv2: u8,
    count_exp: u8,
    rsv3: u8,
    error: u8,
    transfer_count: u16,
    rsv4: [u8; 18],
}

// Set Device Bits FIS yapısı
#[repr(C)]
struct SdbFis {
    fis_type: u8, // 0xA1
    pm_port: u8,
    rsv0: u8,
    control: u8,
    command: u8,
    feature_low: u8,
    feature_high: u8,
    lba0: u8,
    lba1: u8,
    lba2: u8,
    device: u8,
    lba3: u8,
    lba4: u8,
    lba5: u8,
    feature_low_exp: u8,
    feature_high_exp: u8,
    rsv1: u8,
    rsv2: u8,
    count: u8,
    rsv3: u8,
    count_exp: u8,
    rsv4: u8,
    icc: u8,
    rsv5: [u8; 20],
}

// SATA API fonksiyonları

/// SATA kontrolcüsünü başlatır.
///
/// # Güvensiz
///
/// Bu fonksiyon, doğrudan donanıma eriştiği için güvensizdir.
unsafe fn sata_init() {
    let ahci = &mut *(AHCI_BASE as *mut AhciController);

    // Global Host Control register'ını etkinleştir (eğer kapalıysa)
    if ahci.ghc & 0x1 == 0 {
        ahci.ghc |= 0x1;
        // Biraz bekleme eklenebilir (donanıma bağlı)
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
    }

    // Bağlı portları kontrol et
    let ports_implemented = ahci.pi;
    for i in 0..32 {
        if (ports_implemented >> i) & 1 == 1 {
            kprintln!("SATA Port {} bulundu.", i);
            init_port(i);
        }
    }
}

/// Belirli bir SATA portunu başlatır.
///
/// # Güvensiz
///
/// Bu fonksiyon, doğrudan donanıma eriştiği için güvensizdir.
unsafe fn init_port(port_index: usize) {
    let ahci = &mut *(AHCI_BASE as *mut AhciController);
    let port = &mut ahci.ports[port_index];

    // Port'u durdur (eğer çalışıyorsa)
    port.cmd &= !0x1; // Start bitini temizle
    port.cmd &= !0x10; // Command List Running bitini temizle
    while port.cmd & 0x8000 != 0 || port.cmd & 0x10 != 0 { // Bekle BSY ve CR biti temizlenene kadar
        core::hint::spin_loop();
    }

    // Komut listesi ve FIS yapıları için bellek ayır (bu örnekte basit bir adres kullanılıyor)
    // Gerçek bir işletim sisteminde, bellek yönetimi ile yapılmalıdır.
    let command_list_base = 0x40000 + (port_index * 0x1000); // Her port için 4KB ayır
    let fis_base = command_list_base + 0x80; // Komut listesinden sonra

    port.clb = (command_list_base as u64) as u32;
    port.clbu = ((command_list_base as u64) >> 32) as u32;
    port.fb = (fis_base as u64) as u32;
    port.fbu = ((fis_base as u64) >> 32) as u32;

    // Komut listesini temizle
    let cmd_list = &mut *(command_list_base as *mut CommandList);
    for byte in cmd_list as *mut CommandList as *mut u8 .. (command_list_base + core::mem::size_of::<CommandList>()) as *mut u8 {
        *byte = 0;
    }

    // FIS alanını temizle
    let fis = &mut *(fis_base as *mut Fis);
    for byte in fis as *mut Fis as *mut u8 .. (fis_base + core::mem::size_of::<Fis>()) as *mut u8 {
        *byte = 0;
    }

    // Port'u yeniden başlat
    port.cmd |= 0x10; // Command List Running bitini ayarla
    port.cmd |= 0x1;  // Start bitini ayarla
}

/// Bir SATA portuna komut gönderir.
///
/// # Parametreler
///
/// * `port_index`: Komutun gönderileceği portun indeksi.
/// * `command_fis`: Gönderilecek komutun FIS (Frame Information Structure) yapısı.
///
/// # Güvensiz
///
/// Bu fonksiyon, doğrudan donanıma eriştiği için güvensizdir.
unsafe fn sata_send_command(port_index: usize, command_fis: &[u8]) {
    let ahci = &mut *(AHCI_BASE as *mut AhciController);
    if port_index >= 32 || ((ahci.pi >> port_index) & 1) == 0 {
        kprintln!("Geçersiz veya bağlı olmayan SATA portu: {}", port_index);
        return;
    }

    let port = &mut ahci.ports[port_index];
    let command_list_base = 0x40000 + (port_index * 0x1000);
    let cmd_list = &mut *(command_list_base as *mut CommandList);

    // İlk komut tablosu girdisini al
    let command_table_entry = &mut cmd_list.acmd[0];
    command_table_entry.prdtl = 0; // Şu an veri transferi yok

    // Komut FIS'ini ayarla (IDENTIFY DEVICE)
    let h2d_fis = &mut cmd_list.cfis[0..core::mem::size_of::<H2DFis>()];
    h2d_fis.copy_from_slice(command_fis);

    // Komutu başlat
    port.ci = 1; // İlk komut yuvasını etkinleştir

    // Komutun tamamlanmasını bekle (polling)
    while port.ci & 1 != 0 {
        core::hint::spin_loop();
    }

    // Sonucu kontrol et (örneğin, Task File Data register'ı)
    kprintln!("SATA Port {} komut sonucu (TFD): 0x{:x}", port_index, port.tfd);
}

// Örnek bir FIS yapısı (IDENTIFY DEVICE komutu için)
// Host to Device FIS (Tip 0x27)
const IDENTIFY_DEVICE_FIS: &[u8] = &[
    0x27, // FIS Type: H2D
    0x80, // Port multiplier
    0x00, // Reserved
    0x00, // Control
    0xEC, // Command: IDENTIFY DEVICE
    0x00, // Feature (LBA Low)
    0x00, // Feature (LBA High)
    0x00, // LBA 0-7
    0x00, // LBA 8-15
    0x00, // LBA 16-23
    0x00, // Device (LBA 24-27, etc.)
    0x00, // LBA 24-31
    0x00, // LBA 32-39
    0x00, // LBA 40-47
    0x00, // Feature Extended (LBA Low Exp)
    0x00, // Feature Extended (LBA High Exp)
    0x00, // Reserved
    0x00, // Reserved
    0x00, // Count
    0x00, // Reserved
    0x00, // Count Extended
    0x00, // Reserved
    0x00, // ICC
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Basit bir çekirdek yazdırma makrosu (CustomOS'a özel olmalı)
macro_rules! kprintln {
    ($($arg:tt)*) => ({
        // Burada CustomOS'un konsol yazdırma mekanizması çağrılmalı
        // Örneğin: `custom_os::vga::println!($($arg)*);`
        let s = format_args!($($arg)*);
        // Güvensiz blok içinde bir şeyler yapılabilir (örneğin, doğrudan VGA belleğine yazma)
        unsafe {
            let vga_buffer = 0xb8000 as *mut u8;
            let mut offset = 0;
            for byte in s.to_string().bytes() {
                *vga_buffer.add(offset * 2) = byte;
                *vga_buffer.add(offset * 2 + 1) = 0x07; // Beyaz metin, siyah arka plan
                offset += 1;
            }
        }
    });
}

// Ana fonksiyon (çekirdek başlangıcında çağrılabilir)
#[no_mangle]
pub extern "C" fn kernel_main() {
    kprintln!("CustomOS SATA API örneği.");

    // SATA kontrolcüsünü başlat
    unsafe {
        sata_init();

        // İlk SATA portuna IDENTIFY DEVICE komutunu gönder
        sata_send_command(0, IDENTIFY_DEVICE_FIS);
    }

    loop {} // Sonsuz döngü (çekirdek çalışmaya devam eder)
}