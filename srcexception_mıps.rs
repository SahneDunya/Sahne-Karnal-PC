#![no_std]
use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// MIPS İstisna Sebepleri (MIPS mimarisine göre)
#[repr(u32)]
enum MIPSExceptionCause {
    Interrupt = 0,          // Kesme
    AddressErrorLoad = 4,     // Adres Hatası (Yükleme)
    AddressErrorStore = 5,    // Adres Hatası (Saklama)
    BusErrorInstruction = 6,  // Veriyolu Hatası (Talimat Getirme)
    BusErrorData = 7,         // Veriyolu Hatası (Veri Erişimi)
    Syscall = 8,            // Sistem Çağrısı
    Breakpoint = 9,         // Kesme Noktası
    ReservedInstruction = 10, // Ayrılmış Talimat
    CoprocessorUnusable = 11,// Yardımcı İşlemci Kullanılamaz
    Overflow = 12,          // Aritmetik Taşma
    Trap = 13,              // Tuzak Talimatı
    浮点数异常 = 15,       // Floating Point Exception (Türkçe karakterler düzeltildi)
    Unknown = 0xFFFF,       // Bilinmeyen sebepler için
}

impl fmt::Display for MIPSExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MIPSExceptionCause::Interrupt => write!(f, "Kesme"),
            MIPSExceptionCause::AddressErrorLoad => write!(f, "Adres Hatası (Yükleme)"),
            MIPSExceptionCause::AddressErrorStore => write!(f, "Adres Hatası (Saklama)"),
            MIPSExceptionCause::BusErrorInstruction => write!(f, "Veriyolu Hatası (Talimat Getirme)"),
            MIPSExceptionCause::BusErrorData => write!(f, "Veriyolu Hatası (Veri Erişimi)"),
            MIPSExceptionCause::Syscall => write!(f, "Sistem Çağrısı"),
            MIPSExceptionCause::Breakpoint => write!(f, "Kesme Noktası"),
            MIPSExceptionCause::ReservedInstruction => write!(f, "Ayrılmış Talimat"),
            MIPSExceptionCause::CoprocessorUnusable => write!(f, "Yardımcı İşlemci Kullanılamaz"),
            MIPSExceptionCause::Overflow => write!(f, "Aritmetik Taşma"),
            MIPSExceptionCause::Trap => write!(f, "Tuzak Talimatı"),
            MIPSExceptionCause::浮点数异常 => write!(f, "浮点数 İstisnası"), // Düzeltilmiş ve Türkçe'ye yakın anlam
            MIPSExceptionCause::Unknown => write!(f, "Bilinmeyen Sebep"),
        }
    }
}

// MIPS ExceptionContext yapısı
#[derive(Debug)] // Debug trait'ini uygular
#[repr(C)]
pub struct MIPSExceptionContext {
    pub epc: u32,   // İstisna Program Sayacı (Exception Program Counter)
    pub cause: u32, // İstisna Sebebi (Cause Register - COP0 Cause register'ın ExcCode alanı)
    pub badvaddr: u32, // Hatalı Sanal Adres (Bad Virtual Address Register)
    // ... diğer MIPS özel kayıtları eklenebilir (status, config, vb.)
}

// println! makrosu için basit bir uygulama (gerçek bir ortamda daha gelişmiş bir şey kullanın)
mod io {
    use core::fmt::Write;

    // MMIO adresleri (örnek olarak - MIPS için örnek UART adresleri)
    const UART_DATA: u32 = 0xBFD003F8; // Tipik Sadece-Yazma UART Veri Kaydı Adresi
    const UART_STATUS: u32 = 0xBFD003FC; // Örnek UART Durum Kaydı (Kullanılmıyor bu örnekte ama eklenebilir)

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu (MIPS için adaptasyon - adresler değişebilir)
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
extern "C" fn exception_handler(context: &MIPSExceptionContext) {
    let cause = match context.cause & 0x1F { // Cause register'ın ExcCode (5-bit exception code) alanını maskele
        0 => MIPSExceptionCause::Interrupt,
        4 => MIPSExceptionCause::AddressErrorLoad,
        5 => MIPSExceptionCause::AddressErrorStore,
        6 => MIPSExceptionCause::BusErrorInstruction,
        7 => MIPSExceptionCause::BusErrorData,
        8 => MIPSExceptionCause::Syscall,
        9 => MIPSExceptionCause::Breakpoint,
        10 => MIPSExceptionCause::ReservedInstruction,
        11 => MIPSExceptionCause::CoprocessorUnusable,
        12 => MIPSExceptionCause::Overflow,
        13 => MIPSExceptionCause::Trap,
        15 => MIPSExceptionCause::浮点数异常, // Floating Point Exception
        _ => MIPSExceptionCause::Unknown,
    };

    io::println!("İstisna Oluştu (MIPS)!");
    io::println!("EPC (Program Sayacı): {:#x}", context.epc);
    io::println!("Hatalı Sanal Adres (BadVAddr): {:#x}", context.badvaddr);
    io::println!("Sebep: {}", cause);

    // Bazı genel amaçlı MIPS kayıtlarını al ve yazdır (örnek olarak - isimler MIPS'e göre)
    let ra;  // $ra (return address) - x31
    let sp;  // $sp (stack pointer) - x29
    let gp;  // $gp (global pointer) - x28
    let fp;  // $fp (frame pointer) - x30, veya $s8
    let t0;  // $t0 (temporary register) - x8
    let t1;  // $t1 (temporary register) - x9
    let t2;  // $t2 (temporary register) - x10

    unsafe {
        asm!("move {}, $ra", out(reg) ra);
        asm!("move {}, $sp", out(reg) sp);
        asm!("move {}, $gp", out(reg) gp);
        asm!("move {}, $fp", out(reg) fp);
        asm!("move {}, $t0", out(reg) t0);
        asm!("move {}, $t1", out(reg) t1);
        asm!("move {}, $t2", out(reg) t2);
    }

    io::println!("Kayıtlar (MIPS):");
    io::println!("  $ra (x31): {:#x}", ra);
    io::println!("  $sp (x29): {:#x}", sp);
    io::println!("  $gp (x28): {:#x}", gp);
    io::println!("  $fp (x30 veya $s8): {:#x}", fp);
    io::println!("  $t0 (x8): {:#x}", t0);
    io::println!("  $t1 (x9): {:#x}", t1);
    io::println!("  $t2 (x10): {:#x}", t2);
    // io::println!("Bağlam: {:?}", context); // İsteğe bağlı olarak bağlamın tamamını da yazdırabilirsiniz

    panic!("MIPS İstisnası İşlenemedi: {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC! (MIPS)");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satır: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngüde kal
}

pub fn init() {
    // MIPS EBASE kaydının ayarlanması (Exception Base Register)
    // EBASE, istisna vektör taban adresini belirtir.
    // MIPS32 ve MIPS64'te istisna vektör adresinin nasıl belirlendiği farklılık gösterebilir.
    // Bu örnek basitleştirilmiş bir yaklaşım sunar ve EBASE'i exception_handler adresine ayarlar.
    // Gerçek bir sistemde daha detaylı istisna vektör tablosu kurulumu gerekebilir.

    unsafe {
        // MIPS'te COP0 registerlarına erişim için 'mtc0' (move to coprocessor 0) komutu kullanılır.
        // EBASE register'ı genellikle COP0 register'ıdır (kayıt numarası sisteme göre değişebilir).
        // Varsayımsal olarak COP0 register numarası ve EBASE offseti 12 (CP0 Register 12, select 0) olarak kabul edilmiştir.
        // BU DEĞERLER MIPS İŞLEMCİ SPECIFICASYONUNA GÖRE DOĞRULANMALIDIR VE DEĞİŞEBİLİR!

        // Örnek: mtc0 $t0, CP0_REGISTER_EBASE
        // $k0 veya $k1 gibi geçici bir kayıt kullanılabilir, burada $t0 kullanılmıştır.

        // **DİKKAT**: Aşağıdaki assembly kodu tamamen örnektir ve hedef MIPS işlemciye göre AYARLANMALIDIR.
        // Gerçek EBASE register numarası ve doğru assembly syntax'ı MIPS mimarisi referansına bakılarak belirlenmelidir.

        asm!("la $t0, {}",  // exception_handler adresini $t0'a yükle (load address)
             in(reg) exception_handler as u32, // Fonksiyon adresini register olarak geçir
             options(nostack)); // Stack operasyonu olmadığını belirt (gerekliyse)

        // mtc0 komutu ile $t0'daki değeri EBASE'e yaz (COP0 register 12, select 0 varsayımıyla)
        asm!("mtc0 $t0, $cp0r12", //  Move to coprocessor 0 register 12 (EBASE - varsayımsal)
             options(nostack));   // Stack operasyonu olmadığını belirt (gerekliyse)

        // ÖNEMLİ NOT: MIPS mimarisine ve kullanılan toolchain'e göre 'mtc0' syntax'ı ve register isimleri DEĞİŞEBİLİR.
        // MIPS mimarisi ve assembler dokümantasyonunuza başvurun.

    }

    io::println!("MIPS EBASE ayarlandı ve istisna işleyicisi kuruldu."); // Başarılı kurulum onayı
}