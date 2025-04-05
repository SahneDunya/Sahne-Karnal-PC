#![no_std]

use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// İstisna Sebepleri (OpenRISC 1000 mimarisi için)
#[repr(u32)]
enum ExceptionCause {
    Reset = 0x00,
    BusErrorInstructionFetch = 0x01,
    BusErrorDataLoad = 0x02,
    BusErrorDataStore = 0x03,
    IllegalInstruction = 0x04,
    PrivilegedInstruction = 0x05,
    Trap = 0x06,
    SystemCall = 0x07,
    FloatingPointException = 0x08,
    DataPageFault = 0x09,
    InstructionPageFault = 0x0A,
    TickTimer = 0x0B,
    AlignmentError = 0x0C,
    Unknown = 0xFFFF, // Bilinmeyen sebepler için
}

impl fmt::Display for ExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionCause::Reset => write!(f, "Sıfırlama"),
            ExceptionCause::BusErrorInstructionFetch => write!(f, "Veriyolu Hatası (Talimat Getirme)"),
            ExceptionCause::BusErrorDataLoad => write!(f, "Veriyolu Hatası (Veri Yükleme)"),
            ExceptionCause::BusErrorDataStore => write!(f, "Veriyolu Hatası (Veri Saklama)"),
            ExceptionCause::IllegalInstruction => write!(f, "Geçersiz Komut"),
            ExceptionCause::PrivilegedInstruction => write!(f, "Ayrıcalıklı Komut"),
            ExceptionCause::Trap => write!(f, "Tuzak"),
            ExceptionCause::SystemCall => write!(f, "Sistem Çağrısı"),
            ExceptionCause::FloatingPointException => write!(f, "Kayan Nokta İstisnası"),
            ExceptionCause::DataPageFault => write!(f, "Veri Sayfası Hatası"),
            ExceptionCause::InstructionPageFault => write!(f, "Talimat Sayfası Hatası"),
            ExceptionCause::TickTimer => write!(f, "Tick Zamanlayıcı"),
            ExceptionCause::AlignmentError => write!(f, "Hizalama Hatası"),
            ExceptionCause::Unknown => write!(f, "Bilinmeyen Sebep"),
        }
    }
}

// ExceptionContext yapısı (OpenRISC için)
#[derive(Debug)]
#[repr(C)]
pub struct ExceptionContext {
    pub pc: u32,        // Program Sayacı
    pub spr_ адреси: u32, // SPR'lerin adresi (gerektiğinde daha fazla SPR eklenebilir)
    pub vector: u32,    // İstisna Vektör Adresi
    pub epcr: u32,      // İstisna Program Sayacı Kaydı
    pub eear: u32,      // İstisna Etkin Adres Kaydı
    pub esr: u32,       // İstisna Durum Kaydı
    // ... diğer ilgili kayıtlar eklenebilir
}

// Basit çıktı mekanizması (UART MMIO adresleri örnek)
mod io {
    use core::fmt::Write;

    // Örnek MMIO adresleri (OpenRISC için tipik adresler kontrol edilmeli)
    const UART_BASE: u32 = 0x90000000; // Örnek UART temel adresi
    const UART_DATA: u32 = UART_BASE + 0x00; // Veri yazma/okuma adresi
    const UART_STATUS: u32 = UART_BASE + 0x04; // Durum kaydı adresi (gerekirse)

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu
                    core::ptr::write_volatile(UART_DATA as *mut u8, byte);
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
    let cause = match context.esr { // ESR (Exception Status Register) içindeki sebep kodunu kullan
        0x00 => ExceptionCause::Reset,
        0x01 => ExceptionCause::BusErrorInstructionFetch,
        0x02 => ExceptionCause::BusErrorDataLoad,
        0x03 => ExceptionCause::BusErrorDataStore,
        0x04 => ExceptionCause::IllegalInstruction,
        0x05 => ExceptionCause::PrivilegedInstruction,
        0x06 => ExceptionCause::Trap,
        0x07 => ExceptionCause::SystemCall,
        0x08 => ExceptionCause::FloatingPointException,
        0x09 => ExceptionCause::DataPageFault,
        0x0A => ExceptionCause::InstructionPageFault,
        0x0B => ExceptionCause::TickTimer,
        0x0C => ExceptionCause::AlignmentError,
        _ => ExceptionCause::Unknown,
    };

    io::println!("İstisna Oluştu (OpenRISC)!");
    io::println!("PC: {:#x}", context.pc);
    io::println!("EPCR: {:#x}", context.epcr);
    io::println!("EEAR (Hata Adresi): {:#x}", context.eear);
    io::println!("ESR (Sebep Kodu): {:#x}", context.esr);
    io::println!("Sebep: {}", cause);
    io::println!("Bağlam: {:?}", context); // Tüm bağlamı yazdır (isteğe bağlı)


    // Genel amaçlı kayıtları (örnek olarak r1-r7) alıp yazdırma (dikkatli olunmalı, bağlama bağlı)
    let r1;
    let r2;
    let r3;
    let r4;
    let r5;
    let r6;
    let r7;
    unsafe {
        asm!("l.mvz\t{}, r1", out(reg) r1);
        asm!("l.mvz\t{}, r2", out(reg) r2);
        asm!("l.mvz\t{}, r3", out(reg) r3);
        asm!("l.mvz\t{}, r4", out(reg) r4);
        asm!("l.mvz\t{}, r5", out(reg) r5);
        asm!("l.mvz\t{}, r6", out(reg) r6);
        asm!("l.mvz\t{}, r7", out(reg) r7);
    }

    io::println!("Kayıtlar (r1-r7 Örnek):");
    io::println!("  r1: {:#x}", r1);
    io::println!("  r2: {:#x}", r2);
    io::println!("  r3: {:#x}", r3);
    io::println!("  r4: {:#x}", r4);
    io::println!("  r5: {:#x}", r5);
    io::println!("  r6: {:#x}", r6);
    io::println!("  r7: {:#x}", r7);


    panic!("İstisna İşlenemedi (OpenRISC): {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC! (OpenRISC)");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satır: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngü
}

pub fn init() {
    // Vektör tablosunu ayarlayın (OpenRISC için uygun yöntemi kullanın, genellikle SPR'ler aracılığıyla)
    // OpenRISC'te vektör tabanı genellikle SPR'ler aracılığıyla ayarlanır (örneğin, EVBAR - Exception Vector Base Address Register).
    // Ancak, doğrudan Rust'tan SPR'lere erişim karmaşık olabilir ve donanım destek kütüphaneleri veya assembly gerekebilir.
    // Bu örnekte, vektör tablosunu ayarlamak için temel bir yaklaşım sergilenmektedir.
    // Gerçek uygulamada, OpenRISC mimarisine özgü başlatma yöntemlerini takip etmelisiniz.

    // Örnek: Vektör tablosu adresini doğrudan ayarlamak (bu gerçek OpenRISC donanımına göre değişebilir)
    unsafe {
        // **DİKKAT**: Bu satır OpenRISC için doğru olmayabilir.
        // Vektör tablosu kurulumu OpenRISC mimarisine ve kullanılan donanıma özgüdür.
        // Doğru yöntem için OpenRISC mimari referans kılavuzuna ve donanım belgelerine bakın.
        // Tipik olarak, bu işlem assembly kodu ve SPR'ler (Special Purpose Registers) kullanılarak yapılır.

        // Aşağıdaki örnek satır, YANLIŞ bir yöntem olabilir ve sadece kavramsal amaçlıdır.
        // Gerçekte SPR'ler üzerinden vektör tablosu adresi ayarlanmalıdır.
        // asm!("mtspr\t...", in(reg) exception_handler as u32); // YANLIŞ ÖRNEK! SPR adresi ve komut doğru DEĞİL!
        // Doğru SPR ve komutları OpenRISC mimarisi belgelerinden öğrenin.

        // **ÖNEMLİ NOT**: OpenRISC'te istisna vektör tablosu kurulumu RISC-V'den farklıdır ve
        // genellikle SPR'ler (Special Purpose Registers) kullanılarak yapılır.
        // Bu kısım, gerçek OpenRISC donanımında çalışacak şekilde UYARLANMALIDIR.
        // Bu örnek, sadece kavramsal bir başlangıç noktası sunmaktadır.
    }


    io::println!("OpenRISC İstisna işleyicisi başlatıldı (Vektör tablosu AYARLANMADI - UYARLANMALI!)");
    io::println!("Vektör tablosu kurulumu için OpenRISC mimarisi belgelerine başvurun.");
}