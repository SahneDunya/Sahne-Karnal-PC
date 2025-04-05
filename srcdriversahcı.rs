#![no_std]
#![feature(allocator_api)]

use core::{mem::size_of, slice};
use core::ptr::{read_volatile, write_volatile}; // Import for the Volatile struct methods

// AHCI Port Yapısı
#[repr(C, packed)]
struct AhciPort {
    clb: Volatile<u32>,     // Command List Base Address
    clbu: Volatile<u32>,    // Command List Base Address Upper 32 bits
    fb: Volatile<u32>,      // FIS Base Address
    fbu: Volatile<u32>,     // FIS Base Address Upper 32 bits
    is: Volatile<u32>,      // Interrupt Status
    ie: Volatile<u32>,      // Interrupt Enable
    cmd: Volatile<u32>,     // Command and Status
    rsv0: Volatile<u32>,    // Reserved
    tfd: Volatile<u32>,     // Task File Data
    sig: Volatile<u32>,     // Signature
    ssts: Volatile<u32>,    // SATA Status (SCR[x]SSTS)
    sctl: Volatile<u32>,    // SATA Control (SCR[x]SCTL)
    serr: Volatile<u32>,    // SATA Error (SCR[x]SERR)
    sact: Volatile<u32>,    // SATA Active (SCR[x]SACT)
    ci: Volatile<u32>,      // Command Issue
    sntf: Volatile<u32>,    // SATA Notification (SCR[x]SNTF)
    fbs: Volatile<u32>,     // FIS-Based Switching Control
    rsv1: [Volatile<u8>; 96], // Reserved
}

// AHCI Host Controller Yapısı
#[repr(C, packed)]
struct AhciController {
    cap: Volatile<u32>,     // Host Capabilities
    ghc: Volatile<u32>,     // Global Host Control
    is: Volatile<u32>,      // Interrupt Status
    pi: Volatile<u32>,      // Ports Implemented
    vs: Volatile<u32>,      // Version
    ccc_ctl: Volatile<u32>, // Command Completion Coalescing Control
    ccc_ports: Volatile<u32>,// Command Completion Coalescing Ports
    em_loc: Volatile<u32>,  // Enclosure Management Location
    em_ctl: Volatile<u32>,  // Enclosure Management Control
    cap2: Volatile<u32>,    // Host Capabilities Extended
    bohc: Volatile<u32>,    // BIOS/OS Handoff Control
    rsv: [Volatile<u8>; 116], // Reserved
    ports: [AhciPort; 32],   // Portlar
}

// Volatile wrapper (basitleştirilmiş) - Aynı tanımı kullan
#[repr(transparent)]
pub struct Volatile<T>(T);

impl<T> Volatile<T> {
    pub unsafe fn new(value: T) -> Self {
        Volatile(value)
    }

    pub fn read(&self) -> T {
        unsafe { read_volatile(&self.0) }
    }

    pub fn write(&self, value: T) {
        unsafe { write_volatile(&mut self.0, value) }
    }
}

// AHCI Base Address (Example address, adjust for your system)
const AHCI_BASE: usize = 0xF0000000; // Örnek temel adres (ilk örnekten)
const AHCI_PORTS_OFFSET: usize = 0x100; // Port yapıları için ofset (ilk örnekten)
const SECTOR_SIZE: usize = 512;

// Komut listesi yapısı (port başına) - İlk örnekten
#[repr(C, align(128))] // 128 byte hizalama zorunlu
struct CommandList {
    rsv0: [u64; 4], // Ayrılmış alan
    cfis: [u8; 64], // Command FIS (Frame Information Structure)
    acmd: [CommandTableEntry; 32], // Command Table Entries
    rsv1: [u8; 96], // Ayrılmış alan
}

// Komut tablosu girdisi yapısı - İlk örnekten
#[repr(C)]
struct CommandTableEntry {
    prdtl: u16, // Physical Region Descriptor Table Length (girdiler halinde)
    rsv0: u16,
    flags: u32, // Komut bayrakları (örneğin, Write FUA)
    rsv1: [u32; 4],
    prdt: [PhysicalRegionDescriptor; 16], // Fiziksel Bölge Tanımlayıcı Tablosu (örnekte en fazla 16 girdi)
}

// Fiziksel bölge tanımlayıcı yapısı - İlk örnekten
#[repr(C)]
struct PhysicalRegionDescriptor {
    dba: u64, // Data Base Address
    dbc: u32, // Data Byte Count
    rsv0: u32,
}

// FIS yapısı (Frame Information Structure) - İlk örnekten
#[repr(C)]
union Fis {
    d2h: D2HFis, // Device to Host FIS
    h2d: H2DFis, // Host to Device FIS
    pio: PioFis, // PIO Setup FIS
    sdb: SdbFis, // Set Device Bits FIS
    rsv: [u8; 64],
}

// Device to Host FIS yapısı - İlk örnekten
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

// Host to Device FIS yapısı - İlk örnekten
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

// PIO Setup FIS yapısı - İlk örnekten
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

// Set Device Bits FIS yapısı - İlk örnekten
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

// Basit bir çekirdek yazdırma makrosu (CustomOS'a özel olmalı) - İlk örnekten
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

// SATA API fonksiyonları - İlk örnekten

/// SATA kontrolcüsünü başlatır.
///
/// # Güvensiz
///
/// Bu fonksiyon, doğrudan donanıma eriştiği için güvensizdir.
unsafe fn sata_init() {
    let ahci = &mut *(AHCI_BASE as *mut AhciController);

    // Global Host Control register'ını etkinleştir (eğer kapalıysa)
    if ahci.ghc.read() & 0x1 == 0 {
        ahci.ghc.write(ahci.ghc.read() | 0x1);
        // Biraz bekleme eklenebilir (donanıma bağlı)
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
    }

    // Bağlı portları kontrol et
    let ports_implemented = ahci.pi.read();
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
    port.cmd.write(port.cmd.read() & !0x1); // Start bitini temizle
    port.cmd.write(port.cmd.read() & !0x10); // Command List Running bitini temizle
    while port.cmd.read() & 0x8000 != 0 || port.cmd.read() & 0x10 != 0 {
        // Bekle BSY ve CR biti temizlenene kadar
        core::hint::spin_loop();
    }

    // Komut listesi ve FIS yapıları için bellek ayır (bu örnekte basit bir adres kullanılıyor)
    // Gerçek bir işletim sisteminde, bellek yönetimi ile yapılmalıdır.
    let command_list_base = 0x40000 + (port_index * 0x1000); // Her port için 4KB ayır
    let fis_base = command_list_base + 0x80; // Komut listesinden sonra

    port.clb.write((command_list_base as u64) as u32);
    port.clbu.write(((command_list_base as u64) >> 32) as u32);
    port.fb.write((fis_base as u64) as u32);
    port.fbu.write(((fis_base as u64) >> 32) as u32);

    // Komut listesini temizle
    let cmd_list = &mut *(command_list_base as *mut CommandList);
    let cmd_list_ptr = cmd_list as *mut CommandList as *mut u8;
    let cmd_list_size = size_of::<CommandList>();
    for i in 0..cmd_list_size {
        *cmd_list_ptr.add(i) = 0;
    }

    // FIS alanını temizle
    let fis = &mut *(fis_base as *mut Fis);
    let fis_ptr = fis as *mut Fis as *mut u8;
    let fis_size = size_of::<Fis>();
    for i in 0..fis_size {
        *fis_ptr.add(i) = 0;
    }

    // Port'u yeniden başlat
    port.cmd.write(port.cmd.read() | 0x10); // Command List Running bitini ayarla
    port.cmd.write(port.cmd.read() | 0x1);  // Start bitini ayarla
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
    if port_index >= 32 || ((ahci.pi.read() >> port_index) & 1) == 0 {
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
    let h2d_fis = &mut cmd_list.cfis[0..size_of::<H2DFis>()];
    h2d_fis.copy_from_slice(command_fis);

    // Komutu başlat
    port.ci.write(1); // İlk komut yuvasını etkinleştir

    // Komutun tamamlanmasını bekle (polling)
    while port.ci.read() & 1 != 0 {
        core::hint::spin_loop();
    }

    // Sonucu kontrol et (örneğin, Task File Data register'ı)
    kprintln!("SATA Port {} komut sonucu (TFD): 0x{:x}", port_index, port.tfd.read());
}

/// Bir SATA portundan veri okur.
///
/// # Parametreler
///
/// * `port_index`: Verinin okunacağı portun indeksi.
/// * `lba`: Okunacak ilk sektörün LBA adresi.
/// * `sectors`: Okunacak sektör sayısı.
/// * `buffer`: Verinin yazılacağı arabellek.
///
/// # Güvensiz
///
/// Bu fonksiyon, doğrudan donanıma eriştiği için güvensizdir.
unsafe fn sata_read(port_index: usize, lba: u64, sectors: u16, buffer: &mut [u8]) -> Result<(), &'static str> {
    let ahci = &mut *(AHCI_BASE as *mut AhciController);
    if port_index >= 32 || ((ahci.pi.read() >> port_index) & 1) == 0 {
        return Err("Geçersiz veya bağlı olmayan SATA portu");
    }
    if buffer.len() < (sectors as usize) * SECTOR_SIZE {
        return Err("Arabellek çok küçük");
    }
    if sectors > 256 { // Örnek sınırlama, PRDT boyutuna göre ayarlanabilir
        return Err("Çok fazla sektör istendi");
    }

    let port = &mut ahci.ports[port_index];
    let command_list_base = 0x40000 + (port_index * 0x1000);
    let cmd_list = &mut *(command_list_base as *mut CommandList);

    // İlk komut tablosu girdisini al
    let command_table_entry = &mut cmd_list.acmd[0];
    command_table_entry.prdtl = 1; // 1 PRDT girişi kullanacağız

    // PRDT girişini yapılandır
    let prdt_entry = &mut command_table_entry.prdt[0];
    let buffer_ptr = buffer.as_mut_ptr() as u64;
    prdt_entry.dba = buffer_ptr;
    prdt_entry.dbc = (sectors as usize * SECTOR_SIZE) as u32;
    prdt_entry.rsv0 = 0;

    // Komut FIS'ini oluştur (READ DMA EXT)
    let h2d_fis_bytes = &mut cmd_list.cfis[0..size_of::<H2DFis>()];
    let fis = H2DFis { // Doğrudan H2DFis yapısını kullan
        fis_type: 0x27,
        pm_port: 0,
        rsv0: 0,
        control: 0,
        command: 0x25, // READ DMA EXT
        feature_low: 0,
        feature_high: 0,
        lba0: (lba & 0xFF) as u8,
        lba1: ((lba >> 8) & 0xFF) as u8,
        lba2: ((lba >> 16) & 0xFF) as u8,
        device: 0x40, // LBA mode, cihaz 0
        lba3: ((lba >> 24) & 0xFF) as u8,
        lba4: ((lba >> 32) & 0xFF) as u8,
        lba5: ((lba >> 40) & 0xFF) as u8,
        feature_low_exp: 0,
        feature_high_exp: 0,
        rsv1: 0,
        rsv2: 0,
        count: (sectors & 0xFF) as u8,
        rsv3: 0,
        count_exp: ((sectors >> 8) & 0xFF) as u8,
        rsv4: 0,
        icc: 0,
        rsv5: [0; 20],
    };
    let fis_bytes = unsafe { slice::from_raw_parts(&fis as *const H2DFis as *const u8, size_of::<H2DFis>()) };
    h2d_fis_bytes.copy_from_slice(fis_bytes);

    // Komut tablosu adresini ayarla
    let command_table_base = command_list_base + size_of::<CommandList>() - size_of::<[u8; 96]>; // acmd'nin başlangıcı
    command_table_entry.flags = (size_of::<CommandTableEntry>() as u32) & 0xFFFF; // PRDTL
    command_table_entry.rsv0 = 0;
    command_table_entry.flags |= 0 << 16; // Write FUA yok
    command_table_entry.prdtl = 1;

    // Komut tablosunun adresini ayarla
    let cmd_tbl_phys = command_table_base as u64;
    command_table_entry.prdt[0].dba = buffer_ptr;
    command_table_entry.prdt[0].dbau = 0;
    command_table_entry.prdt[0].dbc = (sectors as usize * SECTOR_SIZE) as u32 -1; // Byte count - 1

    // Komutu başlat
    port.ci.write(1); // İlk komut yuvasını etkinleştir

    // Komutun tamamlanmasını bekle (polling)
    let timeout = 10_000_000;
    for _ in 0..timeout {
        if port.ci.read() & 1 == 0 {
            // Komut tamamlandı, sonucu kontrol et
            if port.tfd.read() & 0x1 != 0 {
                return Err("Hata: Aygıt meşgul");
            }
            if port.tfd.read() & 0x8 != 0 {
                return Err("Hata: Veri Hatası");
            }
            if port.tfd.read() & 0x20 != 0 {
                return Err("Hata: Hazır Değil");
            }
            return Ok(());
        }
        core::hint::spin_loop();
    }

    Err("Zaman aşımı: Komut tamamlanmadı")
}

// Örnek bir FIS yapısı (IDENTIFY DEVICE komutu için) - İlk örnekten
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

// Ana fonksiyon (çekirdek başlangıcında çağrılabilir) - İlk örnekten
#[no_mangle]
pub extern "C" fn kernel_main() {
    kprintln!("CustomOS SATA API örneği.");

    // SATA kontrolcüsünü başlat
    unsafe {
        sata_init();

        // İlk SATA portuna IDENTIFY DEVICE komutunu gönder
        sata_send_command(0, IDENTIFY_DEVICE_FIS);

        // İlk SATA portundan veri okuma denemesi
        let mut read_buffer = [0u8; 4096]; // 8 sektörlük arabellek
        match sata_read(0, 0, 8, &mut read_buffer) {
            Ok(_) => {
                kprintln!("SATA Port 0'dan okuma başarılı!");
                // Okunan veriyi inceleyebilirsiniz
                for i in 0..512 {
                    if i % 16 == 0 {
                        if i != 0 { kprintln!(""); }
                        kprintln!("{:04x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                                 i,
                                 read_buffer[i], read_buffer[i + 1], read_buffer[i + 2], read_buffer[i + 3],
                                 read_buffer[i + 4], read_buffer[i + 5], read_buffer[i + 6], read_buffer[i + 7],
                                 read_buffer[i + 8], read_buffer[i + 9], read_buffer[i + 10], read_buffer[i + 11],
                                 read_buffer[i + 12], read_buffer[i + 13], read_buffer[i + 14], read_buffer[i + 15]);
                    }
                }
                kprintln!("");
            }
            Err(e) => {
                kprintln!("SATA Port 0'dan okuma hatası: {}", e);
            }
        }
    }

    loop {} // Sonsuz döngü (çekirdek çalışmaya devam eder)
}
```