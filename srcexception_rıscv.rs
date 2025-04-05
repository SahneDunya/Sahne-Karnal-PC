#![no_std]
use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// İstisna Sebepleri (RISC-V spesifikasyonundan)
#[repr(u32)]
enum ExceptionCause {
    InstructionAddressMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddressMisaligned = 6,
    StoreAccessFault = 7,
    EnvironmentCallFromUMode = 8,
    EnvironmentCallFromSMode = 9,
    EnvironmentCallFromMMode = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    Unknown = 0xFFFF, // Bilinmeyen sebepler için
}

impl fmt::Display for ExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionCause::InstructionAddressMisaligned => write!(f, "Talimat adresi hizalama hatası"),
            ExceptionCause::InstructionAccessFault => write!(f, "Talimat erişim hatası"),
            ExceptionCause::IllegalInstruction => write!(f, "Geçersiz komut"),
            ExceptionCause::Breakpoint => write!(f, "Kesme noktası"),
            ExceptionCause::LoadAddressMisaligned => write!(f, "Yükleme adresi hizalama hatası"),
            ExceptionCause::LoadAccessFault => write!(f, "Yükleme erişim hatası"),
            ExceptionCause::StoreAddressMisaligned => write!(f, "Saklama adresi hizalama hatası"),
            ExceptionCause::StoreAccessFault => write!(f, "Saklama erişim hatası"),
            ExceptionCause::EnvironmentCallFromUMode => write!(f, "Kullanıcı modundan ortam çağrısı"),
            ExceptionCause::EnvironmentCallFromSMode => write!(f, "Süpervizör modundan ortam çağrısı"),
            ExceptionCause::EnvironmentCallFromMMode => write!(f, "Makine modundan ortam çağrısı"),
            ExceptionCause::InstructionPageFault => write!(f, "Talimat sayfası hatası"),
            ExceptionCause::LoadPageFault => write!(f, "Yükleme sayfası hatası"),
            ExceptionCause::StorePageFault => write!(f, "Saklama sayfası hatası"),
            ExceptionCause::Unknown => write!(f, "Bilinmeyen sebep"),
        }
    }
}

// ExceptionContext yapısı
#[derive(Debug)] // Debug trait'ini uygular
#[repr(C)]
pub struct ExceptionContext {
    pub epc: u64,   // İstisna program sayacı
    pub tval: u64,  // Hatalı adres (eğer varsa)
    pub cause: u32, // İstisna sebebi
    // ... diğer kayıtlar (gerekirse)
}

// println! makrosu için basit bir uygulama (gerçek bir ortamda daha gelişmiş bir şey kullanın)
mod io {
    use core::fmt::Write;

    // MMIO adresleri (örnek olarak)
    const UART_DATA: u32 = 0x10000000;
    const UART_STATUS: u32 = 0x10000004;

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu (gerçekte daha karmaşık olabilir)
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
    let cause = match context.cause {
        0..=15 => unsafe { core::mem::transmute(context.cause) },
        _ => ExceptionCause::Unknown,
    };

    io::println!("İstisna Oluştu!");
    io::println!("EPC: {:#x}", context.epc);
    io::println!("tval (Hatalı Adres): {:#x}", context.tval);
    io::println!("Sebep: {}", cause);

    // Bazı genel amaçlı kayıtların değerlerini al ve yazdır (örnek olarak)
    let ra;
    let sp;
    let gp;
    let tp;
    let t0;
    let t1;
    let t2;
    unsafe {
        asm!("mv {}, ra", out(reg) ra);
        asm!("mv {}, sp", out(reg) sp);
        asm!("mv {}, gp", out(reg) gp);
        asm!("mv {}, tp", out(reg) tp);
        asm!("mv {}, t0", out(reg) t0);
        asm!("mv {}, t1", out(reg) t1);
        asm!("mv {}, t2", out(reg) t2);
    }

    io::println!("Kayıtlar:");
    io::println!("  ra (x1): {:#x}", ra);
    io::println!("  sp (x2): {:#x}", sp);
    io::println!("  gp (x3): {:#x}", gp);
    io::println!("  tp (x4): {:#x}", tp);
    io::println!("  t0 (x5): {:#x}", t0);
    io::println!("  t1 (x6): {:#x}", t1);
    io::println!("  t2 (x7): {:#x}", t2);
    // io::println!("Bağlam: {:?}", context); // İsteğe bağlı olarak bağlamın tamamını da yazdırabilirsiniz

    panic!("İstisna İşlenemedi: {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC!");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satır: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngüde kal
}

pub fn init() {
    // stvec (İstisna Vektör Adresi Kaydı) ayarı
    // Bu kayıt, bir istisna (exception) veya kesme (interrupt) oluştuğunda
    // işlemcinin hangi adrese atlaması gerektiğini belirtir.
    // Aşağıdaki satır, istisna handler fonksiyonumuzun adresini stvec'e yazarak,
    // istisna oluştuğunda 'exception_handler' fonksiyonunun çağrılmasını sağlar.
    unsafe {
        asm!("csrw stvec, {}", in(reg) exception_handler as u64);
    }
    // RISC-V stvec için iki temel mod destekler: Direkt ve Vektörel mod.
    // Varsayılan olarak Direkt mod kullanılır. Bu modda, tüm istisnalar aynı adrese (stvec'e yazılan adres) atlar.
    // Vektörel modda ise, istisna sebebine göre farklı adreslere atlama yapılabilir.
    // Bu örnekte Direkt mod kullanılıyor ve tüm istisnalar 'exception_handler' fonksiyonuna yönlendiriliyor.

    io::println!("stvec ayarlandı ve istisna işleyicisi kuruldu."); // Başarılı kurulum onayı
}