#![no_std]

use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// LoongArch için İstisna Sebepleri (MIPS benzeri, LoongArch Mimari Referansından uyarlanmıştır)
#[repr(u32)]
enum ExceptionCause {
    Interrupt = 0,          // Kesme
    TLBModification = 1,      // TLB Değişikliği
    TLBLoadMiss = 2,          // TLB Yükleme Kaçırma
    TLBStoreMiss = 3,         // TLB Saklama Kaçırma
    AddressErrorLoad = 4,     // Adres Hatası (Yükleme)
    AddressErrorStore = 5,    // Adres Hatası (Saklama)
    BusErrorInstructionFetch = 6, // Veriyolu Hatası (Talimat Getirme)
    BusErrorDataLoadStore = 7, // Veriyolu Hatası (Veri Yükleme/Saklama)
    Syscall = 8,              // Sistem Çağrısı
    Breakpoint = 9,           // Kesme Noktası
    ReservedInstruction = 10, // Ayrılmış Komut
    CoprocessorUnusable = 11,  // Yardımcı İşlemci Kullanılamaz
    Overflow = 12,            // Taşma
    Trap = 13,                // Tuzak (Trap)
    VirtualInstruction = 14,  // Sanal Talimat
    FloatingPointException = 15, // Kayan Nokta İstisnası
    Watchpoint = 23,           // İzleme Noktası
    MachineCheck = 30,         // Makine Kontrolü
    Unknown = 0xFFFF,         // Bilinmeyen Sebep
}

impl fmt::Display for ExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionCause::Interrupt => write!(f, "Kesme"),
            ExceptionCause::TLBModification => write!(f, "TLB Değişikliği"),
            ExceptionCause::TLBLoadMiss => write!(f, "TLB Yükleme Kaçırma"),
            ExceptionCause::TLBStoreMiss => write!(f, "TLB Saklama Kaçırma"),
            ExceptionCause::AddressErrorLoad => write!(f, "Adres Hatası (Yükleme)"),
            ExceptionCause::AddressErrorStore => write!(f, "Adres Hatası (Saklama)"),
            ExceptionCause::BusErrorInstructionFetch => write!(f, "Veriyolu Hatası (Talimat Getirme)"),
            ExceptionCause::BusErrorDataLoadStore => write!(f, "Veriyolu Hatası (Veri Yükleme/Saklama)"),
            ExceptionCause::Syscall => write!(f, "Sistem Çağrısı"),
            ExceptionCause::Breakpoint => write!(f, "Kesme Noktası"),
            ExceptionCause::ReservedInstruction => write!(f, "Ayrılmış Komut"),
            ExceptionCause::CoprocessorUnusable => write!(f, "Yardımcı İşlemci Kullanılamaz"),
            ExceptionCause::Overflow => write!(f, "Taşma"),
            ExceptionCause::Trap => write!(f, "Tuzak"),
            ExceptionCause::VirtualInstruction => write!(f, "Sanal Talimat"),
            ExceptionCause::FloatingPointException => write!(f, "Kayan Nokta İstisnası"),
            ExceptionCause::Watchpoint => write!(f, "İzleme Noktası"),
            ExceptionCause::MachineCheck => write!(f, "Makine Kontrolü"),
            ExceptionCause::Unknown => write!(f, "Bilinmeyen Sebep"),
        }
    }
}


// ExceptionContext yapısı (LoongArch için)
#[derive(Debug)]
#[repr(C)]
pub struct ExceptionContext {
    pub pc: u64,       // Program Sayacı (İstisna Adresi)
    pub badvaddr: u64, // Kötü Adres (Adres Hatası Durumunda)
    pub status: u64,   // İşlemci Durum Kaydı (cp0_status) - Gerekirse daha detaylı alt kayıtlar eklenebilir
    pub cause: u32,    // İstisna Sebebi (cp0_cause register'dan alınacak)
    // ... Gerekirse diğer kayıtlar eklenebilir (örn. genel amaçlı kayıtlar belli bir alt kümesi)
}


// Basit IO modülü (UART simülasyonu - adresler örnek)
mod io {
    use core::fmt::Write;

    // Örnek MMIO adresleri (LoongArch mimarisine uygun adresler kontrol edilmeli/ayarlanmalı)
    const UART_DATA: u32 = 0x10000000; // Örnek UART Veri Adresi
    const UART_STATUS: u32 = 0x10000004; // Örnek UART Durum Adresi

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu - UART veri adresine byte yazma
                    core::ptr::write_volatile(UART_DATA as *mut u8, byte);
                    // **Gerçek UART kontrolü için durum kaydını kontrol etme/bekleme gerekebilir.**
                }
            }
            Ok(())
        }
    }

    pub fn println(s: &str) {
        let mut stdout = Stdout;
        core::fmt::write!(&mut stdout, "{}\n", s).unwrap();
    }
}


#[no_mangle]
extern "C" fn exception_handler(context: &ExceptionContext) {
    let cause = match context.cause {
        0..=31 => unsafe { core::mem::transmute(context.cause) }, // LoongArch cause kodları genellikle 0-31 aralığında
        _ => ExceptionCause::Unknown,
    };

    io::println!("İSTISNA OLUSTU (LoongArch)!");
    io::println!("PC: {:#x}", context.pc);
    io::println!("BadVAddr (Hatalı Adres): {:#x}", context.badvaddr); // Eğer adres hatası ise geçerli olabilir
    io::println!("Sebep: {}", cause);
    io::println!("Status Register (CP0_Status): {:#x}", context.status);


    // Örnek olarak bazı genel amaçlı kayıtları (gerekirse LoongArch'e özgü kayıt isimlerini kullanın) yazdırabilirsiniz.
    // **LoongArch için doğru register isimlerini ve assembly sözdizimini kontrol edin!**
    let r0; // zero (her zaman 0 olmalı)
    let r1; // ra (dönüş adresi)
    let r2; // sp (stack pointer)
    let r3; // gp (global pointer)
    let r4; // tp (thread pointer)
    let r5; // t0
    let r6; // t1
    let r7; // t2

    unsafe {
        asm!("move {}, $r0", out(reg) r0); // zero register
        asm!("move {}, $r1", out(reg) r1); // ra
        asm!("move {}, $r2", out(reg) r2); // sp
        asm!("move {}, $r3", out(reg) r3); // gp
        asm!("move {}, $r4", out(reg) r4); // tp
        asm!("move {}, $r5", out(reg) r5); // t0
        asm!("move {}, $r6", out(reg) r6); // t1
        asm!("move {}, $r7", out(reg) r7); // t2
    }

    io::println!("Kayitlar:");
    io::println!("  r0 (zero): {:#x}", r0); // Her zaman 0 olmalı
    io::println!("  r1 (ra):   {:#x}", r1);
    io::println!("  r2 (sp):   {:#x}", r2);
    io::println!("  r3 (gp):   {:#x}", r3);
    io::println!("  r4 (tp):   {:#x}", r4);
    io::println!("  r5 (t0):   {:#x}", r5);
    io::println!("  r6 (t1):   {:#x}", r6);
    io::println!("  r7 (t2):   {:#x}", r7);


    panic!("İstisna İşlenemedi (LoongArch): {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC! (LoongArch)");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satir: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngü
}


pub fn init() {
    // LoongArch için istisna vektör adresi kaydını (TVEC register - Control and Status Register) ayarlayın.
    // **LoongArch mimarisine göre doğru CSR ismini ve yazma yöntemini kontrol edin.**
    unsafe {
        // ** CSR_TVEC register isminin doğru olup olmadığını ve LoongArch assembly sözdizimini doğrulayın **
        asm!("csrwr tvec, {}", in(reg) exception_handler as u64); // Örnek CSR yazma komutu - Doğrulanmalı!
    }

    io::println!("TVEC ayarlandi ve istisna isleyicisi kuruldu. (LoongArch)");
}